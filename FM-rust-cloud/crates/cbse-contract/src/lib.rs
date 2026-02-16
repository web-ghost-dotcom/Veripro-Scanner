// SPDX-License-Identifier: AGPL-3.0

use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use cbse_bitvec::CbseBitVec;
use cbse_bytevec::{ByteVec, UnwrappedBytes};
use cbse_constants::MAX_MEMORY_SIZE;
use cbse_exceptions::CbseException;
use cbse_utils::{hexify, stripped};
use z3::Context;

/// Helper function to convert bitvector to 256 bits
fn uint256<'ctx>(value: &CbseBitVec<'ctx>, ctx: &'ctx Context) -> CbseBitVec<'ctx> {
    let current_size = value.size();
    if current_size == 256 {
        value.clone()
    } else if current_size < 256 {
        value.zero_extend(256, ctx)
    } else {
        CbseBitVec::from_z3(value.as_z3(ctx).extract(255, 0))
    }
}

/// Returns true if the value is concrete (not symbolic)
fn is_concrete<'ctx>(val: &CbseBitVec<'ctx>) -> bool {
    val.is_concrete()
}

/// Converts a bitvector to an integer, raising an error if symbolic
fn int_of<'ctx>(val: &CbseBitVec<'ctx>, msg: &str) -> Result<usize, CbseException> {
    val.as_biguint()
        .map(|bigint| bigint.to_u64_digits().first().copied().unwrap_or(0) as usize)
        .map_err(|_| CbseException::NotConcrete(msg.to_string()))
}

