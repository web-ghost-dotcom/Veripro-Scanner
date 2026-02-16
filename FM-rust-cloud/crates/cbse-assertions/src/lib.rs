// SPDX-License-Identifier: AGPL-3.0

//! Forge Standard Assertions
//!
//! Complete parity with Python halmos/assertions.py
//! Handles assertion cheatcodes from Forge test framework

use cbse_bitvec::CbseBitVec;
use cbse_bytevec::{ByteVec, UnwrappedBytes};
use regex::Regex;
use std::collections::HashMap;
use z3::Context;

/// Forge Standard Assertion result
#[derive(Debug, Clone)]
pub struct VmAssertion<'ctx> {
    /// The condition to check (as a bitvector, where non-zero = true)
    pub cond: CbseBitVec<'ctx>,
    /// Optional error message
    pub msg: Option<String>,
}

impl<'ctx> VmAssertion<'ctx> {
    /// Create a new assertion
    pub fn new(cond: CbseBitVec<'ctx>, msg: Option<String>) -> Self {
        Self { cond, msg }
    }
}

/// Check if bytes are empty
pub fn is_empty_bytes(x: &[u8]) -> bool {
    x.is_empty()
}

/// Create a condition based on binary operator
///
/// Equivalent to Python: `mk_cond(bop, v1, v2)`
pub fn mk_cond<'ctx>(
    ctx: &'ctx Context,
    bop: &str,
    v1: &UnwrappedBytes<'ctx>,
    v2: &UnwrappedBytes<'ctx>,
) -> Result<CbseBitVec<'ctx>, String> {
    // Handle empty bytes arguments
    match (v1, v2) {
        (UnwrappedBytes::Bytes(b1), UnwrappedBytes::Bytes(b2))
            if b1.is_empty() && b2.is_empty() =>
        {
            return match bop {
                "Eq" => Ok(CbseBitVec::from_u64(1, 256)),    // true
                "NotEq" => Ok(CbseBitVec::from_u64(0, 256)), // false
                _ => Err(format!(
                    "mk_cond: invalid arguments: empty bytes {} empty bytes",
                    bop
                )),
            };
        }
        (UnwrappedBytes::Bytes(b1), UnwrappedBytes::Bytes(b2))
            if b1.is_empty() || b2.is_empty() =>
        {
            return match bop {
                "Eq" => Ok(CbseBitVec::from_u64(0, 256)),    // false
                "NotEq" => Ok(CbseBitVec::from_u64(1, 256)), // true
                _ => Err(format!(
                    "mk_cond: invalid arguments with empty bytes: {}",
                    bop
                )),
            };
        }
        _ => {}
    }

    // Convert to bitvectors
    let bv1 = match v1 {
        UnwrappedBytes::Bytes(b) => {
            if b.is_empty() {
                return Err("v1 is empty bytes".to_string());
            }
            CbseBitVec::from_bytes(b, (b.len() * 8) as u32)
        }
        UnwrappedBytes::BitVec(bv) => bv.clone(),
    };

    let bv2 = match v2 {
        UnwrappedBytes::Bytes(b) => {
            if b.is_empty() {
                return Err("v2 is empty bytes".to_string());
            }
            CbseBitVec::from_bytes(b, (b.len() * 8) as u32)
        }
        UnwrappedBytes::BitVec(bv) => bv.clone(),
    };

    // For Eq and NotEq, bitsize can be arbitrary (e.g., arrays)
    if bv1.size() != bv2.size() {
        return match bop {
            "Eq" => Ok(CbseBitVec::from_u64(0, 256)),    // false
            "NotEq" => Ok(CbseBitVec::from_u64(1, 256)), // true
            _ => Err(format!(
                "mk_cond: incompatible size: {} bits {} {} bits",
                bv1.size(),
                bop,
                bv2.size()
            )),
        };
    }

    match bop {
        "Eq" => {
            let eq_result = bv1.eq(&bv2, ctx);
            Ok(eq_result.to_bitvec(ctx, 256))
        }
        "NotEq" => {
            let eq_result = bv1.eq(&bv2, ctx);
            let not_eq = eq_result.not(ctx);
            Ok(not_eq.to_bitvec(ctx, 256))
        }
        // Comparison operators require 256-bit values
        "ULt" | "UGt" | "ULe" | "UGe" | "SLt" | "SGt" | "SLe" | "SGe" => {
            if bv1.size() != 256 || bv2.size() != 256 {
                return Err(format!(
                    "mk_cond: incompatible size for {}: {} bits, {} bits (expected 256)",
                    bop,
                    bv1.size(),
                    bv2.size()
                ));
            }

            let result = match bop {
                "ULt" => bv1.ult(&bv2, ctx),
                "UGt" => bv1.ugt(&bv2, ctx),
                "ULe" => bv1.ule(&bv2, ctx),
                "UGe" => bv1.uge(&bv2, ctx),
                "SLt" => bv1.slt(&bv2, ctx),
                "SGt" => bv1.sgt(&bv2, ctx),
                "SLe" => bv1.sgt(&bv2, ctx).not(ctx), // x <= y is !(x > y)
                "SGe" => bv1.slt(&bv2, ctx).not(ctx), // x >= y is !(x < y)
                _ => unreachable!(),
            };

            // Convert CbseBool to CbseBitVec (256-bit, 0 or 1)
            Ok(result.to_bitvec(ctx, 256))
        }
        _ => Err(format!("mk_cond: unknown bop: {}", bop)),
    }
}

