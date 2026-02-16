// SPDX-License-Identifier: AGPL-3.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once};

/// Selector field types for different AST nodes
pub const SELECTOR_FIELDS: &[(&str, &str)] = &[
    ("VariableDeclaration", "functionSelector"),
    ("FunctionDefinition", "functionSelector"),
    ("EventDefinition", "eventSelector"),
    ("ErrorDefinition", "errorSelector"),
];

const PARSING_IGNORED_NODE_TYPES: &[&str] = &[
    "StructDefinition",
    "EnumDefinition",
    "PragmaDirective",
    "ImportDirective",
    "Block",
];

/// AST Node representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    pub node_type: String,
    pub name: String,
    pub selector: String,
}

impl AstNode {
    pub fn new(node_type: String, name: String, selector: String) -> Self {
        Self {
            node_type,
            name,
            selector,
        }
    }

    pub fn from_dict(node: &serde_json::Value) -> Option<Self> {
        let node_type = node.get("nodeType")?.as_str()?.to_string();
        let name = node
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        // Find the appropriate selector field
        let selector_field = SELECTOR_FIELDS
            .iter()
            .find(|(nt, _)| *nt == node_type)
            .map(|(_, field)| *field)?;

        let selector = node
            .get(selector_field)
            .and_then(|s| s.as_str())
            .map(|s| format!("0x{}", s))
            .unwrap_or_else(|| "0x".to_string());

        Some(Self {
            node_type,
            name,
            selector,
        })
    }
}

/// Contract mapping information
#[derive(Debug, Clone)]
pub struct ContractMappingInfo {
    pub contract_name: String,
    pub bytecode: Option<String>,
    pub nodes: HashMap<String, AstNode>,
}

impl ContractMappingInfo {
    pub fn new(contract_name: String) -> Self {
        Self {
            contract_name,
            bytecode: None,
            nodes: HashMap::new(),
        }
    }

    pub fn with_bytecode(mut self, bytecode: String) -> Self {
        self.bytecode = Some(bytecode);
        self
    }

    pub fn add_node(&mut self, node: AstNode) {
        // Don't overwrite if a node with the same selector already exists
        self.nodes.entry(node.selector.clone()).or_insert(node);
    }

    pub fn with_nodes(mut self, nodes: Vec<AstNode>) -> Self {
        for node in nodes {
            self.add_node(node);
        }
        self
    }

    pub fn get_node(&self, selector: &str) -> Option<&AstNode> {
        self.nodes.get(selector)
    }

    pub fn get_function_name(&self, selector: &str) -> Option<String> {
        self.get_node(selector).map(|node| node.name.clone())
    }
}

/// Explanation for debug output
#[derive(Debug)]
pub struct Explanation {
    enabled: bool,
    content: String,
}

impl Explanation {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            content: String::new(),
        }
    }

    pub fn add(&mut self, text: &str) {
        if self.enabled {
            self.content.push_str(text);
            self.content.push('\n');
        }
    }

    pub fn print(&self) {
        if self.enabled && !self.content.is_empty() {
            println!("{}", self.content);
        }
    }
}

impl Drop for Explanation {
    fn drop(&mut self) {
        self.print();
    }
}

/// Singleton for managing source file mappings and line number calculations
pub struct SourceFileMap {
    root: Mutex<String>,
    id_to_filepath: Mutex<HashMap<i32, String>>,
    line_offsets: Mutex<HashMap<String, Vec<usize>>>,
}

impl SourceFileMap {
    fn new() -> Self {
        Self {
            root: Mutex::new(String::new()),
            id_to_filepath: Mutex::new(HashMap::new()),
            line_offsets: Mutex::new(HashMap::new()),
        }
    }

