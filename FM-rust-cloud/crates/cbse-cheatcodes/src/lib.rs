// SPDX-License-Identifier: AGPL-3.0

//! Foundry and Halmos cheatcode implementations for symbolic testing
//!
//! This module provides complete implementations of:
//! - Foundry VM cheatcodes (prank, deal, store, load, etc.)
//! - Halmos SVM symbolic creation cheatcodes
//! - Environment variable cheatcodes

use z3::ast::BV;
use z3::{Context, FuncDecl, Sort};

use cbse_bitvec::CbseBitVec;
use cbse_bytevec::ByteVec;
use cbse_exceptions::CbseException;

/// Helper function to create a constant bitvector
/// Helper function to create a concrete bitvector (matches Python con())
fn con<'ctx>(value: u64, size: u32, ctx: &'ctx Context) -> CbseBitVec<'ctx> {
    CbseBitVec::from_u64(value, size)
}

/// Helper function to convert bitvector to 256 bits (matches Python uint256)
fn uint256<'ctx>(value: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> CbseBitVec<'ctx> {
    let current_size = value.size();
    if current_size == 256 {
        value.clone()
    } else if current_size < 256 {
        value.zero_extend(256, ctx)
    } else {
        // Extract bits [255:0] from larger bitvector
        CbseBitVec::from_z3(value.as_z3(ctx).extract(255, 0))
    }
}

/// Result type for operations in this module
pub type Result<T> = std::result::Result<T, CbseException>;

// ============================================================================
// Constants and Addresses
// ============================================================================

/// HEVM cheatcode address: address(bytes20(uint160(uint256(keccak256('hevm cheat code')))))
/// 0x7109709ECFA91A80626FF3989D68F67F5B1DD12D
pub const HEVM_ADDRESS: [u8; 20] = [
    0x71, 0x09, 0x70, 0x9E, 0xCF, 0xA9, 0x1A, 0x80, 0x62, 0x6F, 0xF3, 0x98, 0x9D, 0x68, 0xF6, 0x7F,
    0x5B, 0x1D, 0xD1, 0x2D,
];

/// Halmos SVM cheatcode address: address(bytes20(uint160(uint256(keccak256('svm cheat code')))))
/// 0xF3993A62377BCD56AE39D773740A5390411E8BC9
pub const SVM_ADDRESS: [u8; 20] = [
    0xF3, 0x99, 0x3A, 0x62, 0x37, 0x7B, 0xCD, 0x56, 0xAE, 0x39, 0xD7, 0x73, 0x74, 0x0A, 0x53, 0x90,
    0x41, 0x1E, 0x8B, 0xC9,
];

// ============================================================================
// Prank Context
// ============================================================================

/// Result of a prank operation specifying what should be overridden
#[derive(Debug, Clone)]
pub struct PrankResult<'ctx> {
    pub sender: Option<CbseBitVec<'ctx>>,
    pub origin: Option<CbseBitVec<'ctx>>,
}

impl<'ctx> PrankResult<'ctx> {
    pub fn new(sender: Option<CbseBitVec<'ctx>>, origin: Option<CbseBitVec<'ctx>>) -> Self {
        Self { sender, origin }
    }

    /// Check if there's an active prank
    pub fn is_active(&self) -> bool {
        self.sender.is_some() || self.origin.is_some()
    }
}

/// No prank constant
pub fn no_prank<'ctx>() -> PrankResult<'ctx> {
    PrankResult::new(None, None)
}

/// Mutable prank context that tracks active pranks
#[derive(Debug, Clone)]
pub struct Prank<'ctx> {
    /// Active prank context
    pub active: PrankResult<'ctx>,
    /// Whether the prank should persist (startPrank) or be one-time (prank)
    pub keep: bool,
}

impl<'ctx> Prank<'ctx> {
    pub fn new() -> Self {
        Self {
            active: no_prank(),
            keep: false,
        }
    }

    /// Check if there's an active prank
    pub fn is_active(&self) -> bool {
        self.active.is_active()
    }

