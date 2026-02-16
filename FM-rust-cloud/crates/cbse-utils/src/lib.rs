// SPDX-License-Identifier: AGPL-3.0

//! General utility functions for CBSE symbolic execution

use colored::Colorize;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Once;
use std::time::{Duration, Instant};

/// EVM opcode constants
pub struct EVM;

#[allow(dead_code)]
impl EVM {
    pub const STOP: u8 = 0x00;
    pub const ADD: u8 = 0x01;
    pub const MUL: u8 = 0x02;
    pub const SUB: u8 = 0x03;
    pub const DIV: u8 = 0x04;
    pub const SDIV: u8 = 0x05;
    pub const MOD: u8 = 0x06;
    pub const SMOD: u8 = 0x07;
    pub const ADDMOD: u8 = 0x08;
    pub const MULMOD: u8 = 0x09;
    pub const EXP: u8 = 0x0a;
    pub const SIGNEXTEND: u8 = 0x0b;

    pub const LT: u8 = 0x10;
    pub const GT: u8 = 0x11;
    pub const SLT: u8 = 0x12;
    pub const SGT: u8 = 0x13;
    pub const EQ: u8 = 0x14;
    pub const ISZERO: u8 = 0x15;
    pub const AND: u8 = 0x16;
    pub const OR: u8 = 0x17;
    pub const XOR: u8 = 0x18;
    pub const NOT: u8 = 0x19;
    pub const BYTE: u8 = 0x1a;
    pub const SHL: u8 = 0x1b;
    pub const SHR: u8 = 0x1c;
    pub const SAR: u8 = 0x1d;

    pub const SHA3: u8 = 0x20;

    pub const ADDRESS: u8 = 0x30;
    pub const BALANCE: u8 = 0x31;
    pub const ORIGIN: u8 = 0x32;
    pub const CALLER: u8 = 0x33;
    pub const CALLVALUE: u8 = 0x34;
    pub const CALLDATALOAD: u8 = 0x35;
    pub const CALLDATASIZE: u8 = 0x36;
    pub const CALLDATACOPY: u8 = 0x37;
    pub const CODESIZE: u8 = 0x38;
    pub const CODECOPY: u8 = 0x39;
    pub const GASPRICE: u8 = 0x3a;
    pub const EXTCODESIZE: u8 = 0x3b;
    pub const EXTCODECOPY: u8 = 0x3c;
    pub const RETURNDATASIZE: u8 = 0x3d;
    pub const RETURNDATACOPY: u8 = 0x3e;
    pub const EXTCODEHASH: u8 = 0x3f;

    pub const BLOCKHASH: u8 = 0x40;
    pub const COINBASE: u8 = 0x41;
    pub const TIMESTAMP: u8 = 0x42;
    pub const NUMBER: u8 = 0x43;
    pub const DIFFICULTY: u8 = 0x44;
    pub const GASLIMIT: u8 = 0x45;
    pub const CHAINID: u8 = 0x46;
    pub const SELFBALANCE: u8 = 0x47;
    pub const BASEFEE: u8 = 0x48;

    pub const POP: u8 = 0x50;
    pub const MLOAD: u8 = 0x51;
    pub const MSTORE: u8 = 0x52;
    pub const MSTORE8: u8 = 0x53;
    pub const SLOAD: u8 = 0x54;
    pub const SSTORE: u8 = 0x55;
    pub const JUMP: u8 = 0x56;
    pub const JUMPI: u8 = 0x57;
    pub const PC: u8 = 0x58;
    pub const MSIZE: u8 = 0x59;
    pub const GAS: u8 = 0x5a;
    pub const JUMPDEST: u8 = 0x5b;
    pub const TLOAD: u8 = 0x5c;
    pub const TSTORE: u8 = 0x5d;
    pub const MCOPY: u8 = 0x5e;

    pub const PUSH0: u8 = 0x5f;
    pub const PUSH1: u8 = 0x60;
    pub const PUSH32: u8 = 0x7f;

    pub const DUP1: u8 = 0x80;
    pub const DUP16: u8 = 0x8f;

    pub const SWAP1: u8 = 0x90;
    pub const SWAP16: u8 = 0x9f;