    pub fn instance() -> &'static SourceFileMap {
        static mut INSTANCE: Option<SourceFileMap> = None;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                INSTANCE = Some(SourceFileMap::new());
            });
            INSTANCE.as_ref().unwrap()
        }
    }

    pub fn set_root(&self, root: &str) {
        let abspath = std::fs::canonicalize(root)
            .unwrap_or_else(|_| PathBuf::from(root))
            .to_string_lossy()
            .to_string();
        *self.root.lock().unwrap() = abspath;
    }

    pub fn get_root(&self) -> String {
        self.root.lock().unwrap().clone()
    }

    pub fn add_mapping(&self, file_id: i32, filepath: &str) {
        let root = self.get_root();
        let abspath = if Path::new(filepath).is_absolute() {
            filepath.to_string()
        } else {
            Path::new(&root)
                .join(filepath)
                .to_string_lossy()
                .to_string()
        };

        let mut map = self.id_to_filepath.lock().unwrap();
        if let Some(existing) = map.get(&file_id) {
            if existing != &abspath {
                eprintln!(
                    "source file id mapping conflict: file_id={} filepath={} existing={}",
                    file_id, filepath, existing
                );
            }
        }
        map.insert(file_id, abspath);
    }

    pub fn get_file_path(&self, file_id: i32) -> Option<String> {
        self.id_to_filepath.lock().unwrap().get(&file_id).cloned()
    }

    pub fn get_line_number(&self, filepath: &str, byte_offset: usize) -> Option<usize> {
        if byte_offset == 0 {
            return Some(1);
        }

        let mut line_offsets_map = self.line_offsets.lock().unwrap();
        let line_offsets = line_offsets_map
            .entry(filepath.to_string())
            .or_insert_with(|| self.index_lines(filepath).unwrap_or_default());

        if line_offsets.is_empty() {
            return None;
        }

        // Binary search to find the line number
        match line_offsets.binary_search(&byte_offset) {
            Ok(idx) => Some(idx + 1),
            Err(idx) => Some(idx),
        }
    }

    pub fn get_location(
        &self,
        file_id: i32,
        byte_offset: usize,
    ) -> (Option<String>, Option<usize>) {
        match self.get_file_path(file_id) {
            Some(file_path) => {
                let line_number = self.get_line_number(&file_path, byte_offset);
                (Some(file_path), line_number)
            }
            None => (None, None),
        }
    }

    fn index_lines(&self, filepath: &str) -> Result<Vec<usize>, std::io::Error> {
        let file = File::open(filepath)?;
        let reader = BufReader::new(file);
        let mut offsets = vec![0];
        let mut current_offset = 0;

        for line in reader.lines() {
            let line = line?;
            current_offset += line.len() + 1; // +1 for newline
            offsets.push(current_offset);
        }

        Ok(offsets)
    }
}

/// Placeholder tuple (start, end)
type Placeholder = (usize, usize);

/// Build output singleton
pub struct BuildOut {
    build_out_map: Mutex<Option<serde_json::Value>>,
    build_out_map_reverse: Mutex<HashMap<String, HashMap<String, serde_json::Value>>>,
    build_out_map_code: Mutex<HashMap<usize, Vec<CodeData>>>,
}

#[derive(Debug, Clone)]
struct CodeData {
    hexcode: String,
    placeholders: Vec<Placeholder>,
    contract_name: String,
    filename: String,
    source_map: String,
}

impl BuildOut {
    fn new() -> Self {
        Self {
            build_out_map: Mutex::new(None),
            build_out_map_reverse: Mutex::new(HashMap::new()),
            build_out_map_code: Mutex::new(HashMap::new()),
        }
    }

