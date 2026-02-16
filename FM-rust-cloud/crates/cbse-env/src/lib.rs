// SPDX-License-Identifier: AGPL-3.0

use std::env;
use std::path::PathBuf;

/// Initialize environment from .env file
pub fn init_env(path: Option<&str>) {
    let env_path = if let Some(p) = path {
        Some(PathBuf::from(p))
    } else {
        // Look for .env in current directory and parent directories
        find_dotenv()
    };

    let Some(env_path) = env_path else {
        eprintln!("no .env file found");
        return;
    };

    if !env_path.exists() {
        eprintln!("file {} does not exist", env_path.display());
        return;
    }

    let env_file = if env_path.is_dir() {
        env_path.join(".env")
    } else {
        env_path
    };

    if env_file.is_file() {
        eprintln!("loading .env from {}", env_file.display());
        if let Err(e) = dotenv::from_path(&env_file) {
            eprintln!("error loading .env: {}", e);
        }
    } else {
        eprintln!("file {} is not a file", env_file.display());
    }
}

/// Find .env file by walking up directory tree
fn find_dotenv() -> Option<PathBuf> {
    let mut current = env::current_dir().ok()?;

    loop {
        let candidate = current.join(".env");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Check if environment variable exists
pub fn exists(key: &str) -> bool {
    env::var(key).is_ok()
}

/// Parse bytes32 from hex string with expected length
pub fn parse_bytes32(value: &str, expected_hexstr_len: usize) -> Result<Vec<u8>, String> {
    if !value.starts_with("0x") {
        return Err(format!("Missing 0x prefix: {}", value));
    }

    let hex_str = &value[2..];
    if hex_str.len() != expected_hexstr_len {
        return Err(format!(
            "Expected {} characters, got {}: {}",
            expected_hexstr_len,
            hex_str.len(),
            value
        ));
    }

    // Pad to 64 characters (32 bytes)
    let padded = format!("{:0>64}", hex_str);
    hex::decode(&padded).map_err(|e| format!("Invalid hex string: {}", e))
}

/// Get environment variable as string
pub fn get_string(key: &str, default: Option<&str>) -> Result<String, String> {
    match env::var(key) {
        Ok(value) => Ok(value),
        Err(_) => default
            .map(|s| s.to_string())
            .ok_or_else(|| key.to_string()),
    }
}

/// Get environment variable as int (supports hex with 0x prefix and negative values)
pub fn get_int(key: &str) -> Result<i64, String> {
    let value = get_string(key, None)?;

    // Auto-detect base (0x for hex, otherwise decimal)
    if let Some(hex_str) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        i64::from_str_radix(hex_str, 16)
            .map_err(|e| format!("Invalid hex integer '{}': {}", value, e))
    } else if let Some(hex_str) = value
        .strip_prefix("-0x")
        .or_else(|| value.strip_prefix("-0X"))
    {
        i64::from_str_radix(hex_str, 16)
            .map(|v| -v)
            .map_err(|e| format!("Invalid hex integer '{}': {}", value, e))
    } else {
        value
            .parse::<i64>()
            .map_err(|e| format!("Invalid integer '{}': {}", value, e))
    }
}

/// Get environment variable as unsigned int
pub fn get_uint(key: &str) -> Result<u64, String> {
    let value = get_string(key, None)?;

    let result = if let Some(hex_str) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(hex_str, 16)
            .map_err(|e| format!("Invalid hex integer '{}': {}", value, e))?
    } else {
        value
            .parse::<u64>()
            .map_err(|e| format!("Invalid unsigned integer '{}': {}", value, e))?
    };

    Ok(result)
}

/// Get environment variable as bool
pub fn get_bool(key: &str) -> Result<bool, String> {
    let value = get_string(key, None)?;

    match value.to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("Invalid boolean value: {}", value)),
    }
}

/// Get environment variable as address (20 bytes)
pub fn get_address(key: &str) -> Result<[u8; 20], String> {
    let addr_str = get_string(key, None)?;
    let addr_bytes = parse_bytes32(&addr_str, 40)?;

    // Take last 20 bytes
    let mut result = [0u8; 20];
    result.copy_from_slice(&addr_bytes[12..32]);
    Ok(result)
}

/// Get environment variable as bytes32
pub fn get_bytes32(key: &str) -> Result<[u8; 32], String> {
    let value = get_string(key, None)?;
    let bytes = parse_bytes32(&value, 64)?;

    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes);
    Ok(result)
}

