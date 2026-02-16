// SPDX-License-Identifier: AGPL-3.0

//! Configuration management for CBSE
//!
//! This module provides configuration parsing and management functionality,
//! matching the behavior of halmos/config.py

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// Configuration source priority (matches Python ConfigSource IntEnum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum ConfigSource {
    Void = 0,
    Default = 1,
    ConfigFile = 2,
    ContractAnnotation = 3,
    FunctionAnnotation = 4,
    CommandLine = 5,
}

impl ConfigSource {
    pub fn name(&self) -> &'static str {
        match self {
            ConfigSource::Void => "void",
            ConfigSource::Default => "default",
            ConfigSource::ConfigFile => "config_file",
            ConfigSource::ContractAnnotation => "contract_annotation",
            ConfigSource::FunctionAnnotation => "function_annotation",
            ConfigSource::CommandLine => "command_line",
        }
    }
}

/// Trace events to capture (matches Python TraceEvent Enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceEvent {
    #[serde(rename = "LOG")]
    Log,
    #[serde(rename = "SSTORE")]
    SStore,
    #[serde(rename = "SLOAD")]
    SLoad,
}

impl TraceEvent {
    pub fn value(&self) -> &'static str {
        match self {
            TraceEvent::Log => "LOG",
            TraceEvent::SStore => "SSTORE",
            TraceEvent::SLoad => "SLOAD",
        }
    }

    pub fn all() -> Vec<TraceEvent> {
        vec![TraceEvent::Log, TraceEvent::SStore, TraceEvent::SLoad]
    }
}

impl std::str::FromStr for TraceEvent {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "LOG" => Ok(TraceEvent::Log),
            "SSTORE" => Ok(TraceEvent::SStore),
            "SLOAD" => Ok(TraceEvent::SLoad),
            _ => Err(anyhow::anyhow!("Invalid trace event: {}", s)),
        }
    }
}

impl std::fmt::Display for TraceEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value())
    }
}

/// Main CBSE configuration (matches Python Config dataclass)
#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
#[clap(
    name = "cbse",
    version,
    about = "Complete Blockchain Symbolic Executor",
    disable_version_flag = true
)]
pub struct Config {
    // === General options ===
    /// Project root directory
    #[clap(long, default_value = ".")]
    #[serde(default = "default_root")]
    pub root: PathBuf,

    /// Path to the config file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Run tests in the given contract
    #[clap(long, default_value = "")]
    #[serde(default)]
    pub contract: String,

    /// Run tests in contracts matching the given regex
    #[clap(long, short = 'm', default_value = "")]
    #[serde(default)]
    pub match_contract: String,

    /// Run tests matching the given prefix
    #[clap(long, default_value = "(check|invariant)_")]
    #[serde(default = "default_function")]
    pub function: String,

    /// Run tests matching the given regex
    #[clap(long, short = 't', default_value = "")]
    #[serde(default)]
    pub match_test: String,

    /// Panic error codes to treat as test failures
    #[clap(long, default_value = "0x01")]
    #[serde(default = "default_panic_codes")]
    pub panic_error_codes: String,

    /// Depth for invariant testing
    #[clap(long, default_value = "2")]
    #[serde(default = "default_invariant_depth")]
    pub invariant_depth: usize,

    /// Loop unrolling bounds
    #[clap(long, default_value = "2")]
    #[serde(default = "default_loop")]
    pub loop_bound: usize,

    /// Max number of paths (0 = unlimited)
    #[clap(long, default_value = "0")]
    #[serde(default)]
    pub width: usize,

    /// Max length in steps of a single path (0 = unlimited)
    #[clap(long, default_value = "0")]
    #[serde(default)]
    pub depth: usize,

    /// Array lengths specification
    #[clap(long)]
    pub array_lengths: Option<String>,

    // === Protocol Options ===
    /// Initial "Prover Mode" - output becomes a Signed Attestation JSON
    #[clap(long)]
    #[serde(default)]
    pub prover_mode: bool,

    /// Private key for signing attestations (hex)
    #[clap(long, requires = "prover_mode")]
    #[serde(default)]
    pub private_key: Option<String>,