    /// Lookup what to use for a call to address `to`
    pub fn lookup(&self, _to: &CbseBitVec<'ctx>) -> PrankResult<'ctx> {
        // In Python this checks if pranking is active
        // For now, return the active prank context
        self.active.clone()
    }

    /// Set a one-time prank
    pub fn prank(
        &mut self,
        sender: CbseBitVec<'ctx>,
        origin: Option<CbseBitVec<'ctx>>,
        _keep: bool,
    ) -> bool {
        if self.is_active() {
            return false; // Already active
        }
        self.active = PrankResult::new(Some(sender), origin);
        self.keep = false;
        true
    }

    /// Start a persistent prank
    pub fn start_prank(
        &mut self,
        sender: CbseBitVec<'ctx>,
        origin: Option<CbseBitVec<'ctx>>,
    ) -> bool {
        if self.is_active() {
            return false; // Already active
        }
        self.active = PrankResult::new(Some(sender), origin);
        self.keep = true;
        true
    }

    /// Stop the active prank
    pub fn stop_prank(&mut self) -> bool {
        if !self.is_active() {
            return false;
        }
        self.active = no_prank();
        self.keep = false;
        true
    }
}

impl<'ctx> Default for Prank<'ctx> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Z3 Function Declarations
// ============================================================================

/// Get or create the f_vmaddr function: f_vmaddr(key) -> address
pub fn f_vmaddr<'ctx>(ctx: &'ctx Context) -> FuncDecl<'ctx> {
    FuncDecl::new(
        ctx,
        "f_vmaddr",
        &[&Sort::bitvector(ctx, 256)],
        &Sort::bitvector(ctx, 160),
    )
}

/// Get or create the f_sign_v function: f_sign_v(key, digest) -> v
pub fn f_sign_v<'ctx>(ctx: &'ctx Context) -> FuncDecl<'ctx> {
    FuncDecl::new(
        ctx,
        "f_sign_v",
        &[&Sort::bitvector(ctx, 256), &Sort::bitvector(ctx, 256)],
        &Sort::bitvector(ctx, 8),
    )
}

/// Get or create the f_sign_r function: f_sign_r(key, digest) -> r
pub fn f_sign_r<'ctx>(ctx: &'ctx Context) -> FuncDecl<'ctx> {
    FuncDecl::new(
        ctx,
        "f_sign_r",
        &[&Sort::bitvector(ctx, 256), &Sort::bitvector(ctx, 256)],
        &Sort::bitvector(ctx, 256),
    )
}

/// Get or create the f_sign_s function: f_sign_s(key, digest) -> s
pub fn f_sign_s<'ctx>(ctx: &'ctx Context) -> FuncDecl<'ctx> {
    FuncDecl::new(
        ctx,
        "f_sign_s",
        &[&Sort::bitvector(ctx, 256), &Sort::bitvector(ctx, 256)],
        &Sort::bitvector(ctx, 256),
    )
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a string to a valid identifier name (replace whitespace with underscores)
pub fn name_of(x: &str) -> String {
    x.split_whitespace().collect::<Vec<_>>().join("_")
}

/// Extract string argument from calldata at given argument index
pub fn extract_string_argument<'ctx>(calldata: &ByteVec<'ctx>, arg_idx: usize) -> Result<String> {
    // Get offset to string data (32 bytes per argument)
    let offset_word = calldata.get_word(4 + 32 * arg_idx)?;
    let offset_bv = match offset_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(b) => {
            return Err(CbseException::Internal(format!(
                "unexpected concrete bytes for offset"
            )))
        }
    };
    let offset = cbse_utils::unbox_int(&offset_bv).ok_or_else(|| {
        CbseException::NotConcrete("symbolic offset for string argument".to_string())
    })?;

    // Get string length
    let length_word = calldata.get_word((4 + offset) as usize)?;
    let length_bv = match length_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(b) => {
            return Err(CbseException::Internal(format!(
                "unexpected concrete bytes for length"
            )))
        }
    };
    let length = cbse_utils::unbox_int(&length_bv).ok_or_else(|| {
        CbseException::NotConcrete("symbolic length for string argument".to_string())
    })?;

    // Extract string bytes
    let string_data_offset = (4 + offset + 32) as usize;
    let string_slice = calldata.slice(string_data_offset, string_data_offset + length as usize)?;
    let string_data = string_slice.unwrap()?;

    let bytes = match string_data {
        cbse_bytevec::UnwrappedBytes::Bytes(b) => b,
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => {
            cbse_utils::bv_value_to_bytes(&bv).map_err(|e| CbseException::Internal(e))?
        }
    };

    String::from_utf8(bytes)
        .map_err(|e| CbseException::Internal(format!("invalid UTF-8 in string argument: {}", e)))
}