/// Get environment variable as bytes (variable length)
pub fn get_bytes(key: &str) -> Result<Vec<u8>, String> {
    let value = get_string(key, None)?;

    // Optional 0x prefix
    let hex_str = value.strip_prefix("0x").unwrap_or(&value);

    hex::decode(hex_str).map_err(|e| format!("Invalid hex bytes '{}': {}", value, e))
}

/// Get environment variable as array of ints (comma-separated)
pub fn get_int_array(key: &str, delimiter: &str) -> Result<Vec<[u8; 32]>, String> {
    let value = get_string(key, None)?;
    let parts: Vec<&str> = value.split(delimiter).map(|s| s.trim()).collect();

    parts
        .iter()
        .map(|part| {
            // Parse as i64 (supports negative)
            let num = if let Some(hex_str) =
                part.strip_prefix("0x").or_else(|| part.strip_prefix("0X"))
            {
                i64::from_str_radix(hex_str, 16)
                    .map_err(|e| format!("Invalid hex integer '{}': {}", part, e))?
            } else if let Some(hex_str) = part
                .strip_prefix("-0x")
                .or_else(|| part.strip_prefix("-0X"))
            {
                -i64::from_str_radix(hex_str, 16)
                    .map_err(|e| format!("Invalid hex integer '{}': {}", part, e))?
            } else {
                part.parse::<i64>()
                    .map_err(|e| format!("Invalid integer '{}': {}", part, e))?
            };

            // Convert to 32-byte big-endian (signed)
            let mut bytes = [0u8; 32];
            let num_bytes = num.to_be_bytes();

            if num >= 0 {
                bytes[24..32].copy_from_slice(&num_bytes);
            } else {
                // Two's complement for negative numbers
                bytes = [0xFF; 32];
                bytes[24..32].copy_from_slice(&num_bytes);
            }

            Ok(bytes)
        })
        .collect()
}

/// Get environment variable as array of unsigned ints
pub fn get_uint_array(key: &str, delimiter: &str) -> Result<Vec<[u8; 32]>, String> {
    let value = get_string(key, None)?;
    let parts: Vec<&str> = value.split(delimiter).map(|s| s.trim()).collect();

    parts
        .iter()
        .map(|part| {
            let num: u64 = part
                .parse()
                .map_err(|e| format!("Invalid unsigned integer '{}': {}", part, e))?;

            let mut bytes = [0u8; 32];
            bytes[24..32].copy_from_slice(&num.to_be_bytes());
            Ok(bytes)
        })
        .collect()
}

/// Get environment variable as array of addresses
pub fn get_address_array(key: &str, delimiter: &str) -> Result<Vec<[u8; 20]>, String> {
    let value = get_string(key, None)?;
    let addresses: Vec<&str> = value.split(delimiter).map(|s| s.trim()).collect();

    addresses
        .iter()
        .map(|addr_str| {
            let addr_bytes = parse_bytes32(addr_str, 40)?;
            let mut result = [0u8; 20];
            result.copy_from_slice(&addr_bytes[12..32]);
            Ok(result)
        })
        .collect()
}

/// Get environment variable as array of bools
pub fn get_bool_array(key: &str, delimiter: &str) -> Result<Vec<bool>, String> {
    let value = get_string(key, None)?;
    let bool_array: Vec<&str> = value.split(delimiter).map(|s| s.trim()).collect();

    Ok(bool_array
        .iter()
        .map(|s| matches!(s.to_lowercase().as_str(), "1" | "true" | "yes"))
        .collect())
}

/// Get environment variable as array of bytes32
pub fn get_bytes32_array(key: &str, delimiter: &str) -> Result<Vec<[u8; 32]>, String> {
    let value = get_string(key, None)?;
    let parts: Vec<&str> = value.split(delimiter).map(|s| s.trim()).collect();

    parts
        .iter()
        .map(|part| {
            let bytes = parse_bytes32(part, 64)?;
            let mut result = [0u8; 32];
            result.copy_from_slice(&bytes);
            Ok(result)
        })
        .collect()
}

/// Get environment variable as array of strings
pub fn get_string_array(key: &str, delimiter: &str) -> Result<Vec<String>, String> {
    let value = get_string(key, None)?;
    Ok(value
        .split(delimiter)
        .map(|s| s.trim().to_string())
        .collect())
}

