// SPDX-License-Identifier: AGPL-3.0

//! Console logging functionality for CBSE
//!
//! This module provides console.log functionality matching halmos/console.py,
//! supporting various type renderings (uint256, string, bytes, address, bool, etc.)

use anyhow::Result;
use cbse_bitvec::CbseBitVec;
use cbse_utils::{extract_bytes, hexify};
use colored::Colorize;
use num_bigint::BigInt;
use z3::Context;

/// Console logging address (matches forge-std/console2.sol)
/// 0x000000000000000000636F6E736F6C652E6C6F67
/// This is a 160-bit (20-byte) address with "console.log" encoded in the last bytes
pub const CONSOLE_ADDRESS: [u8; 20] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x6F, 0x6E, 0x73, 0x6F, 0x6C, 0x65,
    0x2E, 0x6C, 0x6F, 0x67,
];

/// Extract function selector (first 4 bytes)
fn extract_funsig<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<CbseBitVec<'ctx>> {
    extract_bytes(arg, 0, 4, ctx)
        .map_err(|e| anyhow::anyhow!("Failed to extract function selector: {:?}", e))
}

/// Convert bitvector to u32 (for selector)
fn int_of(bv: &CbseBitVec, _desc: &str) -> Result<u32> {
    if let Ok(val) = bv.as_u64() {
        Ok(val as u32)
    } else {
        anyhow::bail!("Cannot convert symbolic value to integer")
    }
}

/// Extract string argument from ABI-encoded calldata
fn extract_string_argument<'ctx>(
    arg: &CbseBitVec<'ctx>,
    index: usize,
    ctx: &'ctx Context,
) -> Result<String> {
    // Simple implementation: extract from offset
    // In full implementation, this would decode ABI string encoding
    let offset = 4 + (index * 32); // selector + index * word size
    let bytes = extract_bytes(arg, offset, 32, ctx)
        .map_err(|e| anyhow::anyhow!("Failed to extract string: {:?}", e))?;

    // Try to convert to string
    let b = bytes.to_bytes();
    // Find null terminator
    let end = b.iter().position(|&x| x == 0).unwrap_or(b.len());
    Ok(String::from_utf8_lossy(&b[..end]).to_string())
}

/// Extract bytes argument from ABI-encoded calldata
fn extract_bytes_argument<'ctx>(
    arg: &CbseBitVec<'ctx>,
    index: usize,
    ctx: &'ctx Context,
) -> Result<Vec<u8>> {
    // Simple implementation
    let offset = 4 + (index * 32);
    let bytes = extract_bytes(arg, offset, 32, ctx)
        .map_err(|e| anyhow::anyhow!("Failed to extract bytes: {:?}", e))?;
    Ok(bytes.to_bytes())
}

/// Render uint256 value
fn render_uint(bv: &CbseBitVec) -> String {
    let bytes = bv.to_bytes();
    let value = BigInt::from_bytes_be(num_bigint::Sign::Plus, &bytes);
    format!("{}", value)
}

/// Render int256 value (signed)
fn render_int(bv: &CbseBitVec) -> String {
    let bytes = bv.to_bytes();
    // Check if negative (top bit set)
    let is_negative = bytes.first().map(|&b| b & 0x80 != 0).unwrap_or(false);
    if is_negative {
        // Two's complement
        let value = BigInt::from_signed_bytes_be(&bytes);
        format!("{}", value)
    } else {
        let value = BigInt::from_bytes_be(num_bigint::Sign::Plus, &bytes);
        format!("{}", value)
    }
}

/// Render address
fn render_address(bv: &CbseBitVec) -> String {
    let bytes = bv.to_bytes();
    // Take last 20 bytes
    let start = bytes.len().saturating_sub(20);
    let addr_bytes = &bytes[start..];
    format!("0x{}", hex::encode(addr_bytes))
}

/// Render bool
fn render_bool(bv: &CbseBitVec) -> String {
    if let Ok(val) = bv.as_u64() {
        if val != 0 {
            "true".to_string()
        } else {
            "false".to_string()
        }
    } else {
        format!("<symbolic bool>")
    }
}