    /// Default lengths for dynamic arrays
    #[clap(long, default_value = "0,1,2")]
    #[serde(default = "default_array_lengths")]
    pub default_array_lengths: String,

    /// Default lengths for bytes and string
    #[clap(long, default_value = "0,65,1024")]
    #[serde(default = "default_bytes_lengths")]
    pub default_bytes_lengths: String,

    /// Storage layout model
    #[clap(long, default_value = "solidity")]
    #[serde(default = "default_storage_layout")]
    pub storage_layout: String,

    /// Allow FFI to call external functions
    #[clap(long)]
    #[serde(default)]
    pub ffi: bool,

    /// Print version number
    #[clap(long)]
    #[serde(default)]
    pub version: bool,

    /// Coverage report file path
    #[clap(long)]
    pub coverage_output: Option<PathBuf>,

    // === Debugging options ===
    /// Verbosity level (can be repeated: -v, -vv, -vvv)
    #[clap(short, long, action = clap::ArgAction::Count)]
    #[serde(default)]
    pub verbose: u8,

    /// Print statistics
    #[clap(long)]
    #[serde(default)]
    pub statistics: bool,

    /// Disable progress display
    #[clap(long)]
    #[serde(default)]
    pub no_status: bool,

    /// Run in debug mode
    #[clap(long)]
    #[serde(default)]
    pub debug: bool,

    /// Debug config parsing
    #[clap(long)]
    #[serde(default)]
    pub debug_config: bool,

    /// Profile instruction execution frequencies
    #[clap(long)]
    #[serde(default)]
    pub profile_instructions: bool,

    /// Output test results in JSON
    #[clap(long)]
    pub json_output: Option<PathBuf>,

    /// Include minimal information in JSON output
    #[clap(long)]
    #[serde(default)]
    pub minimal_json_output: bool,

    /// Print every execution step
    #[clap(long)]
    #[serde(default)]
    pub print_steps: bool,

    /// Print memory contents with --print-steps
    #[clap(long)]
    #[serde(default)]
    pub print_mem: bool,

    /// Print all final execution states
    #[clap(long)]
    #[serde(default)]
    pub print_states: bool,

    /// Print successful execution states
    #[clap(long)]
    #[serde(default)]
    pub print_success_states: bool,

    /// Print failed execution states
    #[clap(long)]
    #[serde(default)]
    pub print_failed_states: bool,

    /// Print blocked execution states
    #[clap(long)]
    #[serde(default)]
    pub print_blocked_states: bool,

    /// Print setup execution states
    #[clap(long)]
    #[serde(default)]
    pub print_setup_states: bool,

    /// Print full counterexample model
    #[clap(long)]
    #[serde(default)]
    pub print_full_model: bool,

    /// Stop after a counterexample is found
    #[clap(long)]
    #[serde(default)]
    pub early_exit: bool,

    /// Dump SMT queries for assertion violations
    #[clap(long)]
    #[serde(default)]
    pub dump_smt_queries: bool,

    /// Directory to dump SMT queries
    #[clap(long, default_value = "")]
    #[serde(default)]
    pub dump_smt_directory: String,

    /// Disable Python's automatic garbage collection
    #[clap(long)]
    #[serde(default)]
    pub disable_gc: bool,

    /// Trace memory allocations
    #[clap(long)]
    #[serde(default)]
    pub trace_memory: bool,

    /// Include specific events in traces
    #[clap(long)]
    pub trace_events: Option<String>,

    // === Build options ===
    /// Forge build artifacts directory name
    #[clap(long, default_value = "out")]
    #[serde(default = "default_forge_build_out")]
    pub forge_build_out: String,

    // === Solver options ===
    /// SMT solver to use
    #[clap(long, default_value = "yices")]
    #[serde(default = "default_solver")]
    pub solver: String,

    /// Interpret constant power up to N
    #[clap(long, default_value = "2")]
    #[serde(default = "default_smt_exp")]
    pub smt_exp_by_const: usize,

    /// Timeout for solving branching conditions (ms)
    #[clap(long, default_value = "1")]
    #[serde(default = "default_solver_timeout_branching")]
    pub solver_timeout_branching: u64,