    pub const LOG0: u8 = 0xa0;
    pub const LOG1: u8 = 0xa1;
    pub const LOG2: u8 = 0xa2;
    pub const LOG3: u8 = 0xa3;
    pub const LOG4: u8 = 0xa4;

    pub const CREATE: u8 = 0xf0;
    pub const CALL: u8 = 0xf1;
    pub const CALLCODE: u8 = 0xf2;
    pub const RETURN: u8 = 0xf3;
    pub const DELEGATECALL: u8 = 0xf4;
    pub const CREATE2: u8 = 0xf5;
    pub const STATICCALL: u8 = 0xfa;
    pub const REVERT: u8 = 0xfd;
    pub const INVALID: u8 = 0xfe;
    pub const SELFDESTRUCT: u8 = 0xff;
}

/// Get opcode string representation
pub fn opcode_to_string(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "STOP",
        0x01 => "ADD",
        0x02 => "MUL",
        0x03 => "SUB",
        0x04 => "DIV",
        0x05 => "SDIV",
        0x06 => "MOD",
        0x07 => "SMOD",
        0x08 => "ADDMOD",
        0x09 => "MULMOD",
        0x0a => "EXP",
        0x0b => "SIGNEXTEND",
        0x10 => "LT",
        0x11 => "GT",
        0x12 => "SLT",
        0x13 => "SGT",
        0x14 => "EQ",
        0x15 => "ISZERO",
        0x16 => "AND",
        0x17 => "OR",
        0x18 => "XOR",
        0x19 => "NOT",
        0x1a => "BYTE",
        0x1b => "SHL",
        0x1c => "SHR",
        0x1d => "SAR",
        0x20 => "SHA3",
        0x30 => "ADDRESS",
        0x31 => "BALANCE",
        0x32 => "ORIGIN",
        0x33 => "CALLER",
        0x34 => "CALLVALUE",
        0x35 => "CALLDATALOAD",
        0x36 => "CALLDATASIZE",
        0x37 => "CALLDATACOPY",
        0x38 => "CODESIZE",
        0x39 => "CODECOPY",
        0x3a => "GASPRICE",
        0x3b => "EXTCODESIZE",
        0x3c => "EXTCODECOPY",
        0x3d => "RETURNDATASIZE",
        0x3e => "RETURNDATACOPY",
        0x3f => "EXTCODEHASH",
        0x40 => "BLOCKHASH",
        0x41 => "COINBASE",
        0x42 => "TIMESTAMP",
        0x43 => "NUMBER",
        0x44 => "DIFFICULTY",
        0x45 => "GASLIMIT",
        0x46 => "CHAINID",
        0x47 => "SELFBALANCE",
        0x48 => "BASEFEE",
        0x50 => "POP",
        0x51 => "MLOAD",
        0x52 => "MSTORE",
        0x53 => "MSTORE8",
        0x54 => "SLOAD",
        0x55 => "SSTORE",
        0x56 => "JUMP",
        0x57 => "JUMPI",
        0x58 => "PC",
        0x59 => "MSIZE",
        0x5a => "GAS",
        0x5b => "JUMPDEST",
        0x5c => "TLOAD",
        0x5d => "TSTORE",
        0x5e => "MCOPY",
        0x5f => "PUSH0",
        0x60..=0x7f => {
            // PUSH1-PUSH32
            const PUSH_NAMES: [&str; 32] = [
                "PUSH1", "PUSH2", "PUSH3", "PUSH4", "PUSH5", "PUSH6", "PUSH7", "PUSH8", "PUSH9",
                "PUSH10", "PUSH11", "PUSH12", "PUSH13", "PUSH14", "PUSH15", "PUSH16", "PUSH17",
                "PUSH18", "PUSH19", "PUSH20", "PUSH21", "PUSH22", "PUSH23", "PUSH24", "PUSH25",
                "PUSH26", "PUSH27", "PUSH28", "PUSH29", "PUSH30", "PUSH31", "PUSH32",
            ];
            PUSH_NAMES[(opcode - 0x60) as usize]
        }
        0x80..=0x8f => {
            // DUP1-DUP16
            const DUP_NAMES: [&str; 16] = [
                "DUP1", "DUP2", "DUP3", "DUP4", "DUP5", "DUP6", "DUP7", "DUP8", "DUP9", "DUP10",
                "DUP11", "DUP12", "DUP13", "DUP14", "DUP15", "DUP16",
            ];
            DUP_NAMES[(opcode - 0x80) as usize]
        }
        0x90..=0x9f => {
            // SWAP1-SWAP16
            const SWAP_NAMES: [&str; 16] = [
                "SWAP1", "SWAP2", "SWAP3", "SWAP4", "SWAP5", "SWAP6", "SWAP7", "SWAP8", "SWAP9",
                "SWAP10", "SWAP11", "SWAP12", "SWAP13", "SWAP14", "SWAP15", "SWAP16",
            ];
            SWAP_NAMES[(opcode - 0x90) as usize]
        }
        0xa0 => "LOG0",
        0xa1 => "LOG1",
        0xa2 => "LOG2",
        0xa3 => "LOG3",
        0xa4 => "LOG4",
        0xf0 => "CREATE",
        0xf1 => "CALL",
        0xf2 => "CALLCODE",
        0xf3 => "RETURN",
        0xf4 => "DELEGATECALL",
        0xf5 => "CREATE2",
        0xfa => "STATICCALL",
        0xfd => "REVERT",
        0xfe => "INVALID",
        0xff => "SELFDESTRUCT",
        _ => "UNKNOWN",
    }
}

