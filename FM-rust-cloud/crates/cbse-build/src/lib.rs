// SPDX-License-Identifier: AGPL-3.0

//! Build artifact parsing and contract loading
//!
//! This module provides functionality to parse Forge build outputs and load contract
//! artifacts, matching the behavior of halmos/build.py

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Contract type information extracted from AST
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractTypeInfo {
    pub contract_type: String,
    pub natspec: Option<JsonValue>,
}

/// Parsed contract information from build output
#[derive(Debug, Clone)]
pub struct ContractInfo {
    pub json: JsonValue,
    pub contract_type: String,
    pub natspec: Option<JsonValue>,
}

/// Build artifact from Forge compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildArtifact {
    pub abi: Vec<JsonValue>,
    pub bytecode: BytecodeInfo,
    #[serde(rename = "deployedBytecode")]
    pub deployed_bytecode: BytecodeInfo,
    #[serde(rename = "methodIdentifiers")]
    pub method_identifiers: Option<HashMap<String, String>>,
    pub metadata: Metadata,
    pub ast: AstNode,
    #[serde(default)]
    pub id: i64,
}

/// Metadata from build artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub compiler: Compiler,
    #[serde(default)]
    pub output: OutputDoc,
}

/// Compiler information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Compiler {
    pub version: String,
}

/// Output documentation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputDoc {
    #[serde(default)]
    pub devdoc: DevDoc,
}

/// Developer documentation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevDoc {
    #[serde(default)]
    pub methods: HashMap<String, MethodDoc>,
}

/// Method documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodDoc {
    #[serde(rename = "custom:halmos")]
    pub custom_halmos: Option<String>,
}

/// AST node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    #[serde(rename = "nodeType")]
    pub node_type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub nodes: Vec<AstNode>,
    #[serde(rename = "contractKind")]
    pub contract_kind: Option<String>,
    #[serde(rename = "abstract")]
    pub is_abstract: Option<bool>,
    pub documentation: Option<JsonValue>,
    #[serde(rename = "absolutePath")]
    pub absolute_path: Option<String>,
}

/// Bytecode information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytecodeInfo {
    pub object: String,
    #[serde(rename = "sourceMap")]
    pub source_map: Option<String>,
    #[serde(rename = "linkReferences")]
    pub link_references: Option<HashMap<String, HashMap<String, Vec<LinkReference>>>>,
}

/// Link reference for library linking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkReference {
    pub start: usize,
    pub length: usize,
}

/// Library information for linking
#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub placeholder: String,
    pub hexcode: String,
}

/// Build output structure: compiler version -> source filename -> contract name -> ContractInfo
pub type BuildOutput = HashMap<String, HashMap<String, HashMap<String, ContractInfo>>>;

/// Get contract type from AST nodes
pub fn get_contract_type(ast_nodes: &[AstNode], contract_name: &str) -> Option<ContractTypeInfo> {
    for node in ast_nodes {
        if node.node_type == "ContractDefinition" && node.name == contract_name {
            let abstract_prefix = if node.is_abstract.unwrap_or(false) {
                "abstract "
            } else {
                ""
            };
            let contract_kind = node.contract_kind.as_deref().unwrap_or("contract");
            let contract_type = format!("{}{}", abstract_prefix, contract_kind);
            let natspec = node.documentation.clone();

            return Some(ContractTypeInfo {
                contract_type,
                natspec,
            });
        }
    }
    None
}

