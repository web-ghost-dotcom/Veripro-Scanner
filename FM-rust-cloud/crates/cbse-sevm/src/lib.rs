// SPDX-License-Identifier: AGPL-3.0

//! Symbolic EVM implementation for CBSE
//!
//! This module provides the main symbolic execution engine that interprets EVM bytecode
//! and tracks execution paths through the program.

use cbse_bitvec::CbseBitVec;
use cbse_bytevec::{ByteVec, UnwrappedBytes};
use cbse_contract::Contract;
use cbse_exceptions::{CbseException, CbseResult};
use cbse_traces::{CallContext, CallMessage, CallOutput};
use std::collections::HashMap;
use std::rc::Rc;
use z3::{Context, Solver};

mod opcodes;
mod path;
mod state;
mod storage;
mod worklist;

pub use path::*;
pub use state::*;
pub use storage::*;
pub use worklist::*;

/// Message passed between contract calls
#[derive(Debug)]
pub struct Message<'ctx> {
    pub target: [u8; 20],
    pub caller: [u8; 20],
    pub origin: [u8; 20],
    pub value: CbseBitVec<'ctx>,
    pub data: ByteVec<'ctx>,
    pub gas: u64,
    pub is_static: bool,
}

/// Execution state for a single contract call
///
/// This corresponds to Python's Exec class in halmos/sevm.py
#[derive(Debug, Clone)]
pub struct ExecState<'ctx> {
    // Stack and memory
    pub stack: Vec<CbseBitVec<'ctx>>,
    pub memory: ByteVec<'ctx>,

    // Program counter and gas
    pub pc: usize,
    pub gas: u64,

    // Call context
    pub caller: [u8; 20],
    pub address: [u8; 20],
    pub value: u64,

    // Return data from last call
    pub last_return_data: Option<ByteVec<'ctx>>,

    // Trace context (matches Python's Exec.context)
    pub context: CallContext,

    // Path constraints (matches Python's Exec.path)
    pub path: Path<'ctx>,

    // Jump tracking for loop detection (matches Python's Exec.jumpis)
    pub jumpis: HashMap<(usize, Vec<String>), HashMap<bool, usize>>,
}

impl<'ctx> ExecState<'ctx> {
    /// Create a new execution state with call context and path
    pub fn new(ctx: &'ctx Context, call_context: CallContext, solver: Rc<Solver<'ctx>>) -> Self {
        Self {
            stack: Vec::new(),
            memory: ByteVec::new(ctx),
            pc: 0,
            gas: 30_000_000, // Default gas limit
            caller: [0u8; 20],
            address: [0u8; 20],
            value: 0,
            last_return_data: None,
            context: call_context,
            path: Path::new(solver),
            jumpis: HashMap::new(),
        }
    }
}

/// Result of executing a contract
#[derive(Debug)]
pub struct ExecutionResult<'ctx> {
    pub success: bool,
    pub return_data: ByteVec<'ctx>,
    pub gas_used: u64,
}

/// Symbolic EVM - Main execution engine
pub struct SEVM<'ctx> {
    /// Z3 context for symbolic operations
    pub ctx: &'ctx Context,

    /// Z3 solver for path constraints (reference-counted for sharing across paths)
    pub solver: Rc<Solver<'ctx>>,

    /// Contract bytecode storage
    pub contracts: HashMap<[u8; 20], Contract<'ctx>>,

    /// Storage for each contract address using Z3 Arrays for symbolic keys
    /// This matches Python's ex.storage dictionary with StorageData
    pub storage: HashMap<[u8; 20], StorageData<'ctx>>,

    /// Balance for each address
    pub balance: HashMap<[u8; 20], u64>,

    /// Address counter for CREATE opcode (matches Python's new_address())
    address_counter: u64,
}