/// Color utility functions matching Python utils.py
pub fn green(text: &str) -> String {
    text.green().to_string()
}

pub fn red(text: &str) -> String {
    text.red().to_string()
}

pub fn yellow(text: &str) -> String {
    text.yellow().to_string()
}

pub fn cyan(text: &str) -> String {
    text.cyan().to_string()
}

pub fn magenta(text: &str) -> String {
    text.magenta().to_string()
}

/// Color aliases
pub fn color_good(text: &str) -> String {
    green(text)
}

pub fn color_error(text: &str) -> String {
    red(text)
}

pub fn color_warn(text: &str) -> String {
    yellow(text)
}

pub fn color_info(text: &str) -> String {
    cyan(text)
}

pub fn color_debug(text: &str) -> String {
    magenta(text)
}

/// Indent text by n spaces
pub fn indent_text(text: &str, n: usize) -> String {
    let indent = " ".repeat(n);
    text.lines()
        .map(|line| format!("{}{}", indent, line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format byte size in human-readable form
pub fn format_size(num_bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

    if num_bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = num_bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", num_bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// Format time duration in human-readable form
pub fn format_time(seconds: f64) -> String {
    if seconds < 0.001 {
        format!("{:.2} μs", seconds * 1_000_000.0)
    } else if seconds < 1.0 {
        format!("{:.2} ms", seconds * 1000.0)
    } else if seconds < 60.0 {
        format!("{:.2} s", seconds)
    } else if seconds < 3600.0 {
        let minutes = (seconds / 60.0).floor();
        let secs = seconds % 60.0;
        format!("{:.0}m {:.0}s", minutes, secs)
    } else {
        let hours = (seconds / 3600.0).floor();
        let minutes = ((seconds % 3600.0) / 60.0).floor();
        format!("{:.0}h {:.0}m", hours, minutes)
    }
}

/// Parse time string (e.g., "30s", "5m", "2h")
pub fn parse_time(arg: &str, default_unit: Option<&str>) -> Result<f64, String> {
    let re = Regex::new(r"^(\d+(?:\.\d+)?)(s|m|h|ms|us)?$").unwrap();

    if let Some(caps) = re.captures(arg) {
        let value: f64 = caps[1].parse().map_err(|_| "Invalid number")?;
        let unit = caps.get(2).map(|m| m.as_str()).or(default_unit);

        let seconds = match unit {
            Some("us") => value / 1_000_000.0,
            Some("ms") => value / 1000.0,
            Some("s") | None => value,
            Some("m") => value * 60.0,
            Some("h") => value * 3600.0,
            _ => return Err(format!("Unknown time unit: {:?}", unit)),
        };

        Ok(seconds)
    } else {
        Err(format!("Invalid time format: {}", arg))
    }
}

/// Strip hex prefix (0x or 0X)
pub fn stripped(hexstring: &str) -> &str {
    if hexstring.starts_with("0x") || hexstring.starts_with("0X") {
        &hexstring[2..]
    } else {
        hexstring
    }
}

/// Decode hex string to bytes
pub fn decode_hex(hexstring: &str) -> Option<Vec<u8>> {
    let s = stripped(hexstring);
    hex::decode(s).ok()
}

/// Convert bytes to hex string with 0x prefix
pub fn hexify(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

/// Named timer for performance tracking
pub struct NamedTimer {
    name: String,
    start: Instant,
    timings: HashMap<String, Vec<Duration>>,
}

impl NamedTimer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: Instant::now(),
            timings: HashMap::new(),
        }
    }

    pub fn create_subtimer(&mut self, name: &str) -> SubTimer {
        SubTimer {
            name: name.to_string(),
            start: Instant::now(),
            parent: self,
        }
    }

    pub fn record(&mut self, label: &str, duration: Duration) {
        self.timings
            .entry(label.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn report(&self) -> String {
        let mut lines = vec![format!("Timer '{}' report:", self.name)];

        for (label, durations) in &self.timings {
            let total: Duration = durations.iter().sum();
            let count = durations.len();
            let avg = total / count as u32;

            lines.push(format!(
                "  {}: {} calls, total {}, avg {}",
                label,
                count,
                format_time(total.as_secs_f64()),
                format_time(avg.as_secs_f64())
            ));
        }

        lines.push(format!(
            "Total elapsed: {}",
            format_time(self.elapsed().as_secs_f64())
        ));

        lines.join("\n")
    }
}

pub struct SubTimer<'a> {
    name: String,
    start: Instant,
    parent: &'a mut NamedTimer,
}

impl<'a> Drop for SubTimer<'a> {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        self.parent.record(&self.name, elapsed);
    }
}

/// Timed block for measuring execution time
pub struct TimedBlock {
    label: String,
    start: Instant,
}

impl TimedBlock {
    pub fn new(label: &str) -> Self {
        println!("[{}] Starting...", label);
        Self {
            label: label.to_string(),
            start: Instant::now(),
        }
    }
}

impl Drop for TimedBlock {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        println!(
            "[{}] Completed in {}",
            self.label,
            format_time(elapsed.as_secs_f64())
        );
    }
}

/// Generate unique ID
static mut UID_COUNTER: u64 = 0;
static UID_INIT: Once = Once::new();

pub fn uid() -> String {
    unsafe {
        UID_INIT.call_once(|| {
            UID_COUNTER = 0;
        });
        UID_COUNTER += 1;
        format!("uid_{}", UID_COUNTER)
    }
}

/// secp256k1 curve order constant (as hex string since it exceeds u128)
pub const SECP256K1N_HEX: &str = "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141";

// ============================================================================
// BitVec Creation and Conversion Functions (matching Python utils.py)
// ============================================================================

/// Create a constant bit-vector value (matches Python con())
/// Note: This is a lightweight wrapper - actual z3 operations happen in cbse-bitvec
pub fn con(value: u64, size_bits: u32) -> (u64, u32) {
    (value, size_bits)
}

/// Create a constant address value (matches Python con_addr())
/// Returns a 160-bit address as bytes
pub fn con_addr(value: u64) -> [u8; 20] {
    // Check would be: value >= 2^160, but that overflows u64
    // Since u64 max is 2^64-1 which is < 2^160, any u64 is valid
    let mut addr = [0u8; 20];
    let bytes = value.to_be_bytes();
    addr[12..20].copy_from_slice(&bytes);
    addr
}

/// Unbox an integer from various types (matches Python unbox_int())
/// Converts int-like objects to int, returns None for symbolic values
pub fn unbox_int(value: &cbse_bitvec::CbseBitVec) -> Option<u64> {
    value.as_u64().ok()
}

/// Convert a concrete bitvector value to bytes (matches Python bv_value_to_bytes())
pub fn bv_value_to_bytes(value: &cbse_bitvec::CbseBitVec) -> Result<Vec<u8>, String> {
    Ok(value.to_bytes())
}

/// Convert bytes to a bitvector value (matches Python bytes_to_bv_value())
pub fn bytes_to_bv_value(bytes: &[u8]) -> u64 {
    let mut value = 0u64;
    for (i, &byte) in bytes.iter().rev().enumerate().take(8) {
        value |= (byte as u64) << (i * 8);
    }
    value
}

/// Check if a value is concrete (not symbolic)
pub fn is_concrete(value: &cbse_bitvec::CbseBitVec) -> bool {
    value.as_u64().is_ok()
}

/// Extract integer from bitvector (matches Python int_of())
pub fn int_of(value: &cbse_bitvec::CbseBitVec, error_msg: Option<&str>) -> Result<u64, String> {
    value.as_u64().map_err(|_| {
        error_msg
            .unwrap_or("Cannot extract concrete value from symbolic bitvector")
            .to_string()
    })
}

/// Create a 256-bit constant value (matches Python uint256())
pub fn uint256(value: u64) -> cbse_bitvec::CbseBitVec<'static> {
    cbse_bitvec::CbseBitVec::from_u64(value, 256)
}