/// Returns the mnemonic for an opcode
fn str_opcode(opcode: u8) -> String {
    match opcode {
        OP_STOP => "STOP".to_string(),
        OP_ADD => "ADD".to_string(),
        OP_MUL => "MUL".to_string(),
        OP_SUB => "SUB".to_string(),
        OP_DIV => "DIV".to_string(),
        OP_SDIV => "SDIV".to_string(),
        OP_MOD => "MOD".to_string(),
        OP_SMOD => "SMOD".to_string(),
        OP_ADDMOD => "ADDMOD".to_string(),
        OP_MULMOD => "MULMOD".to_string(),
        OP_EXP => "EXP".to_string(),
        OP_SIGNEXTEND => "SIGNEXTEND".to_string(),
        OP_LT => "LT".to_string(),
        OP_GT => "GT".to_string(),
        OP_SLT => "SLT".to_string(),
        OP_SGT => "SGT".to_string(),
        OP_EQ => "EQ".to_string(),
        OP_ISZERO => "ISZERO".to_string(),
        OP_AND => "AND".to_string(),
        OP_OR => "OR".to_string(),
        OP_XOR => "XOR".to_string(),
        OP_NOT => "NOT".to_string(),
        OP_BYTE => "BYTE".to_string(),
        OP_SHL => "SHL".to_string(),
        OP_SHR => "SHR".to_string(),
        OP_SAR => "SAR".to_string(),
        OP_SHA3 => "SHA3".to_string(),
        OP_ADDRESS => "ADDRESS".to_string(),
        OP_BALANCE => "BALANCE".to_string(),
        OP_ORIGIN => "ORIGIN".to_string(),
        OP_CALLER => "CALLER".to_string(),
        OP_CALLVALUE => "CALLVALUE".to_string(),
        OP_CALLDATALOAD => "CALLDATALOAD".to_string(),
        OP_CALLDATASIZE => "CALLDATASIZE".to_string(),
        OP_CALLDATACOPY => "CALLDATACOPY".to_string(),
        OP_CODESIZE => "CODESIZE".to_string(),
        OP_CODECOPY => "CODECOPY".to_string(),
        OP_GASPRICE => "GASPRICE".to_string(),
        OP_EXTCODESIZE => "EXTCODESIZE".to_string(),
        OP_EXTCODECOPY => "EXTCODECOPY".to_string(),
        OP_RETURNDATASIZE => "RETURNDATASIZE".to_string(),
        OP_RETURNDATACOPY => "RETURNDATACOPY".to_string(),
        OP_EXTCODEHASH => "EXTCODEHASH".to_string(),
        OP_BLOCKHASH => "BLOCKHASH".to_string(),
        OP_COINBASE => "COINBASE".to_string(),
        OP_TIMESTAMP => "TIMESTAMP".to_string(),
        OP_NUMBER => "NUMBER".to_string(),
        OP_DIFFICULTY => "DIFFICULTY".to_string(),
        OP_GASLIMIT => "GASLIMIT".to_string(),
        OP_CHAINID => "CHAINID".to_string(),
        OP_SELFBALANCE => "SELFBALANCE".to_string(),
        OP_BASEFEE => "BASEFEE".to_string(),
        OP_POP => "POP".to_string(),
        OP_MLOAD => "MLOAD".to_string(),
        OP_MSTORE => "MSTORE".to_string(),
        OP_MSTORE8 => "MSTORE8".to_string(),
        OP_SLOAD => "SLOAD".to_string(),
        OP_SSTORE => "SSTORE".to_string(),
        OP_JUMP => "JUMP".to_string(),
        OP_JUMPI => "JUMPI".to_string(),
        OP_PC => "PC".to_string(),
        OP_MSIZE => "MSIZE".to_string(),
        OP_GAS => "GAS".to_string(),
        OP_JUMPDEST => "JUMPDEST".to_string(),
        OP_TLOAD => "TLOAD".to_string(),
        OP_TSTORE => "TSTORE".to_string(),
        OP_MCOPY => "MCOPY".to_string(),
        OP_PUSH0 => "PUSH0".to_string(),
        n @ OP_PUSH1..=OP_PUSH32 => format!("PUSH{}", n - OP_PUSH0),
        n @ OP_DUP1..=OP_DUP16 => format!("DUP{}", n - OP_DUP1 + 1),
        n @ OP_SWAP1..=OP_SWAP16 => format!("SWAP{}", n - OP_SWAP1 + 1),
        n @ OP_LOG0..=OP_LOG4 => format!("LOG{}", n - OP_LOG0),
        OP_CREATE => "CREATE".to_string(),
        OP_CALL => "CALL".to_string(),
        OP_CALLCODE => "CALLCODE".to_string(),
        OP_RETURN => "RETURN".to_string(),
        OP_DELEGATECALL => "DELEGATECALL".to_string(),
        OP_CREATE2 => "CREATE2".to_string(),
        OP_STATICCALL => "STATICCALL".to_string(),
        OP_REVERT => "REVERT".to_string(),
        OP_INVALID => "INVALID".to_string(),
        OP_SELFDESTRUCT => "SELFDESTRUCT".to_string(),
        _ => format!("0x{:02x}", opcode),
    }
}