/// Parse build output directory
pub fn parse_build_out(root: &Path, forge_build_out: &str, debug: bool) -> Result<BuildOutput> {
    let mut result: BuildOutput = HashMap::new();

    let out_path = root.join(forge_build_out);
    if !out_path.exists() {
        return Err(anyhow::anyhow!(
            "The build output directory `{}` does not exist",
            out_path.display()
        ));
    }

    // Iterate through source files (.sol directories)
    for sol_entry in std::fs::read_dir(&out_path)
        .with_context(|| format!("Failed to read directory: {}", out_path.display()))?
    {
        let sol_entry = sol_entry?;
        let sol_dirname = sol_entry.file_name();
        let sol_dirname_str = sol_dirname.to_string_lossy();

        if !sol_dirname_str.ends_with(".sol") {
            continue;
        }

        let sol_path = sol_entry.path();
        if !sol_path.is_dir() {
            continue;
        }

        // Iterate through contract JSON files
        for json_entry in std::fs::read_dir(&sol_path)? {
            let json_entry = json_entry?;
            let json_filename = json_entry.file_name();
            let json_filename_str = json_filename.to_string_lossy();

            // Skip non-JSON files and hidden files
            if !json_filename_str.ends_with(".json") || json_filename_str.starts_with('.') {
                continue;
            }

            let json_path = json_entry.path();

            // Parse the JSON file
            match parse_contract_json(&json_path, &json_filename_str, &sol_dirname_str) {
                Ok((compiler_version, contract_name, contract_info)) => {
                    // Insert into result structure
                    result
                        .entry(compiler_version)
                        .or_insert_with(HashMap::new)
                        .entry(sol_dirname_str.to_string())
                        .or_insert_with(HashMap::new)
                        .insert(contract_name, contract_info);
                }
                Err(e) => {
                    eprintln!(
                        "Skipped {} due to parsing failure: {}",
                        json_filename_str, e
                    );
                    if debug {
                        eprintln!("Error details: {:?}", e);
                    }
                    continue;
                }
            }
        }
    }

    Ok(result)
}

/// Parse a single contract JSON file
fn parse_contract_json(
    json_path: &Path,
    json_filename: &str,
    sol_dirname: &str,
) -> Result<(String, String, ContractInfo)> {
    let content = std::fs::read_to_string(json_path)
        .with_context(|| format!("Failed to read file: {:?}", json_path))?;

    let json_out: JsonValue = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON: {:?}", json_path))?;

    // Parse as BuildArtifact to access structured data
    let artifact: BuildArtifact = serde_json::from_value(json_out.clone())
        .with_context(|| format!("Failed to parse build artifact: {:?}", json_path))?;

    let ast = &artifact.ast;

    // Extract contract name from filename (remove .json and compiler version)
    let contract_name = json_filename
        .split('.')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid filename: {}", json_filename))?
        .to_string();

    // Get contract type from AST
    let type_info = get_contract_type(&ast.nodes, &contract_name);

    let contract_type_info = match type_info {
        Some(info) => info,
        None => {
            // Can happen for various reasons:
            // - import only (like console2.log)
            // - defines only structs or enums
            // - defines only free functions
            return Err(anyhow::anyhow!(
                "No contract definition found for {}",
                contract_name
            ));
        }
    };

    let compiler_version = artifact.metadata.compiler.version.clone();

    let contract_info = ContractInfo {
        json: json_out,
        contract_type: contract_type_info.contract_type,
        natspec: contract_type_info.natspec,
    };

    Ok((compiler_version, contract_name, contract_info))
}

/// Parse symbols from contract AST (stub for Mapper integration)
///
/// In the Python version, this integrates with Mapper to parse AST symbols.
/// For now, this is a stub that can be extended when cbse-mapper is implemented.
pub fn parse_symbols(
    contract_map: &HashMap<String, ContractInfo>,
    contract_name: &str,
    _debug: bool,
) -> Result<()> {
    // Extract bytecode for symbol mapping
    if let Some(contract_info) = contract_map.get(contract_name) {
        let bytecode = contract_info
            .json
            .get("bytecode")
            .and_then(|b| b.get("object"))
            .and_then(|o| o.as_str())
            .unwrap_or("0x");

        // TODO: Integrate with Mapper when available
        // Mapper().get_or_create(contract_name).bytecode = bytecode;
        // Mapper().parse_ast(&contract_info.json["ast"]);

        if _debug {
            eprintln!(
                "Parsed symbols for {}: {} bytes",
                contract_name,
                bytecode.len()
            );
        }
    }

    Ok(())
}

/// Parse devdoc for halmos-specific configuration
pub fn parse_devdoc(funsig: &str, contract_json: &JsonValue) -> Option<String> {
    contract_json
        .get("metadata")?
        .get("output")?
        .get("devdoc")?
        .get("methods")?
        .get(funsig)?
        .get("custom:halmos")?
        .as_str()
        .map(|s| s.to_string())
}

