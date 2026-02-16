// SPDX-License-Identifier: AGPL-3.0

//! Calldata generation and ABI handling
//! Complete implementation matching Python halmos/calldata.py

use cbse_bitvec::CbseBitVec;
use cbse_bytevec::{ByteVec, UnwrappedBytes};
use cbse_exceptions::{CbseException, CbseResult};
use cbse_logs::warn_unique;
use regex::Regex;
use std::collections::HashMap;
use z3::Context;

/// Helper function to create a constant bitvector
fn con<'ctx>(value: u64, size: u32, ctx: &'ctx Context) -> CbseBitVec<'ctx> {
    CbseBitVec::from_u64(value, size)
}

/// Type representation for ABI encoding
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Base {
        var: String,
        typ: String,
    },
    FixedArray {
        var: String,
        base: Box<Type>,
        size: usize,
    },
    DynamicArray {
        var: String,
        base: Box<Type>,
    },
    Tuple {
        var: String,
        items: Vec<Type>,
    },
}

impl Type {
    pub fn var(&self) -> &str {
        match self {
            Type::Base { var, .. } => var,
            Type::FixedArray { var, .. } => var,
            Type::DynamicArray { var, .. } => var,
            Type::Tuple { var, .. } => var,
        }
    }
}

/// Parse ABI type from JSON format
pub fn parse_type(var: &str, typ: &str, item: &serde_json::Value) -> CbseResult<Type> {
    let array_re = Regex::new(r"^(.*)(\[([0-9]*)\])$").unwrap();
    if let Some(caps) = array_re.captures(typ) {
        let base_type = caps.get(1).unwrap().as_str();
        let array_len = caps.get(3).unwrap().as_str();
        let base = parse_type("", base_type, item)?;

        if array_len.is_empty() {
            return Ok(Type::DynamicArray {
                var: var.to_string(),
                base: Box::new(base),
            });
        } else {
            let size = array_len
                .parse::<usize>()
                .map_err(|e| CbseException::Internal(format!("Invalid array size: {}", e)))?;
            return Ok(Type::FixedArray {
                var: var.to_string(),
                base: Box::new(base),
                size,
            });
        }
    }

    let type_re = Regex::new(r"^(u?int[0-9]*|address|bool|bytes[0-9]*|string|tuple)$").unwrap();
    if !type_re.is_match(typ) {
        return Err(CbseException::Internal(format!(
            "Not supported type: {}",
            typ
        )));
    }

    if typ == "tuple" {
        let components = item
            .get("components")
            .and_then(|v| v.as_array())
            .ok_or_else(|| CbseException::Internal("Tuple type missing components".to_string()))?;
        return parse_tuple_type(var, components);
    }

    Ok(Type::Base {
        var: var.to_string(),
        typ: typ.to_string(),
    })
}

/// Parse tuple type from components
pub fn parse_tuple_type(var: &str, items: &[serde_json::Value]) -> CbseResult<Type> {
    let mut parsed_items = Vec::new();
    for item in items {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let typ = item
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CbseException::Internal("Component missing type".to_string()))?;
        parsed_items.push(parse_type(&name, typ, item)?);
    }
    Ok(Type::Tuple {
        var: var.to_string(),
        items: parsed_items,
    })
}

/// Encoding result for ABI encoding
#[derive(Debug, Clone)]
pub struct EncodingResult<'ctx> {
    pub data: Vec<CbseBitVec<'ctx>>,
    pub size: usize,
    pub is_static: bool,
}

/// Dynamic parameter information
#[derive(Debug, Clone)]
pub struct DynamicParam<'ctx> {
    pub name: String,
    pub size_choices: Vec<usize>,
    pub size_symbol: CbseBitVec<'ctx>,
    pub typ: Type,
}

impl<'ctx> std::fmt::Display for DynamicParam<'ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={:?}", self.name, self.size_choices)
    }
}

/// Function information
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub contract_name: Option<String>,
    pub name: Option<String>,
    pub sig: Option<String>,
    pub selector: Option<String>,
}

impl FunctionInfo {
    pub fn new() -> Self {
        Self {
            contract_name: None,
            name: None,
            sig: None,
            selector: None,
        }
    }

    pub fn with_selector(selector: String) -> Self {
        Self {
            contract_name: None,
            name: None,
            sig: None,
            selector: Some(selector),
        }
    }