// All EVM opcodes
pub const OP_STOP: u8 = 0x00;
pub const OP_ADD: u8 = 0x01;
pub const OP_MUL: u8 = 0x02;
pub const OP_SUB: u8 = 0x03;
pub const OP_DIV: u8 = 0x04;
pub const OP_SDIV: u8 = 0x05;
pub const OP_MOD: u8 = 0x06;
pub const OP_SMOD: u8 = 0x07;
pub const OP_ADDMOD: u8 = 0x08;
pub const OP_MULMOD: u8 = 0x09;
pub const OP_EXP: u8 = 0x0A;
pub const OP_SIGNEXTEND: u8 = 0x0B;
pub const OP_LT: u8 = 0x10;
pub const OP_GT: u8 = 0x11;
pub const OP_SLT: u8 = 0x12;
pub const OP_SGT: u8 = 0x13;
pub const OP_EQ: u8 = 0x14;
pub const OP_ISZERO: u8 = 0x15;
pub const OP_AND: u8 = 0x16;
pub const OP_OR: u8 = 0x17;
pub const OP_XOR: u8 = 0x18;
pub const OP_NOT: u8 = 0x19;
pub const OP_BYTE: u8 = 0x1A;
pub const OP_SHL: u8 = 0x1B;
pub const OP_SHR: u8 = 0x1C;
pub const OP_SAR: u8 = 0x1D;
pub const OP_SHA3: u8 = 0x20;
pub const OP_ADDRESS: u8 = 0x30;
pub const OP_BALANCE: u8 = 0x31;
pub const OP_ORIGIN: u8 = 0x32;
pub const OP_CALLER: u8 = 0x33;
pub const OP_CALLVALUE: u8 = 0x34;
pub const OP_CALLDATALOAD: u8 = 0x35;
pub const OP_CALLDATASIZE: u8 = 0x36;
pub const OP_CALLDATACOPY: u8 = 0x37;
pub const OP_CODESIZE: u8 = 0x38;
pub const OP_CODECOPY: u8 = 0x39;
pub const OP_GASPRICE: u8 = 0x3A;
pub const OP_EXTCODESIZE: u8 = 0x3B;
pub const OP_EXTCODECOPY: u8 = 0x3C;
pub const OP_RETURNDATASIZE: u8 = 0x3D;
pub const OP_RETURNDATACOPY: u8 = 0x3E;
pub const OP_EXTCODEHASH: u8 = 0x3F;
pub const OP_BLOCKHASH: u8 = 0x40;
pub const OP_COINBASE: u8 = 0x41;
pub const OP_TIMESTAMP: u8 = 0x42;
pub const OP_NUMBER: u8 = 0x43;
pub const OP_DIFFICULTY: u8 = 0x44;
pub const OP_GASLIMIT: u8 = 0x45;
pub const OP_CHAINID: u8 = 0x46;
pub const OP_SELFBALANCE: u8 = 0x47;
pub const OP_BASEFEE: u8 = 0x48;
pub const OP_POP: u8 = 0x50;
pub const OP_MLOAD: u8 = 0x51;
pub const OP_MSTORE: u8 = 0x52;
pub const OP_MSTORE8: u8 = 0x53;
pub const OP_SLOAD: u8 = 0x54;
pub const OP_SSTORE: u8 = 0x55;
pub const OP_JUMP: u8 = 0x56;
pub const OP_JUMPI: u8 = 0x57;
pub const OP_PC: u8 = 0x58;
pub const OP_MSIZE: u8 = 0x59;
pub const OP_GAS: u8 = 0x5A;
pub const OP_JUMPDEST: u8 = 0x5B;
pub const OP_TLOAD: u8 = 0x5C;
pub const OP_TSTORE: u8 = 0x5D;
pub const OP_MCOPY: u8 = 0x5E;
pub const OP_PUSH0: u8 = 0x5F;
pub const OP_PUSH1: u8 = 0x60;
pub const OP_PUSH2: u8 = 0x61;
pub const OP_PUSH3: u8 = 0x62;
pub const OP_PUSH4: u8 = 0x63;
pub const OP_PUSH5: u8 = 0x64;
pub const OP_PUSH6: u8 = 0x65;
pub const OP_PUSH7: u8 = 0x66;
pub const OP_PUSH8: u8 = 0x67;
pub const OP_PUSH9: u8 = 0x68;
pub const OP_PUSH10: u8 = 0x69;
pub const OP_PUSH11: u8 = 0x6A;
pub const OP_PUSH12: u8 = 0x6B;
pub const OP_PUSH13: u8 = 0x6C;
pub const OP_PUSH14: u8 = 0x6D;
pub const OP_PUSH15: u8 = 0x6E;
pub const OP_PUSH16: u8 = 0x6F;
pub const OP_PUSH17: u8 = 0x70;
pub const OP_PUSH18: u8 = 0x71;
pub const OP_PUSH19: u8 = 0x72;
pub const OP_PUSH20: u8 = 0x73;
pub const OP_PUSH21: u8 = 0x74;
pub const OP_PUSH22: u8 = 0x75;
pub const OP_PUSH23: u8 = 0x76;
pub const OP_PUSH24: u8 = 0x77;
pub const OP_PUSH25: u8 = 0x78;
pub const OP_PUSH26: u8 = 0x79;
pub const OP_PUSH27: u8 = 0x7A;
pub const OP_PUSH28: u8 = 0x7B;
pub const OP_PUSH29: u8 = 0x7C;
pub const OP_PUSH30: u8 = 0x7D;
pub const OP_PUSH31: u8 = 0x7E;
pub const OP_PUSH32: u8 = 0x7F;
pub const OP_DUP1: u8 = 0x80;
pub const OP_DUP2: u8 = 0x81;
pub const OP_DUP3: u8 = 0x82;
pub const OP_DUP4: u8 = 0x83;
pub const OP_DUP5: u8 = 0x84;
pub const OP_DUP6: u8 = 0x85;
pub const OP_DUP7: u8 = 0x86;
pub const OP_DUP8: u8 = 0x87;
pub const OP_DUP9: u8 = 0x88;
pub const OP_DUP10: u8 = 0x89;
pub const OP_DUP11: u8 = 0x8A;
pub const OP_DUP12: u8 = 0x8B;
pub const OP_DUP13: u8 = 0x8C;
pub const OP_DUP14: u8 = 0x8D;
pub const OP_DUP15: u8 = 0x8E;
pub const OP_DUP16: u8 = 0x8F;
pub const OP_SWAP1: u8 = 0x90;
pub const OP_SWAP2: u8 = 0x91;
pub const OP_SWAP3: u8 = 0x92;
pub const OP_SWAP4: u8 = 0x93;
pub const OP_SWAP5: u8 = 0x94;
pub const OP_SWAP6: u8 = 0x95;
pub const OP_SWAP7: u8 = 0x96;
pub const OP_SWAP8: u8 = 0x97;
pub const OP_SWAP9: u8 = 0x98;
pub const OP_SWAP10: u8 = 0x99;
pub const OP_SWAP11: u8 = 0x9A;
pub const OP_SWAP12: u8 = 0x9B;
pub const OP_SWAP13: u8 = 0x9C;
pub const OP_SWAP14: u8 = 0x9D;
pub const OP_SWAP15: u8 = 0x9E;
pub const OP_SWAP16: u8 = 0x9F;
pub const OP_LOG0: u8 = 0xA0;
pub const OP_LOG1: u8 = 0xA1;
pub const OP_LOG2: u8 = 0xA2;
pub const OP_LOG3: u8 = 0xA3;
pub const OP_LOG4: u8 = 0xA4;
pub const OP_CREATE: u8 = 0xF0;
pub const OP_CALL: u8 = 0xF1;
pub const OP_CALLCODE: u8 = 0xF2;
pub const OP_RETURN: u8 = 0xF3;
pub const OP_DELEGATECALL: u8 = 0xF4;
pub const OP_CREATE2: u8 = 0xF5;
pub const OP_STATICCALL: u8 = 0xFA;
pub const OP_REVERT: u8 = 0xFD;
pub const OP_INVALID: u8 = 0xFE;
pub const OP_SELFDESTRUCT: u8 = 0xFF;