/// Render bytes
fn render_bytes(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

/// Console logging facility
pub struct Console;

impl Console {
    /// Core log function - prints with [console.log] prefix and magenta color
    pub fn log(message: &str) {
        println!("[console.log] {}", message.magenta());
    }

    /// Log uint256 value
    /// Function selector: 0xF82C50F1
    pub fn log_uint256<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let bytes = extract_bytes(arg, 4, 32, ctx)?;
        let rendered = render_uint(&bytes);
        Console::log(&rendered);
        Ok(())
    }

    /// Log uint (alias for log_uint256)
    /// Function selector: 0xF5B1BBA9
    pub fn log_uint<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        Console::log_uint256(arg, ctx)
    }

    /// Log string value
    /// Function selector: 0x41304FAC
    pub fn log_string<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let str_val = extract_string_argument(arg, 0, ctx)?;
        Console::log(&str_val);
        Ok(())
    }

    /// Log bytes value
    /// Function selector: 0x0BE77F56
    pub fn log_bytes<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let bytes = extract_bytes_argument(arg, 0, ctx)?;
        let rendered = render_bytes(&bytes);
        Console::log(&rendered);
        Ok(())
    }

    /// Log string and address
    /// Function selector: 0x319AF333
    pub fn log_string_address<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let str_val = extract_string_argument(arg, 0, ctx)?;
        let addr = extract_bytes(arg, 36, 32, ctx)?;
        let rendered_addr = render_address(&addr);
        Console::log(&format!("{} {}", str_val, rendered_addr));
        Ok(())
    }

    /// Log address value
    /// Function selector: 0x2C2ECBC2
    pub fn log_address<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let addr = extract_bytes(arg, 4, 32, ctx)?;
        let rendered = render_address(&addr);
        Console::log(&rendered);
        Ok(())
    }

    /// Log string and bool
    /// Function selector: 0xC3B55635
    pub fn log_string_bool<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let str_val = extract_string_argument(arg, 0, ctx)?;
        let bool_val = extract_bytes(arg, 36, 32, ctx)?;
        let rendered_bool = render_bool(&bool_val);
        Console::log(&format!("{} {}", str_val, rendered_bool));
        Ok(())
    }

    /// Log bool value
    /// Function selector: 0x32458EED
    pub fn log_bool<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let bool_val = extract_bytes(arg, 4, 32, ctx)?;
        let rendered = render_bool(&bool_val);
        Console::log(&rendered);
        Ok(())
    }

    /// Log two strings
    /// Function selector: 0x4B5C4277
    pub fn log_string_string<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let str1_val = extract_string_argument(arg, 0, ctx)?;
        let str2_val = extract_string_argument(arg, 1, ctx)?;
        Console::log(&format!("{} {}", str1_val, str2_val));
        Ok(())
    }

    /// Log bytes32 value
    /// Function selector: 0x27B7CF85
    pub fn log_bytes32<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let bytes = extract_bytes(arg, 4, 32, ctx)?;
        let bytes_vec = bytes.to_bytes();
        let hex_str = hexify(&bytes_vec);
        Console::log(&hex_str);
        Ok(())
    }

    /// Log string and int256
    /// Function selector: 0x3CA6268E
    pub fn log_string_int256<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let str_val = extract_string_argument(arg, 0, ctx)?;
        let int_val = extract_bytes(arg, 36, 32, ctx)?;
        let rendered_int = render_int(&int_val);
        Console::log(&format!("{} {}", str_val, rendered_int));
        Ok(())
    }

    /// Log int256 value
    /// Function selector: 0x2D5B6CB9
    pub fn log_int256<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let int_val = extract_bytes(arg, 4, 32, ctx)?;
        let rendered = render_int(&int_val);
        Console::log(&rendered);
        Ok(())
    }

    /// Log string and uint256
    /// Function selector: 0xB60E72CC
    pub fn log_string_uint256<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        let str_val = extract_string_argument(arg, 0, ctx)?;
        let uint_val = extract_bytes(arg, 36, 32, ctx)?;
        let rendered_uint = render_uint(&uint_val);
        Console::log(&format!("{} {}", str_val, rendered_uint));
        Ok(())
    }

    /// Handle console.log call with given argument
    ///
    /// Extracts function selector and dispatches to appropriate handler.
    /// Matches Python's console.handle() function.
    pub fn handle<'ctx>(arg: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> Result<()> {
        // Wrap in try-catch to avoid failing execution due to console.log issues
        let result = (|| -> Result<()> {
            // Extract function selector (first 4 bytes)
            let funsig = extract_funsig(arg, ctx)?;
            let selector = int_of(&funsig, "symbolic console function selector")?;

            // Dispatch based on selector
            match selector {
                0xF82C50F1 => Console::log_uint256(arg, ctx)?,
                0xF5B1BBA9 => Console::log_uint(arg, ctx)?,
                0x41304FAC => Console::log_string(arg, ctx)?,
                0x0BE77F56 => Console::log_bytes(arg, ctx)?,
                0x319AF333 => Console::log_string_address(arg, ctx)?,
                0x2C2ECBC2 => Console::log_address(arg, ctx)?,
                0xC3B55635 => Console::log_string_bool(arg, ctx)?,
                0x32458EED => Console::log_bool(arg, ctx)?,
                0x4B5C4277 => Console::log_string_string(arg, ctx)?,
                0x27B7CF85 => Console::log_bytes32(arg, ctx)?,
                0x3CA6268E => Console::log_string_int256(arg, ctx)?,
                0x2D5B6CB9 => Console::log_int256(arg, ctx)?,
                0xB60E72CC => Console::log_string_uint256(arg, ctx)?,
                _ => {
                    eprintln!(
                        "[console.log] Unsupported console function: selector = 0x{:0>8x}",
                        selector
                    );
                }
            }
            Ok(())
        })();

        // Don't propagate console.log errors - just warn
        if let Err(e) = result {
            eprintln!("[console.log] Warning: {}", e);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_address() {
        // Verify console address matches Python constant
        // 0x000000000000000000636F6E736F6C652E6C6F67
        assert_eq!(CONSOLE_ADDRESS[9], 0x63); // 'c'
        assert_eq!(CONSOLE_ADDRESS[10], 0x6F); // 'o'
        assert_eq!(CONSOLE_ADDRESS[11], 0x6E); // 'n'
        assert_eq!(CONSOLE_ADDRESS[12], 0x73); // 's'
        assert_eq!(CONSOLE_ADDRESS[13], 0x6F); // 'o'
        assert_eq!(CONSOLE_ADDRESS[14], 0x6C); // 'l'
        assert_eq!(CONSOLE_ADDRESS[15], 0x65); // 'e'
        assert_eq!(CONSOLE_ADDRESS[16], 0x2E); // '.'
        assert_eq!(CONSOLE_ADDRESS[17], 0x6C); // 'l'
        assert_eq!(CONSOLE_ADDRESS[18], 0x6F); // 'o'
        assert_eq!(CONSOLE_ADDRESS[19], 0x67); // 'g'

        // Verify full address (20 bytes)
        let expected = hex::decode("000000000000000000636f6e736f6c652e6c6f67").unwrap();
        assert_eq!(&CONSOLE_ADDRESS[..], &expected[..]);
    }

    #[test]
    fn test_log_uint256() {
        let ctx = Context::new(&z3::Config::new());

        // Create calldata: selector (4 bytes) + uint256 value (32 bytes)
        // Selector 0xF82C50F1 + value 42
        let mut calldata = vec![0xF8, 0x2C, 0x50, 0xF1]; // selector
        calldata.extend_from_slice(&[0u8; 31]); // padding
        calldata.push(42); // value = 42

        let bv = CbseBitVec::from_bytes(&calldata, (calldata.len() * 8) as u32);

        // Should not panic
        let result = Console::log_uint256(&bv, &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_bool() {
        let ctx = Context::new(&z3::Config::new());

        // Create calldata: selector (4 bytes) + bool value (32 bytes)
        // Selector 0x32458EED + value true (1)
        let mut calldata = vec![0x32, 0x45, 0x8E, 0xED]; // selector
        calldata.extend_from_slice(&[0u8; 31]); // padding
        calldata.push(1); // value = true

        let bv = CbseBitVec::from_bytes(&calldata, (calldata.len() * 8) as u32);

        // Should not panic
        let result = Console::log_bool(&bv, &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_unknown_selector() {
        let ctx = Context::new(&z3::Config::new());

        // Create calldata with unknown selector
        let calldata = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x00];
        let bv = CbseBitVec::from_bytes(&calldata, (calldata.len() * 8) as u32);

        // Should not panic, just print warning
        let result = Console::handle(&bv, &ctx);
        assert!(result.is_ok());
    }
}