/// Create an 8-bit constant value (matches Python uint8())
pub fn uint8(value: u8) -> cbse_bitvec::CbseBitVec<'static> {
    cbse_bitvec::CbseBitVec::from_u64(value as u64, 8)
}

/// Create a 160-bit constant value (matches Python uint160())
pub fn uint160(value: u64) -> cbse_bitvec::CbseBitVec<'static> {
    cbse_bitvec::CbseBitVec::from_u64(value, 160)
}

/// Extract bytes from a BitVec or ByteVec (matches Python extract_bytes())
///
/// Extracts `size_bytes` bytes from `data` starting at `offset`.
/// Zero-pads if the extraction goes out of bounds.
pub fn extract_bytes<'ctx>(
    data: &cbse_bitvec::CbseBitVec<'ctx>,
    offset: usize,
    size_bytes: usize,
    ctx: &'ctx z3::Context,
) -> anyhow::Result<cbse_bitvec::CbseBitVec<'ctx>> {
    use z3::ast::BV;

    if size_bytes == 0 {
        return Ok(cbse_bitvec::CbseBitVec::from_u64(0, 8)); // Minimum 8 bits
    }

    let size_bits = size_bytes * 8;

    // Get the underlying z3 bitvector
    let bv = data.as_z3(ctx);
    let n = bv.get_size() as usize;

    if n % 8 != 0 {
        anyhow::bail!("BitVec size {} is not a multiple of 8", n);
    }

    // Calculate bit positions for extraction
    // In Z3, bits are numbered from LSB (0) to MSB (n-1)
    // We want to extract from MSB side
    let hi = n.saturating_sub(1 + offset * 8);
    let lo_target = n as i32 - (offset * 8) as i32 - (size_bytes * 8) as i32;
    let lo = if lo_target < 0 { 0 } else { lo_target as usize };

    // Extract the bits
    let val = if hi >= lo && hi < n && lo < n {
        bv.extract(hi as u32, lo as u32)
    } else {
        // Out of bounds, return zero
        BV::from_u64(ctx, 0, size_bits as u32)
    };

    // Zero-pad if needed
    let extracted_size = val.get_size() as usize;
    let final_val = if extracted_size < size_bits {
        let zero_padding = size_bits - extracted_size;
        val.concat(&BV::from_u64(ctx, 0, zero_padding as u32))
    } else if extracted_size > size_bits {
        // Truncate if too large
        val.extract((size_bits - 1) as u32, 0)
    } else {
        val
    };

    Ok(cbse_bitvec::CbseBitVec::from_z3(final_val))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_to_string() {
        assert_eq!(opcode_to_string(EVM::STOP), "STOP");
        assert_eq!(opcode_to_string(EVM::ADD), "ADD");
        assert_eq!(opcode_to_string(EVM::PUSH1), "PUSH1");
        assert_eq!(opcode_to_string(EVM::PUSH32), "PUSH32");
        assert_eq!(opcode_to_string(EVM::DUP1), "DUP1");
        assert_eq!(opcode_to_string(EVM::SWAP1), "SWAP1");
        assert_eq!(opcode_to_string(EVM::RETURN), "RETURN");
    }

    #[test]
    fn test_color_functions() {
        let text = "test";
        assert!(green(text).contains("test"));
        assert!(red(text).contains("test"));
        assert!(yellow(text).contains("test"));
        assert!(cyan(text).contains("test"));
        assert!(magenta(text).contains("test"));
    }

    #[test]
    fn test_color_aliases() {
        let text = "test";
        assert_eq!(color_good(text), green(text));
        assert_eq!(color_error(text), red(text));
        assert_eq!(color_warn(text), yellow(text));
        assert_eq!(color_info(text), cyan(text));
        assert_eq!(color_debug(text), magenta(text));
    }

    #[test]
    fn test_indent_text() {
        let text = "line1\nline2";
        let indented = indent_text(text, 4);
        assert!(indented.starts_with("    "));
        assert!(indented.contains("    line2"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(100), "100 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_time() {
        assert!(format_time(0.0001).contains("μs"));
        assert!(format_time(0.5).contains("ms"));
        assert!(format_time(5.0).contains("s"));
        assert!(format_time(90.0).contains("m"));
        assert!(format_time(3700.0).contains("h"));
    }

    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time("30s", None).unwrap(), 30.0);
        assert_eq!(parse_time("5m", None).unwrap(), 300.0);
        assert_eq!(parse_time("2h", None).unwrap(), 7200.0);
        assert_eq!(parse_time("500ms", None).unwrap(), 0.5);
        assert_eq!(parse_time("100", Some("s")).unwrap(), 100.0);
    }

    #[test]
    fn test_parse_time_error() {
        assert!(parse_time("invalid", None).is_err());
        assert!(parse_time("30x", None).is_err());
    }

    #[test]
    fn test_stripped() {
        assert_eq!(stripped("0x1234"), "1234");
        assert_eq!(stripped("0X5678"), "5678");
        assert_eq!(stripped("abcd"), "abcd");
    }

    #[test]
    fn test_decode_hex() {
        assert_eq!(decode_hex("0x1234"), Some(vec![0x12, 0x34]));
        assert_eq!(decode_hex("abcd"), Some(vec![0xab, 0xcd]));
        assert!(decode_hex("xyz").is_none());
    }

    #[test]
    fn test_hexify() {
        assert_eq!(hexify(&[0x12, 0x34]), "0x1234");
        assert_eq!(hexify(&[0xab, 0xcd, 0xef]), "0xabcdef");
    }

    #[test]
    fn test_uid() {
        let id1 = uid();
        let id2 = uid();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("uid_"));
    }

    #[test]
    fn test_named_timer() {
        let mut timer = NamedTimer::new("test");
        std::thread::sleep(Duration::from_millis(10));

        {
            let _sub = timer.create_subtimer("operation");
            std::thread::sleep(Duration::from_millis(5));
        }

        let report = timer.report();
        assert!(report.contains("test"));
        assert!(report.contains("operation"));
    }

    #[test]
    fn test_timed_block() {
        let _block = TimedBlock::new("test_operation");
        std::thread::sleep(Duration::from_millis(10));
    }

    #[test]
    fn test_evm_constants() {
        assert_eq!(EVM::STOP, 0x00);
        assert_eq!(EVM::ADD, 0x01);
        assert_eq!(EVM::PUSH1, 0x60);
        assert_eq!(EVM::RETURN, 0xf3);
    }

    #[test]
    fn test_secp256k1n_constant() {
        assert!(!SECP256K1N_HEX.is_empty());
        assert_eq!(SECP256K1N_HEX.len(), 64);
    }

    #[test]
    fn test_opcodes_complete() {
        // Test all major opcode categories
        assert_eq!(opcode_to_string(0x00), "STOP");
        assert_eq!(opcode_to_string(0x20), "SHA3");
        assert_eq!(opcode_to_string(0x5f), "PUSH0");
        assert_eq!(opcode_to_string(0xa0), "LOG0");
        assert_eq!(opcode_to_string(0xf0), "CREATE");
        assert_eq!(opcode_to_string(0xff), "SELFDESTRUCT");
    }

    #[test]
    fn test_format_size_edge_cases() {
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1025), "1.00 KB");
    }

    #[test]
    fn test_parse_time_with_default() {
        assert_eq!(parse_time("100", Some("s")).unwrap(), 100.0);
        assert_eq!(parse_time("100", Some("m")).unwrap(), 6000.0);
    }
}