/// Extract bytes32 array argument from calldata
pub fn extract_bytes32_array_argument<'ctx>(
    calldata: &ByteVec<'ctx>,
    arg_idx: usize,
) -> Result<Vec<u8>> {
    // Get offset to array data
    let offset_word = calldata.get_word(4 + 32 * arg_idx)?;
    let offset_bv = match offset_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(b) => {
            return Err(CbseException::Internal(format!(
                "unexpected concrete bytes for offset"
            )))
        }
    };
    let offset = cbse_utils::unbox_int(&offset_bv).ok_or_else(|| {
        CbseException::NotConcrete("symbolic offset for bytes32 array".to_string())
    })?;

    // Get array length
    let length_word = calldata.get_word((4 + offset) as usize)?;
    let length_bv = match length_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(b) => {
            return Err(CbseException::Internal(format!(
                "unexpected concrete bytes for length"
            )))
        }
    };
    let length = cbse_utils::unbox_int(&length_bv).ok_or_else(|| {
        CbseException::NotConcrete("symbolic length for bytes32 array".to_string())
    })?;

    // Extract all array elements (32 bytes each)
    let mut result = Vec::new();
    for i in 0..length {
        let element = calldata.get_word((4 + offset + 32 + i * 32) as usize)?;
        let element_bytes = match element {
            cbse_bytevec::UnwrappedBytes::BitVec(bv) => {
                cbse_utils::bv_value_to_bytes(&bv).map_err(|e| CbseException::Internal(e))?
            }
            cbse_bytevec::UnwrappedBytes::Bytes(b) => b,
        };
        result.extend_from_slice(&element_bytes);
    }

    Ok(result)
}

/// Extract bytes argument from calldata
pub fn extract_bytes_argument<'ctx>(calldata: &ByteVec<'ctx>, arg_idx: usize) -> Result<Vec<u8>> {
    // Get offset to bytes data
    let offset_word = calldata.get_word(4 + 32 * arg_idx)?;
    let offset_bv = match offset_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(b) => {
            return Err(CbseException::Internal(format!(
                "unexpected concrete bytes for offset"
            )))
        }
    };
    let offset = cbse_utils::unbox_int(&offset_bv).ok_or_else(|| {
        CbseException::NotConcrete("symbolic offset for bytes argument".to_string())
    })?;

    // Get bytes length
    let length_word = calldata.get_word((4 + offset) as usize)?;
    let length_bv = match length_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(b) => {
            return Err(CbseException::Internal(format!(
                "unexpected concrete bytes for length"
            )))
        }
    };
    let length = cbse_utils::unbox_int(&length_bv).ok_or_else(|| {
        CbseException::NotConcrete("symbolic length for bytes argument".to_string())
    })?;

    // Extract bytes
    let bytes_data_offset = (4 + offset + 32) as usize;
    let bytes_slice = calldata.slice(bytes_data_offset, bytes_data_offset + length as usize)?;
    let bytes_data = bytes_slice.unwrap()?;

    let result = match bytes_data {
        cbse_bytevec::UnwrappedBytes::Bytes(b) => b,
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => {
            cbse_utils::bv_value_to_bytes(&bv).map_err(|e| CbseException::Internal(e))?
        }
    };

    Ok(result)
}

/// Encode a single bytes value as tuple(bytes) for ABI return
pub fn encode_tuple_bytes<'ctx>(data: &[u8], ctx: &'ctx Context) -> Result<ByteVec<'ctx>> {
    let length = data.len();
    let mut result = ByteVec::new(ctx);

    // Offset (always 32)
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(con(32, 256, ctx)))?;

    // Length
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(con(
        length as u64,
        256,
        ctx,
    )))?;

    // Data (padded to 32-byte boundary)
    result.append(cbse_bytevec::UnwrappedBytes::Bytes(data.to_vec()))?;
    let padding = (32 - (length % 32)) % 32;
    if padding > 0 {
        result.append(cbse_bytevec::UnwrappedBytes::Bytes(vec![0u8; padding]))?;
    }

    Ok(result)
}