    pub fn instance() -> &'static BuildOut {
        static mut INSTANCE: Option<BuildOut> = None;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                INSTANCE = Some(BuildOut::new());
            });
            INSTANCE.as_ref().unwrap()
        }
    }

    pub fn set_build_out(&self, build_out: serde_json::Value) {
        *self.build_out_map.lock().unwrap() = Some(build_out);
        *self.build_out_map_reverse.lock().unwrap() = HashMap::new();
        *self.build_out_map_code.lock().unwrap() = HashMap::new();
    }

    pub fn get_placeholders(
        &self,
        deployed: &serde_json::Value,
    ) -> Result<Vec<Placeholder>, String> {
        let mut placeholders = Vec::new();

        // Process immutableReferences
        if let Some(immutables) = deployed.get("immutableReferences") {
            if let Some(obj) = immutables.as_object() {
                for links in obj.values() {
                    if let Some(arr) = links.as_array() {
                        for link in arr {
                            let start = link
                                .get("start")
                                .and_then(|v| v.as_u64())
                                .ok_or("Invalid start")?
                                as usize;
                            let length = link
                                .get("length")
                                .and_then(|v| v.as_u64())
                                .ok_or("Invalid length")?
                                as usize;
                            placeholders.push((start, start + length));
                        }
                    }
                }
            }
        }

        // Process linkReferences
        if let Some(link_refs) = deployed.get("linkReferences") {
            if let Some(obj) = link_refs.as_object() {
                for libs in obj.values() {
                    if let Some(lib_obj) = libs.as_object() {
                        for links in lib_obj.values() {
                            if let Some(arr) = links.as_array() {
                                for link in arr {
                                    let start = link
                                        .get("start")
                                        .and_then(|v| v.as_u64())
                                        .ok_or("Invalid start")?
                                        as usize;
                                    let length = link
                                        .get("length")
                                        .and_then(|v| v.as_u64())
                                        .ok_or("Invalid length")?
                                        as usize;
                                    placeholders.push((start, start + length));
                                }
                            }
                        }
                    }
                }
            }
        }

        placeholders.sort_by_key(|p| p.0);

        // Sanity check
        let mut last = 0;
        for (start, end) in &placeholders {
            if !(last <= *start && start < end) {
                return Err("Invalid placeholders".to_string());
            }
            last = *end;
        }

        // Check if last placeholder exceeds bytecode length
        if let Some(object) = deployed.get("object").and_then(|v| v.as_str()) {
            let bytecode_len = (object.len().saturating_sub(2)) / 2; // Remove "0x" and convert to bytes
            if last > bytecode_len {
                return Err("Invalid placeholders: exceed bytecode length".to_string());
            }
        }

        Ok(placeholders)
    }
}

/// Deploy address mapper
#[derive(Debug, Clone)]
pub struct DeployAddressMapper {
    deployed_contracts: HashMap<String, String>,
    // For backward compatibility with byte-based API
    byte_mappings: HashMap<Vec<u8>, String>,
}

impl DeployAddressMapper {
    pub fn new() -> Self {
        let mut mapper = Self {
            deployed_contracts: HashMap::new(),
            byte_mappings: HashMap::new(),
        };

        // Set up default mappings
        mapper.add_deployed_contract("0x7109709ecfa91a80626ff3989d68f67f5b1dd12d", "hevm");
        mapper.add_deployed_contract("0xf3993a62377bcd56ae39d773740a5390411e8bc9", "svm");
        mapper.add_deployed_contract("0x636f6e736f6c652e6c6f67", "console");

        mapper
    }

    pub fn add_deployed_contract(&mut self, address: &str, contract_name: &str) {
        self.deployed_contracts
            .insert(address.to_string(), contract_name.to_string());
    }

    pub fn get_deployed_contract(&self, address: &str) -> String {
        self.deployed_contracts
            .get(address)
            .cloned()
            .unwrap_or_else(|| address.to_string())
    }

    // Backward compatibility API with Vec<u8>
    pub fn add_mapping(&mut self, address: Vec<u8>, name: String) {
        self.byte_mappings.insert(address, name);
    }

    pub fn get_name(&self, address: &[u8]) -> Option<&String> {
        self.byte_mappings.get(address)
    }

    pub fn get_address(&self, name: &str) -> Option<&Vec<u8>> {
        self.byte_mappings
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(addr, _)| addr)
    }
}

impl Default for DeployAddressMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Mapper for contracts with AST parsing
pub struct Mapper {
    contracts: Mutex<HashMap<String, ContractMappingInfo>>,
    pub deploy_addresses: DeployAddressMapper,
}

impl Mapper {
    pub fn new() -> Self {
        Self {
            contracts: Mutex::new(HashMap::new()),
            deploy_addresses: DeployAddressMapper::new(),
        }
    }