// Opcode groups
pub const CALL_OPCODES: &[u8] = &[OP_CALL, OP_CALLCODE, OP_DELEGATECALL, OP_STATICCALL];
pub const CREATE_OPCODES: &[u8] = &[OP_CREATE, OP_CREATE2];
pub const TERMINATING_OPCODES: &[u8] = &[OP_STOP, OP_RETURN, OP_REVERT, OP_INVALID];

// ERC-1167 minimal proxy constants
const ERC1167_PREFIX: &[u8] = &[0x36, 0x3d, 0x3d, 0x37, 0x3d, 0x3d, 0x3d, 0x36, 0x3d, 0x73];
const ERC1167_SUFFIX: &[u8] = &[
    0x5a, 0xf4, 0x3d, 0x82, 0x80, 0x3e, 0x90, 0x3d, 0x91, 0x60, 0x2b, 0x57, 0xfd, 0x5b, 0xf3,
];

/// Returns the length of an instruction with the given opcode
pub fn insn_len(opcode: u8) -> usize {
    if (OP_PUSH1..=OP_PUSH32).contains(&opcode) {
        1 + (opcode - OP_PUSH0) as usize
    } else {
        1
    }
}

/// Returns a human-readable mnemonic for an opcode
pub fn mnemonic(opcode: u8) -> String {
    str_opcode(opcode)
}