    pub fn with_sig(sig: String) -> Self {
        Self {
            contract_name: None,
            name: None,
            sig: Some(sig),
            selector: None,
        }
    }
}

impl Default for FunctionInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for calldata generation
#[derive(Debug, Clone)]
pub struct CalldataConfig {
    pub array_lengths: HashMap<String, Vec<usize>>,
    pub default_array_lengths: Vec<usize>,
    pub default_bytes_lengths: Vec<usize>,
}

impl CalldataConfig {
    pub fn new() -> Self {
        Self {
            array_lengths: HashMap::new(),
            default_array_lengths: vec![0, 1, 2],
            default_bytes_lengths: vec![0, 1, 32, 33],
        }
    }

    pub fn with_array_length(mut self, name: String, lengths: Vec<usize>) -> Self {
        self.array_lengths.insert(name, lengths);
        self
    }
}

impl Default for CalldataConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Calldata generator
pub struct Calldata<'ctx> {
    config: CalldataConfig,
    dyn_params: Vec<DynamicParam<'ctx>>,
    symbol_counter: usize,
    ctx: &'ctx Context,
}

impl<'ctx> Calldata<'ctx> {
    pub fn new(ctx: &'ctx Context, config: CalldataConfig) -> Self {
        Self {
            config,
            dyn_params: Vec::new(),
            symbol_counter: 0,
            ctx,
        }
    }

    fn next_symbol_id(&mut self) -> usize {
        let id = self.symbol_counter;
        self.symbol_counter += 1;
        id
    }

    fn get_dyn_sizes(&mut self, name: &str, typ: &Type) -> (Vec<usize>, CbseBitVec<'ctx>) {
        let sizes = self
            .config
            .array_lengths
            .get(name)
            .cloned()
            .unwrap_or_else(|| {
                let default_sizes = match typ {
                    Type::DynamicArray { .. } => &self.config.default_array_lengths,
                    _ => &self.config.default_bytes_lengths,
                };
                warn_unique(&format!(
                    "no size provided for {}; default value {:?} will be used.",
                    name, default_sizes
                ));
                default_sizes.clone()
            });

        let size_var = CbseBitVec::symbolic(
            self.ctx,
            &format!("p_{}_length_uid{:02}", name, self.next_symbol_id()),
            256,
        );

        self.dyn_params.push(DynamicParam {
            name: name.to_string(),
            size_choices: sizes.clone(),
            size_symbol: size_var.clone(),
            typ: typ.clone(),
        });

        (sizes, size_var)
    }

    pub fn create(
        mut self,
        abi: &HashMap<String, serde_json::Value>,
        fun_info: &FunctionInfo,
    ) -> CbseResult<(ByteVec<'ctx>, Vec<DynamicParam<'ctx>>)> {
        let mut calldata = ByteVec::new(self.ctx);

        if let Some(selector_hex) = &fun_info.selector {
            let selector_bytes = hex::decode(selector_hex.trim_start_matches("0x"))
                .map_err(|e| CbseException::Internal(format!("Invalid selector hex: {}", e)))?;
            calldata.append(UnwrappedBytes::Bytes(selector_bytes))?;
        }

        let sig = fun_info
            .sig
            .as_ref()
            .ok_or_else(|| CbseException::Internal("Missing function signature".to_string()))?;

        let fun_abi = abi
            .get(sig)
            .ok_or_else(|| CbseException::Internal(format!("Function not found: {}", sig)))?;

        let inputs = fun_abi
            .get("inputs")
            .and_then(|v| v.as_array())
            .ok_or_else(|| CbseException::Internal("Function missing inputs".to_string()))?;

        let tuple_type = parse_tuple_type("", inputs)?;

        if let Type::Tuple { items, .. } = &tuple_type {
            if items.is_empty() {
                return Ok((calldata, self.dyn_params));
            }
        }

        let starting_size = calldata.len();
        let encoded = self.encode("", &tuple_type)?;

        for data in encoded.data {
            calldata.append(UnwrappedBytes::BitVec(data))?;
        }

        let calldata_size = calldata.len() - starting_size;
        if calldata_size != encoded.size {
            return Err(CbseException::Internal(format!(
                "Calldata size mismatch: expected {}, got {}",
                encoded.size, calldata_size
            )));
        }

        Ok((calldata, self.dyn_params))
    }

