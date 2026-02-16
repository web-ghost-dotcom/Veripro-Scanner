// SPDX-License-Identifier: AGPL-3.0

//! Test result reporting
//! Corresponds to Python's TestResult and MainResult dataclasses

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main execution result (matches Python MainResult)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainResult {
    pub exitcode: i32,
    pub total_passed: usize,
    pub total_failed: usize,
    pub total_found: usize,
    #[serde(with = "humantime_serde")]
    pub duration: Duration,
}

/// Individual test result (matches Python TestResult)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String, // test function name (funsig)
    pub exitcode: i32,
    pub num_models: Option<usize>,
    pub num_paths: Option<(usize, usize, usize)>, // (total, success, blocked)
    pub num_bounded_loops: Option<usize>,
}

/// Exit codes (matches Python Exitcode enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exitcode {
    Pass = 0,
    Counterexample = 1,
    Timeout = 2,
    Stuck = 3,
    RevertAll = 4,
    Exception = 5,
}

impl TestResult {
    pub fn new(name: String) -> Self {
        Self {
            name,
            exitcode: Exitcode::Pass as i32,
            num_models: None,
            num_paths: None,
            num_bounded_loops: None,
        }
    }

    pub fn passed(&self) -> bool {
        self.exitcode == Exitcode::Pass as i32
    }

    pub fn failed(&self) -> bool {
        !self.passed()
    }
}

impl MainResult {
    pub fn empty() -> Self {
        Self {
            exitcode: 0,
            total_passed: 0,
            total_failed: 0,
            total_found: 0,
            duration: Duration::from_secs(0),
        }
    }

    pub fn has_failures(&self) -> bool {
        self.total_failed > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exitcode_values() {
        assert_eq!(Exitcode::Pass as i32, 0);
        assert_eq!(Exitcode::Counterexample as i32, 1);
        assert_eq!(Exitcode::Timeout as i32, 2);
    }

    #[test]
    fn test_test_result() {
        let result = TestResult::new("test_foo".to_string());
        assert!(result.passed());
        assert!(!result.failed());
    }

    #[test]
    fn test_main_result() {
        let result = MainResult::empty();
        assert!(!result.has_failures());
        assert_eq!(result.exitcode, 0);
    }
}