    /// Timeout for solving assertion violations (seconds)
    #[clap(long, default_value = "60")]
    #[serde(default = "default_solver_timeout_assertion")]
    pub solver_timeout_assertion: u64,

    /// Memory limit for solver in MB (0 = no limit)
    #[clap(long, default_value = "0")]
    #[serde(default)]
    pub solver_max_memory: usize,

    /// Exact solver command to use
    #[clap(long, default_value = "")]
    #[serde(default)]
    pub solver_command: String,

    /// Number of threads for parallel solvers
    #[clap(long)]
    pub solver_threads: Option<usize>,

    /// Cache unsat queries using unsat cores
    #[clap(long)]
    #[serde(default)]
    pub cache_solver: bool,

    // === Experimental options ===
    /// Support symbolic jump destination
    #[clap(long)]
    #[serde(default)]
    pub symbolic_jump: bool,

    /// Generate flamegraph of execution
    #[clap(long)]
    #[serde(default)]
    pub flamegraph: bool,

    // === Remote execution options (SSH) ===
    /// Execute on remote SSH node instead of locally
    #[clap(long)]
    #[serde(default)]
    pub ssh: bool,

    /// SSH hostname (e.g., node10@node10 or just node10)
    #[clap(long, default_value = "")]
    #[serde(default)]
    pub ssh_host: String,

    /// SSH port
    #[clap(long, default_value = "22")]
    #[serde(default = "default_ssh_port")]
    pub ssh_port: u16,

    /// SSH username (optional, extracted from ssh_host if format is user@host)
    #[clap(long)]
    #[serde(default)]
    pub ssh_user: Option<String>,

    /// Remote CBSE binary path
    #[clap(long, default_value = "/usr/local/bin/cbse")]
    #[serde(default = "default_ssh_remote_binary")]
    pub ssh_remote_binary: String,

    /// Remote working directory for jobs
    #[clap(long, default_value = "/tmp/cbse-jobs")]
    #[serde(default = "default_ssh_remote_workdir")]
    pub ssh_remote_workdir: String,

    /// Test SSH connection and exit
    #[clap(long)]
    #[serde(default)]
    pub ssh_test: bool,

    /// Worker mode: read from --input, write to --output (internal use)
    #[clap(long)]
    #[serde(default)]
    pub worker_mode: bool,

    /// Input artifact path (worker mode)
    #[clap(long)]
    pub input: Option<PathBuf>,

    /// Output result path (worker mode)
    #[clap(long)]
    pub output: Option<PathBuf>,

    // === Deprecated options ===
    /// (Deprecated) Run tests in parallel
    #[clap(long)]
    #[serde(default)]
    pub test_parallel: bool,

    /// (Deprecated) Run assertion solvers in parallel
    #[clap(long)]
    #[serde(default)]
    pub solver_parallel: bool,

    /// (Deprecated) Log execution steps in JSON
    #[clap(long)]
    pub log: Option<PathBuf>,

    /// (Deprecated) Uninterpreted unknown calls
    #[clap(long, default_value = "0x150b7a02,0x1626ba7e,0xf23a6e61,0xbc197c81")]
    #[serde(default = "default_uninterpreted")]
    pub uninterpreted_unknown_calls: String,

    /// (Deprecated) Return size of unknown calls
    #[clap(long, default_value = "32")]
    #[serde(default = "default_return_size")]
    pub return_size_of_unknown_calls: usize,
}

// Default value functions
fn default_root() -> PathBuf {
    PathBuf::from(".")
}

fn default_function() -> String {
    "(check|invariant)_".to_string()
}

fn default_panic_codes() -> String {
    "0x01".to_string()
}

fn default_invariant_depth() -> usize {
    2
}

fn default_loop() -> usize {
    2
}

fn default_array_lengths() -> String {
    "0,1,2".to_string()
}

fn default_bytes_lengths() -> String {
    "0,65,1024".to_string()
}

fn default_ssh_port() -> u16 {
    22
}

fn default_ssh_remote_binary() -> String {
    "/usr/local/bin/cbse".to_string()
}

