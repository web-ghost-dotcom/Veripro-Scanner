// SPDX-License-Identifier: AGPL-3.0

use lazy_static::lazy_static;
use num_bigint::BigUint;

/// Verbosity levels for tracing
pub const VERBOSITY_TRACE_COUNTEREXAMPLE: u8 = 2;
pub const VERBOSITY_TRACE_SETUP: u8 = 3;
pub const VERBOSITY_TRACE_PATHS: u8 = 4;
pub const VERBOSITY_TRACE_CONSTRUCTOR: u8 = 5;

/// Maximum memory size (2^20 bytes = 1MB)
pub const MAX_MEMORY_SIZE: usize = 1 << 20;

/// Maximum ETH value (2^128) - compatible with startHoax(address)
/// See: https://github.com/a16z/halmos/issues/338
lazy_static! {
    pub static ref MAX_ETH: BigUint = BigUint::from(1u128) << 128;
}

/// Common EVM constants
pub const WORD_SIZE: usize = 32;
pub const ADDRESS_SIZE: usize = 20;
pub const HASH_SIZE: usize = 32;

/// Maximum call depth for EVM execution
pub const MAX_CALL_DEPTH: usize = 1024;

/// Empty Keccak-256 hash (keccak256(""))
pub const EMPTY_KECCAK: [u8; 32] = [
    0xC5, 0xD2, 0x46, 0x01, 0x86, 0xF7, 0x23, 0x3C, 0x92, 0x7E, 0x7D, 0xB2, 0xDC, 0xC7, 0x03, 0xC0,
    0xE5, 0x00, 0xB6, 0x53, 0xCA, 0x82, 0x27, 0x3B, 0x7B, 0xFA, 0xD8, 0x04, 0x5D, 0x85, 0xA4, 0x70,
];

/// Panic(uint256) selector - bytes4(keccak256("Panic(uint256)"))
pub const PANIC_SELECTOR: [u8; 4] = [0x4E, 0x48, 0x7B, 0x71];

/// Precompile addresses (stored as 20-byte arrays)
pub const ECRECOVER_PRECOMPILE: [u8; 20] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
pub const SHA256_PRECOMPILE: [u8; 20] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2];
pub const RIPEMD160_PRECOMPILE: [u8; 20] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3];
pub const IDENTITY_PRECOMPILE: [u8; 20] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4];
pub const MODEXP_PRECOMPILE: [u8; 20] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5];
pub const ECADD_PRECOMPILE: [u8; 20] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6];
pub const ECMUL_PRECOMPILE: [u8; 20] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7];
pub const ECPAIRING_PRECOMPILE: [u8; 20] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8];
pub const BLAKE2F_PRECOMPILE: [u8; 20] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9];
pub const POINT_EVALUATION_PRECOMPILE: [u8; 20] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10];

/// Gas constants
pub const GAS_LIMIT_DEFAULT: u64 = 30_000_000;
pub const GAS_STIPEND_CALL: u64 = 2300;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_eth() {
        assert_eq!(MAX_ETH.bits(), 129);
    }

    #[test]
    fn test_max_memory_size() {
        assert_eq!(MAX_MEMORY_SIZE, 1048576);
    }

    #[test]
    fn test_verbosity_levels() {
        assert_eq!(VERBOSITY_TRACE_COUNTEREXAMPLE, 2);
        assert_eq!(VERBOSITY_TRACE_SETUP, 3);
        assert_eq!(VERBOSITY_TRACE_PATHS, 4);
        assert_eq!(VERBOSITY_TRACE_CONSTRUCTOR, 5);
    }

    #[test]
    fn test_evm_constants() {
        assert_eq!(WORD_SIZE, 32);
        assert_eq!(ADDRESS_SIZE, 20);
        assert_eq!(HASH_SIZE, 32);
        assert_eq!(MAX_CALL_DEPTH, 1024);
    }

    #[test]
    fn test_empty_keccak() {
        // Verify against known keccak256("")
        assert_eq!(EMPTY_KECCAK[0], 0xC5);
        assert_eq!(EMPTY_KECCAK[31], 0x70);
    }

    #[test]
    fn test_panic_selector() {
        assert_eq!(PANIC_SELECTOR, [0x4E, 0x48, 0x7B, 0x71]);
    }

    #[test]
    fn test_precompile_addresses() {
        assert_eq!(ECRECOVER_PRECOMPILE[19], 1);
        assert_eq!(SHA256_PRECOMPILE[19], 2);
        assert_eq!(RIPEMD160_PRECOMPILE[19], 3);
        assert_eq!(IDENTITY_PRECOMPILE[19], 4);
        assert_eq!(MODEXP_PRECOMPILE[19], 5);
        assert_eq!(ECADD_PRECOMPILE[19], 6);
        assert_eq!(ECMUL_PRECOMPILE[19], 7);
        assert_eq!(ECPAIRING_PRECOMPILE[19], 8);
        assert_eq!(BLAKE2F_PRECOMPILE[19], 9);
        assert_eq!(POINT_EVALUATION_PRECOMPILE[19], 10);
    }
}