    pub fn instance() -> &'static Mapper {
        static mut INSTANCE: Option<Mapper> = None;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                INSTANCE = Some(Mapper::new());
            });
            INSTANCE.as_ref().unwrap()
        }
    }

    // Backward compatibility: add_contract
    pub fn add_contract(&mut self, info: ContractMappingInfo) {
        let mut contracts = self.contracts.lock().unwrap();
        contracts.insert(info.contract_name.clone(), info);
    }

    // Backward compatibility: get_contract
    pub fn get_contract(&self, name: &str) -> Option<ContractMappingInfo> {
        self.get_by_name(name)
    }

    // Backward compatibility: contracts field access
    pub fn contracts(&self) -> std::sync::MutexGuard<'_, HashMap<String, ContractMappingInfo>> {
        self.contracts.lock().unwrap()
    }

    // Backward compatibility: get_function_name with contract name
    pub fn get_function_name(&self, contract: &str, selector: &str) -> Option<String> {
        self.get_by_name(contract)
            .and_then(|c| c.get_function_name(selector))
    }

    pub fn add_mapping(&self, mapping: ContractMappingInfo) -> Result<(), String> {
        let contract_name = mapping.contract_name.clone();
        let mut contracts = self.contracts.lock().unwrap();

        if contracts.contains_key(&contract_name) {
            return Err(format!("Contract {} already exists", contract_name));
        }

        contracts.insert(contract_name, mapping);
        Ok(())
    }

    pub fn get_or_create(&self, contract_name: &str) -> ContractMappingInfo {
        let mut contracts = self.contracts.lock().unwrap();

        if !contracts.contains_key(contract_name) {
            contracts.insert(
                contract_name.to_string(),
                ContractMappingInfo::new(contract_name.to_string()),
            );
        }

        contracts.get(contract_name).unwrap().clone()
    }

    pub fn get_by_name(&self, contract_name: &str) -> Option<ContractMappingInfo> {
        self.contracts.lock().unwrap().get(contract_name).cloned()
    }

    pub fn get_by_bytecode(&self, bytecode: &str) -> Option<ContractMappingInfo> {
        let contracts = self.contracts.lock().unwrap();
        for info in contracts.values() {
            if let Some(contract_bytecode) = &info.bytecode {
                if contract_bytecode.ends_with(bytecode) {
                    return Some(info.clone());
                }
            }
        }
        None
    }

    pub fn add_node(&self, contract_name: Option<&str>, node: AstNode) {
        if let Some(name) = contract_name {
            let mut contracts = self.contracts.lock().unwrap();
            let info = contracts
                .entry(name.to_string())
                .or_insert_with(|| ContractMappingInfo::new(name.to_string()));
            info.add_node(node);
        }
    }

    pub fn parse_ast(&self, node: &serde_json::Value, explain: bool) {
        self.parse_ast_internal(node, None, explain, 0);
    }

    fn parse_ast_internal(
        &self,
        node: &serde_json::Value,
        contract_name: Option<String>,
        explain: bool,
        depth: usize,
    ) {
        let node_type = node.get("nodeType").and_then(|v| v.as_str()).unwrap_or("");
        let node_name = node.get("name").and_then(|v| v.as_str());

        let mut expl = Explanation::new(explain);
        let indent = "  ".repeat(depth);
        let node_name_str = node_name.map(|n| format!(": {}", n)).unwrap_or_default();
        expl.add(&format!("{}{}{}", indent, node_type, node_name_str));

        if PARSING_IGNORED_NODE_TYPES.contains(&node_type) {
            expl.add(" (ignored node type)");
            return;
        }

        let current_contract = if node_type == "ContractDefinition" {
            if contract_name.is_some() {
                eprintln!("Warning: parsing contract but found nested contract definition");
            }

            let contract_name = node_name.map(|s| s.to_string());
            if let Some(ref name) = contract_name {
                let info = self.get_or_create(name);
                if info.nodes.is_empty() {
                    // Continue parsing
                } else {
                    expl.add(" (skipped, already parsed)");
                    return;
                }
            }
            contract_name
        } else {
            contract_name
        };

        if let Some(ast_node) = AstNode::from_dict(node) {
            if ast_node.selector != "0x" {
                self.add_node(current_contract.as_deref(), ast_node.clone());
                expl.add(&format!(
                    " (added node with selector={})",
                    ast_node.selector
                ));
            }
        }

        // Parse child nodes
        if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
            for child_node in nodes {
                self.parse_ast_internal(child_node, current_contract.clone(), explain, depth + 1);
            }
        }

        // Parse body
        if let Some(body) = node.get("body") {
            self.parse_ast_internal(body, current_contract.clone(), explain, depth + 1);
        }
    }

    pub fn lookup_selector(&self, selector: &str, contract_name: Option<&str>) -> String {
        if selector == "0x" {
            return selector.to_string();
        }

        // Check in the specified contract first
        if let Some(name) = contract_name {
            if let Some(mapping) = self.get_by_name(name) {
                if let Some(node) = mapping.get_node(selector) {
                    return node.name.clone();
                }
            }
        }

        // Search in all contracts
        let contracts = self.contracts.lock().unwrap();
        for mapping in contracts.values() {
            if let Some(node) = mapping.get_node(selector) {
                return node.name.clone();
            }
        }

        selector.to_string()
    }
}