fn default_ssh_remote_workdir() -> String {
    "/tmp/cbse-jobs".to_string()
}

fn default_storage_layout() -> String {
    "solidity".to_string()
}

fn default_forge_build_out() -> String {
    "out".to_string()
}

fn default_solver() -> String {
    "yices".to_string()
}

fn default_smt_exp() -> usize {
    2
}

fn default_solver_timeout_branching() -> u64 {
    1
}

fn default_solver_timeout_assertion() -> u64 {
    60
}

fn default_uninterpreted() -> String {
    "0x150b7a02,0x1626ba7e,0xf23a6e61,0xbc197c81".to_string()
}

fn default_return_size() -> usize {
    32
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root: default_root(),
            config: None,
            contract: String::new(),
            match_contract: String::new(),
            function: default_function(),
            match_test: String::new(),
            panic_error_codes: default_panic_codes(),
            invariant_depth: default_invariant_depth(),
            loop_bound: default_loop(),
            width: 0,
            depth: 0,
            array_lengths: None,
            prover_mode: false,
            private_key: None,
            default_array_lengths: default_array_lengths(),
            default_bytes_lengths: default_bytes_lengths(),
            storage_layout: default_storage_layout(),
            ffi: false,
            version: false,
            coverage_output: None,
            verbose: 0,
            statistics: false,
            no_status: false,
            debug: false,
            debug_config: false,
            profile_instructions: false,
            json_output: None,
            minimal_json_output: false,
            print_steps: false,
            print_mem: false,
            print_states: false,
            print_success_states: false,
            print_failed_states: false,
            print_blocked_states: false,
            print_setup_states: false,
            print_full_model: false,
            early_exit: false,
            dump_smt_queries: false,
            dump_smt_directory: String::new(),
            disable_gc: false,
            trace_memory: false,
            trace_events: None,
            forge_build_out: default_forge_build_out(),
            solver: default_solver(),
            smt_exp_by_const: default_smt_exp(),
            solver_timeout_branching: default_solver_timeout_branching(),
            solver_timeout_assertion: default_solver_timeout_assertion(),
            solver_max_memory: 0,
            solver_command: String::new(),
            solver_threads: None,
            cache_solver: false,
            symbolic_jump: false,
            flamegraph: false,
            ssh: false,
            ssh_host: String::new(),
            ssh_port: default_ssh_port(),
            ssh_user: None,
            ssh_remote_binary: default_ssh_remote_binary(),
            ssh_remote_workdir: default_ssh_remote_workdir(),
            ssh_test: false,
            worker_mode: false,
            input: None,
            output: None,
            test_parallel: false,
            solver_parallel: false,
            log: None,
            uninterpreted_unknown_calls: default_uninterpreted(),
            return_size_of_unknown_calls: default_return_size(),
        }
    }
}

impl Config {
    /// Load configuration from TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        let parsed: TomlConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {:?}", path))?;