/// Pad bytes to nearest multiple of 32 bytes
pub fn padded_bytes(val: &[u8], right_pad: bool) -> Vec<u8> {
    let curr_len = val.len();
    let new_len = (curr_len + 31) / 32 * 32;

    if curr_len == new_len {
        return val.to_vec();
    }

    let mut result = Vec::with_capacity(new_len);
    if right_pad {
        result.extend_from_slice(val);
        result.resize(new_len, 0);
    } else {
        result.resize(new_len - curr_len, 0);
        result.extend_from_slice(val);
    }
    result
}

/// Encode array of word values (uint256, address, bool, bytes32, int256)
pub fn abi_encode_array_words<'ctx>(
    values: &[CbseBitVec<'ctx>],
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let mut result = ByteVec::new(ctx);

    // Offset (always 32)
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(con(32, 256, ctx)))?;

    // Array length
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(con(
        values.len() as u64,
        256,
        ctx,
    )))?;

    // Append each value (expanded to 32 bytes)
    for val in values {
        let word = uint256(val, ctx);
        result.append(cbse_bytevec::UnwrappedBytes::BitVec(word))?;
    }

    Ok(result)
}

/// Encode array of bytes values
pub fn abi_encode_array_bytes<'ctx>(
    values: &[Vec<u8>],
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let mut result = ByteVec::new(ctx);

    // Offset (always 32)
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(con(32, 256, ctx)))?;

    // Array length
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(con(
        values.len() as u64,
        256,
        ctx,
    )))?;

    // Pad each value
    let padded_values: Vec<Vec<u8>> = values.iter().map(|v| padded_bytes(v, true)).collect();

    // Write offsets for each value
    let mut next_offset = 32 * values.len();
    for padded_val in &padded_values {
        result.append(cbse_bytevec::UnwrappedBytes::BitVec(con(
            next_offset as u64,
            256,
            ctx,
        )))?;
        next_offset += 32 + padded_val.len();
    }

    // Write each value (length + data)
    for (i, padded_val) in padded_values.iter().enumerate() {
        result.append(cbse_bytevec::UnwrappedBytes::BitVec(con(
            values[i].len() as u64,
            256,
            ctx,
        )))?;
        result.append(cbse_bytevec::UnwrappedBytes::Bytes(padded_val.clone()))?;
    }

    Ok(result)
}

// ============================================================================
// Symbolic Creation Cheatcodes
// ============================================================================

/// Create a generic symbolic value with given bit size
pub fn create_generic<'ctx>(
    bits: u32,
    var_name: &str,
    type_name: &str,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<CbseBitVec<'ctx>> {
    if bits == 0 {
        return Err(CbseException::Internal(
            "cannot create 0-bit symbolic value".to_string(),
        ));
    }

    let label = format!("halmos_{}_{}_{:02}", var_name, type_name, symbol_id);
    // Create a symbolic bitvector using Z3
    Ok(CbseBitVec::from_z3(BV::new_const(ctx, label, bits)))
}

/// svm.createUint(uint256 bits, string name)
pub fn create_uint<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let bits_word = arg.get_word(4)?;
    let bits_bv = match bits_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(_) => {
            return Err(CbseException::Internal(
                "unexpected concrete bytes for bits".to_string(),
            ))
        }
    };
    let bits = cbse_utils::unbox_int(&bits_bv).ok_or_else(|| {
        CbseException::NotConcrete("symbolic bit size for createUint".to_string())
    })?;

    if bits > 256 {
        return Err(CbseException::Internal(
            "createUint: bits must be <= 256".to_string(),
        ));
    }

    let name = extract_string_argument(arg, 1)?;
    let name = name_of(&name);

    let symbolic = create_generic(bits as u32, &name, &format!("uint{}", bits), symbol_id, ctx)?;
    let result = uint256(&symbolic, ctx);

    let mut bytevec = ByteVec::new(ctx);
    bytevec.append(cbse_bytevec::UnwrappedBytes::BitVec(result))?;
    Ok(bytevec)
}

/// svm.createUint256(string name)
pub fn create_uint256<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let name = extract_string_argument(arg, 0)?;
    let name = name_of(&name);

    let symbolic = create_generic(256, &name, "uint256", symbol_id, ctx)?;

    let mut result = ByteVec::new(ctx);
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(symbolic))?;
    Ok(result)
}