/// Create a binary assertion handler
///
/// Equivalent to Python: `vm_assert_binary(bop, typ, log)`
pub fn vm_assert_binary<'ctx>(
    bop: &str,
    typ: &str,
    log: bool,
    calldata: &ByteVec<'ctx>,
    ctx: &'ctx Context,
) -> Result<VmAssertion<'ctx>, String> {
    let is_array = typ.ends_with("[]");
    let base_type = typ.trim_end_matches("[]");
    let is_bytes_type = base_type == "bytes" || base_type == "string";

    if !is_array {
        // Scalar types: bool, uint256, int256, address, bytes32
        if !is_bytes_type {
            // Extract two 32-byte values from calldata
            let v1 = calldata
                .slice(4, 36)
                .map_err(|e| format!("Failed to extract v1: {:?}", e))?
                .unwrap()
                .map_err(|e| format!("Failed to unwrap v1: {:?}", e))?;

            let v2 = calldata
                .slice(36, 68)
                .map_err(|e| format!("Failed to extract v2: {:?}", e))?
                .unwrap()
                .map_err(|e| format!("Failed to unwrap v2: {:?}", e))?;

            let cond_result = mk_cond(ctx, bop, &v1, &v2)?;

            let msg = if log {
                extract_string_argument(calldata, 2, ctx)?
            } else {
                None
            };

            Ok(VmAssertion::new(cond_result, msg))
        } else {
            // bytes, string
            let v1 = extract_bytes_argument(calldata, 0, ctx)?;
            let v2 = extract_bytes_argument(calldata, 1, ctx)?;

            let cond_result = mk_cond(ctx, bop, &v1, &v2)?;

            let msg = if log {
                extract_string_argument(calldata, 2, ctx)?
            } else {
                None
            };

            Ok(VmAssertion::new(cond_result, msg))
        }
    } else {
        // Array types
        if !is_bytes_type {
            // bool[], uint256[], int256[], address[], bytes32[]
            let v1 = extract_bytes32_array_argument(calldata, 0, ctx)?;
            let v2 = extract_bytes32_array_argument(calldata, 1, ctx)?;

            let cond_result = mk_cond(ctx, bop, &v1, &v2)?;

            let msg = if log {
                extract_string_argument(calldata, 2, ctx)?
            } else {
                None
            };

            Ok(VmAssertion::new(cond_result, msg))
        } else {
            // bytes[], string[]
            Err(format!(
                "assert {} {}[] not yet implemented",
                bop, base_type
            ))
        }
    }
}