        // Convert from TomlConfig to Config
        parsed.to_config()
    }

    /// Merge with another configuration (command line overrides file config)
    pub fn merge(&mut self, other: Self) {
        // Override with non-default values from other config
        if !other.contract.is_empty() {
            self.contract = other.contract;
        }
        if !other.match_contract.is_empty() {
            self.match_contract = other.match_contract;
        }
        if !other.match_test.is_empty() {
            self.match_test = other.match_test;
        }
        if other.function != default_function() {
            self.function = other.function;
        }
        if other.verbose > 0 {
            self.verbose = other.verbose;
        }
        if other.debug {
            self.debug = other.debug;
        }
        if other.loop_bound != default_loop() {
            self.loop_bound = other.loop_bound;
        }
        if other.width > 0 {
            self.width = other.width;
        }
        if other.depth > 0 {
            self.depth = other.depth;
        }
        if other.solver_timeout_assertion != default_solver_timeout_assertion() {
            self.solver_timeout_assertion = other.solver_timeout_assertion;
        }
        if other.solver_timeout_branching != default_solver_timeout_branching() {
            self.solver_timeout_branching = other.solver_timeout_branching;
        }
        // Add more fields as needed
    }

    /// Parse array lengths specification
    /// Format: name1={1,2,3},name2=5
    pub fn parse_array_lengths(&self) -> Result<HashMap<String, Vec<usize>>> {
        let mut result = HashMap::new();

        if let Some(spec) = &self.array_lengths {
            parse_array_lengths_string(spec, &mut result)?;
        }

        Ok(result)
    }

    /// Parse panic error codes
    pub fn parse_panic_error_codes(&self) -> Result<Vec<u64>> {
        if self.panic_error_codes == "*" {
            return Ok(vec![]); // Empty means match all
        }

        let mut codes = Vec::new();
        for part in self.panic_error_codes.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            // Support hex (0x...) and decimal
            let code = if part.starts_with("0x") || part.starts_with("0X") {
                u64::from_str_radix(&part[2..], 16)?
            } else {
                part.parse::<u64>()?
            };
            codes.push(code);
        }

        if codes.is_empty() {
            return Err(anyhow::anyhow!("Panic error codes list cannot be empty"));
        }

        Ok(codes)
    }

    /// Parse trace events
    pub fn parse_trace_events(&self) -> Result<Vec<TraceEvent>> {
        if let Some(events_str) = &self.trace_events {
            parse_csv_trace_events(events_str)
        } else {
            // Default: all events
            Ok(TraceEvent::all())
        }
    }

    /// Parse default array lengths
    pub fn parse_default_array_lengths(&self) -> Result<Vec<usize>> {
        parse_csv_int(&self.default_array_lengths)
    }

    /// Parse default bytes lengths
    pub fn parse_default_bytes_lengths(&self) -> Result<Vec<usize>> {
        parse_csv_int(&self.default_bytes_lengths)
    }

    /// Get solver threads (defaults to CPU count)
    pub fn get_solver_threads(&self) -> usize {
        self.solver_threads.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        })
    }

    /// Resolve config file path
    pub fn resolve_config_path(&self) -> Option<PathBuf> {
        if let Some(config) = &self.config {
            Some(config.clone())
        } else {
            let default_path = self.root.join("halmos.toml");
            if default_path.exists() {
                Some(default_path)
            } else {
                None
            }
        }
    }

    /// Resolve solver command from solver name or explicit command
    /// Matches Python's resolved_solver_command property
    pub fn resolved_solver_command(&self) -> Result<Vec<String>> {
        // If solver_command is explicitly set, use it
        if !self.solver_command.is_empty() {
            return Ok(shell_words::split(&self.solver_command)?);
        }

        // Otherwise, resolve from solver name
        get_solver_command(&self.solver)
    }

    /// Unparse panic error codes to string (for TOML generation)
    pub fn unparse_panic_error_codes(&self) -> String {
        if self.panic_error_codes.is_empty() || self.panic_error_codes == "*" {
            "*".to_string()
        } else {
            self.panic_error_codes.clone()
        }
    }

    /// Unparse trace events to string
    pub fn unparse_trace_events(&self) -> String {
        if let Some(events) = &self.trace_events {
            events.clone()
        } else {
            TraceEvent::all()
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(",")
        }
    }

    /// Unparse array lengths to string
    pub fn unparse_array_lengths(&self) -> String {
        if let Some(spec) = &self.array_lengths {
            spec.clone()
        } else {
            String::new()
        }
    }

    /// Parse timeout string (supports units: ms, s, m, h)
    /// Matches Python's parse_time utility
    pub fn parse_timeout(timeout_str: &str) -> Result<u64> {
        parse_time(timeout_str, "ms")
    }

    /// Unparse timeout to string
    pub fn unparse_timeout(timeout_ms: u64) -> String {
        if timeout_ms < 1000 {
            format!("{}ms", timeout_ms)
        } else {
            format!("{}s", timeout_ms / 1000)
        }
    }
}

/// TOML configuration structure (for parsing from file)
#[derive(Debug, Deserialize)]
struct TomlConfig {
    #[serde(default)]
    global: HashMap<String, toml::Value>,
}