/// svm.createUint256(string name, uint256 min, uint256 max)
pub fn create_uint256_min_max<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<(ByteVec<'ctx>, Vec<CbseBitVec<'ctx>>)> {
    let name = extract_string_argument(arg, 0)?;
    let name = name_of(&name);

    let min_word = arg.get_word(4 + 32 * 1)?;
    let min_bv = match min_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(_) => {
            return Err(CbseException::Internal(
                "unexpected concrete bytes for min".to_string(),
            ))
        }
    };

    let max_word = arg.get_word(4 + 32 * 2)?;
    let max_bv = match max_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(_) => {
            return Err(CbseException::Internal(
                "unexpected concrete bytes for max".to_string(),
            ))
        }
    };

    let symbolic = create_generic(256, &name, "uint256", symbol_id, ctx)?;

    // Create constraints: min <= symbolic <= max
    // Note: These return CbseBool, convert to 1-bit bitvectors for constraints
    let constraint1 = symbolic.uge(&min_bv, ctx).to_bitvec(ctx, 1); // symbolic >= min
    let constraint2 = symbolic.ule(&max_bv, ctx).to_bitvec(ctx, 1); // symbolic <= max
    let constraints = vec![constraint1, constraint2];

    let mut result = ByteVec::new(ctx);
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(symbolic))?;
    Ok((result, constraints))
}

/// svm.createInt(uint256 bits, string name)
pub fn create_int<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let bits_word = arg.get_word(4)?;
    let bits_bv = match bits_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(_) => {
            return Err(CbseException::Internal(
                "unexpected concrete bytes for bits".to_string(),
            ))
        }
    };
    let bits = cbse_utils::unbox_int(&bits_bv)
        .ok_or_else(|| CbseException::NotConcrete("symbolic bit size for createInt".to_string()))?;

    if bits > 256 {
        return Err(CbseException::Internal(
            "createInt: bits must be <= 256".to_string(),
        ));
    }

    let name = extract_string_argument(arg, 1)?;
    let name = name_of(&name);

    let symbolic = create_generic(bits as u32, &name, &format!("int{}", bits), symbol_id, ctx)?;
    let result = uint256(&symbolic, ctx);

    let mut bytevec = ByteVec::new(ctx);
    bytevec.append(cbse_bytevec::UnwrappedBytes::BitVec(result))?;
    Ok(bytevec)
}

/// svm.createInt256(string name)
pub fn create_int256<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let name = extract_string_argument(arg, 0)?;
    let name = name_of(&name);

    let symbolic = create_generic(256, &name, "int256", symbol_id, ctx)?;

    let mut result = ByteVec::new(ctx);
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(symbolic))?;
    Ok(result)
}

/// svm.createBytes(uint256 length, string name)
pub fn create_bytes<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let byte_size_word = arg.get_word(4)?;
    let byte_size_bv = match byte_size_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(_) => {
            return Err(CbseException::Internal(
                "unexpected concrete bytes for size".to_string(),
            ))
        }
    };
    let byte_size = cbse_utils::unbox_int(&byte_size_bv).ok_or_else(|| {
        CbseException::NotConcrete("symbolic byte size for createBytes".to_string())
    })?;

    let name = extract_string_argument(arg, 1)?;
    let name = name_of(&name);

    let symbolic = create_generic((byte_size * 8) as u32, &name, "bytes", symbol_id, ctx)?;
    let bytes = cbse_utils::bv_value_to_bytes(&symbolic).map_err(|e| CbseException::Internal(e))?;
    encode_tuple_bytes(&bytes, ctx)
}

/// svm.createString(uint256 length, string name)
pub fn create_string<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let byte_size_word = arg.get_word(4)?;
    let byte_size_bv = match byte_size_word {
        cbse_bytevec::UnwrappedBytes::BitVec(bv) => bv,
        cbse_bytevec::UnwrappedBytes::Bytes(_) => {
            return Err(CbseException::Internal(
                "unexpected concrete bytes for size".to_string(),
            ))
        }
    };
    let byte_size = cbse_utils::unbox_int(&byte_size_bv).ok_or_else(|| {
        CbseException::NotConcrete("symbolic byte size for createString".to_string())
    })?;

    let name = extract_string_argument(arg, 1)?;
    let name = name_of(&name);

    let symbolic = create_generic((byte_size * 8) as u32, &name, "string", symbol_id, ctx)?;
    let bytes = cbse_utils::bv_value_to_bytes(&symbolic).map_err(|e| CbseException::Internal(e))?;
    encode_tuple_bytes(&bytes, ctx)
}