/// Represents a single EVM instruction with its metadata
#[derive(Clone, Debug)]
pub struct Instruction<'ctx> {
    pub opcode: u8,
    pub pc: isize,
    pub next_pc: isize,
    pub operand: Option<CbseBitVec<'ctx>>,
    pub source_file: Option<String>,
    pub source_line: Option<usize>,
}

impl<'ctx> Instruction<'ctx> {
    /// Creates a new instruction
    pub fn new(opcode: u8, pc: isize, next_pc: isize, operand: Option<CbseBitVec<'ctx>>) -> Self {
        Self {
            opcode,
            pc,
            next_pc,
            operand,
            source_file: None,
            source_line: None,
        }
    }

    /// Returns the STOP singleton instruction
    pub fn stop(ctx: &'ctx Context) -> Self {
        Self::new(OP_STOP, -1, -1, None)
    }

    /// Returns the length of this instruction in bytes
    pub fn len(&self) -> usize {
        insn_len(self.opcode)
    }

    /// Sets the source mapping for this instruction
    pub fn set_srcmap(&mut self, source_file: Option<String>, source_line: Option<usize>) {
        self.source_file = source_file;
        self.source_line = source_line;
    }

    /// Returns a string representation of this instruction
    pub fn to_string(&self, ctx: &'ctx Context) -> String {
        if let Some(ref operand) = self.operand {
            let operand_size_bytes = self.len() - 1;
            // Convert bitvector to bytes for hexification
            if let Ok(bytes) = cbse_utils::bv_value_to_bytes(operand) {
                format!("{} {}", mnemonic(self.opcode), hexify(&bytes))
            } else {
                // If conversion fails (symbolic value), show as symbolic
                format!("{} <symbolic>", mnemonic(self.opcode))
            }
        } else {
            mnemonic(self.opcode)
        }
    }
}

/// Abstraction over contract bytecode with instruction decoding
pub struct Contract<'ctx> {
    code: ByteVec<'ctx>,
    fastcode: Option<Vec<u8>>,
    insn: Vec<Option<Instruction<'ctx>>>,
    jumpdests: Option<HashSet<usize>>,
    ctx: &'ctx Context,

    pub contract_name: Option<String>,
    pub filename: Option<String>,
    pub source_map: Option<String>,
}

impl<'ctx> Contract<'ctx> {
    /// Creates a new contract from bytecode
    pub fn new(
        code: ByteVec<'ctx>,
        ctx: &'ctx Context,
        contract_name: Option<String>,
        filename: Option<String>,
        source_map: Option<String>,
    ) -> Self {
        let len = code.len();

        // Extract concrete prefix for fast access - try to unwrap and get concrete bytes
        let fastcode = code.unwrap().ok().and_then(|unwrapped| match unwrapped {
            UnwrappedBytes::Bytes(bytes) => Some(bytes),
            _ => None,
        });

        Self {
            code,
            fastcode,
            insn: vec![None; len],
            jumpdests: None,
            ctx,
            contract_name,
            filename,
            source_map,
        }
    }

