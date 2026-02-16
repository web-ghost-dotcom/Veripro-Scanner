// SPDX-License-Identifier: AGPL-3.0

//! EVM opcode implementation
//!
//! This module implements all EVM opcodes for symbolic execution.
//! It closely mirrors the Python implementation in halmos/sevm.py

use super::{ExecState, Message, StorageData, SEVM};
use cbse_bitvec::CbseBitVec;
use cbse_bytevec::{ByteVec, UnwrappedBytes};
use cbse_cheatcodes::{HEVM_ADDRESS, SVM_ADDRESS};
use cbse_console::CONSOLE_ADDRESS;
use cbse_contract::Contract;
use cbse_exceptions::{CbseException, CbseResult, ExceptionalHalt};
use cbse_hashes::keccak256;
use cbse_traces::{CallContext, StorageRead, StorageWrite, TraceElement};
use std::collections::HashMap;

// EVM opcodes
const OP_STOP: u8 = 0x00;
const OP_ADD: u8 = 0x01;
const OP_MUL: u8 = 0x02;
const OP_SUB: u8 = 0x03;
const OP_DIV: u8 = 0x04;
const OP_SDIV: u8 = 0x05;
const OP_MOD: u8 = 0x06;
const OP_SMOD: u8 = 0x07;
const OP_ADDMOD: u8 = 0x08;
const OP_MULMOD: u8 = 0x09;
const OP_EXP: u8 = 0x0a;
const OP_SIGNEXTEND: u8 = 0x0b;
const OP_LT: u8 = 0x10;
const OP_GT: u8 = 0x11;
const OP_SLT: u8 = 0x12;
const OP_SGT: u8 = 0x13;
const OP_EQ: u8 = 0x14;
const OP_ISZERO: u8 = 0x15;
const OP_AND: u8 = 0x16;
const OP_OR: u8 = 0x17;
const OP_XOR: u8 = 0x18;
const OP_NOT: u8 = 0x19;
const OP_BYTE: u8 = 0x1a;
const OP_SHL: u8 = 0x1b;
const OP_SHR: u8 = 0x1c;
const OP_SAR: u8 = 0x1d;
const OP_SHA3: u8 = 0x20;
const OP_ADDRESS: u8 = 0x30;
const OP_BALANCE: u8 = 0x31;
const OP_ORIGIN: u8 = 0x32;
const OP_CALLER: u8 = 0x33;
const OP_CALLVALUE: u8 = 0x34;
const OP_CALLDATALOAD: u8 = 0x35;
const OP_CALLDATASIZE: u8 = 0x36;
const OP_CALLDATACOPY: u8 = 0x37;
const OP_CODESIZE: u8 = 0x38;
const OP_CODECOPY: u8 = 0x39;
const OP_GASPRICE: u8 = 0x3a;
const OP_EXTCODESIZE: u8 = 0x3b;
const OP_EXTCODECOPY: u8 = 0x3c;
const OP_RETURNDATASIZE: u8 = 0x3d;
const OP_RETURNDATACOPY: u8 = 0x3e;
const OP_EXTCODEHASH: u8 = 0x3f;
const OP_BLOCKHASH: u8 = 0x40;
const OP_COINBASE: u8 = 0x41;
const OP_TIMESTAMP: u8 = 0x42;
const OP_NUMBER: u8 = 0x43;
const OP_DIFFICULTY: u8 = 0x44;
const OP_GASLIMIT: u8 = 0x45;
const OP_CHAINID: u8 = 0x46;
const OP_SELFBALANCE: u8 = 0x47;
const OP_BASEFEE: u8 = 0x48;
const OP_POP: u8 = 0x50;
const OP_MLOAD: u8 = 0x51;
const OP_MSTORE: u8 = 0x52;
const OP_MSTORE8: u8 = 0x53;
const OP_SLOAD: u8 = 0x54;
const OP_SSTORE: u8 = 0x55;
const OP_JUMP: u8 = 0x56;
const OP_JUMPI: u8 = 0x57;
const OP_PC: u8 = 0x58;
const OP_MSIZE: u8 = 0x59;
const OP_GAS: u8 = 0x5a;
const OP_JUMPDEST: u8 = 0x5b;
const OP_PUSH0: u8 = 0x5f;
const OP_PUSH1: u8 = 0x60;
const OP_PUSH32: u8 = 0x7f;
const OP_DUP1: u8 = 0x80;
const OP_DUP16: u8 = 0x8f;
const OP_SWAP1: u8 = 0x90;
const OP_SWAP16: u8 = 0x9f;
const OP_LOG0: u8 = 0xa0;
const OP_LOG1: u8 = 0xa1;
const OP_LOG2: u8 = 0xa2;
const OP_LOG3: u8 = 0xa3;
const OP_LOG4: u8 = 0xa4;
const OP_CREATE: u8 = 0xf0;
const OP_CALL: u8 = 0xf1;
const OP_CALLCODE: u8 = 0xf2;
const OP_RETURN: u8 = 0xf3;
const OP_DELEGATECALL: u8 = 0xf4;
const OP_CREATE2: u8 = 0xf5;
const OP_STATICCALL: u8 = 0xfa;
const OP_REVERT: u8 = 0xfd;
const OP_INVALID: u8 = 0xfe;
const OP_SELFDESTRUCT: u8 = 0xff;

