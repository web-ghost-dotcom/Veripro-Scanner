// SPDX-License-Identifier: AGPL-3.0

//! Ethereum Virtual Machine (EVM) Exceptions
//!
//! Exceptions thrown during EVM execution.
//! Based on execution-specs' vm/exceptions

use thiserror::Error;

/// Base trait for any exception that should stop the current path exploration.
///
/// Stopping path exploration means stopping not only the current EVM context
/// but also its parent contexts if any.
pub trait PathEndingException: std::error::Error {}

/// Base class for unexpected internal errors happening during a test run.
/// Inherits from PathEndingException because it should stop further path exploration.
#[derive(Error, Debug)]
pub enum HalmosException {
    #[error("Value is not concrete: {0}")]
    NotConcrete(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl PathEndingException for HalmosException {}

/// Raised when the value is not concrete (i.e., it's symbolic)
#[derive(Error, Debug, Clone)]
#[error("Value is not concrete: {0}")]
pub struct NotConcreteError(pub String);

impl NotConcreteError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

/// Raised when the current path condition turns out to be infeasible
#[derive(Error, Debug, Clone)]
#[error("Infeasible path: {0}")]
pub struct InfeasiblePath(pub String);

impl PathEndingException for InfeasiblePath {}

/// Raised when invoking DSTest's fail() pseudo-cheatcode
/// Inherits from PathEndingException because it should stop further path exploration
#[derive(Error, Debug, Clone)]
#[error("Fail cheatcode invoked")]
pub struct FailCheatcode;

impl PathEndingException for FailCheatcode {}

/// Base class for all EVM exceptions
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum EvmException {
    /// Raised by the `REVERT` opcode.
    /// Unlike other EVM exceptions this does not result in the consumption of all gas.
    #[error("Revert: {0}")]
    Revert(String),

    /// Exceptional halt - all gas is consumed
    #[error("Exceptional halt: {0}")]
    ExceptionalHalt(#[from] ExceptionalHalt),
}

/// Raised by the REVERT opcode
/// Unlike other EVM exceptions this does not result in the consumption of all gas
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Revert")]
pub struct Revert;

/// Indicates that the EVM has experienced an exceptional halt.
/// This causes execution to immediately end with all gas being consumed.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ExceptionalHalt {
    #[error("Stack underflow")]
    StackUnderflow,

    #[error("Stack overflow")]
    StackOverflow,

    #[error("Out of gas")]
    OutOfGas,

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Invalid opcode: {0:#x}")]
    InvalidOpcode(u8),

    #[error("Invalid jump destination: {0:#x}")]
    InvalidJumpDest(usize),

    #[error("Message depth limit exceeded (>1024)")]
    MessageDepthLimit,

    #[error("Write in static context")]
    WriteInStaticContext,

    #[error("Out of bounds read at offset {0}")]
    OutOfBoundsRead(usize),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Invalid contract prefix (0xEF)")]
    InvalidContractPrefix,

    #[error("Address collision at {0:?}")]
    AddressCollision([u8; 20]),

    #[error("Contract size limit exceeded")]
    ContractSizeLimit,

    #[error("Return data out of bounds")]
    ReturnDataOutOfBounds,
}

/// Occurs when a pop is executed on an empty stack
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Stack underflow")]
pub struct StackUnderflowError;

/// Occurs when a push is executed on a stack at max capacity
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Stack overflow")]
pub struct StackOverflowError;

/// Occurs when an operation costs more than the amount of gas left in the frame
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Out of gas")]
pub struct OutOfGasError;

/// Occurs when an account is trying to send more value than its balance
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Insufficient funds")]
pub struct InsufficientFunds;

/// Raised when an invalid opcode is encountered
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid opcode: {0:#x}")]
pub struct InvalidOpcode(pub u8);

/// Occurs when the destination of a jump operation doesn't meet the criteria
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid jump destination: {0:#x}")]
pub struct InvalidJumpDestError(pub usize);

/// Raised when the message depth is greater than 1024
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Message depth limit exceeded (>1024)")]
pub struct MessageDepthLimitError;

/// Raised when an attempt is made to modify the state while operating inside of a STATICCALL context
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Write in static context")]
pub struct WriteInStaticContext;

/// Raised when an attempt was made to read data beyond the boundaries of the buffer
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Out of bounds read at offset {0}")]
pub struct OutOfBoundsRead(pub usize);

/// Raised when invalid parameters are passed
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid parameter: {0}")]
pub struct InvalidParameter(pub String);

/// Raised when the new contract code starts with 0xEF
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid contract prefix (0xEF)")]
pub struct InvalidContractPrefix;

/// Raised when trying to deploy into a non-empty address
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Address collision at {0:?}")]
pub struct AddressCollision(pub [u8; 20]);

/// Legacy enum for backward compatibility
#[derive(Error, Debug)]
pub enum CbseException {
    #[error("Value is not concrete: {0}")]
    NotConcrete(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Infeasible path: {0}")]
    InfeasiblePath(String),

    #[error("Revert")]
    Revert,

    #[error("Fail cheatcode invoked")]
    FailCheatcode,

    #[error("Solver timeout")]
    SolverTimeout,

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl PathEndingException for CbseException {}

/// Result type for CBSE operations
pub type CbseResult<T> = Result<T, CbseException>;

/// Result type for EVM operations
pub type EvmResult<T> = Result<T, EvmException>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_halmos_exception() {
        let err = HalmosException::NotConcrete("test".to_string());
        assert_eq!(err.to_string(), "Value is not concrete: test");
    }

    #[test]
    fn test_not_concrete_error() {
        let err = NotConcreteError::new("symbolic value");
        assert_eq!(err.to_string(), "Value is not concrete: symbolic value");
    }

    #[test]
    fn test_infeasible_path() {
        let err = InfeasiblePath("contradiction".to_string());
        assert_eq!(err.to_string(), "Infeasible path: contradiction");
    }

    #[test]
    fn test_fail_cheatcode() {
        let err = FailCheatcode;
        assert_eq!(err.to_string(), "Fail cheatcode invoked");
    }

    #[test]
    fn test_evm_exception_revert() {
        let err = EvmException::Revert("test revert".to_string());
        assert_eq!(err.to_string(), "Revert: test revert");
    }

    #[test]
    fn test_exceptional_halt() {
        let err = ExceptionalHalt::StackUnderflow;
        assert_eq!(err.to_string(), "Stack underflow");

        let err = ExceptionalHalt::InvalidOpcode(0xFE);
        assert_eq!(err.to_string(), "Invalid opcode: 0xfe");
    }

    #[test]
    fn test_stack_errors() {
        let err = StackUnderflowError;
        assert_eq!(err.to_string(), "Stack underflow");

        let err = StackOverflowError;
        assert_eq!(err.to_string(), "Stack overflow");
    }

    #[test]
    fn test_out_of_gas_error() {
        let err = OutOfGasError;
        assert_eq!(err.to_string(), "Out of gas");
    }

    #[test]
    fn test_invalid_jump_dest() {
        let err = InvalidJumpDestError(0x1234);
        assert_eq!(err.to_string(), "Invalid jump destination: 0x1234");
    }

    #[test]
    fn test_address_collision() {
        let addr = [0u8; 20];
        let err = AddressCollision(addr);
        assert!(err.to_string().contains("Address collision"));
    }

    #[test]
    fn test_cbse_exception_display() {
        let err = CbseException::NotConcrete("test".to_string());
        assert_eq!(err.to_string(), "Value is not concrete: test");
    }
}