    /// Creates a contract from hex string
    pub fn from_hexcode(hexcode: &str, ctx: &'ctx Context) -> Result<Self, CbseException> {
        if hexcode.len() % 2 != 0 {
            return Err(CbseException::Internal(format!(
                "Invalid hex length: {}",
                hexcode
            )));
        }

        if hexcode.contains("__") {
            eprintln!("Warning: contract hexcode contains library placeholder");
        }

        let stripped_hex = stripped(hexcode);
        let bytes = hex::decode(stripped_hex).map_err(|e| {
            CbseException::Internal(format!("Invalid hex: {} (hexcode={})", e, hexcode))
        })?;

        Ok(Self::new(
            ByteVec::from_bytes(bytes, ctx)?,
            ctx,
            None,
            None,
            None,
        ))
    }

    /// Scans the bytecode for valid jump destinations
    fn get_jumpdests(&self) -> HashSet<usize> {
        let mut jumpdests = HashSet::new();
        let mut pc = 0;

        // Try fastcode first for performance
        if let Some(ref fastcode) = self.fastcode {
            let n = fastcode.len();
            while pc < n {
                let opcode = fastcode[pc];
                if opcode == OP_JUMPDEST {
                    jumpdests.insert(pc);
                    pc += 1;
                } else {
                    pc += insn_len(opcode);
                }
            }
            return jumpdests;
        }

        // Fallback to slow path with symbolic code
        let n = self.code.len();
        while pc < n {
            match self.get_byte(pc) {
                Ok(opcode) => {
                    if opcode == OP_JUMPDEST {
                        jumpdests.insert(pc);
                        pc += 1;
                    } else {
                        pc += insn_len(opcode);
                    }
                }
                Err(_) => break, // Stop on error or symbolic byte
            }
        }

        jumpdests
    }

    /// Processes source mapping and adds location info to instructions
    pub fn process_source_mapping(&mut self, ctx: &'ctx Context) {
        let source_map = match &self.source_map {
            Some(sm) => sm.clone(),
            None => return,
        };

        let mut pc = 0;
        let mut byte_offset = 0;
        let mut file_id = 0;

        for item in source_map.split(';') {
            let data: Vec<&str> = item.split(':').collect();

            // Update byte_offset and file_id if present
            if !data.is_empty() && !data[0].is_empty() {
                byte_offset = data[0].parse().unwrap_or(byte_offset);
            }
            if data.len() > 2 && !data[2].is_empty() {
                file_id = data[2].parse().unwrap_or(file_id);
            }

            // Get location from source file map (would need implementation)
            // let (file_path, line_number) = SourceFileMap::instance().get_location(file_id, byte_offset);
            // CoverageReporter::instance().record_lines_found(&file_path, line_number);

            // Decode instruction and set source mapping
            if let Ok(mut insn) = self.decode_instruction(pc, ctx) {
                // insn.set_srcmap(Some(file_path), Some(line_number));
                pc = insn.next_pc as usize;
            } else {
                break;
            }
        }
    }

    /// Decodes instruction at given PC (internal, no caching)
    fn decode_instruction_internal(
        &self,
        pc: usize,
        ctx: &'ctx Context,
    ) -> Result<Instruction<'ctx>, CbseException> {
        let opcode = self.get_byte(pc)?;
        let length = insn_len(opcode);
        let next_pc = pc + length;

