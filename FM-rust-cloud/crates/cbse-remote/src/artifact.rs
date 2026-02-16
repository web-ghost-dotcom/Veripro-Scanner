// SPDX-License-Identifier: AGPL-3.0

//! Data structures for remote job artifacts and results

use cbse_config::Config;
use serde::{Deserialize, Serialize};

/// Job artifact containing all necessary data for remote execution
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobArtifact {
    pub contracts: Vec<ContractData>,
    pub config: ExecutionConfig,
    pub metadata: ArtifactMetadata,
}

/// Data for a single contract to test
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContractData {
    pub name: String,
    pub bytecode: String,
    pub abi: serde_json::Value,
    pub test_functions: Vec<String>,
}

/// Configuration for symbolic execution
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionConfig {
    // Core execution parameters
    pub verbosity: u8,
    pub solver_timeout_ms: u64,
    pub loop_bound: usize,
    pub depth: usize,
    pub width: usize,
    pub storage_layout: Option<String>,

    // Debug and output flags
    pub debug: bool,
    pub debug_config: bool,
    pub print_steps: bool,
    pub print_mem: bool,
    pub print_states: bool,
    pub print_success_states: bool,
    pub print_failed_states: bool,
    pub print_blocked_states: bool,
    pub print_setup_states: bool,
    pub print_full_model: bool,
    pub statistics: bool,
    pub dump_smt_queries: bool,
    pub dump_smt_directory: String,

    // Solver configuration
    pub solver: String,
    pub smt_exp_by_const: u64,
    pub solver_timeout_branching: u64,
    pub solver_max_memory: u64,
    pub solver_command: String,
    pub solver_threads: Option<usize>,
    pub cache_solver: bool,

    // Other execution options
    pub symbolic_jump: bool,
    pub early_exit: bool,
    pub uninterpreted_unknown_calls: String,
    pub return_size_of_unknown_calls: usize,
}

/// Metadata about the artifact
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArtifactMetadata {
    pub created_at: String,
    pub cbse_version: String,
}

/// Result from remote job execution
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobResult {
    pub status: String,
    pub test_results: Vec<TestResult>,
    pub execution_time_ms: u64,
    pub traces: Vec<String>,
    pub counterexamples: Vec<String>,
}

/// Result of a single test execution
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub counterexample: Option<String>,
    pub gas_used: u64,
}

impl JobArtifact {
    /// Create a new empty job artifact
    pub fn new() -> Self {
        Self {
            contracts: Vec::new(),
            config: ExecutionConfig {
                verbosity: 0,
                solver_timeout_ms: 30000,
                loop_bound: 2,
                depth: 0,
                width: 0,
                storage_layout: None,
                debug: false,
                debug_config: false,
                print_steps: false,
                print_mem: false,
                print_states: false,
                print_success_states: false,
                print_failed_states: false,
                print_blocked_states: false,
                print_setup_states: false,
                print_full_model: false,
                statistics: false,
                dump_smt_queries: false,
                dump_smt_directory: String::new(),
                solver: "z3".to_string(),
                smt_exp_by_const: 2,
                solver_timeout_branching: 1000,
                solver_max_memory: 0,
                solver_command: String::new(),
                solver_threads: None,
                cache_solver: false,
                symbolic_jump: false,
                early_exit: false,
                uninterpreted_unknown_calls: "all".to_string(),
                return_size_of_unknown_calls: 32,
            },
            metadata: ArtifactMetadata {
                created_at: chrono::Utc::now().to_rfc3339(),
                cbse_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    /// Set configuration from Config struct
    pub fn set_config(&mut self, config: &Config) {
        // Core execution parameters
        self.config.verbosity = config.verbose;
        self.config.solver_timeout_ms = config.solver_timeout_assertion;
        self.config.loop_bound = config.loop_bound;
        self.config.depth = config.depth;
        self.config.width = config.width;
        self.config.storage_layout = if config.storage_layout.is_empty() {
            None
        } else {
            Some(config.storage_layout.clone())
        };

        // Debug and output flags
        self.config.debug = config.debug;
        self.config.debug_config = config.debug_config;
        self.config.print_steps = config.print_steps;
        self.config.print_mem = config.print_mem;
        self.config.print_states = config.print_states;
        self.config.print_success_states = config.print_success_states;
        self.config.print_failed_states = config.print_failed_states;
        self.config.print_blocked_states = config.print_blocked_states;
        self.config.print_setup_states = config.print_setup_states;
        self.config.print_full_model = config.print_full_model;
        self.config.statistics = config.statistics;
        self.config.dump_smt_queries = config.dump_smt_queries;
        self.config.dump_smt_directory = config.dump_smt_directory.clone();

        // Solver configuration
        self.config.solver = config.solver.clone();
        self.config.smt_exp_by_const = config.smt_exp_by_const as u64;
        self.config.solver_timeout_branching = config.solver_timeout_branching;
        self.config.solver_max_memory = config.solver_max_memory as u64;
        self.config.solver_command = config.solver_command.clone();
        self.config.solver_threads = config.solver_threads;
        self.config.cache_solver = config.cache_solver;

        // Other execution options
        self.config.symbolic_jump = config.symbolic_jump;
        self.config.early_exit = config.early_exit;
        self.config.uninterpreted_unknown_calls = config.uninterpreted_unknown_calls.clone();
        self.config.return_size_of_unknown_calls = config.return_size_of_unknown_calls;
    }

    /// Add a contract to test
    pub fn add_contract(
        &mut self,
        name: String,
        bytecode: String,
        abi: serde_json::Value,
        test_functions: Vec<String>,
    ) {
        self.contracts.push(ContractData {
            name,
            bytecode,
            abi,
            test_functions,
        });
    }
}

impl Default for JobArtifact {
    fn default() -> Self {
        Self::new()
    }
}