impl TomlConfig {
    fn to_config(self) -> Result<Config> {
        let mut config = Config::default();

        for (key, value) in self.global {
            // Convert kebab-case to snake_case
            let key = key.replace('-', "_");

            match key.as_str() {
                "root" => config.root = parse_toml_path(&value)?,
                "contract" => config.contract = parse_toml_string(&value)?,
                "match_contract" => config.match_contract = parse_toml_string(&value)?,
                "function" => config.function = parse_toml_string(&value)?,
                "match_test" => config.match_test = parse_toml_string(&value)?,
                "panic_error_codes" => config.panic_error_codes = parse_toml_string(&value)?,
                "invariant_depth" => config.invariant_depth = parse_toml_usize(&value)?,
                "loop_bound" | "loop" => config.loop_bound = parse_toml_usize(&value)?,
                "width" => config.width = parse_toml_usize(&value)?,
                "depth" => config.depth = parse_toml_usize(&value)?,
                "array_lengths" => config.array_lengths = Some(parse_toml_string(&value)?),
                "default_array_lengths" => {
                    config.default_array_lengths = parse_toml_string(&value)?
                }
                "default_bytes_lengths" => {
                    config.default_bytes_lengths = parse_toml_string(&value)?
                }
                "storage_layout" => config.storage_layout = parse_toml_string(&value)?,
                "ffi" => config.ffi = parse_toml_bool(&value)?,
                "verbose" => config.verbose = parse_toml_u8(&value)?,
                "statistics" => config.statistics = parse_toml_bool(&value)?,
                "debug" => config.debug = parse_toml_bool(&value)?,
                "forge_build_out" => config.forge_build_out = parse_toml_string(&value)?,
                "solver" => config.solver = parse_toml_string(&value)?,
                "solver_timeout_assertion" => {
                    config.solver_timeout_assertion = parse_toml_u64(&value)?
                }
                "solver_timeout_branching" => {
                    config.solver_timeout_branching = parse_toml_u64(&value)?
                }
                "cache_solver" => config.cache_solver = parse_toml_bool(&value)?,
                "print_full_model" => config.print_full_model = parse_toml_bool(&value)?,
                "dump_smt_queries" => config.dump_smt_queries = parse_toml_bool(&value)?,
                _ => {
                    // Ignore unknown fields (allows forward compatibility)
                }
            }
        }

        Ok(config)
    }
}

// TOML parsing helpers
fn parse_toml_string(value: &toml::Value) -> Result<String> {
    value
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Expected string, got {:?}", value))
}

fn parse_toml_bool(value: &toml::Value) -> Result<bool> {
    value
        .as_bool()
        .ok_or_else(|| anyhow::anyhow!("Expected bool, got {:?}", value))
}

fn parse_toml_usize(value: &toml::Value) -> Result<usize> {
    value
        .as_integer()
        .and_then(|i| usize::try_from(i).ok())
        .ok_or_else(|| anyhow::anyhow!("Expected usize, got {:?}", value))
}

fn parse_toml_u8(value: &toml::Value) -> Result<u8> {
    value
        .as_integer()
        .and_then(|i| u8::try_from(i).ok())
        .ok_or_else(|| anyhow::anyhow!("Expected u8, got {:?}", value))
}

fn parse_toml_u64(value: &toml::Value) -> Result<u64> {
    value
        .as_integer()
        .and_then(|i| u64::try_from(i).ok())
        .ok_or_else(|| anyhow::anyhow!("Expected u64, got {:?}", value))
}

fn parse_toml_path(value: &toml::Value) -> Result<PathBuf> {
    Ok(PathBuf::from(parse_toml_string(value)?))
}

// CSV parsing utilities (matching Python parse_csv)
fn parse_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_csv_int(s: &str) -> Result<Vec<usize>> {
    let parts = parse_csv(s);
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Required a non-empty list"));
    }

    parts
        .iter()
        .map(|s| {
            s.parse::<usize>()
                .map_err(|e| anyhow::anyhow!("Parse error: {}", e))
        })
        .collect()
}

fn parse_csv_trace_events(s: &str) -> Result<Vec<TraceEvent>> {
    let parts = parse_csv(s);
    if parts.is_empty() {
        return Ok(vec![]); // Empty is ok
    }

    parts.iter().map(|s| s.parse::<TraceEvent>()).collect()
}