        if length > 1 {
            let operand = self.unwrapped_slice(pc + 1, next_pc, ctx)?;
            let operand_256 = uint256(&operand, ctx);
            Ok(Instruction::new(
                opcode,
                pc as isize,
                next_pc as isize,
                Some(operand_256),
            ))
        } else {
            Ok(Instruction::new(
                opcode,
                pc as isize,
                next_pc as isize,
                None,
            ))
        }
    }

    /// Decodes instruction at given PC with caching
    pub fn decode_instruction(
        &mut self,
        pc: usize,
        ctx: &'ctx Context,
    ) -> Result<Instruction<'ctx>, CbseException> {
        // Check cache
        if pc < self.insn.len() {
            if let Some(ref insn) = self.insn[pc] {
                return Ok(insn.clone());
            }
        } else if pc >= self.code.len() {
            return Ok(Instruction::stop(ctx));
        } else {
            return Err(CbseException::Internal(format!("invalid pc={}", pc)));
        }

        // Decode and cache
        let insn = self.decode_instruction_internal(pc, ctx)?;
        if pc < self.insn.len() {
            self.insn[pc] = Some(insn.clone());
        }
        Ok(insn)
    }

    /// Returns the next PC after the instruction at the given PC
    pub fn next_pc(&mut self, pc: usize, ctx: &'ctx Context) -> Result<usize, CbseException> {
        Ok(self.decode_instruction(pc, ctx)?.next_pc as usize)
    }

    /// Slices the bytecode
    pub fn slice(&self, start: usize, size: usize) -> Result<ByteVec<'ctx>, CbseException> {
        if size > MAX_MEMORY_SIZE {
            return Err(CbseException::Internal(format!(
                "code read exceeds MAX_MEMORY_SIZE"
            )));
        }

        let stop = start + size;

        // Fast path for concrete prefix
        if let Some(ref fastcode) = self.fastcode {
            if stop <= fastcode.len() {
                return ByteVec::from_bytes(fastcode[start..stop].to_vec(), self.ctx);
            }
        }

        self.code.slice(start, stop)
    }

    /// Returns unwrapped BV slice
    pub fn unwrapped_slice(
        &self,
        start: usize,
        stop: usize,
        ctx: &'ctx Context,
    ) -> Result<CbseBitVec<'ctx>, CbseException> {
        // Fast path for concrete prefix
        if let Some(ref fastcode) = self.fastcode {
            if stop <= fastcode.len() {
                let size = (stop - start) * 8; // bits
                return Ok(CbseBitVec::from_bytes(&fastcode[start..stop], size as u32));
            }
        }

        let slice = self.code.slice(start, stop)?;
        match slice.unwrap()? {
            UnwrappedBytes::Bytes(bytes) => {
                let size = bytes.len() * 8;
                Ok(CbseBitVec::from_bytes(&bytes, size as u32))
            }
            UnwrappedBytes::BitVec(bv) => Ok(bv),
        }
    }

    /// Returns byte at given index
    pub fn get_byte(&self, key: usize) -> Result<u8, CbseException> {
        // Fast path
        if let Some(ref fastcode) = self.fastcode {
            if key < fastcode.len() {
                return Ok(fastcode[key]);
            }
        }

        // Slow path - try to get byte from ByteVec
        match self.code.get_byte(key) {
            Ok(byte) => match byte {
                UnwrappedBytes::Bytes(bytes) if !bytes.is_empty() => Ok(bytes[0]),
                _ => Err(CbseException::NotConcrete("symbolic byte".to_string())),
            },
            Err(e) => Err(e),
        }
    }

    /// Returns the length of the bytecode
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// Returns the set of valid jump destinations
    pub fn valid_jumpdests(&mut self) -> &HashSet<usize> {
        if self.jumpdests.is_none() {
            self.jumpdests = Some(self.get_jumpdests());
        }
        self.jumpdests.as_ref().unwrap()
    }

    /// Extracts the target address from an ERC-1167 minimal proxy contract
    pub fn extract_erc1167_target(&self, _ctx: &'ctx Context) -> Option<[u8; 20]> {
        let m = ERC1167_PREFIX.len();
        let n = ERC1167_SUFFIX.len();
        let erc1167_len = m + 20 + n;

        if self.code.len() != erc1167_len {
            return None;
        }

        // Check prefix - compare concrete bytes
        if let Ok(prefix_slice) = self.slice(0, m) {
            if let Ok(UnwrappedBytes::Bytes(bytes)) = prefix_slice.unwrap() {
                if bytes != ERC1167_PREFIX {
                    return None;
                }
            } else {
                return None;
            }
        } else {
            return None;
        }

        // Check suffix
        if let Ok(suffix_slice) = self.slice(m + 20, n) {
            if let Ok(UnwrappedBytes::Bytes(bytes)) = suffix_slice.unwrap() {
                if bytes != ERC1167_SUFFIX {
                    return None;
                }
            } else {
                return None;
            }
        } else {
            return None;
        }

        // Extract 20-byte address
        if let Ok(target) = self.slice(m, 20) {
            match target.unwrap() {
                Ok(UnwrappedBytes::Bytes(bytes)) => {
                    if bytes.len() >= 20 {
                        let mut addr = [0u8; 20];
                        addr.copy_from_slice(&bytes[..20]);
                        Some(addr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

/// Singleton for tracking test coverage
pub struct CoverageReporter {
    instruction_coverage_data: Mutex<HashMap<String, HashMap<usize, usize>>>,
}

impl CoverageReporter {
    /// Returns the global singleton instance
    pub fn instance() -> &'static CoverageReporter {
        static INSTANCE: Lazy<CoverageReporter> = Lazy::new(|| CoverageReporter {
            instruction_coverage_data: Mutex::new(HashMap::new()),
        });
        &INSTANCE
    }

    /// Records that a line was found in the source
    pub fn record_lines_found(&self, file_path: &str, line_number: usize) {
        let mut data = self.instruction_coverage_data.lock().unwrap();
        data.entry(file_path.to_string())
            .or_insert_with(HashMap::new)
            .entry(line_number)
            .or_insert(0);
    }

    /// Records instruction execution
    pub fn record_instruction<'ctx>(&self, instruction: &Instruction<'ctx>) {
        if let (Some(ref file), Some(line)) = (&instruction.source_file, instruction.source_line) {
            let mut data = self.instruction_coverage_data.lock().unwrap();
            *data
                .entry(file.clone())
                .or_insert_with(HashMap::new)
                .entry(line)
                .or_insert(0) += 1;
        }
    }

    /// Generates LCOV format coverage report
    pub fn generate_lcov_report(&self) -> String {
        let data = self.instruction_coverage_data.lock().unwrap();
        let mut lines = Vec::new();

        for (file_path, line_coverage) in data.iter() {
            lines.push(format!("SF:{}", file_path));

            // Line data
            let mut sorted_lines: Vec<_> = line_coverage.iter().collect();
            sorted_lines.sort_by_key(|(line_num, _)| *line_num);
            for (line_number, count) in sorted_lines {
                lines.push(format!("DA:{},{}", line_number, count));
            }

            // Lines found
            lines.push(format!("LF:{}", line_coverage.len()));

            // Lines hit
            let lines_hit = line_coverage.values().filter(|&&count| count > 0).count();
            lines.push(format!("LH:{}", lines_hit));

            lines.push("end_of_record".to_string());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insn_len() {
        assert_eq!(insn_len(OP_STOP), 1);
        assert_eq!(insn_len(OP_PUSH1), 2);
        assert_eq!(insn_len(OP_PUSH32), 33);
        assert_eq!(insn_len(OP_ADD), 1);
    }

    #[test]
    fn test_mnemonic() {
        assert_eq!(str_opcode(OP_STOP), "STOP");
        assert_eq!(str_opcode(OP_ADD), "ADD");
        assert_eq!(str_opcode(OP_PUSH1), "PUSH1");
    }

    #[test]
    fn test_contract_from_hexcode() {
        let cfg = z3::Config::new();
        let ctx = Context::new(&cfg);
        let contract = Contract::from_hexcode("6080604052", &ctx).unwrap();
        assert_eq!(contract.len(), 5);
    }

    #[test]
    fn test_instruction_len() {
        let cfg = z3::Config::new();
        let ctx = Context::new(&cfg);
        let insn = Instruction::new(OP_PUSH1, 0, 2, None);
        assert_eq!(insn.len(), 2);
    }

    #[test]
    fn test_opcode_groups() {
        assert!(CALL_OPCODES.contains(&OP_CALL));
        assert!(CREATE_OPCODES.contains(&OP_CREATE));
        assert!(TERMINATING_OPCODES.contains(&OP_STOP));
    }
}
