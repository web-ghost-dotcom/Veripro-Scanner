// SPDX-License-Identifier: AGPL-3.0

#[cfg(test)]
mod tests {
    use cbse_env::*;
    use std::env;

    #[test]
    fn test_exists_true() {
        env::set_var("TEST_VAR_EXISTS", "value");
        assert!(exists("TEST_VAR_EXISTS"));
        env::remove_var("TEST_VAR_EXISTS");
    }

    #[test]
    fn test_exists_false() {
        env::remove_var("TEST_VAR_NOT_EXISTS");
        assert!(!exists("TEST_VAR_NOT_EXISTS"));
    }

    #[test]
    fn test_get_string_with_default() {
        env::remove_var("TEST_STRING");
        let result = get_string("TEST_STRING", Some("default"));
        assert_eq!(result.unwrap(), "default");
    }

    #[test]
    fn test_get_string_without_default() {
        env::remove_var("TEST_STRING_NOT_EXISTS");
        let result = get_string("TEST_STRING_NOT_EXISTS", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_string_set_value() {
        env::set_var("TEST_STRING", "hello world");
        assert_eq!(get_string("TEST_STRING", None).unwrap(), "hello world");
        env::remove_var("TEST_STRING");
    }

    #[test]
    fn test_get_int_decimal() {
        env::set_var("TEST_INT", "42");
        assert_eq!(get_int("TEST_INT").unwrap(), 42);
        env::remove_var("TEST_INT");
    }

    #[test]
    fn test_get_int_hex() {
        env::set_var("TEST_INT", "0x2A");
        assert_eq!(get_int("TEST_INT").unwrap(), 42);
        env::remove_var("TEST_INT");
    }

    #[test]
    fn test_get_int_negative() {
        env::set_var("TEST_INT", "-123");
        assert_eq!(get_int("TEST_INT").unwrap(), -123);
        env::remove_var("TEST_INT");
    }

    #[test]
    fn test_get_int_negative_hex() {
        env::set_var("TEST_INT", "-0x10");
        assert_eq!(get_int("TEST_INT").unwrap(), -16);
        env::remove_var("TEST_INT");
    }

    #[test]
    fn test_get_int_invalid() {
        env::set_var("TEST_INT", "not_a_number");
        assert!(get_int("TEST_INT").is_err());
        env::remove_var("TEST_INT");
    }

    #[test]
    fn test_get_uint() {
        env::set_var("TEST_UINT", "42");
        assert_eq!(get_uint("TEST_UINT").unwrap(), 42);
        env::remove_var("TEST_UINT");
    }

    #[test]
    fn test_get_uint_hex() {
        env::set_var("TEST_UINT", "0xFF");
        assert_eq!(get_uint("TEST_UINT").unwrap(), 255);
        env::remove_var("TEST_UINT");
    }

    #[test]
    fn test_get_uint_negative_error() {
        env::set_var("TEST_UINT", "-5");
        assert!(get_uint("TEST_UINT").is_err());
        env::remove_var("TEST_UINT");
    }

    #[test]
    fn test_get_bool_true() {
        env::set_var("TEST_BOOL", "true");
        assert_eq!(get_bool("TEST_BOOL").unwrap(), true);
        env::remove_var("TEST_BOOL");
    }

    #[test]
    fn test_get_bool_false() {
        env::set_var("TEST_BOOL", "false");
        assert_eq!(get_bool("TEST_BOOL").unwrap(), false);
        env::remove_var("TEST_BOOL");
    }

    #[test]
    fn test_get_bool_case_insensitive() {
        env::set_var("TEST_BOOL", "TRUE");
        assert_eq!(get_bool("TEST_BOOL").unwrap(), true);

        env::set_var("TEST_BOOL", "FALSE");
        assert_eq!(get_bool("TEST_BOOL").unwrap(), false);

        env::remove_var("TEST_BOOL");
    }

    #[test]
    fn test_get_bool_invalid() {
        env::set_var("TEST_BOOL", "maybe");
        assert!(get_bool("TEST_BOOL").is_err());
        env::remove_var("TEST_BOOL");
    }

    #[test]
    fn test_get_address() {
        env::set_var("TEST_ADDR", "0x1234567890123456789012345678901234567890");
        let result = get_address("TEST_ADDR").unwrap();
        assert_eq!(result.len(), 20);
        env::remove_var("TEST_ADDR");
    }

    #[test]
    fn test_get_bytes32() {
        env::set_var(
            "TEST_BYTES32",
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );
        let result = get_bytes32("TEST_BYTES32").unwrap();
        assert_eq!(result.len(), 32);
        env::remove_var("TEST_BYTES32");
    }

    #[test]
    fn test_get_bytes() {
        env::set_var("TEST_BYTES", "0x48656c6c6f");
        let result = get_bytes("TEST_BYTES").unwrap();
        assert_eq!(result, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
        env::remove_var("TEST_BYTES");
    }

    #[test]
    fn test_get_bytes_no_prefix() {
        env::set_var("TEST_BYTES", "48656c6c6f");
        let result = get_bytes("TEST_BYTES").unwrap();
        assert_eq!(result, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
        env::remove_var("TEST_BYTES");
    }

    #[test]
    fn test_get_int_array() {
        env::set_var("TEST_INT_ARR", "1,2,3,4,5");
        let result = get_int_array("TEST_INT_ARR", ",").unwrap();
        assert_eq!(result.len(), 5);
        env::remove_var("TEST_INT_ARR");
    }

    #[test]
    fn test_get_int_array_negative() {
        env::set_var("TEST_INT_ARR", "-1,-2,-3");
        let result = get_int_array("TEST_INT_ARR", ",").unwrap();
        assert_eq!(result.len(), 3);
        env::remove_var("TEST_INT_ARR");
    }

    #[test]
    fn test_get_uint_array() {
        env::set_var("TEST_UINT_ARR", "10,20,30,40");
        let result = get_uint_array("TEST_UINT_ARR", ",").unwrap();
        assert_eq!(result.len(), 4);
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
        env::set_var("TEST_BOOL_ARR", "true,false,1,0,yes,no");
        let result = get_bool_array("TEST_BOOL_ARR", ",").unwrap();
        assert_eq!(result, vec![true, false, true, false, true, false]);
        env::remove_var("TEST_BOOL_ARR");
    }

    #[test]
    fn test_get_string_array() {
        env::set_var("TEST_STR_ARR", "apple,banana,cherry");
        let result = get_string_array("TEST_STR_ARR", ",").unwrap();
        assert_eq!(result, vec!["apple", "banana", "cherry"]);
        env::remove_var("TEST_STR_ARR");
    }

    #[test]
    fn test_get_bytes_array() {
        env::set_var("TEST_BYTES_ARR", "0x48656c6c6f,0x576f726c64");
        let result = get_bytes_array("TEST_BYTES_ARR", ",").unwrap();
        assert_eq!(result.len(), 2);
        env::remove_var("TEST_BYTES_ARR");
    }

    #[test]
    fn test_parse_bytes32_valid() {
        let result = parse_bytes32(
            "0x1234567890123456789012345678901234567890123456789012345678901234",
            64,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_parse_bytes32_invalid_prefix() {
        let result = parse_bytes32("1234", 4);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_bytes32_invalid_length() {
        let result = parse_bytes32("0x12", 4);
        assert!(result.is_err());
    }

    #[test]
    fn test_init_env() {
        // Just verify init_env doesn't panic
        init_env(None);
    }
}