/// Parse array lengths string
/// Format: name1={1,2,3},name2=5
fn parse_array_lengths_string(spec: &str, result: &mut HashMap<String, Vec<usize>>) -> Result<()> {
    // Remove all whitespace
    let spec = spec
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();

    // Validate format
    let format_regex = Regex::new(r"^([^=,\{\}]+=(\{[\d,]+\}|\d+)(,|$))*$").unwrap();
    if !format_regex.is_match(&spec) {
        return Err(anyhow::anyhow!("Invalid array lengths format: {}", spec));
    }

    // Parse name=sizes pairs
    let pair_regex = Regex::new(r"([^=,\{\}]+)=(?:\{([\d,]+)\}|(\d+))").unwrap();

    for cap in pair_regex.captures_iter(&spec) {
        let name = cap.get(1).unwrap().as_str().trim().to_string();
        let sizes_list = cap.get(2).map(|m| m.as_str());
        let single_size = cap.get(3).map(|m| m.as_str());

        let sizes = if let Some(list) = sizes_list {
            // Multiple sizes: {1,2,3}
            parse_csv_int(list)?
        } else if let Some(single) = single_size {
            // Single size: 5
            vec![single.parse::<usize>()?]
        } else {
            return Err(anyhow::anyhow!("Invalid array length specification"));
        };

        if sizes.is_empty() {
            return Err(anyhow::anyhow!(
                "Array lengths cannot be empty for {}",
                name
            ));
        }

        result.insert(name, sizes);
    }

    Ok(())
}

/// Parse time string with unit support (matches Python's parse_time)
/// Supports: "100ms", "5s", "2m", "1h", or plain numbers (default_unit)
pub fn parse_time(time_str: &str, default_unit: &str) -> Result<u64> {
    let time_str = time_str.trim();

    // Check for unit suffix
    if let Some(num_str) = time_str.strip_suffix("ms") {
        return Ok(num_str.trim().parse::<u64>()?);
    }
    if let Some(num_str) = time_str.strip_suffix('s') {
        return Ok(num_str.trim().parse::<u64>()? * 1000);
    }
    if let Some(num_str) = time_str.strip_suffix('m') {
        return Ok(num_str.trim().parse::<u64>()? * 60 * 1000);
    }
    if let Some(num_str) = time_str.strip_suffix('h') {
        return Ok(num_str.trim().parse::<u64>()? * 60 * 60 * 1000);
    }

    // No unit, use default
    let value = time_str.parse::<u64>()?;
    match default_unit {
        "ms" => Ok(value),
        "s" => Ok(value * 1000),
        "m" => Ok(value * 60 * 1000),
        "h" => Ok(value * 60 * 60 * 1000),
        _ => Err(anyhow::anyhow!("Invalid default unit: {}", default_unit)),
    }
}