impl<'ctx> SEVM<'ctx> {
    /// Create a new SEVM instance
    pub fn new(ctx: &'ctx Context) -> Self {
        let solver = Rc::new(Solver::new(ctx));

        Self {
            ctx,
            solver,
            contracts: HashMap::new(),
            storage: HashMap::new(),
            balance: HashMap::new(),
            address_counter: 0x1000, // Start at 0x1000 for created contracts
        }
    }

    /// Deploy a contract at the given address
    pub fn deploy_contract(&mut self, address: [u8; 20], contract: Contract<'ctx>) {
        self.contracts.insert(address, contract);
    }

    /// Set storage value for a contract (SSTORE)
    ///
    /// Uses Z3 Array Store operation for symbolic storage keys.
    /// Matches Python's SolidityStorage.store() at sevm.py:1804-1825
    pub fn set_storage(
        &mut self,
        address: [u8; 20],
        slot: CbseBitVec<'ctx>,
        value: CbseBitVec<'ctx>,
        path_conditions: &mut Vec<z3::ast::Bool<'ctx>>,
    ) -> CbseResult<()> {
        // For now, treat slot directly as the storage location (scalar storage)
        // In full implementation, this would decode the slot using SolidityStorage::decode
        // and handle nested mappings/arrays

        // Initialize storage if needed
        SolidityStorage::init(&mut self.storage, address, 0, 0, 0, self.ctx)?;

        // Store with symbolic array operations
        SolidityStorage::store(
            &mut self.storage,
            address,
            0,       // base slot (would be decoded from slot in full implementation)
            &[slot], // keys - treating slot as the key
            value,
            self.ctx,
        )?;

        Ok(())
    }

