// SPDX-License-Identifier: AGPL-3.0

//! # cbse-remote
//!
//! Remote execution module for CBSE (Constraint-Based Symbolic Execution Engine).
//!
//! This crate handles SSH-based remote execution of symbolic tests on dedicated nodes,
//! allowing users to offload computation-intensive symbolic execution to remote machines
//! while compiling contracts locally.
//!
//! ## Features
//!
//! - SSH password authentication (compatible with Tailscale)
//! - Secure artifact upload/download via SFTP
//! - Remote job execution with result retrieval
//! - Connection testing and validation
//!
//! ## Example
//!
//! ```no_run
//! use cbse_remote::{RemoteExecutor, JobArtifact, ExecutionConfig, ArtifactMetadata};
//!
//! # fn main() -> anyhow::Result<()> {
//! let executor = RemoteExecutor::new(
//!     "node10",        // hostname
//!     22,              // port
//!     "node10",        // username
//!     "password",      // password
//!     "/tmp/cbse-jobs",
//!     "/usr/local/bin/cbse",
//! )?;
//!
//! // Test connection
//! executor.test_connection()?;
//!
//! // Execute job
//! let artifact = JobArtifact {
//!     bytecode: "0x...".to_string(),
//!     abi: serde_json::json!({}),
//!     test_functions: vec!["test_foo".to_string()],
//!     config: ExecutionConfig {
//!         verbosity: 1,
//!         solver_timeout_ms: 30000,
//!         loop_bound: 2,
//!         depth: 0,
//!         width: 0,
//!         storage_layout: None,
//!     },
//!     metadata: ArtifactMetadata {
//!         contract_name: "MyContract".to_string(),
//!         source_file: "test.sol".to_string(),
//!         compiler_version: "0.8.0".to_string(),
//!         created_at: chrono::Utc::now().to_rfc3339(),
//!     },
//! };
//!
//! let result = executor.execute(&artifact)?;
//! println!("Tests passed: {}/{}",
//!     result.test_results.iter().filter(|t| t.passed).count(),
//!     result.test_results.len()
//! );
//! # Ok(())
//! # }
//! ```

mod artifact;
mod executor;
mod ssh;

pub use artifact::{ArtifactMetadata, ExecutionConfig, JobArtifact, JobResult, TestResult};
pub use executor::RemoteExecutor;
pub use ssh::SshConnection;

use anyhow::Result;

/// Prompt for password from stdin (without echoing)
pub fn prompt_password(prompt: &str) -> Result<String> {
    rpassword::prompt_password(prompt)
        .map_err(|e| anyhow::anyhow!("Failed to read password: {}", e))
}

/// Quick test function for remote connection
pub fn test_remote_connection(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    remote_binary: &str,
) -> Result<()> {
    let executor = RemoteExecutor::new(
        host,
        port,
        username,
        password,
        "/tmp/cbse-jobs",
        remote_binary,
    )?;

    executor.test_connection()
}