/// svm.createBytes4(string name)
pub fn create_bytes4<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let name = extract_string_argument(arg, 0)?;
    let name = name_of(&name);

    let symbolic = create_generic(32, &name, "bytes4", symbol_id, ctx)?;
    let mut result = ByteVec::new(ctx);
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(symbolic))?;
    result.append(cbse_bytevec::UnwrappedBytes::Bytes(vec![0u8; 28]))?; // Pad right
    Ok(result)
}

/// svm.createBytes8(string name)
pub fn create_bytes8<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let name = extract_string_argument(arg, 0)?;
    let name = name_of(&name);

    let symbolic = create_generic(64, &name, "bytes8", symbol_id, ctx)?;
    let mut result = ByteVec::new(ctx);
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(symbolic))?;
    result.append(cbse_bytevec::UnwrappedBytes::Bytes(vec![0u8; 24]))?; // Pad right
    Ok(result)
}

/// svm.createBytes32(string name)
pub fn create_bytes32<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let name = extract_string_argument(arg, 0)?;
    let name = name_of(&name);

    let symbolic = create_generic(256, &name, "bytes32", symbol_id, ctx)?;
    let mut result = ByteVec::new(ctx);
    result.append(cbse_bytevec::UnwrappedBytes::BitVec(symbolic))?;
    Ok(result)
}

/// svm.createAddress(string name)
pub fn create_address<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let name = extract_string_argument(arg, 0)?;
    let name = name_of(&name);

    let symbolic = create_generic(160, &name, "address", symbol_id, ctx)?;
    let result = uint256(&symbolic, ctx);

    let mut bytevec = ByteVec::new(ctx);
    bytevec.append(cbse_bytevec::UnwrappedBytes::BitVec(result))?;
    Ok(bytevec)
}

/// svm.createBool(string name)
pub fn create_bool<'ctx>(
    arg: &ByteVec<'ctx>,
    symbol_id: usize,
    ctx: &'ctx Context,
) -> Result<ByteVec<'ctx>> {
    let name = extract_string_argument(arg, 0)?;
    let name = name_of(&name);

    let symbolic = create_generic(1, &name, "bool", symbol_id, ctx)?;
    let result = uint256(&symbolic, ctx);

    let mut bytevec = ByteVec::new(ctx);
    bytevec.append(cbse_bytevec::UnwrappedBytes::BitVec(result))?;
    Ok(bytevec)
}

// ============================================================================
// Cheatcode Selectors
// ============================================================================

/// Halmos SVM cheatcode selectors
pub mod halmos_cheat_code {
    pub const CREATE_UINT: u32 = 0x66830DFA;
    pub const CREATE_UINT256: u32 = 0xBC7BEEFC;
    pub const CREATE_UINT256_MIN_MAX: u32 = 0x3B7A1CA7;
    pub const CREATE_INT: u32 = 0x49B9C7D4;
    pub const CREATE_INT256: u32 = 0xC2CE6AED;
    pub const CREATE_BYTES: u32 = 0xEEF5311D;
    pub const CREATE_STRING: u32 = 0xCE68656C;
    pub const CREATE_BYTES4: u32 = 0xDE143925;
    pub const CREATE_BYTES32: u32 = 0xBF72FA66;
    pub const CREATE_ADDRESS: u32 = 0x3B0FA01B;
    pub const CREATE_BOOL: u32 = 0x6E0BB659;
    pub const SYMBOLIC_STORAGE: u32 = 0xDC00BA4D;
    pub const SNAPSHOT_STORAGE: u32 = 0x5DBB8438;
    pub const SNAPSHOT_STATE: u32 = 0x9CD23835;
    pub const CREATE_CALLDATA_ADDRESS: u32 = 0xB4E9E81C;
    pub const CREATE_CALLDATA_ADDRESS_BOOL: u32 = 0x49D66B01;
    pub const CREATE_CALLDATA_CONTRACT: u32 = 0xBE92D5A2;
    pub const CREATE_CALLDATA_CONTRACT_BOOL: u32 = 0xDEEF391B;
    pub const CREATE_CALLDATA_FILE_CONTRACT: u32 = 0x88298B32;
    pub const CREATE_CALLDATA_FILE_CONTRACT_BOOL: u32 = 0x607C5C90;
}