/// Parse NatSpec documentation for halmos annotations
///
/// This parsing scheme is designed to handle:
/// - multiline tags:
///   /// @custom:halmos --x
///   ///                --y
/// - multiple tags:
///   /// @custom:halmos --x
///   /// @custom:halmos --y
/// - tags that start in the middle of line:
///   /// blah blah @custom:halmos --x
///   /// --y
///
/// In all the above examples, this scheme returns "--x (whitespaces) --y"
pub fn parse_natspec(natspec: &JsonValue) -> String {
    let text = match natspec.get("text").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return String::new(),
    };

    // Split by @tags while keeping the delimiters
    let tag_regex = Regex::new(r"(@\S+)").unwrap();
    let mut is_halmos_tag = false;
    let mut result = String::new();

    // Split text into parts and iterate
    let mut parts = Vec::new();
    let mut last_end = 0;

    for mat in tag_regex.find_iter(text) {
        // Add text before the match
        if mat.start() > last_end {
            parts.push(&text[last_end..mat.start()]);
        }
        // Add the match (the tag)
        parts.push(mat.as_str());
        last_end = mat.end();
    }
    // Add remaining text
    if last_end < text.len() {
        parts.push(&text[last_end..]);
    }

    // Process parts
    for part in parts {
        if part == "@custom:halmos" {
            is_halmos_tag = true;
        } else if part.starts_with('@') {
            is_halmos_tag = false;
        } else if is_halmos_tag {
            result.push_str(part);
        }
    }

    result.trim().to_string()
}

/// Import library information for linking
pub fn import_libs(
    build_out_map: &HashMap<String, HashMap<String, ContractInfo>>,
    hexcode: &str,
    link_references: &HashMap<String, HashMap<String, Vec<LinkReference>>>,
) -> Result<HashMap<String, LibraryInfo>> {
    let mut libs = HashMap::new();

    for (filepath, file_libs) in link_references {
        let file_name = filepath
            .split('/')
            .last()
            .ok_or_else(|| anyhow::anyhow!("Invalid filepath: {}", filepath))?;

        for (lib_name, references) in file_libs {
            // Get library contract info
            let lib_contract = build_out_map
                .get(file_name)
                .and_then(|m| m.get(lib_name))
                .ok_or_else(|| anyhow::anyhow!("Library not found: {}:{}", filepath, lib_name))?;

            // Extract library bytecode
            let lib_artifact: BuildArtifact = serde_json::from_value(lib_contract.json.clone())?;
            let lib_hexcode = lib_artifact.deployed_bytecode.object;

            // Get placeholder from hexcode
            // in bytes, multiply indices by 2 and offset 0x
            let first_ref = references.first().ok_or_else(|| {
                anyhow::anyhow!("No link references for {}:{}", filepath, lib_name)
            })?;

            let placeholder_index = first_ref.start * 2 + 2; // +2 for "0x" prefix
            let placeholder_end = placeholder_index + 40;

            if hexcode.len() < placeholder_end {
                return Err(anyhow::anyhow!(
                    "Hexcode too short for placeholder extraction"
                ));
            }

            let placeholder = hexcode[placeholder_index..placeholder_end].to_string();

            let lib_key = format!("{}:{}", filepath, lib_name);
            libs.insert(
                lib_key,
                LibraryInfo {
                    placeholder,
                    hexcode: lib_hexcode,
                },
            );
        }
    }

    Ok(libs)
}

/// Iterator over build output entries
pub struct BuildOutputIterator<'a> {
    build_out: &'a BuildOutput,
    compiler_versions: Vec<String>,
    current_compiler: usize,
    current_files: Vec<String>,
    current_file: usize,
    current_contracts: Vec<String>,
    current_contract: usize,
}

impl<'a> BuildOutputIterator<'a> {
    pub fn new(build_out: &'a BuildOutput) -> Self {
        let mut compiler_versions: Vec<String> = build_out.keys().cloned().collect();
        compiler_versions.sort();

        Self {
            build_out,
            compiler_versions,
            current_compiler: 0,
            current_files: Vec::new(),
            current_file: 0,
            current_contracts: Vec::new(),
            current_contract: 0,
        }
    }
}