    fn encode(&mut self, name: &str, typ: &Type) -> CbseResult<EncodingResult<'ctx>> {
        match typ {
            Type::Tuple { items, .. } => {
                let prefix = if name.is_empty() {
                    String::new()
                } else {
                    format!("{}.", name)
                };
                let mut encoded_items = Vec::new();
                for item in items {
                    let item_name = format!("{}{}", prefix, item.var());
                    encoded_items.push(self.encode(&item_name, item)?);
                }
                self.encode_tuple(encoded_items)
            }

            Type::FixedArray { base, size, .. } => {
                let mut items = Vec::new();
                for i in 0..*size {
                    items.push(self.encode(&format!("{}[{}]", name, i), base)?);
                }
                self.encode_tuple(items)
            }

            Type::DynamicArray { base, .. } => {
                let (sizes, size_var) = self.get_dyn_sizes(name, typ);
                let max_size = *sizes.iter().max().unwrap_or(&0);
                let mut items = Vec::new();
                for i in 0..max_size {
                    items.push(self.encode(&format!("{}[{}]", name, i), base)?);
                }
                let encoded = self.encode_tuple(items)?;
                let mut data = vec![size_var];
                data.extend(encoded.data);
                Ok(EncodingResult {
                    data,
                    size: 32 + encoded.size,
                    is_static: false,
                })
            }

            Type::Base { typ, .. } => {
                let new_symbol = format!("p_{}_{}_uid{:02}", name, typ, self.next_symbol_id());

                if typ == "bytes" || typ == "string" {
                    let (sizes, size_var) = self.get_dyn_sizes(
                        name,
                        &Type::Base {
                            var: name.to_string(),
                            typ: typ.clone(),
                        },
                    );
                    let size = *sizes.iter().max().unwrap_or(&0);
                    let size_pad_right = ((size + 31) / 32) * 32;
                    let data = if size > 0 {
                        vec![CbseBitVec::symbolic(
                            self.ctx,
                            &new_symbol,
                            (8 * size_pad_right) as u32,
                        )]
                    } else {
                        vec![]
                    };
                    let mut result_data = vec![size_var];
                    result_data.extend(data);
                    Ok(EncodingResult {
                        data: result_data,
                        size: 32 + size_pad_right,
                        is_static: false,
                    })
                } else {
                    Ok(EncodingResult {
                        data: vec![CbseBitVec::symbolic(self.ctx, &new_symbol, 256)],
                        size: 32,
                        is_static: true,
                    })
                }
            }
        }
    }

    fn encode_tuple(
        &mut self,
        items: Vec<EncodingResult<'ctx>>,
    ) -> CbseResult<EncodingResult<'ctx>> {
        let total_head_size: usize = items
            .iter()
            .map(|x| if x.is_static { x.size } else { 32 })
            .sum();

        let mut total_size = total_head_size;
        let mut heads = Vec::new();
        let mut tails = Vec::new();

        for item in items {
            if item.is_static {
                heads.extend(item.data);
            } else {
                heads.push(con(total_size as u64, 256, self.ctx));
                tails.extend(item.data);
                total_size += item.size;
            }
        }

        let is_static = tails.is_empty();
        let mut data = heads;
        data.extend(tails);

        Ok(EncodingResult {
            data,
            size: total_size,
            is_static,
        })
    }
}

/// Construct a function signature string from ABI item
pub fn str_abi(item: &serde_json::Value) -> CbseResult<String> {
    fn str_tuple(args: &[serde_json::Value]) -> CbseResult<String> {
        let mut ret = Vec::new();
        for arg in args {
            let typ = arg
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CbseException::Internal("Arg missing type".to_string()))?;
            let tuple_re = Regex::new(r"^tuple((\[[0-9]*\])*)$").unwrap();
            if let Some(caps) = tuple_re.captures(typ) {
                let components = arg
                    .get("components")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| {
                        CbseException::Internal("Tuple missing components".to_string())
                    })?;
                let tuple_str = str_tuple(components)?;
                let suffix = caps.get(1).unwrap().as_str();
                ret.push(format!("{}{}", tuple_str, suffix));
            } else {
                ret.push(typ.to_string());
            }
        }
        Ok(format!("({})", ret.join(",")))
    }

    let item_type = item
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CbseException::Internal("Item missing type".to_string()))?;
    if item_type != "function" {
        return Err(CbseException::Internal(format!(
            "Expected function type, got: {}",
            item_type
        )));
    }

    let name = item
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CbseException::Internal("Function missing name".to_string()))?;
    let inputs = item
        .get("inputs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CbseException::Internal("Function missing inputs".to_string()))?;
    let tuple_str = str_tuple(inputs)?;
    Ok(format!("{}{}", name, tuple_str))
}