/// Get environment variable as array of bytes
pub fn get_bytes_array(key: &str, delimiter: &str) -> Result<Vec<Vec<u8>>, String> {
    let value = get_string(key, None)?;
    let parts: Vec<&str> = value.split(delimiter).map(|s| s.trim()).collect();

    parts
        .iter()
        .map(|part| {
            let hex_str = part.strip_prefix("0x").unwrap_or(part);
            hex::decode(hex_str).map_err(|e| format!("Invalid hex bytes '{}': {}", part, e))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_exists() {
        env::set_var("TEST_VAR", "value");
        assert!(exists("TEST_VAR"));
        assert!(!exists("NONEXISTENT_VAR"));
        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_parse_bytes32() {
        // Valid 64-char hex string
        let result = parse_bytes32(
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            64,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);

        // Valid 40-char address
        let result = parse_bytes32("0x1234567890abcdef1234567890abcdef12345678", 40);
        assert!(result.is_ok());

        // Missing 0x prefix
        let result = parse_bytes32("1234567890abcdef", 16);
        assert!(result.is_err());

        // Wrong length
        let result = parse_bytes32("0x1234", 64);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_string() {
        // With default
        env::remove_var("TEST_STRING");
        let result = get_string("TEST_STRING", Some("default_value"));
        assert_eq!(result.unwrap(), "default_value");

        // Without default (should error)
        let result = get_string("TEST_STRING", None);
        assert!(result.is_err());

        // With set value
        env::set_var("TEST_STRING", "hello");
        let result = get_string("TEST_STRING", Some("default"));
        assert_eq!(result.unwrap(), "hello");
        env::remove_var("TEST_STRING");
    }

    #[test]
    fn test_get_int() {
        // Decimal
        env::set_var("TEST_INT", "42");
        assert_eq!(get_int("TEST_INT").unwrap(), 42);

        // Negative decimal
        env::set_var("TEST_INT", "-123");
        assert_eq!(get_int("TEST_INT").unwrap(), -123);

        // Hex with 0x
        env::set_var("TEST_INT", "0x2A");
        assert_eq!(get_int("TEST_INT").unwrap(), 42);

        // Negative hex
        env::set_var("TEST_INT", "-0x10");
        assert_eq!(get_int("TEST_INT").unwrap(), -16);

        // Invalid
        env::set_var("TEST_INT", "not_a_number");
        assert!(get_int("TEST_INT").is_err());

        env::remove_var("TEST_INT");
    }

    #[test]
    fn test_get_uint() {
        // Decimal
        env::set_var("TEST_UINT", "42");
        assert_eq!(get_uint("TEST_UINT").unwrap(), 42);

        // Hex
        env::set_var("TEST_UINT", "0xFF");
        assert_eq!(get_uint("TEST_UINT").unwrap(), 255);

        // Invalid (negative not supported for uint)
        env::set_var("TEST_UINT", "-5");
        assert!(get_uint("TEST_UINT").is_err());

        env::remove_var("TEST_UINT");
    }

    #[test]
    fn test_get_bool() {
        env::set_var("TEST_BOOL", "true");
        assert_eq!(get_bool("TEST_BOOL").unwrap(), true);

        env::set_var("TEST_BOOL", "false");
        assert_eq!(get_bool("TEST_BOOL").unwrap(), false);

        env::set_var("TEST_BOOL", "TRUE");
        assert_eq!(get_bool("TEST_BOOL").unwrap(), true);

        // Invalid (strict matching)
        env::set_var("TEST_BOOL", "1");
        assert!(get_bool("TEST_BOOL").is_err());

        env::set_var("TEST_BOOL", "yes");
        assert!(get_bool("TEST_BOOL").is_err());

        env::remove_var("TEST_BOOL");
    }

    #[test]
    fn test_get_address() {
        // Valid address (40 hex chars)
        env::set_var("TEST_ADDR", "0x1234567890123456789012345678901234567890");
        let result = get_address("TEST_ADDR").unwrap();
        assert_eq!(result.len(), 20);

        // Invalid length
        env::set_var("TEST_ADDR", "0x1234");
        assert!(get_address("TEST_ADDR").is_err());

        env::remove_var("TEST_ADDR");
    }

    #[test]
    fn test_get_bytes32() {
        // Valid bytes32 (64 hex chars)
        env::set_var(
            "TEST_BYTES32",
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );
        let result = get_bytes32("TEST_BYTES32").unwrap();
        assert_eq!(result.len(), 32);

        // Invalid length
        env::set_var("TEST_BYTES32", "0x1234");
        assert!(get_bytes32("TEST_BYTES32").is_err());

        env::remove_var("TEST_BYTES32");
    }

    #[test]
    fn test_get_bytes() {
        // With 0x prefix
        env::set_var("TEST_BYTES", "0x48656c6c6f");
        let result = get_bytes("TEST_BYTES").unwrap();
        assert_eq!(result, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]); // "Hello"

        // Without 0x prefix
        env::set_var("TEST_BYTES", "48656c6c6f");
        let result = get_bytes("TEST_BYTES").unwrap();
        assert_eq!(result, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);

        // Invalid hex
        env::set_var("TEST_BYTES", "0xGGGG");
        assert!(get_bytes("TEST_BYTES").is_err());

        env::remove_var("TEST_BYTES");
    }

    #[test]
    fn test_get_int_array() {
        // Comma-separated
        env::set_var("TEST_INT_ARR", "1,2,3,4,5");
        let result = get_int_array("TEST_INT_ARR", ",").unwrap();
        assert_eq!(result.len(), 5);

        // Negative values
        env::set_var("TEST_INT_ARR", "-1,-2,-3");
        let result = get_int_array("TEST_INT_ARR", ",").unwrap();
        assert_eq!(result.len(), 3);

        // Hex values
        env::set_var("TEST_INT_ARR", "0x10,0x20,0x30");
        let result = get_int_array("TEST_INT_ARR", ",").unwrap();
        assert_eq!(result.len(), 3);

        env::remove_var("TEST_INT_ARR");
    }

    #[test]
    fn test_get_uint_array() {
        env::set_var("TEST_UINT_ARR", "10,20,30,40");
        let result = get_uint_array("TEST_UINT_ARR", ",").unwrap();
        assert_eq!(result.len(), 4);

        // Invalid (negative)
        env::set_var("TEST_UINT_ARR", "1,2,-3");
        assert!(get_uint_array("TEST_UINT_ARR", ",").is_err());

        env::remove_var("TEST_UINT_ARR");
    }

    #[test]
    fn test_get_address_array() {
        env::set_var(
            "TEST_ADDR_ARR",
            "0x1234567890123456789012345678901234567890,0x0987654321098765432109876543210987654321",
        );
        let result = get_address_array("TEST_ADDR_ARR", ",").unwrap();
        assert_eq!(result.len(), 2);

        env::remove_var("TEST_ADDR_ARR");
    }

    #[test]
    fn test_get_bool_array() {
        // Flexible matching for arrays
        env::set_var("TEST_BOOL_ARR", "true,false,1,0,yes,no");
        let result = get_bool_array("TEST_BOOL_ARR", ",").unwrap();
        assert_eq!(result, vec![true, false, true, false, true, false]);

        // Case insensitive
        env::set_var("TEST_BOOL_ARR", "TRUE,FALSE,Yes,No");
        let result = get_bool_array("TEST_BOOL_ARR", ",").unwrap();
        assert_eq!(result, vec![true, false, true, false]);

        env::remove_var("TEST_BOOL_ARR");
    }

    #[test]
    fn test_get_bytes32_array() {
        env::set_var(
            "TEST_B32_ARR",
            "0x1234567890123456789012345678901234567890123456789012345678901234,0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        );
        let result = get_bytes32_array("TEST_B32_ARR", ",").unwrap();
        assert_eq!(result.len(), 2);

        env::remove_var("TEST_B32_ARR");
    }

    #[test]
    fn test_get_string_array() {
        env::set_var("TEST_STR_ARR", "apple,banana,cherry");
        let result = get_string_array("TEST_STR_ARR", ",").unwrap();
        assert_eq!(result, vec!["apple", "banana", "cherry"]);

        // Different delimiter
        env::set_var("TEST_STR_ARR", "one:two:three");
        let result = get_string_array("TEST_STR_ARR", ":").unwrap();
        assert_eq!(result, vec!["one", "two", "three"]);

        env::remove_var("TEST_STR_ARR");
    }

    #[test]
    fn test_get_bytes_array() {
        env::set_var("TEST_BYTES_ARR", "0x48656c6c6f,0x576f726c64");
        let result = get_bytes_array("TEST_BYTES_ARR", ",").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]); // "Hello"
        assert_eq!(result[1], vec![0x57, 0x6f, 0x72, 0x6c, 0x64]); // "World"

        env::remove_var("TEST_BYTES_ARR");
    }
}