/// Create a unary assertion handler
///
/// Equivalent to Python: `vm_assert_unary(expected, log)`
pub fn vm_assert_unary<'ctx>(
    expected: bool,
    log: bool,
    calldata: &ByteVec<'ctx>,
    ctx: &'ctx Context,
) -> Result<VmAssertion<'ctx>, String> {
    // Extract the actual boolean value from calldata at offset 4
    let actual_word = calldata
        .get_word(4)
        .map_err(|e| format!("Failed to get word: {:?}", e))?;

    // Convert to bitvector
    let actual = match actual_word {
        UnwrappedBytes::Bytes(b) => CbseBitVec::from_bytes(&b, (b.len() * 8) as u32),
        UnwrappedBytes::BitVec(bv) => bv,
    };

    // Test the value (equivalent to Python's test() function)
    let cond = if expected {
        // assertTrue: check if value != 0
        let zero = CbseBitVec::from_u64(0, 256);
        let is_not_zero = actual.eq(&zero, ctx).not(ctx);
        is_not_zero.to_bitvec(ctx, 256)
    } else {
        // assertFalse: check if value == 0
        let zero = CbseBitVec::from_u64(0, 256);
        let is_zero = zero.eq(&actual, ctx);
        is_zero.to_bitvec(ctx, 256)
    };

    let msg = if log {
        extract_string_argument(calldata, 1, ctx)?
    } else {
        None
    };

    Ok(VmAssertion::new(cond, msg))
}

/// Extract a string argument from calldata
///
/// Equivalent to Python: `extract_string_argument(data, arg_idx)`
fn extract_string_argument<'ctx>(
    calldata: &ByteVec<'ctx>,
    arg_idx: usize,
    _ctx: &'ctx Context,
) -> Result<Option<String>, String> {
    // Skip function selector (4 bytes) + arg_idx * 32 bytes
    let offset = 4 + arg_idx * 32;

    // Read the offset pointer to the string data
    let ptr_word = calldata
        .get_word(offset)
        .map_err(|e| format!("Failed to get offset pointer: {:?}", e))?;

    let ptr = match ptr_word {
        UnwrappedBytes::Bytes(b) => {
            let mut val = 0usize;
            for &byte in &b {
                val = (val << 8) | (byte as usize);
            }
            val
        }
        UnwrappedBytes::BitVec(_) => {
            // Symbolic pointer - cannot extract string
            return Ok(None);
        }
    };

    // Read the length at that offset (skip selector)
    let len_offset = 4 + ptr;
    let len_word = calldata
        .get_word(len_offset)
        .map_err(|e| format!("Failed to get string length: {:?}", e))?;

    let len = match len_word {
        UnwrappedBytes::Bytes(b) => {
            let mut val = 0usize;
            for &byte in &b {
                val = (val << 8) | (byte as usize);
            }
            val
        }
        UnwrappedBytes::BitVec(_) => return Ok(None),
    };

    if len == 0 {
        return Ok(Some(String::new()));
    }

    // Read the string bytes
    let str_start = len_offset + 32;
    let str_slice = calldata
        .slice(str_start, str_start + len)
        .map_err(|e| format!("Failed to slice string: {:?}", e))?
        .unwrap()
        .map_err(|e| format!("Failed to unwrap string: {:?}", e))?;

    match str_slice {
        UnwrappedBytes::Bytes(b) => Ok(String::from_utf8(b).ok()),
        UnwrappedBytes::BitVec(_) => Ok(None),
    }
}

/// Extract a bytes argument from calldata
///
/// Equivalent to Python: `extract_bytes_argument(data, arg_idx)`
fn extract_bytes_argument<'ctx>(
    calldata: &ByteVec<'ctx>,
    arg_idx: usize,
    _ctx: &'ctx Context,
) -> Result<UnwrappedBytes<'ctx>, String> {
    // Similar to extract_string_argument but returns bytes
    let offset = 4 + arg_idx * 32;

    let ptr_word = calldata
        .get_word(offset)
        .map_err(|e| format!("Failed to get offset pointer: {:?}", e))?;

    let ptr = match &ptr_word {
        UnwrappedBytes::Bytes(b) => {
            let mut val = 0usize;
            for &byte in b {
                val = (val << 8) | (byte as usize);
            }
            val
        }
        UnwrappedBytes::BitVec(_) => return Ok(ptr_word), // Return symbolic
    };

    let len_offset = 4 + ptr;
    let len_word = calldata
        .get_word(len_offset)
        .map_err(|e| format!("Failed to get bytes length: {:?}", e))?;

    let len = match &len_word {
        UnwrappedBytes::Bytes(b) => {
            let mut val = 0usize;
            for &byte in b {
                val = (val << 8) | (byte as usize);
            }
            val
        }
        UnwrappedBytes::BitVec(_) => return Ok(len_word), // Return symbolic
    };

    let bytes_start = len_offset + 32;
    calldata
        .slice(bytes_start, bytes_start + len)
        .map_err(|e| format!("Failed to slice bytes: {:?}", e))?
        .unwrap()
        .map_err(|e| format!("Failed to unwrap bytes: {:?}", e))
}