/// Get solver command for a given solver name
/// Matches Python's get_solver_command from solvers module
pub fn get_solver_command(solver: &str) -> Result<Vec<String>> {
    match solver {
        "z3" => Ok(vec!["z3".to_string(), "-in".to_string()]),
        "yices" => Ok(vec!["yices-smt2".to_string()]),
        "cvc5" => Ok(vec!["cvc5".to_string(), "--incremental".to_string()]),
        "bitwuzla" => Ok(vec!["bitwuzla".to_string()]),
        _ => Err(anyhow::anyhow!("Unknown solver: {}", solver)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.loop_bound, 2);
        assert_eq!(config.solver, "yices");
        assert_eq!(config.function, "(check|invariant)_");
    }

    #[test]
    fn test_parse_array_lengths() {
        let mut config = Config::default();
        config.array_lengths = Some("arr1={1,2,3},arr2=5".to_string());

        let lengths = config.parse_array_lengths().unwrap();
        assert_eq!(lengths.get("arr1").unwrap(), &vec![1, 2, 3]);
        assert_eq!(lengths.get("arr2").unwrap(), &vec![5]);
    }

    #[test]
    fn test_parse_array_lengths_with_spaces() {
        let mut config = Config::default();
        config.array_lengths = Some("arr1 = {1, 2, 3}, arr2 = 5".to_string());

        let lengths = config.parse_array_lengths().unwrap();
        assert_eq!(lengths.get("arr1").unwrap(), &vec![1, 2, 3]);
        assert_eq!(lengths.get("arr2").unwrap(), &vec![5]);
    }

    #[test]
    fn test_parse_panic_codes_hex() {
        let mut config = Config::default();
        config.panic_error_codes = "0x01,0x11,0x12".to_string();

        let codes = config.parse_panic_error_codes().unwrap();
        assert_eq!(codes, vec![1, 17, 18]);
    }

    #[test]
    fn test_parse_panic_codes_wildcard() {
        let mut config = Config::default();
        config.panic_error_codes = "*".to_string();

        let codes = config.parse_panic_error_codes().unwrap();
        assert_eq!(codes, vec![]); // Empty means match all
    }

    #[test]
    fn test_trace_event_parse() {
        assert_eq!("LOG".parse::<TraceEvent>().unwrap(), TraceEvent::Log);
        assert_eq!("SSTORE".parse::<TraceEvent>().unwrap(), TraceEvent::SStore);
        assert_eq!("SLOAD".parse::<TraceEvent>().unwrap(), TraceEvent::SLoad);
    }

    #[test]
    fn test_trace_event_display() {
        assert_eq!(TraceEvent::Log.to_string(), "LOG");
        assert_eq!(TraceEvent::SStore.to_string(), "SSTORE");
        assert_eq!(TraceEvent::SLoad.to_string(), "SLOAD");
    }

    #[test]
    fn test_config_source_ordering() {
        assert!(ConfigSource::CommandLine > ConfigSource::ConfigFile);
        assert!(ConfigSource::ConfigFile > ConfigSource::Default);
        assert!(ConfigSource::FunctionAnnotation > ConfigSource::ContractAnnotation);
    }

    #[test]
    fn test_parse_csv_int() {
        let result = parse_csv_int("1,2,3").unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_csv_int_with_spaces() {
        let result = parse_csv_int("1 , 2 , 3").unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_default_array_lengths() {
        let config = Config::default();
        let lengths = config.parse_default_array_lengths().unwrap();
        assert_eq!(lengths, vec![0, 1, 2]);
    }

    #[test]
    fn test_parse_default_bytes_lengths() {
        let config = Config::default();
        let lengths = config.parse_default_bytes_lengths().unwrap();
        assert_eq!(lengths, vec![0, 65, 1024]);
    }

    #[test]
    fn test_get_solver_threads() {
        let config = Config::default();
        let threads = config.get_solver_threads();
        assert!(threads >= 1);
    }

    #[test]
    fn test_parse_time_ms() {
        assert_eq!(parse_time("100ms", "ms").unwrap(), 100);
        assert_eq!(parse_time("5s", "ms").unwrap(), 5000);
        assert_eq!(parse_time("2m", "ms").unwrap(), 120000);
        assert_eq!(parse_time("1h", "ms").unwrap(), 3600000);
    }

    #[test]
    fn test_parse_time_default_unit() {
        assert_eq!(parse_time("100", "ms").unwrap(), 100);
        assert_eq!(parse_time("5", "s").unwrap(), 5000);
    }

    #[test]
    fn test_unparse_timeout() {
        assert_eq!(Config::unparse_timeout(100), "100ms");
        assert_eq!(Config::unparse_timeout(5000), "5s");
        assert_eq!(Config::unparse_timeout(60000), "60s");
    }

    #[test]
    fn test_resolved_solver_command() {
        let config = Config::default();
        let cmd = config.resolved_solver_command().unwrap();
        assert!(!cmd.is_empty());
        assert_eq!(cmd[0], "yices-smt2");
    }

    #[test]
    fn test_resolved_solver_command_explicit() {
        let mut config = Config::default();
        config.solver_command = "z3 -in -smt2".to_string();
        let cmd = config.resolved_solver_command().unwrap();
        assert_eq!(cmd, vec!["z3", "-in", "-smt2"]);
    }
}