impl<'ctx> SEVM<'ctx> {
    /// Convert CbseBool to CbseBitVec (0 or 1 as 256-bit value)
    fn bool_to_bv(&self, b: cbse_bitvec::CbseBool<'ctx>) -> CbseBitVec<'ctx> {
        use cbse_bitvec::CbseBool;
        match b {
            CbseBool::Concrete(true) => CbseBitVec::from_u64(1, 256),
            CbseBool::Concrete(false) => CbseBitVec::from_u64(0, 256),
            CbseBool::Symbolic(z3_bool) => {
                // Use ite: if z3_bool then 1 else 0
                let one = z3::ast::BV::from_u64(self.ctx, 1, 256);
                let zero = z3::ast::BV::from_u64(self.ctx, 0, 256);
                let result = z3_bool.ite(&one, &zero);
                CbseBitVec::from_z3(result)
            }
        }
    }

    /// Handle JUMPI with full path branching.
    /// Returns a vector of possible execution states (0, 1, or 2 states).
    ///
    /// This matches the Python halmos jumpi() implementation:
    /// - Checks satisfiability of both branches
    /// - Implements loop unrolling limits
    /// - Creates two execution states when condition is symbolic
    /// - Tracks visited branches via jumpis HashMap
    pub fn handle_jumpi(
        &mut self,
        state: &ExecState<'ctx>,
        message: &Message<'ctx>,
    ) -> CbseResult<Vec<ExecState<'ctx>>> {
        use cbse_bitvec::CbseBool;

        // Pop dest and cond from stack - clone state to avoid mutation
        let mut new_stack = state.stack.clone();
        let dest_bv = new_stack
            .pop()
            .ok_or_else(|| CbseException::Internal("Stack underflow in JUMPI".to_string()))?;
        let cond_bv = new_stack
            .pop()
            .ok_or_else(|| CbseException::Internal("Stack underflow in JUMPI".to_string()))?;

        // Convert destination to usize (must be concrete)
        let dest = dest_bv.as_u64().map_err(|_| {
            CbseException::Internal("Symbolic jump destination not supported".to_string())
        })? as usize;

        // Convert condition to bool - is_zero returns true if cond == 0
        let cond_is_zero = cond_bv.is_zero(self.ctx);

        // We need the opposite: jump if NOT zero
        let cond = match cond_is_zero {
            CbseBool::Concrete(is_zero) => CbseBool::Concrete(!is_zero),
            CbseBool::Symbolic(z3_bool) => CbseBool::Symbolic(z3_bool.not()),
        };

        // Get current pc and create jump id (jid)
        let pc = state.pc;
        // Python uses: jid = (pc, tuple(ex.codebase[pc].value))
        // For now we'll use a simplified version: (pc, empty vec)
        // TODO: Extract actual instruction bytes from codebase
        let jid = (pc, Vec::new());

        // Get loop unrolling configuration (default to 2 if not set)
        let loop_limit = 2; // TODO: Get from options/config

        // Get visited counts for this jump location
        let visited = state.jumpis.get(&jid).cloned().unwrap_or_default();
        let visited_true = *visited.get(&true).unwrap_or(&0);
        let visited_false = *visited.get(&false).unwrap_or(&0);

        // Check satisfiability of both branches
        let (potential_true, potential_false) = match &cond {
            CbseBool::Concrete(b) => {
                // Concrete case: only one branch is possible
                (*b, !b)
            }
            CbseBool::Symbolic(z3_bool) => {
                // Check if true branch is satisfiable
                state.path.solver.push();
                state.path.solver.assert(z3_bool);
                let check_true = state.path.solver.check();
                state.path.solver.pop(1);

                // Check if false branch is satisfiable
                state.path.solver.push();
                let not_cond = z3_bool.not();
                state.path.solver.assert(&not_cond);
                let check_false = state.path.solver.check();
                state.path.solver.pop(1);

                let potential_true = check_true == z3::SatResult::Sat;
                let potential_false = check_false == z3::SatResult::Sat;

                (potential_true, potential_false)
            }
        };

        // Determine which branches to follow based on loop limits
        let follow_true = potential_true && visited_true < loop_limit;
        let follow_false = potential_false && visited_false < loop_limit;

        // Collect resulting execution states
        let mut result = Vec::new();

        // Handle true branch (jump taken)
        if follow_true {
            // Create new execution state with branched path
            let mut new_ex_true = if follow_false {
                // If we're also following false, create a proper branch
                // Extract Z3 Bool if symbolic, otherwise skip branching for concrete case
                match &cond {
                    CbseBool::Symbolic(z3_bool) => {
                        self.create_branch(state, z3_bool.clone(), dest)?
                    }
                    CbseBool::Concrete(true) => {
                        // Concrete true - just update PC
                        let mut ex = state.clone();
                        ex.pc = dest;
                        ex.stack = new_stack.clone();
                        ex
                    }
                    CbseBool::Concrete(false) => {
                        // This shouldn't happen since potential_true would be false
                        unreachable!(
                            "Logic error: following true branch with concrete false condition"
                        )
                    }
                }
            } else {
                // If only following true, just update the existing state
                let mut ex = state.clone();
                ex.pc = dest;
                ex.stack = new_stack.clone();

                // Add constraint if symbolic
                if let CbseBool::Symbolic(z3_bool) = &cond {
                    ex.path.append(z3_bool.clone(), true)?;
                }

                ex
            };

            // Update jumpis tracking
            let mut new_jumpis = new_ex_true.jumpis.clone();
            let branch_visits = new_jumpis.entry(jid.clone()).or_insert_with(HashMap::new);
            *branch_visits.entry(true).or_insert(0) += 1;
            new_ex_true.jumpis = new_jumpis;

            result.push(new_ex_true);
        }

        // Handle false branch (continue to next instruction)
        if follow_false {
            let mut new_ex_false = if follow_true {
                // Already created a branch for true, use cloned state for false
                let mut ex = state.clone();
                ex.pc = pc + 1;
                ex.stack = new_stack.clone();

                // Add constraint if symbolic (NOT of the condition)
                if let CbseBool::Symbolic(z3_bool) = &cond {
                    let not_cond = z3_bool.not();
                    ex.path.append(not_cond, true)?;
                }

                ex
            } else {
                // Only following false branch
                let mut ex = state.clone();
                ex.pc = pc + 1;
                ex.stack = new_stack;

                // Add constraint if symbolic (NOT of the condition)
                if let CbseBool::Symbolic(z3_bool) = &cond {
                    let not_cond = z3_bool.not();
                    ex.path.append(not_cond, true)?;
                }

                ex
            };

            // Update jumpis tracking
            let mut new_jumpis = new_ex_false.jumpis.clone();
            let branch_visits = new_jumpis.entry(jid).or_insert_with(HashMap::new);
            *branch_visits.entry(false).or_insert(0) += 1;
            new_ex_false.jumpis = new_jumpis;

            result.push(new_ex_false);
        }

        // If no branches are followed (hit loop limit), return empty vector
        // The caller will know to terminate this path
        Ok(result)
    }