/// Extract a bytes32 array argument from calldata
///
/// Equivalent to Python: `extract_bytes32_array_argument(data, arg_idx)`
fn extract_bytes32_array_argument<'ctx>(
    calldata: &ByteVec<'ctx>,
    arg_idx: usize,
    _ctx: &'ctx Context,
) -> Result<UnwrappedBytes<'ctx>, String> {
    let offset = 4 + arg_idx * 32;

    let ptr_word = calldata
        .get_word(offset)
        .map_err(|e| format!("Failed to get offset pointer: {:?}", e))?;

    let ptr = match &ptr_word {
        UnwrappedBytes::Bytes(b) => {
            let mut val = 0usize;
            for &byte in b {
                val = (val << 8) | (byte as usize);
            }
            val
        }
        UnwrappedBytes::BitVec(_) => return Ok(ptr_word),
    };

    let len_offset = 4 + ptr;
    let len_word = calldata
        .get_word(len_offset)
        .map_err(|e| format!("Failed to get array length: {:?}", e))?;

    let len = match &len_word {
        UnwrappedBytes::Bytes(b) => {
            let mut val = 0usize;
            for &byte in b {
                val = (val << 8) | (byte as usize);
            }
            val
        }
        UnwrappedBytes::BitVec(_) => return Ok(len_word),
    };

    // Read all array elements (each is 32 bytes)
    let array_start = len_offset + 32;
    let array_end = array_start + len * 32;

    calldata
        .slice(array_start, array_end)
        .map_err(|e| format!("Failed to slice array: {:?}", e))?
        .unwrap()
        .map_err(|e| format!("Failed to unwrap array: {:?}", e))
}

/// Create an assertion handler from a signature
///
/// Equivalent to Python: `mk_assert_handler(signature)`
pub fn mk_assert_handler(signature: &str) -> Result<String, String> {
    // Parse the signature
    let re = Regex::new(r"assert([^(]+)\(([^)]*)\)").unwrap();

    let caps = re
        .captures(signature)
        .ok_or_else(|| format!("not supported signatures: {}", signature))?;

    let operator = caps.get(1).unwrap().as_str();
    let params_str = caps.get(2).unwrap().as_str();
    let params: Vec<&str> = if params_str.is_empty() {
        vec![]
    } else {
        params_str.split(',').collect()
    };

    // Determine operator type
    let is_binary = operator != "True" && operator != "False";

    // Determine if it has log message
    let has_log = params.len() > if is_binary { 2 } else { 1 };

    if is_binary {
        let typ = params[0]; // params[0] == params[1]
        let bop = if operator == "Eq" || operator == "NotEq" {
            operator.to_string()
        } else {
            // For comparison operators, identify unsigned or signed
            let sign = if typ == "uint256" { "U" } else { "S" };
            format!("{}{}", sign, operator)
        };

        Ok(format!("binary:{}:{}:{}", bop, typ, has_log))
    } else {
        let expected = operator == "True";
        Ok(format!("unary:{}:{}", expected, has_log))
    }
}