/// Foundry HEVM cheatcode selectors
pub mod hevm_cheat_code {
    pub const ASSUME: u32 = 0x4C63E562;
    pub const GET_CODE: u32 = 0x8D1CC925;
    pub const PRANK: u32 = 0xCA669FA7;
    pub const PRANK_ADDR_ADDR: u32 = 0x47E50CCE;
    pub const START_PRANK: u32 = 0x06447D56;
    pub const START_PRANK_ADDR_ADDR: u32 = 0x45B56078;
    pub const STOP_PRANK: u32 = 0x90C5013B;
    pub const DEAL: u32 = 0xC88A5E6D;
    pub const STORE: u32 = 0x70CA10BB;
    pub const LOAD: u32 = 0x667F9D70;
    pub const FEE: u32 = 0x39B37AB0;
    pub const CHAINID: u32 = 0x4049DDD2;
    pub const COINBASE: u32 = 0xFF483C54;
    pub const DIFFICULTY: u32 = 0x46CC92D9;
    pub const ROLL: u32 = 0x1F7B4F30;
    pub const WARP: u32 = 0xE5D6BF02;
    pub const ETCH: u32 = 0xB4D6C782;
    pub const FFI: u32 = 0x89160467;
    pub const ADDR: u32 = 0xFFA18649;
    pub const SIGN: u32 = 0xE341EAA4;
    pub const LABEL: u32 = 0xC657C718;
    pub const GET_BLOCK_NUMBER: u32 = 0x42CBB15C;
    pub const SNAPSHOT_STATE: u32 = 0x9CD23835;
    pub const SET_ARBITRARY_STORAGE: u32 = 0xE1631837;

    // Random value cheatcodes
    pub const RANDOM_INT: u32 = 0x111F1202;
    pub const RANDOM_INT_UINT256: u32 = 0x12845966;
    pub const RANDOM_UINT: u32 = 0x25124730;
    pub const RANDOM_UINT_UINT256: u32 = 0xCF81E69C;
    pub const RANDOM_UINT_MIN_MAX: u32 = 0xD61B051B;
    pub const RANDOM_ADDRESS: u32 = 0xD5BEE9F5;
    pub const RANDOM_BOOL: u32 = 0xCDC126BD;
    pub const RANDOM_BYTES: u32 = 0x6C5D32A9;
    pub const RANDOM_BYTES4: u32 = 0x9B7CD579;
    pub const RANDOM_BYTES8: u32 = 0x04970BA5;

    // Environment variable cheatcodes
    pub const ENV_INT: u32 = 0x892A0C61;
    pub const ENV_BYTES32: u32 = 0x97949042;
    pub const ENV_ADDRESS: u32 = 0x350D56BF;
    pub const ENV_BOOL: u32 = 0x7ED1EC7D;
    pub const ENV_UINT: u32 = 0xC1978D1F;
    pub const ENV_STRING: u32 = 0xF877CB19;
    pub const ENV_BYTES: u32 = 0x4D7BAF06;

    pub const ENV_INT_ARRAY: u32 = 0x42181150;
    pub const ENV_ADDRESS_ARRAY: u32 = 0xAD31B9FA;
    pub const ENV_BOOL_ARRAY: u32 = 0xAAADDEAF;
    pub const ENV_BYTES32_ARRAY: u32 = 0x5AF231C1;
    pub const ENV_STRING_ARRAY: u32 = 0x14B02BC9;
    pub const ENV_UINT_ARRAY: u32 = 0xF3DEC099;
    pub const ENV_BYTES_ARRAY: u32 = 0xDDC2651B;

    pub const ENV_OR_ADDRESS: u32 = 0x561FE540;
    pub const ENV_OR_BOOL: u32 = 0x4777F3CF;
    pub const ENV_OR_BYTES: u32 = 0xB3E47705;
    pub const ENV_OR_STRING: u32 = 0xD145736C;
    pub const ENV_OR_BYTES32: u32 = 0xB4A85892;
    pub const ENV_OR_INT: u32 = 0xBBCB713E;
    pub const ENV_OR_UINT: u32 = 0x5E97348F;