/// Get ABI mapping from contract JSON
pub fn get_abi(
    contract_json: &mut serde_json::Value,
) -> CbseResult<HashMap<String, serde_json::Value>> {
    if let Some(abi_dict) = contract_json.get("abi_dict") {
        return serde_json::from_value(abi_dict.clone())
            .map_err(|e| CbseException::Internal(format!("Failed to parse abi_dict: {}", e)));
    }

    let abi_array = contract_json
        .get("abi")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CbseException::Internal("Contract missing abi".to_string()))?;

    let mut abi_dict = HashMap::new();
    for item in abi_array {
        if item.get("type").and_then(|v| v.as_str()) == Some("function") {
            let sig = str_abi(item)?;
            abi_dict.insert(sig, item.clone());
        }
    }

    let abi_dict_value = serde_json::to_value(&abi_dict)
        .map_err(|e| CbseException::Internal(format!("Failed to serialize abi_dict: {}", e)))?;

    if let Some(obj) = contract_json.as_object_mut() {
        obj.insert("abi_dict".to_string(), abi_dict_value);
    }

    Ok(abi_dict)
}

/// Create calldata for a function (convenience function)
pub fn mk_calldata<'ctx>(
    ctx: &'ctx Context,
    abi: &HashMap<String, serde_json::Value>,
    fun_info: &FunctionInfo,
    config: CalldataConfig,
) -> CbseResult<(ByteVec<'ctx>, Vec<DynamicParam<'ctx>>)> {
    Calldata::new(ctx, config).create(abi, fun_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_base_type() {
        let item = serde_json::json!({});
        let typ = parse_type("x", "uint256", &item).unwrap();
        match typ {
            Type::Base { var, typ } => {
                assert_eq!(var, "x");
                assert_eq!(typ, "uint256");
            }
            _ => panic!("Expected base type"),
        }
    }

    #[test]
    fn test_parse_fixed_array() {
        let item = serde_json::json!({});
        let typ = parse_type("arr", "uint256[5]", &item).unwrap();
        match typ {
            Type::FixedArray { var, base, size } => {
                assert_eq!(var, "arr");
                assert_eq!(size, 5);
                match *base {
                    Type::Base { ref typ, .. } => assert_eq!(typ, "uint256"),
                    _ => panic!("Expected base type"),
                }
            }
            _ => panic!("Expected fixed array type"),
        }
    }

    #[test]
    fn test_parse_dynamic_array() {
        let item = serde_json::json!({});
        let typ = parse_type("arr", "uint256[]", &item).unwrap();
        match typ {
            Type::DynamicArray { var, base } => {
                assert_eq!(var, "arr");
                match *base {
                    Type::Base { ref typ, .. } => assert_eq!(typ, "uint256"),
                    _ => panic!("Expected base type"),
                }
            }
            _ => panic!("Expected dynamic array type"),
        }
    }

    #[test]
    fn test_str_abi_simple() {
        let item = serde_json::json!({
            "type": "function",
            "name": "transfer",
            "inputs": [{"type": "address"}, {"type": "uint256"}]
        });
        let sig = str_abi(&item).unwrap();
        assert_eq!(sig, "transfer(address,uint256)");
    }

    #[test]
    fn test_str_abi_tuple() {
        let item = serde_json::json!({
            "type": "function",
            "name": "swap",
            "inputs": [{
                "type": "tuple",
                "components": [{"type": "uint256"}, {"type": "address"}]
            }]
        });
        let sig = str_abi(&item).unwrap();
        assert_eq!(sig, "swap((uint256,address))");
    }

    #[test]
    fn test_calldata_config() {
        let config = CalldataConfig::new().with_array_length("arr".to_string(), vec![1, 2, 3]);
        assert_eq!(config.array_lengths.get("arr"), Some(&vec![1, 2, 3]));
    }
}