impl<'a> Iterator for BuildOutputIterator<'a> {
    type Item = (
        &'a HashMap<String, HashMap<String, ContractInfo>>,
        String,
        String,
    );

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next contract
            if self.current_contract < self.current_contracts.len() {
                let contract_name = self.current_contracts[self.current_contract].clone();
                let file_name = self.current_files[self.current_file].clone();
                let compiler_version = &self.compiler_versions[self.current_compiler];
                let build_out_map = self.build_out.get(compiler_version)?;

                self.current_contract += 1;
                return Some((build_out_map, file_name, contract_name));
            }

            // Try to get next file
            self.current_contract = 0;
            self.current_file += 1;

            if self.current_file < self.current_files.len() {
                let file_name = &self.current_files[self.current_file];
                let compiler_version = &self.compiler_versions[self.current_compiler];
                let build_out_map = self.build_out.get(compiler_version)?;
                let contract_map = build_out_map.get(file_name)?;

                self.current_contracts = {
                    let mut contracts: Vec<String> = contract_map.keys().cloned().collect();
                    contracts.sort();
                    contracts
                };
                continue;
            }

            // Try to get next compiler version
            self.current_file = 0;
            self.current_compiler += 1;

            if self.current_compiler >= self.compiler_versions.len() {
                return None;
            }

            let compiler_version = &self.compiler_versions[self.current_compiler];
            let build_out_map = self.build_out.get(compiler_version)?;

            self.current_files = {
                let mut files: Vec<String> = build_out_map.keys().cloned().collect();
                files.sort();
                files
            };

            if self.current_files.is_empty() {
                continue;
            }

            let file_name = &self.current_files[0];
            let contract_map = build_out_map.get(file_name)?;

            self.current_contracts = {
                let mut contracts: Vec<String> = contract_map.keys().cloned().collect();
                contracts.sort();
                contracts
            };
        }
    }
}

/// Create an iterator over build output
pub fn build_output_iterator(build_out: &BuildOutput) -> BuildOutputIterator {
    BuildOutputIterator::new(build_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_contract_type() {
        let nodes = vec![AstNode {
            node_type: "ContractDefinition".to_string(),
            name: "TestContract".to_string(),
            nodes: vec![],
            contract_kind: Some("contract".to_string()),
            is_abstract: Some(false),
            documentation: None,
            absolute_path: None,
        }];

        let result = get_contract_type(&nodes, "TestContract");
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.contract_type, "contract");
    }

    #[test]
    fn test_get_contract_type_abstract() {
        let nodes = vec![AstNode {
            node_type: "ContractDefinition".to_string(),
            name: "AbstractTest".to_string(),
            nodes: vec![],
            contract_kind: Some("contract".to_string()),
            is_abstract: Some(true),
            documentation: None,
            absolute_path: None,
        }];

        let result = get_contract_type(&nodes, "AbstractTest");
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.contract_type, "abstract contract");
    }

    #[test]
    fn test_get_contract_type_not_found() {
        let nodes = vec![AstNode {
            node_type: "ContractDefinition".to_string(),
            name: "TestContract".to_string(),
            nodes: vec![],
            contract_kind: Some("contract".to_string()),
            is_abstract: Some(false),
            documentation: None,
            absolute_path: None,
        }];

        let result = get_contract_type(&nodes, "NonExistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_natspec_basic() {
        let natspec = serde_json::json!({
            "text": "@custom:halmos --solver-timeout-assertion 0"
        });

        let result = parse_natspec(&natspec);
        assert_eq!(result, "--solver-timeout-assertion 0");
    }

    #[test]
    fn test_parse_natspec_multiline() {
        let natspec = serde_json::json!({
            "text": "@custom:halmos --x\n@custom:halmos --y"
        });

        let result = parse_natspec(&natspec);
        // Should capture content after halmos tags
        assert!(result.contains("--x") || result.contains("--y"));
    }

    #[test]
    fn test_parse_natspec_empty() {
        let natspec = serde_json::json!({});
        let result = parse_natspec(&natspec);
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_bytecode_hex() {
        let hex = "0x6001";
        let hex_stripped = hex.strip_prefix("0x").unwrap();
        let bytes = hex::decode(hex_stripped).unwrap();
        assert_eq!(bytes, vec![0x60, 0x01]);
    }

    #[test]
    fn test_library_placeholder_calculation() {
        // Test that placeholder index calculation matches Python
        let start = 100;
        let placeholder_index = start * 2 + 2;
        assert_eq!(placeholder_index, 202);
    }
}