/// Get the assertion cheatcode handler mapping
///
/// Maps function selector (first 4 bytes) to assertion signature
pub fn get_assert_cheatcode_handlers() -> HashMap<u32, String> {
    let mut handlers = HashMap::new();

    // assertTrue/False
    handlers.insert(0x0C9FD581, "assertTrue(bool)".to_string());
    handlers.insert(0xA34EDC03, "assertTrue(bool,string)".to_string());
    handlers.insert(0xA5982885, "assertFalse(bool)".to_string());
    handlers.insert(0x7BA04809, "assertFalse(bool,string)".to_string());

    // assertEq(T, T)
    handlers.insert(0xF7FE3477, "assertEq(bool,bool)".to_string());
    handlers.insert(0x4DB19E7E, "assertEq(bool,bool,string)".to_string());
    handlers.insert(0x98296C54, "assertEq(uint256,uint256)".to_string());
    handlers.insert(0x88B44C85, "assertEq(uint256,uint256,string)".to_string());
    handlers.insert(0xFE74F05B, "assertEq(int256,int256)".to_string());
    handlers.insert(0x714A2F13, "assertEq(int256,int256,string)".to_string());
    handlers.insert(0x515361F6, "assertEq(address,address)".to_string());
    handlers.insert(0x2F2769D1, "assertEq(address,address,string)".to_string());
    handlers.insert(0x7C84C69B, "assertEq(bytes32,bytes32)".to_string());
    handlers.insert(0xC1FA1ED0, "assertEq(bytes32,bytes32,string)".to_string());
    handlers.insert(0xF320D963, "assertEq(string,string)".to_string());
    handlers.insert(0x36F656D8, "assertEq(string,string,string)".to_string());
    handlers.insert(0x97624631, "assertEq(bytes,bytes)".to_string());
    handlers.insert(0xE24FED00, "assertEq(bytes,bytes,string)".to_string());

    // assertEq(T[], T[])
    handlers.insert(0x707DF785, "assertEq(bool[],bool[])".to_string());
    handlers.insert(0xE48A8F8D, "assertEq(bool[],bool[],string)".to_string());
    handlers.insert(0x975D5A12, "assertEq(uint256[],uint256[])".to_string());
    handlers.insert(
        0x5D18C73A,
        "assertEq(uint256[],uint256[],string)".to_string(),
    );
    handlers.insert(0x711043AC, "assertEq(int256[],int256[])".to_string());
    handlers.insert(0x191F1B30, "assertEq(int256[],int256[],string)".to_string());
    handlers.insert(0x3868AC34, "assertEq(address[],address[])".to_string());
    handlers.insert(
        0x3E9173C5,
        "assertEq(address[],address[],string)".to_string(),
    );
    handlers.insert(0x0CC9EE84, "assertEq(bytes32[],bytes32[])".to_string());
    handlers.insert(
        0xE03E9177,
        "assertEq(bytes32[],bytes32[],string)".to_string(),
    );
    handlers.insert(0xCF1C049C, "assertEq(string[],string[])".to_string());
    handlers.insert(0xEFF6B27D, "assertEq(string[],string[],string)".to_string());
    handlers.insert(0xE5FB9B4A, "assertEq(bytes[],bytes[])".to_string());
    handlers.insert(0xF413F0B6, "assertEq(bytes[],bytes[],string)".to_string());

    // assertNotEq(T, T)
    handlers.insert(0x236E4D66, "assertNotEq(bool,bool)".to_string());
    handlers.insert(0x1091A261, "assertNotEq(bool,bool,string)".to_string());
    handlers.insert(0xB7909320, "assertNotEq(uint256,uint256)".to_string());
    handlers.insert(
        0x98F9BDBD,
        "assertNotEq(uint256,uint256,string)".to_string(),
    );
    handlers.insert(0xF4C004E3, "assertNotEq(int256,int256)".to_string());
    handlers.insert(0x4724C5B9, "assertNotEq(int256,int256,string)".to_string());
    handlers.insert(0xB12E1694, "assertNotEq(address,address)".to_string());
    handlers.insert(
        0x8775A591,
        "assertNotEq(address,address,string)".to_string(),
    );
    handlers.insert(0x898E83FC, "assertNotEq(bytes32,bytes32)".to_string());
    handlers.insert(
        0xB2332F51,
        "assertNotEq(bytes32,bytes32,string)".to_string(),
    );
    handlers.insert(0x6A8237B3, "assertNotEq(string,string)".to_string());
    handlers.insert(0x78BDCEA7, "assertNotEq(string,string,string)".to_string());
    handlers.insert(0x3CF78E28, "assertNotEq(bytes,bytes)".to_string());
    handlers.insert(0x9507540E, "assertNotEq(bytes,bytes,string)".to_string());

    // assertNotEq(T[], T[])
    handlers.insert(0x286FAFEA, "assertNotEq(bool[],bool[])".to_string());
    handlers.insert(0x62C6F9FB, "assertNotEq(bool[],bool[],string)".to_string());
    handlers.insert(0x56F29CBA, "assertNotEq(uint256[],uint256[])".to_string());
    handlers.insert(
        0x9A7FBD8F,
        "assertNotEq(uint256[],uint256[],string)".to_string(),
    );
    handlers.insert(0x0B72F4EF, "assertNotEq(int256[],int256[])".to_string());
    handlers.insert(
        0xD3977322,
        "assertNotEq(int256[],int256[],string)".to_string(),
    );
    handlers.insert(0x46D0B252, "assertNotEq(address[],address[])".to_string());
    handlers.insert(
        0x72C7E0B5,
        "assertNotEq(address[],address[],string)".to_string(),
    );
    handlers.insert(0x0603EA68, "assertNotEq(bytes32[],bytes32[])".to_string());
    handlers.insert(
        0xB873634C,
        "assertNotEq(bytes32[],bytes32[],string)".to_string(),
    );
    handlers.insert(0xBDFACBE8, "assertNotEq(string[],string[])".to_string());
    handlers.insert(
        0xB67187F3,
        "assertNotEq(string[],string[],string)".to_string(),
    );
    handlers.insert(0xEDECD035, "assertNotEq(bytes[],bytes[])".to_string());
    handlers.insert(
        0x1DCD1F68,
        "assertNotEq(bytes[],bytes[],string)".to_string(),
    );

    // assertLt/Gt/Le/Ge
    handlers.insert(0xB12FC005, "assertLt(uint256,uint256)".to_string());
    handlers.insert(0x65D5C135, "assertLt(uint256,uint256,string)".to_string());
    handlers.insert(0x3E914080, "assertLt(int256,int256)".to_string());
    handlers.insert(0x9FF531E3, "assertLt(int256,int256,string)".to_string());
    handlers.insert(0xDB07FCD2, "assertGt(uint256,uint256)".to_string());
    handlers.insert(0xD9A3C4D2, "assertGt(uint256,uint256,string)".to_string());
    handlers.insert(0x5A362D45, "assertGt(int256,int256)".to_string());
    handlers.insert(0xF8D33B9B, "assertGt(int256,int256,string)".to_string());
    handlers.insert(0x8466F415, "assertLe(uint256,uint256)".to_string());
    handlers.insert(0xD17D4B0D, "assertLe(uint256,uint256,string)".to_string());
    handlers.insert(0x95FD154E, "assertLe(int256,int256)".to_string());
    handlers.insert(0x4DFE692C, "assertLe(int256,int256,string)".to_string());
    handlers.insert(0xA8D4D1D9, "assertGe(uint256,uint256)".to_string());
    handlers.insert(0xE25242C0, "assertGe(uint256,uint256,string)".to_string());
    handlers.insert(0x0A30B771, "assertGe(int256,int256)".to_string());
    handlers.insert(0xA84328DD, "assertGe(int256,int256,string)".to_string());

    handlers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_empty_bytes() {
        assert!(is_empty_bytes(&[]));
        assert!(!is_empty_bytes(&[1, 2, 3]));
    }

    #[test]
    fn test_mk_assert_handler() {
        let result = mk_assert_handler("assertTrue(bool)").unwrap();
        assert!(result.contains("unary"));
        assert!(result.contains("true"));

        let result = mk_assert_handler("assertEq(uint256,uint256)").unwrap();
        assert!(result.contains("binary"));
        assert!(result.contains("Eq"));
    }

    #[test]
    fn test_get_assert_cheatcode_handlers() {
        let handlers = get_assert_cheatcode_handlers();
        assert!(handlers.len() > 60); // Should have all assertion handlers
        assert_eq!(
            handlers.get(&0x0C9FD581),
            Some(&"assertTrue(bool)".to_string())
        );
    }
}