impl Default for Mapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_node_creation() {
        let node = AstNode::new(
            "FunctionDefinition".to_string(),
            "transfer".to_string(),
            "0x12345678".to_string(),
        );
        assert_eq!(node.node_type, "FunctionDefinition");
        assert_eq!(node.name, "transfer");
        assert_eq!(node.selector, "0x12345678");
    }

    #[test]
    fn test_ast_node_from_dict() {
        let json = serde_json::json!({
            "nodeType": "FunctionDefinition",
            "name": "transfer",
            "functionSelector": "a9059cbb"
        });

        let node = AstNode::from_dict(&json).unwrap();
        assert_eq!(node.node_type, "FunctionDefinition");
        assert_eq!(node.name, "transfer");
        assert_eq!(node.selector, "0xa9059cbb");
    }

    #[test]
    fn test_contract_with_nodes() {
        let nodes = vec![
            AstNode::new(
                "FunctionDefinition".to_string(),
                "transfer".to_string(),
                "0x12345678".to_string(),
            ),
            AstNode::new(
                "EventDefinition".to_string(),
                "Transfer".to_string(),
                "0xabcdef00".to_string(),
            ),
        ];

        let info = ContractMappingInfo::new("TestContract".to_string()).with_nodes(nodes);

        assert_eq!(info.nodes.len(), 2);
        assert!(info.get_node("0x12345678").is_some());
        assert!(info.get_node("0xabcdef00").is_some());
    }

    #[test]
    fn test_deploy_address_mapper() {
        let mut mapper = DeployAddressMapper::new();
        mapper.add_deployed_contract("0x1234567890abcdef", "TestContract");

        assert_eq!(
            mapper.get_deployed_contract("0x1234567890abcdef"),
            "TestContract"
        );
        assert_eq!(mapper.get_deployed_contract("0xunknown"), "0xunknown");
    }

    #[test]
    fn test_deploy_address_mapper_defaults() {
        let mapper = DeployAddressMapper::new();
        assert_eq!(
            mapper.get_deployed_contract("0x7109709ecfa91a80626ff3989d68f67f5b1dd12d"),
            "hevm"
        );
        assert_eq!(
            mapper.get_deployed_contract("0xf3993a62377bcd56ae39d773740a5390411e8bc9"),
            "svm"
        );
        assert_eq!(
            mapper.get_deployed_contract("0x636f6e736f6c652e6c6f67"),
            "console"
        );
    }

    #[test]
    fn test_explanation() {
        let mut expl = Explanation::new(true);
        expl.add("Test message 1");
        expl.add("Test message 2");
        assert!(expl.content.contains("Test message 1"));
        assert!(expl.content.contains("Test message 2"));
    }

    #[test]
    fn test_explanation_disabled() {
        let mut expl = Explanation::new(false);
        expl.add("This should not be added");
        assert!(expl.content.is_empty());
    }

    #[test]
    fn test_source_file_map_singleton() {
        let map1 = SourceFileMap::instance();
        let map2 = SourceFileMap::instance();
        assert!(std::ptr::eq(map1, map2));
    }

    #[test]
    fn test_source_file_map_root() {
        let map = SourceFileMap::instance();
        map.set_root("/tmp");
        let root = map.get_root();
        assert!(root.contains("tmp") || root.contains("private"));
    }

    #[test]
    fn test_source_file_map_add_mapping() {
        let map = SourceFileMap::instance();
        map.add_mapping(1, "contracts/Test.sol");
        let path = map.get_file_path(1);
        assert!(path.is_some());
    }

    #[test]
    fn test_mapper_singleton() {
        let mapper1 = Mapper::instance();
        let mapper2 = Mapper::instance();
        assert!(std::ptr::eq(mapper1, mapper2));
    }

    #[test]
    fn test_mapper_add_and_get() {
        let mapper = Mapper::instance();
        let info = ContractMappingInfo::new("SingletonTestUnique123".to_string());
        let _ = mapper.add_mapping(info);

        let retrieved = mapper.get_by_name("SingletonTestUnique123");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_mapper_parse_ast() {
        let mapper = Mapper::instance();
        let json = serde_json::json!({
            "nodeType": "ContractDefinition",
            "name": "TestContractParseUnique456",
            "nodes": [
                {
                    "nodeType": "FunctionDefinition",
                    "name": "testFunction",
                    "functionSelector": "12345678"
                }
            ]
        });

        mapper.parse_ast(&json, false);
        let info = mapper.get_by_name("TestContractParseUnique456");
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.nodes.len(), 1);
        assert!(info.get_node("0x12345678").is_some());
    }

    #[test]
    fn test_mapper_lookup_selector() {
        let mapper = Mapper::instance();
        let mut info = ContractMappingInfo::new("LookupTestUnique789".to_string());
        info.add_node(AstNode::new(
            "FunctionDefinition".to_string(),
            "myFunction".to_string(),
            "0xabcdef00".to_string(),
        ));
        let _ = mapper.add_mapping(info);

        let name = mapper.lookup_selector("0xabcdef00", Some("LookupTestUnique789"));
        assert_eq!(name, "myFunction");

        let unknown = mapper.lookup_selector("0xunknown", Some("LookupTestUnique789"));
        assert_eq!(unknown, "0xunknown");
    }

    #[test]
    fn test_mapper_get_by_bytecode() {
        let mapper = Mapper::instance();
        let info = ContractMappingInfo::new("BytecodeTestUnique001".to_string())
            .with_bytecode("0x6080604052".to_string());
        let _ = mapper.add_mapping(info);

        let found = mapper.get_by_bytecode("604052");
        assert!(found.is_some());
        assert_eq!(found.unwrap().contract_name, "BytecodeTestUnique001");
    }

    #[test]
    fn test_contract_mapping_no_overwrite() {
        let mut info = ContractMappingInfo::new("NoOverwrite".to_string());
        let node1 = AstNode::new(
            "FunctionDefinition".to_string(),
            "first".to_string(),
            "0x1234".to_string(),
        );
        let node2 = AstNode::new(
            "FunctionDefinition".to_string(),
            "second".to_string(),
            "0x1234".to_string(),
        );

        info.add_node(node1);
        info.add_node(node2);

        // Should keep the first node, not overwrite
        assert_eq!(info.get_node("0x1234").unwrap().name, "first");
    }

    #[test]
    fn test_build_out_singleton() {
        let build1 = BuildOut::instance();
        let build2 = BuildOut::instance();
        assert!(std::ptr::eq(build1, build2));
    }

    #[test]
    fn test_build_out_placeholders_immutables() {
        let build_out = BuildOut::instance();
        let deployed = serde_json::json!({
            "object": "0x6080604052600436100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "immutableReferences": {
                "123": [
                    {"start": 10, "length": 32},
                    {"start": 50, "length": 32}
                ]
            }
        });

        let placeholders = build_out.get_placeholders(&deployed).unwrap();
        assert_eq!(placeholders.len(), 2);
        assert_eq!(placeholders[0], (10, 42));
        assert_eq!(placeholders[1], (50, 82));
    }

    #[test]
    fn test_build_out_placeholders_links() {
        let build_out = BuildOut::instance();
        let deployed = serde_json::json!({
            "object": "0x608060405260043610000000000000000000000000000000000000000000000000000000",
            "linkReferences": {
                "lib/Library.sol": {
                    "Library": [
                        {"start": 15, "length": 20}
                    ]
                }
            }
        });

        let placeholders = build_out.get_placeholders(&deployed).unwrap();
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0], (15, 35));
    }
}
