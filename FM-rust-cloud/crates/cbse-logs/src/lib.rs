// SPDX-License-Identifier: AGPL-3.0

//! Logging and diagnostic utilities matching Python halmos logs.py

use colored::*;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::sync::Mutex;

/// Warnings base URL
pub const WARNINGS_BASE_URL: &str = "https://github.com/a16z/halmos/wiki/warnings";

/// Error codes for warnings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    ParsingError,
    InternalError,
    LibraryPlaceholder,
    CounterexampleInvalid,
    CounterexampleUnknown,
    UnsupportedOpcode,
    RevertAll,
    LoopBound,
}

impl ErrorCode {
    pub fn code(&self) -> &'static str {
        match self {
            ErrorCode::ParsingError => "parsing-error",
            ErrorCode::InternalError => "internal-error",
            ErrorCode::LibraryPlaceholder => "library-placeholder",
            ErrorCode::CounterexampleInvalid => "counterexample-invalid",
            ErrorCode::CounterexampleUnknown => "counterexample-unknown",
            ErrorCode::UnsupportedOpcode => "unsupported-opcode",
            ErrorCode::RevertAll => "revert-all",
            ErrorCode::LoopBound => "loop-bound",
        }
    }

    pub fn url(&self) -> String {
        format!("{}#{}", WARNINGS_BASE_URL, self.code())
    }
}

/// Logger state for tracking unique messages
static UNIQUE_MESSAGES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// Check if a message has been logged (for unique logging)
fn is_logged(message: &str) -> bool {
    let messages = UNIQUE_MESSAGES.lock().unwrap();
    messages.contains(message)
}

/// Mark a message as logged
fn mark_logged(message: &str) {
    let mut messages = UNIQUE_MESSAGES.lock().unwrap();
    messages.insert(message.to_string());
}

/// Log a debug message
pub fn debug(text: &str, allow_duplicate: bool) {
    if allow_duplicate || !is_logged(text) {
        eprintln!("{}", text.dimmed());
        if !allow_duplicate {
            mark_logged(text);
        }
    }
}

/// Log an info message
pub fn info(text: &str, allow_duplicate: bool) {
    if allow_duplicate || !is_logged(text) {
        println!("{}", text);
        if !allow_duplicate {
            mark_logged(text);
        }
    }
}

/// Log a warning message
pub fn warn(text: &str, allow_duplicate: bool) {
    if allow_duplicate || !is_logged(text) {
        eprintln!("{}", text.yellow());
        if !allow_duplicate {
            mark_logged(text);
        }
    }
}

/// Log an error message
pub fn error(text: &str, allow_duplicate: bool) {
    if allow_duplicate || !is_logged(text) {
        eprintln!("{}", text.red());
        if !allow_duplicate {
            mark_logged(text);
        }
    }
}

/// Log a debug message once (no duplicates)
pub fn debug_once(text: &str) {
    debug(text, false);
}

/// Log a warning with an error code
pub fn warn_code(error_code: ErrorCode, msg: &str, allow_duplicate: bool) {
    let full_msg = format!("{}\n(see {})", msg, error_code.url());
    warn(&full_msg, allow_duplicate);
}

/// Log a unique warning (alias for warn with allow_duplicate=false)
pub fn warn_unique(text: &str) {
    warn(text, false);
}

/// Clear all logged messages (useful for testing)
pub fn clear_logged_messages() {
    let mut messages = UNIQUE_MESSAGES.lock().unwrap();
    messages.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_url() {
        let code = ErrorCode::ParsingError;
        assert_eq!(code.code(), "parsing-error");
        assert!(code.url().contains("parsing-error"));
        assert!(code.url().starts_with(WARNINGS_BASE_URL));
    }

    #[test]
    fn test_all_error_codes() {
        let codes = vec![
            ErrorCode::ParsingError,
            ErrorCode::InternalError,
            ErrorCode::LibraryPlaceholder,
            ErrorCode::CounterexampleInvalid,
            ErrorCode::CounterexampleUnknown,
            ErrorCode::UnsupportedOpcode,
            ErrorCode::RevertAll,
            ErrorCode::LoopBound,
        ];

        for code in codes {
            assert!(!code.code().is_empty());
            assert!(code.url().contains(code.code()));
        }
    }

    #[test]
    fn test_unique_logging() {
        clear_logged_messages();

        let msg = "test unique message";
        assert!(!is_logged(msg));

        debug(msg, false);
        assert!(is_logged(msg));

        // Second call should not log again
        debug(msg, false);
        assert!(is_logged(msg));

        clear_logged_messages();
        assert!(!is_logged(msg));
    }

    #[test]
    fn test_debug_once() {
        clear_logged_messages();

        let msg = "debug once message";
        debug_once(msg);
        assert!(is_logged(msg));

        // Should not log again
        debug_once(msg);
    }

    #[test]
    fn test_allow_duplicate() {
        clear_logged_messages();

        let msg = "duplicate message";
        info(msg, true);
        assert!(!is_logged(msg)); // Should not be tracked when allow_duplicate=true

        info(msg, false);
        assert!(is_logged(msg)); // Should be tracked when allow_duplicate=false
    }

    #[test]
    fn test_warn_code() {
        clear_logged_messages();

        warn_code(ErrorCode::InternalError, "Something went wrong", true);
        // Just verify it doesn't panic
    }
}