    /// Get storage value for a contract (SLOAD)
    ///
    /// Uses Z3 Array Select operation for symbolic storage keys.
    /// Matches Python's SolidityStorage.load() at sevm.py:1779-1802
    pub fn get_storage(&mut self, address: [u8; 20], slot: &CbseBitVec<'ctx>) -> CbseBitVec<'ctx> {
        // Initialize storage if needed
        if SolidityStorage::init(&mut self.storage, address, 0, 0, 0, self.ctx).is_err() {
            return CbseBitVec::from_u64(0, 256);
        }

        // Load with symbolic array operations
        SolidityStorage::load(&self.storage, address, 0, &[slot.clone()], self.ctx)
            .unwrap_or_else(|_| CbseBitVec::from_u64(0, 256))
    }

    /// Set balance for an address
    pub fn set_balance(&mut self, address: [u8; 20], balance: u64) {
        self.balance.insert(address, balance);
    }

    /// Get balance for an address
    pub fn get_balance(&self, address: &[u8; 20]) -> u64 {
        self.balance.get(address).copied().unwrap_or(0)
    }

    /// Generate a new contract address for CREATE opcode
    ///
    /// This matches Python's new_address() method which generates sequential addresses
    /// for newly created contracts. The Python implementation uses a counter to ensure
    /// unique addresses.
    ///
    /// # Returns
    /// A new 20-byte address
    pub fn new_address(&mut self) -> [u8; 20] {
        self.address_counter += 1;
        let mut addr = [0u8; 20];
        let bytes = self.address_counter.to_be_bytes();
        addr[12..20].copy_from_slice(&bytes);
        addr
    }

    /// Create a branched execution state with a new path condition
    ///
    /// This corresponds to Python's create_branch() at line 2908 in halmos/sevm.py.
    /// It deep-copies the execution state and branches the path with the given condition.
    ///
    /// # Arguments
    /// * `state` - The current execution state to branch from
    /// * `cond` - The Z3 boolean condition to add to the new path
    /// * `target_pc` - The program counter value for the new branch
    ///
    /// # Returns
    /// A new ExecState with the branched path and updated PC
    pub fn create_branch(
        &self,
        state: &ExecState<'ctx>,
        cond: z3::ast::Bool<'ctx>,
        target_pc: usize,
    ) -> CbseResult<ExecState<'ctx>> {
        // Branch the path with the condition (Python: new_path = ex.path.branch(cond))
        let new_path = state.path.branch(cond)?;

        // Deep-copy the execution state
        // Python performs deepcopy on: storage, transient_storage, block, context, st, jumpis
        // For ByteVec and Option<ByteVec>, we create new instances to avoid clone issues
        let new_state = ExecState {
            stack: state.stack.clone(),
            memory: ByteVec::new(self.ctx), // Create fresh memory - will be populated during execution
            pc: target_pc,                  // Set to target PC for the branch
            gas: state.gas,
            caller: state.caller,
            address: state.address,
            value: state.value,
            last_return_data: None, // Reset return data for new branch
            context: state.context.clone(),
            path: new_path,
            jumpis: state.jumpis.clone(),
        };

        Ok(new_state)
    }
    /// Execute a call to another contract
    /// Returns (success, return_data, gas_used, call_context)
    ///
    /// This uses a worklist-based execution loop to explore multiple paths,
    /// matching Python's run() method at lines 3024-3697
    pub fn execute_call(
        &mut self,
        target: [u8; 20],
        caller: [u8; 20],
        origin: [u8; 20],
        value: u64,
        calldata: Vec<u8>,
        gas: u64,
        is_static: bool,
    ) -> CbseResult<(bool, Vec<u8>, u64, CallContext)> {
        // Temporarily remove contract from HashMap to avoid borrow checker issues
        // This matches Python's pattern where Exec owns contracts separately
        let contract = match self.contracts.remove(&target) {
            Some(c) => c,
            None => {
                // No contract at address - return empty
                let empty_message = CallMessage::new(
                    Self::address_to_u64(&target),
                    Self::address_to_u64(&caller),
                    value,
                    calldata,
                    0xF1, // CALL
                    is_static,
                );
                let empty_output = CallOutput::new(Some(Vec::new()), None, Some(0xF3)); // RETURN
                let empty_context = CallContext::new(empty_message, empty_output, 0);
                return Ok((false, Vec::new(), 0, empty_context));
            }
        };

        // Create CallMessage for trace
        let call_message = CallMessage::new(
            Self::address_to_u64(&target),
            Self::address_to_u64(&caller),
            value,
            calldata.clone(),
            0xF1, // CALL opcode
            is_static,
        );

        // Create CallOutput (will be updated after execution)
        let call_output = CallOutput::new(None, None, None);

        // Create CallContext
        let call_context = CallContext::new(call_message, call_output, 0);

        // Create message
        let message = Message {
            target,
            caller,
            origin, // Track original transaction origin through nested calls
            value: CbseBitVec::from_u64(value, 256),
            data: ByteVec::from_bytes(calldata.clone(), self.ctx)?,
            gas,
            is_static,
        };

        // Create initial execution state
        let initial_state = ExecState {
            stack: Vec::new(),
            memory: ByteVec::new(self.ctx),
            pc: 0,
            gas,
            caller,
            address: target,
            value,
            last_return_data: None,
            context: call_context,
            path: Path::new(Rc::clone(&self.solver)),
            jumpis: HashMap::new(),
        };

        // Initialize worklist with the initial state
        let mut worklist: Worklist<ExecState<'ctx>> = Worklist::new();
        let mut next_state: Option<ExecState> = Some(initial_state);

        // Execution statistics
        let mut steps = 0;
        const MAX_STEPS: usize = 100_000; // Prevent infinite loops

        // Track completed paths - for now we'll just use the first completed path
        let mut completed_state: Option<ExecState> = None;

        // Main execution loop - matches Python's while (ex := next_ex or stack.pop()) is not None
        while let Some(mut state) = next_state.take().or_else(|| worklist.pop()) {
            steps += 1;
            if steps > MAX_STEPS {
                return Err(CbseException::Internal(
                    "Maximum execution steps exceeded".to_string(),
                ));
            }

            // Activate pending path conditions (Python: ex.path.activate())
            state.path.activate();

            // Check path feasibility - terminate early if infeasible
            // This matches Python's ex.check() and prevents exploring impossible paths
            if !state.path.is_feasible() {
                // Path is infeasible (UNSAT) - terminate this path
                worklist.completed_paths += 1;
                continue;
            }

            // Check if PC is out of bounds
            let code_len = contract.len();
            if state.pc >= code_len {
                // Execution fell off the end - treat as STOP
                if completed_state.is_none() {
                    completed_state = Some(state);
                }
                worklist.completed_paths += 1;
                continue;
            }

            // Fetch opcode
            let opcode = contract.get_byte(state.pc)?;

            // Special handling for JUMPI - it creates multiple paths
            if opcode == 0x57 {
                // OP_JUMPI
                let branches = self.handle_jumpi(&state, &message)?;

                // Push all branches to the worklist (handle_jumpi already checks feasibility)
                for branch in branches {
                    worklist.push(branch);
                }

                // Continue to next iteration (don't use next_state fast path)
                continue;
            }

            // Execute the opcode (state.context will be updated with traces)
            let should_halt = self.execute_opcode(opcode, &mut state, &message, &contract)?;

            if should_halt {
                // Path completed (RETURN, REVERT, STOP, etc.)
                if completed_state.is_none() {
                    completed_state = Some(state);
                }
                worklist.completed_paths += 1;
                continue;
            }

            // Fast path: continue with this state in the next iteration
            // This avoids pushing/popping from worklist for linear execution
            next_state = Some(state);
        }

        // Use the first completed state, or create a default one if none completed
        let mut final_state = completed_state.unwrap_or_else(|| ExecState {
            stack: Vec::new(),
            memory: ByteVec::new(self.ctx),
            pc: 0,
            gas: 0,
            caller,
            address: target,
            value,
            last_return_data: None,
            context: CallContext::new(
                CallMessage::new(
                    Self::address_to_u64(&target),
                    Self::address_to_u64(&caller),
                    value,
                    calldata,
                    0xF1,
                    is_static,
                ),
                CallOutput::new(Some(Vec::new()), None, Some(0xF3)),
                0,
            ),
            path: Path::new(Rc::clone(&self.solver)),
            jumpis: HashMap::new(),
        });

        // Extract return data
        let return_data = if let Some(ref data) = final_state.last_return_data {
            // Convert ByteVec to Vec<u8>
            // Try to unwrap the ByteVec to get concrete bytes
            match data.unwrap() {
                Ok(UnwrappedBytes::Bytes(bytes)) => bytes.to_vec(),
                Ok(UnwrappedBytes::BitVec(_)) => {
                    // BitVec case - symbolic data
                    // For now, return empty - symbolic return data handling needs more work
                    Vec::new()
                }
                Err(_) => {
                    // Failed to unwrap - return empty
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        // Calculate gas used (simplified - just return remaining gas)
        let gas_used = gas.saturating_sub(final_state.gas);

        // Check if execution was successful (no revert)
        let success = !return_data.starts_with(&[0x4e, 0x48, 0x7b, 0x71]); // Not Panic selector

        // Check for assertion failures and generate counterexample if needed
        let (has_assertion_failure, counterexample) = self.check_assertions(&final_state)?;
        if has_assertion_failure {
            // Print counterexample to stderr for visibility
            eprintln!("❌ Assertion Failure Detected!");
            eprintln!("{}", counterexample);
            eprintln!("Completed paths explored: {}", worklist.completed_paths);
        }

        // Update CallContext output
        final_state.context.output.data = Some(return_data.clone());
        final_state.context.output.return_scheme = Some(if success { 0xF3 } else { 0xFD }); // RETURN or REVERT

        // Put the contract back into the HashMap
        self.contracts.insert(target, contract);

        Ok((success, return_data, gas_used, final_state.context))
    }

    /// Convert address to u64 for trace
    fn address_to_u64(addr: &[u8; 20]) -> u64 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&addr[12..20]); // Use last 8 bytes
        u64::from_be_bytes(bytes)
    }

    /// Handle cheatcode calls
    pub fn handle_cheatcode(&mut self, selector: [u8; 4], data: &[u8]) -> CbseResult<Vec<u8>> {
        // vm.assume(bool condition) - selector: 0x4c63e562
        if selector == [0x4c, 0x63, 0xe5, 0x62] {
            // Extract condition from calldata (first 32 bytes after selector)
            if data.len() >= 32 {
                let mut cond_bytes = [0u8; 32];
                cond_bytes.copy_from_slice(&data[0..32]);
                let cond = CbseBitVec::from_bytes(&cond_bytes, 256);

                // Check if condition is zero (false) or non-zero (true)
                let is_zero = cond.is_zero(self.ctx);

                match is_zero {
                    cbse_bitvec::CbseBool::Concrete(true) => {
                        // Assuming false - path is infeasible
                        return Err(CbseException::Internal(
                            "vm.assume(false) makes path infeasible".to_string(),
                        ));
                    }
                    cbse_bitvec::CbseBool::Concrete(false) => {
                        // Assuming true - always satisfied, no constraint needed
                    }
                    cbse_bitvec::CbseBool::Symbolic(z3_bool) => {
                        // Add symbolic constraint that condition is NOT zero (i.e., true)
                        self.solver.assert(&z3_bool.not());
                    }
                }
            }
            return Ok(Vec::new()); // vm.assume returns nothing
        }

        // vm.prank(address) - selector: 0xca669fa7
        // TODO: Implement prank functionality
        if selector == [0xca, 0x66, 0x9f, 0xa7] {
            // For now, just return success
            return Ok(Vec::new());
        }

        // For other cheatcodes, return empty result
        // TODO: Implement remaining cheatcodes (prank, deal, store, load, etc.)
        Ok(Vec::new())
    }

    /// Convert ByteVec to concrete bytes
    fn bytevec_to_bytes(&self, bytevec: &ByteVec<'ctx>) -> CbseResult<Vec<u8>> {
        let mut result = Vec::new();
        for i in 0..bytevec.len() {
            let byte = bytevec.get_byte(i)?;
            match byte {
                UnwrappedBytes::Bytes(bytes) => {
                    if !bytes.is_empty() {
                        result.push(bytes[0]);
                    } else {
                        result.push(0);
                    }
                }
                UnwrappedBytes::BitVec(bv) => {
                    let val = bv.as_u64().unwrap_or(0) as u8;
                    result.push(val);
                }
            }
        }
        Ok(result)
    }

    /// Stack operations
    fn push(&self, state: &mut ExecState<'ctx>, value: CbseBitVec<'ctx>) -> CbseResult<()> {
        if state.stack.len() >= 1024 {
            return Err(CbseException::Internal("Stack overflow".to_string()));
        }
        state.stack.push(value);
        Ok(())
    }

    fn pop(&self, state: &mut ExecState<'ctx>) -> CbseResult<CbseBitVec<'ctx>> {
        state
            .stack
            .pop()
            .ok_or_else(|| CbseException::Internal("Stack underflow".to_string()))
    }

    fn peek(&self, state: &ExecState<'ctx>, n: usize) -> CbseResult<CbseBitVec<'ctx>> {
        if state.stack.len() < n {
            return Err(CbseException::Internal("Stack underflow".to_string()));
        }
        Ok(state.stack[state.stack.len() - n].clone())
    }

    /// Check if an execution state represents an assertion failure
    ///
    /// Detects Panic errors, which indicate assertion violations in Solidity.
    /// Returns true if the state contains a Panic(0x01) error (assertion failure).
    pub fn is_assertion_failure(&self, state: &ExecState<'ctx>) -> bool {
        if let Some(ref return_data) = state.last_return_data {
            // Check for Panic signature: 0x4e487b71
            // Panic(uint256) selector
            if return_data.len() >= 36 {
                // Get first 4 bytes for selector
                let mut selector = [0u8; 4];
                for i in 0..4 {
                    if let Ok(byte) = return_data.get_byte(i) {
                        match byte {
                            UnwrappedBytes::Bytes(bytes) if !bytes.is_empty() => {
                                selector[i] = bytes[0];
                            }
                            _ => {}
                        }
                    }
                }

                // Check if it's Panic selector
                if selector == [0x4e, 0x48, 0x7b, 0x71] {
                    // Get panic code (next 32 bytes)
                    // Panic(0x01) = assertion failure
                    // Panic(0x11) = arithmetic overflow
                    // Panic(0x12) = divide by zero
                    // etc.
                    if let Ok(byte) = return_data.get_byte(35) {
                        if let UnwrappedBytes::Bytes(bytes) = byte {
                            if !bytes.is_empty() && bytes[0] == 0x01 {
                                return true; // Panic(0x01) - assertion failure
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Generate and display a counterexample for an assertion failure
    ///
    /// This extracts a satisfying model from the solver showing concrete values
    /// for symbolic variables that cause the assertion to fail.
    ///
    /// Matches Python's counterexample generation in __main__.py lines 791-1000
    pub fn generate_counterexample(&self, state: &ExecState<'ctx>) -> CbseResult<String> {
        // Extract model from the path's solver
        let model = state.path.get_model()?;

        if model.is_empty() {
            return Ok("No counterexample found (path may be infeasible)".to_string());
        }

        // Format the counterexample
        let formatted = Path::format_counterexample(&model);
        Ok(format!("Counterexample:\n    {}", formatted))
    }

    /// Check for assertion failures and generate counterexamples
    ///
    /// This is called after execution completes to check if any assertion failed.
    /// If a failure is detected, it extracts and displays the counterexample.
    ///
    /// Returns (has_failure, counterexample_message)
    pub fn check_assertions(&self, state: &ExecState<'ctx>) -> CbseResult<(bool, String)> {
        if self.is_assertion_failure(state) {
            let counterexample = self.generate_counterexample(state)?;
            Ok((true, counterexample))
        } else {
            Ok((false, String::new()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sevm_creation() {
        let cfg = z3::Config::new();
        let ctx = Context::new(&cfg);
        let sevm = SEVM::new(&ctx);

        assert_eq!(sevm.contracts.len(), 0);
    }

    #[test]
    fn test_exec_state() {
        let cfg = z3::Config::new();
        let ctx = Context::new(&cfg);
        let solver = Rc::new(Solver::new(&ctx));

        // Create a dummy call context for testing
        let message = CallMessage::new(0, 0, 0, Vec::new(), 0xF1, false);
        let output = CallOutput::new(None, None, None);
        let call_context = CallContext::new(message, output, 0);

        let state = ExecState::new(&ctx, call_context, solver);

        assert_eq!(state.pc, 0);
        assert_eq!(state.stack.len(), 0);
    }

    #[test]
    fn test_assertion_failure_detection() {
        let cfg = z3::Config::new();
        let ctx = Context::new(&cfg);
        let sevm = SEVM::new(&ctx);
        let solver = Rc::new(Solver::new(&ctx));

        // Create a state with Panic(0x01) error
        let message = CallMessage::new(0, 0, 0, Vec::new(), 0xF1, false);
        let output = CallOutput::new(None, None, None);
        let call_context = CallContext::new(message, output, 0);

        let mut state = ExecState::new(&ctx, call_context, solver);

        // Create Panic(0x01) return data: selector (4 bytes) + panic code (32 bytes)
        let mut panic_data = vec![0x4e, 0x48, 0x7b, 0x71]; // Panic selector
        panic_data.extend(vec![0u8; 31]); // 31 zero bytes
        panic_data.push(0x01); // Panic code 0x01

        state.last_return_data = Some(ByteVec::from_bytes(panic_data, &ctx).unwrap());

        assert!(sevm.is_assertion_failure(&state));
    }
}