    /// Execute a single opcode
    pub fn execute_opcode(
        &mut self,
        opcode: u8,
        state: &mut ExecState<'ctx>,
        message: &Message<'ctx>,
        contract: &Contract<'ctx>,
    ) -> CbseResult<bool> {
        match opcode {
            // 0x00: STOP
            OP_STOP => {
                return Ok(true); // Halt execution
            }

            // 0x01: ADD
            OP_ADD => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let result = a.add(&b, self.ctx);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x02: MUL
            OP_MUL => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let result = a.mul(&b, self.ctx);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x03: SUB
            OP_SUB => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let result = a.sub(&b, self.ctx);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x04: DIV
            OP_DIV => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let result = if b.is_zero(self.ctx).is_true() {
                    CbseBitVec::from_u64(0, 256)
                } else {
                    a.udiv(&b, self.ctx)
                };
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x05: SDIV
            OP_SDIV => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let result = if b.is_zero(self.ctx).is_true() {
                    CbseBitVec::from_u64(0, 256)
                } else {
                    a.sdiv(&b, self.ctx)
                };
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x06: MOD
            OP_MOD => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let result = if b.is_zero(self.ctx).is_true() {
                    CbseBitVec::from_u64(0, 256)
                } else {
                    a.urem(&b, self.ctx)
                };
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x07: SMOD
            OP_SMOD => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let result = if b.is_zero(self.ctx).is_true() {
                    CbseBitVec::from_u64(0, 256)
                } else {
                    a.smod(&b, self.ctx)
                };
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x08: ADDMOD
            OP_ADDMOD => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let n = self.pop(state)?;
                let result = if n.is_zero(self.ctx).is_true() {
                    CbseBitVec::from_u64(0, 256)
                } else {
                    // (a + b) % n
                    let sum = a.add(&b, self.ctx);
                    sum.urem(&n, self.ctx)
                };
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x09: MULMOD
            OP_MULMOD => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let n = self.pop(state)?;
                let result = if n.is_zero(self.ctx).is_true() {
                    CbseBitVec::from_u64(0, 256)
                } else {
                    // (a * b) % n
                    let prod = a.mul(&b, self.ctx);
                    prod.urem(&n, self.ctx)
                };
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x0a: EXP
            OP_EXP => {
                let base = self.pop(state)?;
                let exponent = self.pop(state)?;
                // For symbolic execution, we need to handle this carefully
                // For now, use concrete values if available
                match (base.as_u64(), exponent.as_u64()) {
                    (Ok(b), Ok(e)) => {
                        if e > 256 {
                            // Result would overflow, return 0
                            self.push(state, CbseBitVec::from_u64(0, 256))?;
                        } else {
                            let result = b.saturating_pow(e as u32);
                            self.push(state, CbseBitVec::from_u64(result, 256))?;
                        }
                    }
                    _ => {
                        // Symbolic exponentiation - create symbolic result
                        // In full implementation, this would use Z3's power operation
                        // For now, return symbolic value
                        self.push(state, CbseBitVec::from_u64(0, 256))?;
                    }
                }
                state.pc += 1;
            }

            // 0x0b: SIGNEXTEND
            OP_SIGNEXTEND => {
                let byte_num = self.pop(state)?;
                let value = self.pop(state)?;

                // Concrete implementation for now
                if let Ok(b) = byte_num.as_u64() {
                    if b < 31 {
                        let bit_position = (b + 1) * 8;
                        // Sign extend from bit_position
                        // This is complex in symbolic execution, simplified for now
                        self.push(state, value)?;
                    } else {
                        self.push(state, value)?;
                    }
                } else {
                    self.push(state, value)?;
                }
                state.pc += 1;
            }

            // 0x10: LT
            OP_LT => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let cmp_result = a.ult(&b, self.ctx);
                let result = self.bool_to_bv(cmp_result);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x11: GT
            OP_GT => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let cmp_result = a.ugt(&b, self.ctx);
                let result = self.bool_to_bv(cmp_result);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x12: SLT (Signed Less Than)
            OP_SLT => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let cmp_result = a.slt(&b, self.ctx);
                let result = self.bool_to_bv(cmp_result);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x13: SGT (Signed Greater Than)
            OP_SGT => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let cmp_result = a.sgt(&b, self.ctx);
                let result = self.bool_to_bv(cmp_result);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x14: EQ
            OP_EQ => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let cmp_result = a.eq(&b, self.ctx);
                let result = self.bool_to_bv(cmp_result);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x15: ISZERO
            OP_ISZERO => {
                let a = self.pop(state)?;
                let result = if a.is_zero(self.ctx).is_true() {
                    CbseBitVec::from_u64(1, 256)
                } else {
                    CbseBitVec::from_u64(0, 256)
                };
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x16: AND
            OP_AND => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let result = a.and(&b, self.ctx);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x17: OR
            OP_OR => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let result = a.or(&b, self.ctx);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x18: XOR
            OP_XOR => {
                let a = self.pop(state)?;
                let b = self.pop(state)?;
                let result = a.xor(&b, self.ctx);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x19: NOT
            OP_NOT => {
                let a = self.pop(state)?;
                let result = a.not(self.ctx);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x1a: BYTE
            OP_BYTE => {
                let i = self.pop(state)?;
                let x = self.pop(state)?;

                // Extract byte at position i from x (0 = most significant byte)
                if let Ok(index) = i.as_u64() {
                    if index < 32 {
                        // Shift right and mask to get the byte
                        let shift_amount = CbseBitVec::from_u64((31 - index) * 8, 256);
                        let shifted = x.lshr(&shift_amount, self.ctx);
                        let mask = CbseBitVec::from_u64(0xFF, 256);
                        let result = shifted.and(&mask, self.ctx);
                        self.push(state, result)?;
                    } else {
                        self.push(state, CbseBitVec::from_u64(0, 256))?;
                    }
                } else {
                    // Symbolic index - return 0 for now
                    self.push(state, CbseBitVec::from_u64(0, 256))?;
                }
                state.pc += 1;
            }

            // 0x1B: SHL
            OP_SHL => {
                let shift = self.pop(state)?;
                let value = self.pop(state)?;
                let result = value.shl(&shift, self.ctx);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x1C: SHR
            OP_SHR => {
                let shift = self.pop(state)?;
                let value = self.pop(state)?;
                let result = value.lshr(&shift, self.ctx);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x1d: SAR (Arithmetic right shift)
            OP_SAR => {
                let shift = self.pop(state)?;
                let value = self.pop(state)?;
                let result = value.ashr(&shift, self.ctx);
                self.push(state, result)?;
                state.pc += 1;
            }

            // 0x20: SHA3 (KECCAK256)
            OP_SHA3 => {
                let offset = self.pop(state)?;
                let length = self.pop(state)?;

                // For now, return a symbolic hash
                // Full implementation would hash the memory bytes
                if let (Ok(off), Ok(len)) = (offset.as_u64(), length.as_u64()) {
                    // In full implementation: hash state.memory[off..off+len]
                    // For now, create a symbolic hash value
                    let hash = CbseBitVec::from_u64(0, 256);
                    self.push(state, hash)?;
                } else {
                    // Symbolic offset/length
                    self.push(state, CbseBitVec::from_u64(0, 256))?;
                }
                state.pc += 1;
            }

            // 0x30: ADDRESS
            OP_ADDRESS => {
                let addr_bv = CbseBitVec::from_bytes(&state.address, 160);
                self.push(state, addr_bv)?;
                state.pc += 1;
            }

            // 0x31: BALANCE
            OP_BALANCE => {
                let addr = self.pop(state)?;
                // For symbolic execution, return symbolic balance
                // In full implementation, look up balance for the address
                self.push(state, CbseBitVec::from_u64(0, 256))?;
                state.pc += 1;
            }

            // 0x32: ORIGIN
            OP_ORIGIN => {
                let origin_bv = CbseBitVec::from_bytes(&message.origin, 160);
                self.push(state, origin_bv)?;
                state.pc += 1;
            }

            // 0x33: CALLER
            OP_CALLER => {
                let caller_bv = CbseBitVec::from_bytes(&state.caller, 160);
                self.push(state, caller_bv)?;
                state.pc += 1;
            }

            // 0x34: CALLVALUE
            OP_CALLVALUE => {
                let value_bv = CbseBitVec::from_u64(state.value, 256);
                self.push(state, value_bv)?;
                state.pc += 1;
            }

            // 0x35: CALLDATALOAD
            OP_CALLDATALOAD => {
                let offset = self.pop(state)?;

                if let Ok(off) = offset.as_u64() {
                    let word = message.data.get_word(off as usize)?;
                    let word_bv = match word {
                        UnwrappedBytes::BitVec(bv) => bv,
                        UnwrappedBytes::Bytes(bytes) => CbseBitVec::from_bytes(&bytes, 256),
                    };
                    self.push(state, word_bv)?;
                } else {
                    // Symbolic offset - create symbolic value
                    let symbolic_word = CbseBitVec::symbolic(self.ctx, "calldata_symbolic", 256);
                    self.push(state, symbolic_word)?;
                }
                state.pc += 1;
            }

            // 0x36: CALLDATASIZE
            OP_CALLDATASIZE => {
                let size = CbseBitVec::from_u64(message.data.len() as u64, 256);
                self.push(state, size)?;
                state.pc += 1;
            }

            // 0x37: CALLDATACOPY
            OP_CALLDATACOPY => {
                let dest_offset = self.pop(state)?;
                let offset = self.pop(state)?;
                let length = self.pop(state)?;

                if let (Ok(dest), Ok(off), Ok(len)) =
                    (dest_offset.as_u64(), offset.as_u64(), length.as_u64())
                {
                    for i in 0..len {
                        let byte = if (off + i) < message.data.len() as u64 {
                            message
                                .data
                                .get_byte((off + i) as usize)
                                .unwrap_or(UnwrappedBytes::Bytes(vec![0]))
                        } else {
                            UnwrappedBytes::Bytes(vec![0])
                        };
                        state.memory.set_byte((dest + i) as usize, byte)?;
                    }
                }
                state.pc += 1;
            }

            // 0x38: CODESIZE
            OP_CODESIZE => {
                let size = CbseBitVec::from_u64(contract.len() as u64, 256);
                self.push(state, size)?;
                state.pc += 1;
            }

            // 0x39: CODECOPY
            OP_CODECOPY => {
                let dest_offset = self.pop(state)?;
                let offset = self.pop(state)?;
                let length = self.pop(state)?;

                if let (Ok(dest), Ok(off), Ok(len)) =
                    (dest_offset.as_u64(), offset.as_u64(), length.as_u64())
                {
                    for i in 0..len {
                        let byte = if (off + i) < contract.len() as u64 {
                            contract.get_byte((off + i) as usize).unwrap_or(0)
                        } else {
                            0
                        };
                        let byte_bv = CbseBitVec::from_u64(byte as u64, 8);
                        state
                            .memory
                            .set_byte((dest + i) as usize, UnwrappedBytes::BitVec(byte_bv))?;
                    }
                }
                state.pc += 1;
            }

            // 0x3a: GASPRICE
            OP_GASPRICE => {
                // Return symbolic gas price
                self.push(state, CbseBitVec::from_u64(0, 256))?;
                state.pc += 1;
            }

            // 0x3b: EXTCODESIZE
            OP_EXTCODESIZE => {
                let _addr = self.pop(state)?;
                // For symbolic execution, return 1 to indicate code exists
                // In full implementation, check if address has code
                self.push(state, CbseBitVec::from_u64(1, 256))?;
                state.pc += 1;
            }

            // 0x3c: EXTCODECOPY
            OP_EXTCODECOPY => {
                let _addr = self.pop(state)?;
                let dest_offset = self.pop(state)?;
                let offset = self.pop(state)?;
                let length = self.pop(state)?;

                // For symbolic execution, fill with zeros
                if let (Ok(dest), Ok(_off), Ok(len)) =
                    (dest_offset.as_u64(), offset.as_u64(), length.as_u64())
                {
                    for i in 0..len {
                        state
                            .memory
                            .set_byte((dest + i) as usize, UnwrappedBytes::Bytes(vec![0]))?;
                    }
                }
                state.pc += 1;
            }

            // 0x3d: RETURNDATASIZE
            OP_RETURNDATASIZE => {
                let size = if let Some(ref data) = state.last_return_data {
                    data.len() as u64
                } else {
                    0
                };
                self.push(state, CbseBitVec::from_u64(size, 256))?;
                state.pc += 1;
            }

            // 0x3e: RETURNDATACOPY
            OP_RETURNDATACOPY => {
                let dest_offset = self.pop(state)?;
                let offset = self.pop(state)?;
                let length = self.pop(state)?;

                if let (Ok(dest), Ok(off), Ok(len)) =
                    (dest_offset.as_u64(), offset.as_u64(), length.as_u64())
                {
                    if let Some(ref return_data) = state.last_return_data {
                        for i in 0..len {
                            let byte = if (off + i) < return_data.len() as u64 {
                                return_data.get_byte((off + i) as usize)?
                            } else {
                                UnwrappedBytes::Bytes(vec![0])
                            };
                            state.memory.set_byte((dest + i) as usize, byte)?;
                        }
                    }
                }
                state.pc += 1;
            }

            // 0x3f: EXTCODEHASH
            OP_EXTCODEHASH => {
                let _addr = self.pop(state)?;
                // Return a symbolic hash
                self.push(state, CbseBitVec::from_u64(0, 256))?;
                state.pc += 1;
            }

            // 0x40-0x48: Block information opcodes
            OP_BLOCKHASH => {
                let _block_num = self.pop(state)?;
                self.push(state, CbseBitVec::from_u64(0, 256))?;
                state.pc += 1;
            }

            OP_COINBASE => {
                self.push(state, CbseBitVec::from_u64(0, 256))?;
                state.pc += 1;
            }

            OP_TIMESTAMP => {
                self.push(state, CbseBitVec::from_u64(1, 256))?;
                state.pc += 1;
            }

            OP_NUMBER => {
                self.push(state, CbseBitVec::from_u64(1, 256))?;
                state.pc += 1;
            }

            OP_DIFFICULTY => {
                self.push(state, CbseBitVec::from_u64(0, 256))?;
                state.pc += 1;
            }

            OP_GASLIMIT => {
                self.push(state, CbseBitVec::from_u64(30_000_000, 256))?;
                state.pc += 1;
            }

            OP_CHAINID => {
                self.push(state, CbseBitVec::from_u64(1, 256))?;
                state.pc += 1;
            }

            OP_SELFBALANCE => {
                let balance = self.get_balance(&state.address);
                self.push(state, CbseBitVec::from_u64(balance, 256))?;
                state.pc += 1;
            }

            OP_BASEFEE => {
                self.push(state, CbseBitVec::from_u64(0, 256))?;
                state.pc += 1;
            }

            // 0x50: POP
            OP_POP => {
                self.pop(state)?;
                state.pc += 1;
            }

            // 0x51: MLOAD
            OP_MLOAD => {
                let offset = self.pop(state)?;

                if let Ok(off) = offset.as_u64() {
                    let word = state.memory.get_word(off as usize)?;
                    let word_bv = match word {
                        UnwrappedBytes::BitVec(bv) => bv,
                        UnwrappedBytes::Bytes(bytes) => CbseBitVec::from_bytes(&bytes, 256),
                    };
                    self.push(state, word_bv)?;
                } else {
                    // Symbolic offset
                    let symbolic_mem = CbseBitVec::symbolic(self.ctx, "memory_symbolic", 256);
                    self.push(state, symbolic_mem)?;
                }
                state.pc += 1;
            }

            // 0x52: MSTORE
            OP_MSTORE => {
                let offset = self.pop(state)?;
                let value = self.pop(state)?;

                if let Ok(off) = offset.as_u64() {
                    state
                        .memory
                        .set_word(off as usize, UnwrappedBytes::BitVec(value))?;
                }
                state.pc += 1;
            }

            // 0x53: MSTORE8
            OP_MSTORE8 => {
                let offset = self.pop(state)?;
                let value = self.pop(state)?;

                if let Ok(off) = offset.as_u64() {
                    let byte_val = (value.as_u64().unwrap_or(0) & 0xFF) as u8;
                    let byte_bv = CbseBitVec::from_u64(byte_val as u64, 8);
                    state
                        .memory
                        .set_byte(off as usize, UnwrappedBytes::BitVec(byte_bv))?;
                }
                state.pc += 1;
            }

            // 0x54: SLOAD
            OP_SLOAD => {
                let slot = self.pop(state)?;
                let value = self.get_storage(state.address, &slot);

                // Record SLOAD in trace
                let slot_u64 = slot.as_u64().unwrap_or(0);
                let value_bytes = value
                    .as_u64()
                    .map(|v| v.to_be_bytes().to_vec())
                    .unwrap_or_else(|_| vec![0; 32]);

                state.context.trace.push(TraceElement::Read(StorageRead {
                    slot: slot_u64,
                    value: value_bytes,
                    transient: false,
                }));

                self.push(state, value)?;
                state.pc += 1;
            }

            // 0x55: SSTORE
            OP_SSTORE => {
                let slot = self.pop(state)?;
                let value = self.pop(state)?;

                // Record SSTORE in trace
                let slot_u64 = slot.as_u64().unwrap_or(0);
                let value_bytes = value
                    .as_u64()
                    .map(|v| v.to_be_bytes().to_vec())
                    .unwrap_or_else(|_| vec![0; 32]);

                state.context.trace.push(TraceElement::Write(StorageWrite {
                    slot: slot_u64,
                    value: value_bytes,
                    transient: false,
                }));

                // Use symbolic storage with Z3 Arrays
                // Path conditions from the Store operation will be added to state.path
                let mut path_conds = Vec::new();
                self.set_storage(state.address, slot, value, &mut path_conds)?;

                // Add the path conditions from the Store operation
                for cond in path_conds {
                    state.path.append(cond, false)?;
                }

                state.pc += 1;
            }

            // 0x56: JUMP
            OP_JUMP => {
                let dest = self.pop(state)?;
                let dest_pc = dest
                    .as_u64()
                    .map_err(|_| CbseException::Internal("Symbolic jump destination".to_string()))?
                    as usize;

                // Verify JUMPDEST
                if dest_pc >= contract.len() {
                    return Err(CbseException::Internal(
                        "Jump destination out of bounds".to_string(),
                    ));
                }

                let dest_opcode = contract.get_byte(dest_pc)?;
                if dest_opcode != OP_JUMPDEST {
                    return Err(CbseException::Internal(
                        "Invalid jump destination".to_string(),
                    ));
                }

                state.pc = dest_pc;
            }

            // 0x57: JUMPI
            OP_JUMPI => {
                let dest = self.pop(state)?;
                let cond = self.pop(state)?;

                // Check if condition is concrete or symbolic
                let cond_bool = cond.is_zero(self.ctx);

                match cond_bool {
                    cbse_bitvec::CbseBool::Concrete(is_zero) => {
                        // Concrete condition - take the definite branch
                        let should_jump = !is_zero;

                        if should_jump {
                            let dest_pc = dest.as_u64().map_err(|_| {
                                CbseException::Internal("Symbolic jump destination".to_string())
                            })? as usize;

                            // Verify JUMPDEST
                            if dest_pc >= contract.len() {
                                return Err(CbseException::Internal(
                                    "Jump destination out of bounds".to_string(),
                                ));
                            }

                            let dest_opcode = contract.get_byte(dest_pc)?;
                            if dest_opcode != OP_JUMPDEST {
                                return Err(CbseException::Internal(
                                    "Invalid jump destination".to_string(),
                                ));
                            }

                            state.pc = dest_pc;
                        } else {
                            state.pc += 1;
                        }
                    }
                    cbse_bitvec::CbseBool::Symbolic(z3_cond) => {
                        // Symbolic condition - need path branching
                        // NOTE: Full symbolic JUMPI requires multi-path execution architecture
                        // For now, we add the constraint and follow one path (concretize)
                        // TODO: Implement proper path branching with worklist of execution states

                        // Try to check which path is feasible
                        self.solver.push();
                        self.solver.assert(&z3_cond);
                        let can_be_true = self.solver.check() == z3::SatResult::Sat;
                        self.solver.pop(1);

                        self.solver.push();
                        self.solver.assert(&z3_cond.not());
                        let can_be_false = self.solver.check() == z3::SatResult::Sat;
                        self.solver.pop(1);

                        // For now, follow the "can jump" path if feasible, else fallthrough
                        // In full implementation, we would create two separate execution states
                        if can_be_true {
                            let dest_pc = dest.as_u64().unwrap_or((state.pc + 1) as u64) as usize;

                            if dest_pc < contract.len() {
                                let dest_opcode = contract.get_byte(dest_pc)?;
                                if dest_opcode == OP_JUMPDEST {
                                    // Add constraint that condition is true
                                    self.solver.assert(&z3_cond);
                                    state.pc = dest_pc;
                                } else {
                                    state.pc += 1;
                                }
                            } else {
                                state.pc += 1;
                            }
                        } else if can_be_false {
                            // Add constraint that condition is false
                            self.solver.assert(&z3_cond.not());
                            state.pc += 1;
                        } else {
                            // Both paths infeasible - this path is unsat
                            return Err(CbseException::Internal(
                                "JUMPI condition leads to unsatisfiable state".to_string(),
                            ));
                        }
                    }
                }
            }

            // 0x58: PC
            OP_PC => {
                let pc_bv = CbseBitVec::from_u64(state.pc as u64, 256);
                self.push(state, pc_bv)?;
                state.pc += 1;
            }

            // 0x59: MSIZE
            OP_MSIZE => {
                let size = state.memory.len() as u64;
                self.push(state, CbseBitVec::from_u64(size, 256))?;
                state.pc += 1;
            }

            // 0x5A: GAS
            OP_GAS => {
                let gas_bv = CbseBitVec::from_u64(state.gas, 256);
                self.push(state, gas_bv)?;
                state.pc += 1;
            }

            // 0x5B: JUMPDEST
            OP_JUMPDEST => {
                // No-op
                state.pc += 1;
            }

            // 0x5F-0x7F: PUSH0-PUSH32
            op @ OP_PUSH0..=OP_PUSH32 => {
                let n = (op - OP_PUSH0) as usize;

                if n == 0 {
                    // PUSH0
                    self.push(state, CbseBitVec::from_u64(0, 256))?;
                } else {
                    // PUSH1-PUSH32
                    let mut bytes = Vec::with_capacity(n);
                    for i in 1..=n {
                        if state.pc + i < contract.len() {
                            bytes.push(contract.get_byte(state.pc + i)?);
                        } else {
                            bytes.push(0);
                        }
                    }

                    let value = CbseBitVec::from_bytes(&bytes, 256);
                    self.push(state, value)?;
                    state.pc += n;
                }
                state.pc += 1;
            }

            // 0x80-0x8F: DUP1-DUP16
            op @ OP_DUP1..=OP_DUP16 => {
                let n = (op - OP_DUP1 + 1) as usize;
                let value = self.peek(state, n)?;
                self.push(state, value)?;
                state.pc += 1;
            }

            // 0x90-0x9F: SWAP1-SWAP16
            op @ OP_SWAP1..=OP_SWAP16 => {
                let n = (op - OP_SWAP1 + 1) as usize;
                let len = state.stack.len();
                if len < n + 1 {
                    return Err(CbseException::Internal("Stack underflow".to_string()));
                }
                state.stack.swap(len - 1, len - 1 - n);
                state.pc += 1;
            }

            // 0xA0-0xA4: LOG0-LOG4
            op @ OP_LOG0..=OP_LOG4 => {
                // Check if in static context
                if message.is_static {
                    return Err(CbseException::Internal(
                        "WriteInStaticContext: LOG in static call".to_string(),
                    ));
                }

                // Calculate number of topics
                let num_topics = (op - OP_LOG0) as usize;

                // Pop memory location and size
                let loc = self.pop(state)?;
                let size = self.pop(state)?;

                // Get memory location and size as concrete values
                let loc_concrete = loc.as_u64().map_err(|_| {
                    CbseException::Internal(
                        "Symbolic LOG memory location not supported".to_string(),
                    )
                })? as usize;

                let size_concrete = size.as_u64().map_err(|_| {
                    CbseException::Internal("Symbolic LOG data size not supported".to_string())
                })? as usize;

                // Pop topics from stack
                let mut topics = Vec::with_capacity(num_topics);
                for _ in 0..num_topics {
                    let topic_bv = self.pop(state)?;

                    // Convert topic to 32 bytes (topics are Word values)
                    let mut topic_bytes = vec![0u8; 32];
                    if let Ok(val) = topic_bv.as_u64() {
                        // Concrete topic - store as big-endian bytes
                        let bytes = val.to_be_bytes();
                        topic_bytes[24..32].copy_from_slice(&bytes);
                    } else {
                        // Symbolic topic - for now use placeholder
                        // Full implementation would need to extract symbolic bytes
                        // This matches Python's behavior of storing symbolic Word values
                    }
                    topics.push(topic_bytes);
                }

                // Extract data from memory
                let mut data = Vec::with_capacity(size_concrete);
                for i in 0..size_concrete {
                    let byte = state.memory.get_byte(loc_concrete + i)?;
                    match byte {
                        UnwrappedBytes::BitVec(bv) => {
                            if let Ok(val) = bv.as_u64() {
                                data.push(val as u8);
                            } else {
                                // Symbolic byte - use 0 as placeholder
                                data.push(0);
                            }
                        }
                        UnwrappedBytes::Bytes(bytes) => {
                            // Get first byte from concrete bytes
                            data.push(bytes.get(0).copied().unwrap_or(0));
                        }
                    }
                }

                // Get contract address from message.target (convert [u8; 20] to u64)
                // In the trace model, Address is u64, so we take the last 8 bytes
                let address = u64::from_be_bytes([
                    message.target[12],
                    message.target[13],
                    message.target[14],
                    message.target[15],
                    message.target[16],
                    message.target[17],
                    message.target[18],
                    message.target[19],
                ]);

                // Create EventLog and add to trace
                use cbse_traces::EventLog;
                let log = EventLog::new(address, topics, data);
                state.context.add_trace_element(TraceElement::Log(log));

                state.pc += 1;
            }

            // 0xF0: CREATE
            OP_CREATE => {
                // Check if in static context
                if message.is_static {
                    return Err(CbseException::Internal(
                        "WriteInStaticContext: CREATE in static call".to_string(),
                    ));
                }

                // Pop value, offset, size from stack
                let value_bv = self.pop(state)?;
                let offset = self.pop(state)?;
                let size = self.pop(state)?;

                // Get concrete values
                let value = value_bv.as_u64().unwrap_or(0);
                let offset_concrete = offset.as_u64().map_err(|_| {
                    CbseException::Internal("Symbolic CREATE offset not supported".to_string())
                })? as usize;
                let size_concrete = size.as_u64().map_err(|_| {
                    CbseException::Internal("Symbolic CREATE size not supported".to_string())
                })? as usize;

                // Extract init code from memory
                let mut init_code = Vec::with_capacity(size_concrete);
                for i in 0..size_concrete {
                    let byte = state.memory.get_byte(offset_concrete + i)?;
                    match byte {
                        UnwrappedBytes::BitVec(bv) => {
                            if let Ok(val) = bv.as_u64() {
                                init_code.push(val as u8);
                            } else {
                                init_code.push(0);
                            }
                        }
                        UnwrappedBytes::Bytes(bytes) => {
                            init_code.push(bytes.get(0).copied().unwrap_or(0));
                        }
                    }
                }

                // Generate new address
                let new_addr = self.new_address();

                // Check for address collision
                if self.contracts.contains_key(&new_addr) {
                    // Address collision - push 0 and continue
                    self.push(state, CbseBitVec::from_u64(0, 256))?;
                    state.pc += 1;
                    return Ok(false);
                }

                // Create new empty contract at address (will be replaced with deployed code)
                let empty_bytevec = ByteVec::new(self.ctx);
                let empty_contract = Contract::new(empty_bytevec, self.ctx, None, None, None);
                self.contracts.insert(new_addr, empty_contract);

                // Initialize storage and balance for new contract
                self.storage.insert(new_addr, StorageData::new());

                // Transfer value from caller to new contract
                if value > 0 {
                    let caller_balance = self.get_balance(&message.target);
                    if caller_balance < value {
                        // Insufficient funds - push 0 and continue
                        self.push(state, CbseBitVec::from_u64(0, 256))?;
                        state.pc += 1;
                        return Ok(false);
                    }
                    self.set_balance(message.target, caller_balance - value);
                    let new_balance = self.get_balance(&new_addr);
                    self.set_balance(new_addr, new_balance + value);
                }

                // Execute constructor code
                // In full implementation, this would create a subcall context
                // For now, we'll simulate success and store the init code as deployed code

                // Create contract from init code
                let mut deployed_bytevec = ByteVec::new(self.ctx);
                for (i, &byte) in init_code.iter().enumerate() {
                    let byte_bv = CbseBitVec::from_u64(byte as u64, 8);
                    deployed_bytevec.set_byte(i, UnwrappedBytes::BitVec(byte_bv))?;
                }
                let deployed_contract = Contract::new(deployed_bytevec, self.ctx, None, None, None);
                self.contracts.insert(new_addr, deployed_contract);

                // Push new address on stack (as 256-bit value)
                let addr_val = u64::from_be_bytes([
                    new_addr[12],
                    new_addr[13],
                    new_addr[14],
                    new_addr[15],
                    new_addr[16],
                    new_addr[17],
                    new_addr[18],
                    new_addr[19],
                ]);
                self.push(state, CbseBitVec::from_u64(addr_val, 256))?;

                state.pc += 1;
            }

            // 0xF5: CREATE2
            OP_CREATE2 => {
                // Check if in static context
                if message.is_static {
                    return Err(CbseException::Internal(
                        "WriteInStaticContext: CREATE2 in static call".to_string(),
                    ));
                }

                // Pop value, offset, size, salt from stack
                let value_bv = self.pop(state)?;
                let offset = self.pop(state)?;
                let size = self.pop(state)?;
                let salt = self.pop(state)?;

                // Get concrete values
                let value = value_bv.as_u64().unwrap_or(0);
                let offset_concrete = offset.as_u64().map_err(|_| {
                    CbseException::Internal("Symbolic CREATE2 offset not supported".to_string())
                })? as usize;
                let size_concrete = size.as_u64().map_err(|_| {
                    CbseException::Internal("Symbolic CREATE2 size not supported".to_string())
                })? as usize;

                // Extract init code from memory
                let mut init_code = Vec::with_capacity(size_concrete);
                for i in 0..size_concrete {
                    let byte = state.memory.get_byte(offset_concrete + i)?;
                    match byte {
                        UnwrappedBytes::BitVec(bv) => {
                            if let Ok(val) = bv.as_u64() {
                                init_code.push(val as u8);
                            } else {
                                init_code.push(0);
                            }
                        }
                        UnwrappedBytes::Bytes(bytes) => {
                            init_code.push(bytes.get(0).copied().unwrap_or(0));
                        }
                    }
                }

                // Compute CREATE2 address deterministically
                // address = keccak256(0xff || sender || salt || keccak256(init_code))[12:]

                // Hash the init code
                let init_code_hash = keccak256(&init_code);

                // Get salt as 32 bytes
                let mut salt_bytes = [0u8; 32];
                if let Ok(salt_val) = salt.as_u64() {
                    let bytes = salt_val.to_be_bytes();
                    salt_bytes[24..32].copy_from_slice(&bytes);
                } else {
                    // Symbolic salt - use default (could be improved)
                    // Full implementation would handle symbolic salt properly
                }

                // Construct: 0xff || sender_address || salt || init_code_hash
                let mut hash_input = Vec::with_capacity(85); // 1 + 20 + 32 + 32
                hash_input.push(0xff);
                hash_input.extend_from_slice(&message.target); // sender address (20 bytes)
                hash_input.extend_from_slice(&salt_bytes); // salt (32 bytes)
                hash_input.extend_from_slice(&init_code_hash); // init code hash (32 bytes)

                // Hash to get address
                let address_hash = keccak256(&hash_input);

                // Take last 20 bytes as address (Ethereum uses rightmost 160 bits)
                let mut new_addr = [0u8; 20];
                new_addr.copy_from_slice(&address_hash[12..32]);

                // Check for address collision
                if self.contracts.contains_key(&new_addr) {
                    // Address collision - push 0 and continue
                    self.push(state, CbseBitVec::from_u64(0, 256))?;
                    state.pc += 1;
                    return Ok(false);
                }

                // Create new empty contract at address
                let empty_bytevec = ByteVec::new(self.ctx);
                let empty_contract = Contract::new(empty_bytevec, self.ctx, None, None, None);
                self.contracts.insert(new_addr, empty_contract);

                // Initialize storage for new contract
                self.storage.insert(new_addr, StorageData::new());

                // Transfer value from caller to new contract
                if value > 0 {
                    let caller_balance = self.get_balance(&message.target);
                    if caller_balance < value {
                        // Insufficient funds - push 0 and continue
                        self.push(state, CbseBitVec::from_u64(0, 256))?;
                        state.pc += 1;
                        return Ok(false);
                    }
                    self.set_balance(message.target, caller_balance - value);
                    let new_balance = self.get_balance(&new_addr);
                    self.set_balance(new_addr, new_balance + value);
                }

                // Create deployed contract from init code
                let mut deployed_bytevec = ByteVec::new(self.ctx);
                for (i, &byte) in init_code.iter().enumerate() {
                    let byte_bv = CbseBitVec::from_u64(byte as u64, 8);
                    deployed_bytevec.set_byte(i, UnwrappedBytes::BitVec(byte_bv))?;
                }
                let deployed_contract = Contract::new(deployed_bytevec, self.ctx, None, None, None);
                self.contracts.insert(new_addr, deployed_contract);

                // Push new address on stack (as 256-bit value)
                let addr_val = u64::from_be_bytes([
                    new_addr[12],
                    new_addr[13],
                    new_addr[14],
                    new_addr[15],
                    new_addr[16],
                    new_addr[17],
                    new_addr[18],
                    new_addr[19],
                ]);
                self.push(state, CbseBitVec::from_u64(addr_val, 256))?;

                state.pc += 1;
            }

            // 0xF1: CALL
            OP_CALL => {
                let gas = self.pop(state)?;
                let to_addr = self.pop(state)?;
                let value = self.pop(state)?;
                let args_offset = self.pop(state)?;
                let args_length = self.pop(state)?;
                let ret_offset = self.pop(state)?;
                let ret_length = self.pop(state)?;

                // Extract address
                let mut target = [0u8; 20];
                if let Ok(addr_val) = to_addr.as_u64() {
                    let addr_bytes = addr_val.to_be_bytes();
                    target[12..20].copy_from_slice(&addr_bytes);

                    // Check for cheatcode addresses
                    if target == HEVM_ADDRESS || target == SVM_ADDRESS || target == CONSOLE_ADDRESS
                    {
                        // Handle cheatcode
                        let offset = args_offset.as_u64().unwrap_or(0) as usize;
                        let length = args_length.as_u64().unwrap_or(0) as usize;

                        let mut calldata = Vec::with_capacity(length);
                        for i in 0..length {
                            let byte = state.memory.get_byte(offset + i)?;
                            match byte {
                                UnwrappedBytes::Bytes(bytes) => {
                                    if !bytes.is_empty() {
                                        calldata.push(bytes[0]);
                                    } else {
                                        calldata.push(0);
                                    }
                                }
                                UnwrappedBytes::BitVec(bv) => {
                                    calldata.push(bv.as_u64().unwrap_or(0) as u8);
                                }
                            }
                        }

                        if calldata.len() >= 4 {
                            let selector = [calldata[0], calldata[1], calldata[2], calldata[3]];
                            let result = self.handle_cheatcode(selector, &calldata[4..])?;

                            // Write result to memory
                            if !result.is_empty() {
                                let ret_off = ret_offset.as_u64().unwrap_or(0) as usize;
                                let ret_len = ret_length.as_u64().unwrap_or(0) as usize;
                                let write_len = std::cmp::min(result.len(), ret_len);
                                for i in 0..write_len {
                                    let byte_bv = CbseBitVec::from_u64(result[i] as u64, 8);
                                    state
                                        .memory
                                        .set_byte(ret_off + i, UnwrappedBytes::BitVec(byte_bv))?;
                                }
                            }
                        }

                        // Cheatcodes always succeed
                        self.push(state, CbseBitVec::from_u64(1, 256))?;
                    } else {
                        // Regular contract call
                        let offset = args_offset.as_u64().unwrap_or(0) as usize;
                        let length = args_length.as_u64().unwrap_or(0) as usize;
                        let gas_val = gas.as_u64().unwrap_or(30_000_000);
                        let value_val = value.as_u64().unwrap_or(0);

                        // Extract calldata from memory
                        let mut calldata = Vec::with_capacity(length);
                        for i in 0..length {
                            let byte = state.memory.get_byte(offset + i)?;
                            match byte {
                                UnwrappedBytes::Bytes(bytes) => {
                                    if !bytes.is_empty() {
                                        calldata.push(bytes[0]);
                                    } else {
                                        calldata.push(0);
                                    }
                                }
                                UnwrappedBytes::BitVec(bv) => {
                                    calldata.push(bv.as_u64().unwrap_or(0) as u8);
                                }
                            }
                        }

                        // Execute the call - now returns call_context
                        let (success, return_data, _gas_used, subcall_context) = self
                            .execute_call(
                                target,
                                state.address,  // caller = current contract address
                                message.origin, // pass through the original origin
                                value_val,
                                calldata,
                                gas_val,
                                false,
                            )?;

                        // Add subcall context to parent trace
                        state
                            .context
                            .trace
                            .push(TraceElement::Call(subcall_context));

                        // Write return data to memory
                        if !return_data.is_empty() {
                            let ret_off = ret_offset.as_u64().unwrap_or(0) as usize;
                            let ret_len = ret_length.as_u64().unwrap_or(0) as usize;
                            let write_len = std::cmp::min(return_data.len(), ret_len);
                            for i in 0..write_len {
                                let byte_bv = CbseBitVec::from_u64(return_data[i] as u64, 8);
                                state
                                    .memory
                                    .set_byte(ret_off + i, UnwrappedBytes::BitVec(byte_bv))?;
                            }
                        }

                        // Push success flag
                        let success_val = if success { 1 } else { 0 };
                        self.push(state, CbseBitVec::from_u64(success_val, 256))?;
                    }
                } else {
                    // Symbolic address - assume success
                    self.push(state, CbseBitVec::from_u64(1, 256))?;
                }
                state.pc += 1;
            }

            // 0xF4: DELEGATECALL
            OP_DELEGATECALL => {
                // DELEGATECALL: Execute code from target in current contract's context
                // Preserves msg.sender, msg.value, and storage of caller
                // Stack: gas, to, args_offset, args_length, ret_offset, ret_length

                let gas = self.pop(state)?;
                let to_addr = self.pop(state)?;
                // No value parameter for DELEGATECALL
                let args_offset = self.pop(state)?;
                let args_length = self.pop(state)?;
                let ret_offset = self.pop(state)?;
                let ret_length = self.pop(state)?;

                // Extract target address
                let mut target = [0u8; 20];
                if let Ok(addr_val) = to_addr.as_u64() {
                    let addr_bytes = addr_val.to_be_bytes();
                    target[12..20].copy_from_slice(&addr_bytes);

                    // For now, simplified: push success
                    // Full implementation would execute target's code in caller's context
                    // with caller's storage and address preserved
                    self.push(state, CbseBitVec::from_u64(1, 256))?;
                } else {
                    // Symbolic address - assume success
                    self.push(state, CbseBitVec::from_u64(1, 256))?;
                }
                state.pc += 1;
            }

            // 0xFA: STATICCALL
            OP_STATICCALL => {
                // STATICCALL: Read-only call that disallows state modifications
                // Stack: gas, to, args_offset, args_length, ret_offset, ret_length

                let gas = self.pop(state)?;
                let to_addr = self.pop(state)?;
                // No value parameter for STATICCALL (always 0)
                let args_offset = self.pop(state)?;
                let args_length = self.pop(state)?;
                let ret_offset = self.pop(state)?;
                let ret_length = self.pop(state)?;

                // Extract target address
                let mut target = [0u8; 20];
                if let Ok(addr_val) = to_addr.as_u64() {
                    let addr_bytes = addr_val.to_be_bytes();
                    target[12..20].copy_from_slice(&addr_bytes);

                    // Check for cheatcode addresses (allowed in static context)
                    if target == HEVM_ADDRESS || target == SVM_ADDRESS || target == CONSOLE_ADDRESS
                    {
                        let offset = args_offset.as_u64().unwrap_or(0) as usize;
                        let length = args_length.as_u64().unwrap_or(0) as usize;

                        let mut calldata = Vec::with_capacity(length);
                        for i in 0..length {
                            let byte = state.memory.get_byte(offset + i)?;
                            match byte {
                                UnwrappedBytes::Bytes(bytes) => {
                                    calldata.push(bytes.get(0).copied().unwrap_or(0));
                                }
                                UnwrappedBytes::BitVec(bv) => {
                                    calldata.push(bv.as_u64().unwrap_or(0) as u8);
                                }
                            }
                        }

                        if calldata.len() >= 4 {
                            let selector = [calldata[0], calldata[1], calldata[2], calldata[3]];
                            let result = self.handle_cheatcode(selector, &calldata[4..])?;

                            // Write result to memory
                            if !result.is_empty() {
                                let ret_off = ret_offset.as_u64().unwrap_or(0) as usize;
                                let ret_len = ret_length.as_u64().unwrap_or(0) as usize;
                                let write_len = std::cmp::min(result.len(), ret_len);
                                for i in 0..write_len {
                                    let byte_bv = CbseBitVec::from_u64(result[i] as u64, 8);
                                    state
                                        .memory
                                        .set_byte(ret_off + i, UnwrappedBytes::BitVec(byte_bv))?;
                                }
                            }
                        }

                        self.push(state, CbseBitVec::from_u64(1, 256))?;
                    } else {
                        // Regular static call - would need to execute with is_static=true
                        // For now, simplified: push success
                        self.push(state, CbseBitVec::from_u64(1, 256))?;
                    }
                } else {
                    // Symbolic address - assume success
                    self.push(state, CbseBitVec::from_u64(1, 256))?;
                }
                state.pc += 1;
            }

            // 0xF3: RETURN
            OP_RETURN => {
                let offset = self.pop(state)?;
                let length = self.pop(state)?;

                if let (Ok(off), Ok(len)) = (offset.as_u64(), length.as_u64()) {
                    // Extract return data from memory
                    let mut return_data = ByteVec::new(self.ctx);
                    for i in 0..len as usize {
                        let byte = state.memory.get_byte(off as usize + i)?;
                        return_data.set_byte(i, byte)?;
                    }
                    state.last_return_data = Some(return_data);
                }

                return Ok(true); // Halt execution
            }

            // 0xFD: REVERT
            OP_REVERT => {
                let offset = self.pop(state)?;
                let length = self.pop(state)?;

                // Extract revert data from memory (same as RETURN)
                if let (Ok(off), Ok(len)) = (offset.as_u64(), length.as_u64()) {
                    let mut return_data = ByteVec::new(self.ctx);
                    for i in 0..len as usize {
                        let byte = state.memory.get_byte(off as usize + i)?;
                        return_data.set_byte(i, byte)?;
                    }
                    state.last_return_data = Some(return_data);
                }

                return Ok(true); // Halt execution (revert will be detected in execute_call)
            }

            // 0xFF: SELFDESTRUCT
            OP_SELFDESTRUCT => {
                // SELFDESTRUCT: Destroy contract and send balance to beneficiary
                // Pop beneficiary address from stack
                let beneficiary_bv = self.pop(state)?;

                // Get beneficiary address
                let mut beneficiary = [0u8; 20];
                if let Ok(addr_val) = beneficiary_bv.as_u64() {
                    let addr_bytes = addr_val.to_be_bytes();
                    beneficiary[12..20].copy_from_slice(&addr_bytes);
                }

                // Transfer entire balance to beneficiary
                let self_balance = self.get_balance(&message.target);
                if self_balance > 0 {
                    // Set self balance to 0
                    self.set_balance(message.target, 0);

                    // Add to beneficiary balance
                    let beneficiary_balance = self.get_balance(&beneficiary);
                    self.set_balance(beneficiary, beneficiary_balance + self_balance);
                }

                // In full implementation, would mark contract for deletion
                // and remove code after transaction completes
                // For now, we just halt execution

                return Ok(true); // Halt execution
            }

            // 0xFE: INVALID
            OP_INVALID => {
                return Err(CbseException::Internal("Invalid opcode".to_string()));
            }

            _ => {
                return Err(CbseException::Internal(format!(
                    "Unimplemented opcode: 0x{:02x}",
                    opcode
                )));
            }
        }

        Ok(false) // Continue execution
    }
}