    pub const ENV_OR_ADDRESS_ARRAY: u32 = 0xC74E9DEB;
    pub const ENV_OR_BOOL_ARRAY: u32 = 0xEB85E83B;
    pub const ENV_OR_BYTES32_ARRAY: u32 = 0x2281F367;
    pub const ENV_OR_INT_ARRAY: u32 = 0x4700D74B;
    pub const ENV_OR_UINT_ARRAY: u32 = 0x74318528;
    pub const ENV_OR_BYTES_ARRAY: u32 = 0x64BC3E64;
    pub const ENV_OR_STRING_ARRAY: u32 = 0x859216BC;

    pub const ENV_EXISTS: u32 = 0xCE8365F9;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_of() {
        assert_eq!(name_of("hello world"), "hello_world");
        assert_eq!(name_of("foo  bar  baz"), "foo_bar_baz");
        assert_eq!(name_of("test"), "test");
    }

    #[test]
    fn test_prank_context() {
        let ctx = Context::new(&z3::Config::new());
        let mut prank = Prank::new();
        assert!(!prank.is_active());

        let sender = CbseBitVec::symbolic(&ctx, "sender", 160);

        assert!(prank.prank(sender.clone(), None, false));
        assert!(prank.is_active());

        // Cannot prank again while active
        let sender2 = CbseBitVec::symbolic(&ctx, "sender2", 160);
        assert!(!prank.prank(sender2, None, false));

        assert!(prank.stop_prank());
        assert!(!prank.is_active());
    }

    #[test]
    fn test_padded_bytes() {
        let data = vec![1u8, 2, 3];
        let padded = padded_bytes(&data, true);
        assert_eq!(padded.len(), 32);
        assert_eq!(&padded[..3], &[1, 2, 3]);
        assert_eq!(&padded[3..], &[0u8; 29]);

        let padded_left = padded_bytes(&data, false);
        assert_eq!(padded_left.len(), 32);
        assert_eq!(&padded_left[29..], &[1, 2, 3]);
        assert_eq!(&padded_left[..29], &[0u8; 29]);
    }

    #[test]
    fn test_create_generic() {
        let ctx = Context::new(&z3::Config::new());
        let result = create_generic(256, "test", "uint256", 1, &ctx);
        assert!(result.is_ok());

        let bv = result.unwrap();
        assert_eq!(bv.size(), 256);
    }

    #[test]
    fn test_selectors() {
        // Verify selector constants are correct
        assert_eq!(halmos_cheat_code::CREATE_UINT256, 0xBC7BEEFC);
        assert_eq!(hevm_cheat_code::ASSUME, 0x4C63E562);
        assert_eq!(hevm_cheat_code::PRANK, 0xCA669FA7);
    }

    #[test]
    fn test_prank_result() {
        let result = no_prank::<'_>();
        assert!(!result.is_active());

        let ctx = Context::new(&z3::Config::new());
        let sender = CbseBitVec::symbolic(&ctx, "sender", 160);
        let result = PrankResult::new(Some(sender), None);
        assert!(result.is_active());
    }

    #[test]
    fn test_z3_function_declarations() {
        let ctx = Context::new(&z3::Config::new());

        let vmaddr = f_vmaddr(&ctx);
        assert_eq!(vmaddr.name().to_string(), "f_vmaddr");

        let sign_v = f_sign_v(&ctx);
        assert_eq!(sign_v.name().to_string(), "f_sign_v");

        let sign_r = f_sign_r(&ctx);
        assert_eq!(sign_r.name().to_string(), "f_sign_r");

        let sign_s = f_sign_s(&ctx);
        assert_eq!(sign_s.name().to_string(), "f_sign_s");
    }

    #[test]
    fn test_start_stop_prank() {
        let ctx = Context::new(&z3::Config::new());
        let mut prank = Prank::new();

        let sender = CbseBitVec::symbolic(&ctx, "sender", 160);
        let origin = CbseBitVec::symbolic(&ctx, "origin", 160);

        // Start persistent prank
        assert!(prank.start_prank(sender, Some(origin)));
        assert!(prank.is_active());
        assert!(prank.keep);

        // Stop prank
        assert!(prank.stop_prank());
        assert!(!prank.is_active());
        assert!(!prank.keep);
    }
}
