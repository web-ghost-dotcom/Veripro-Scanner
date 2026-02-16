// SPDX-License-Identifier: AGPL-3.0

//! CBSE - Complete Blockchain Symbolic Executor
//! Main entry point matching Python's halmos/__main__.py

use anyhow::{Context as AnyhowContext, Result};
use cbse_config::Config;
use cbse_constants::{
    VERBOSITY_TRACE_CONSTRUCTOR, VERBOSITY_TRACE_COUNTEREXAMPLE, VERBOSITY_TRACE_PATHS,
    VERBOSITY_TRACE_SETUP,
};
use cbse_contract::Contract;
use cbse_protocol::{VerificationAttestation, VerificationResult};
use cbse_sevm::SEVM;
use cbse_traces::{render_trace, DeployAddressMapper, TraceEvent};
use clap::Parser;
use colored::Colorize;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use z3::Context as Z3Context;

mod report;

use report::{Exitcode, MainResult, TestResult};

fn main() -> Result<()> {
    let result = _main()?;
    std::process::exit(result.exitcode)
}

/// Main execution function (matches Python _main())
fn _main() -> Result<MainResult> {
    let start_time = Instant::now();

    // Parse command line arguments (matches Python load_config())
    let config = Config::parse();

    // Print version if requested
    if config.version {
        println!("cbse version {}", env!("CARGO_PKG_VERSION"));
        return Ok(MainResult {
            exitcode: 0,
            total_passed: 0,
            total_failed: 0,
            total_found: 0,
            duration: start_time.elapsed(),
        });
    }

    // Handle worker mode (remote execution worker)
    if config.worker_mode {
        return run_worker_mode(&config);
    }

    // Handle SSH test mode (test connection only)
    if config.ssh_test {
        return test_ssh_connection(&config);
    }

    // Handle SSH execution mode
    if config.ssh {
        return run_ssh_mode(&config, start_time);
    }

    // Print banner
    print_banner();

    // Build with forge (matches Python forge build command)
    println!("{}", "Building contracts with forge...".cyan());
    run_forge_build(&config)?;

    // Load build artifacts (matches Python parse_build_out)
    let artifacts_path = config.root.join(&config.forge_build_out);

    if !artifacts_path.exists() {
        eprintln!(
            "{}",
            format!(
                "Artifacts directory not found: {:?}\nRun 'forge build' first",
                artifacts_path
            )
            .red()
        );
        return Ok(MainResult {
            exitcode: 1,
            total_passed: 0,
            total_failed: 0,
            total_found: 0,
            duration: start_time.elapsed(),
        });
    }

    // Parse build output (matches Python parse_build_out)
    let build_out = parse_build_out(&artifacts_path, &config)?;

    // Compile regex patterns for filtering
    let contract_regex = make_contract_regex(&config)?;
    let test_regex = make_test_regex(&config)?;

    // Find and run test contracts
    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut total_found = 0;
    let mut test_results_map: HashMap<String, Vec<TestResult>> = HashMap::new();

    // Iterate over build output (matches Python build_output_iterator)
    for (compiler_version, files_map) in &build_out {
        for (filename, contracts_map) in files_map {
            for (contract_name, (contract_json, contract_type, _natspec)) in contracts_map {
                // Filter by contract name regex
                if !contract_regex.is_match(contract_name) {
                    continue;
                }

                // Skip non-contract types (libraries, interfaces)
                if contract_type != "contract" {
                    continue;
                }

                // Find test methods matching the pattern
                let method_identifiers = contract_json
                    .get("methodIdentifiers")
                    .and_then(|v| v.as_object())
                    .context("Missing methodIdentifiers")?;

                let test_functions: Vec<String> = method_identifiers
                    .keys()
                    .filter(|name| test_regex.is_match(name))
                    .cloned()
                    .collect();

                let num_found = test_functions.len();
                if num_found == 0 {
                    continue;
                }

                // Get contract path
                let absolute_path = contract_json
                    .get("ast")
                    .and_then(|v| v.get("absolutePath"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(filename);

                let contract_path = format!("{}:{}", absolute_path, contract_name);

                println!(
                    "\n{} {} tests for {}",
                    "Running".green(),
                    num_found,
                    contract_path.cyan()
                );

                // Run tests for this contract
                let test_results =
                    run_contract_tests(&config, contract_name, &test_functions, contract_json)?;

                let num_passed = test_results.iter().filter(|r| r.passed()).count();
                let num_failed = num_found - num_passed;

                println!(
                    "Symbolic test result: {} passed; {} failed",
                    num_passed.to_string().green(),
                    num_failed.to_string().red()
                );

                total_found += num_found;
                total_passed += num_passed;
                total_failed += num_failed;

                test_results_map.insert(contract_path, test_results);
            }
        }
    }

    // Handle no tests found
    if total_found == 0 {
        eprintln!(
            "{}",
            format!(
                "No tests found with --match-contract '{}' --match-test '{}'",
                config.match_contract, config.match_test
            )
            .red()
        );
        return Ok(MainResult {
            exitcode: 1,
            total_passed: 0,
            total_failed: 0,
            total_found: 0,
            duration: start_time.elapsed(),
        });
    }

    // Handle Prover Mode
    if config.prover_mode {
        let passed = total_failed == 0;

        let details = serde_json::to_string(&test_results_map).unwrap_or_default();

        let verification_result = VerificationResult {
            passed,
            contract_bytecode_hash:
                "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470".to_string(), // MVP: Valid dummy bytes32
            spec_hash: "0x0000000000000000000000000000000000000000000000000000000000000001"
                .to_string(), // MVP: Valid dummy bytes32
            timestamp: chrono::Utc::now().timestamp() as u64,
            details,
        };

        if let Some(key) = &config.private_key {
            let attestation = VerificationAttestation::sign(
                verification_result,
                key,
                env!("CARGO_PKG_VERSION").to_string(),
            )
            .map_err(|e| anyhow::anyhow!("Failed to sign attestation: {}", e))?;

            // Print JSON attestation to stdout for the Coordinator to pick up
            println!("{}", serde_json::to_string(&attestation).unwrap());

            return Ok(MainResult {
                exitcode: if passed { 0 } else { 1 },
                total_passed,
                total_failed,
                total_found,
                duration: start_time.elapsed(),
            });
        } else {
            eprintln!("{}", "Error: --prover-mode requires --private-key".red());
            return Ok(MainResult {
                exitcode: 1,
                total_passed: 0,
                total_failed: 0,
                total_found: 0,
                duration: start_time.elapsed(),
            });
        }
    }

    // Print summary
    print_summary(
        total_found,
        total_passed,
        total_failed,
        start_time.elapsed(),
    );

    // Write JSON output if requested
    if let Some(json_path) = &config.json_output {
        let result = MainResult {
            exitcode: if total_failed == 0 { 0 } else { 1 },
            total_passed,
            total_failed,
            total_found,
            duration: start_time.elapsed(),
        };
        let json_str = serde_json::to_string_pretty(&result)?;
        fs::write(json_path, json_str)?;
        println!("JSON output written to: {}", json_path.display());
    }

    let exitcode = if total_failed == 0 { 0 } else { 1 };
    Ok(MainResult {
        exitcode,
        total_passed,
        total_failed,
        total_found,
        duration: start_time.elapsed(),
    })
}

/// Run tests for a single contract
fn run_contract_tests(
    config: &Config,
    contract_name: &str,
    test_functions: &[String],
    contract_json: &Value,
) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    // Create Z3 context for symbolic execution
    let z3_config = z3::Config::new();
    let ctx = Z3Context::new(&z3_config);

    // Extract bytecode from contract JSON
    let deployed_bytecode = contract_json
        .get("deployedBytecode")
        .and_then(|b| b.get("object"))
        .and_then(|o| o.as_str())
        .context("Missing deployed bytecode")?;

    // Parse bytecode (remove 0x prefix if present)
    let bytecode_hex = deployed_bytecode
        .strip_prefix("0x")
        .unwrap_or(deployed_bytecode);

    // Create contract instance
    let contract = Contract::from_hexcode(bytecode_hex, &ctx)
        .context("Failed to create contract from bytecode")?;

    // Initialize SEVM
    let mut sevm = SEVM::new(&ctx);

    // Deploy test contract at Foundry test address
    let test_address: [u8; 20] = [
        0x7F, 0xA9, 0x38, 0x5b, 0xE1, 0x02, 0xac, 0x3E, 0xAc, 0x29, 0x74, 0x83, 0xDd, 0x62, 0x33,
        0xD6, 0x2b, 0x3e, 0x14, 0x96,
    ];
    sevm.deploy_contract(test_address, contract);

    // Caller address (Foundry caller)
    let caller_address: [u8; 20] = [
        0x18, 0x04, 0xc8, 0xAB, 0x1F, 0x12, 0xE6, 0xbb, 0xf3, 0x89, 0x4d, 0x40, 0x83, 0xf3, 0x3e,
        0x07, 0x30, 0x9d, 0x1f, 0x38,
    ];

    // Run each test function
    for test_name in test_functions {
        if config.verbose >= 1 {
            println!("  Executing {}", test_name.dimmed());
        }

        // Get function selector from methodIdentifiers
        let method_identifiers = contract_json
            .get("methodIdentifiers")
            .and_then(|m| m.as_object())
            .context("Missing methodIdentifiers")?;

        let selector_str = method_identifiers
            .get(test_name)
            .and_then(|s| s.as_str())
            .context(format!(
                "Function {} not found in methodIdentifiers",
                test_name
            ))?;

        // Convert selector to bytes (first 4 bytes of calldata)
        let selector_bytes =
            hex::decode(selector_str).context("Failed to decode function selector")?;

        // Build calldata: selector + encoded parameters (empty for parameterless tests)
        let mut calldata = selector_bytes;
        // TODO: For fuzz tests, generate symbolic parameters here

        // Execute the test function with SEVM
        let exec_result = sevm.execute_call(
            test_address,
            caller_address,
            caller_address, // origin = caller for top-level calls
            0,              // value
            calldata.clone(),
            u64::MAX, // unlimited gas
            false,    // not static
        );

        // Analyze execution results
        let (exitcode, num_paths) = match exec_result {
            Ok((success, returndata, gas_used, call_context)) => {
                if config.verbose >= 2 {
                    println!(
                        "    Success: {}, Gas: {}, Return: {} bytes",
                        success,
                        gas_used,
                        returndata.len()
                    );
                }

                // Check for assertion failures in returndata
                // Solidity assertions revert with Panic(uint256)
                // Panic codes: 0x01 = assert(false), 0x11 = arithmetic overflow, etc.
                let has_panic = check_for_panic(&returndata, config);

                // Determine result and render trace on failure
                let (exitcode, should_show_trace) = if success && !has_panic {
                    (Exitcode::Pass as i32, false)
                } else if has_panic {
                    if config.verbose >= 1 {
                        println!("    {} Assertion failed (Panic detected)", "✗".red());
                        if returndata.len() >= 36 {
                            let panic_code = returndata[35];
                            println!("    Panic code: 0x{:02x}", panic_code);
                        }
                    }
                    (Exitcode::Counterexample as i32, true)
                } else {
                    if config.verbose >= 1 {
                        println!("    {} Execution reverted", "✗".red());
                    }
                    (Exitcode::RevertAll as i32, true)
                };

                // Render trace for failures (counterexamples/reverts) when verbose >= 2
                // Or always render when verbose >= VERBOSITY_TRACE_PATHS (4)
                if (should_show_trace && config.verbose >= VERBOSITY_TRACE_COUNTEREXAMPLE)
                    || config.verbose >= VERBOSITY_TRACE_PATHS
                {
                    println!("    {}", "Trace:".cyan());
                    let mapper = DeployAddressMapper::new();
                    let trace_events = vec![TraceEvent::Sload, TraceEvent::Sstore, TraceEvent::Log];
                    let _ = render_trace(&call_context, &mapper, &trace_events, &mut io::stdout());
                }

                (exitcode, (1, 1, 0))
            }
            Err(e) => {
                if config.verbose >= 1 {
                    println!("    {} Execution error: {:?}", "✗".red(), e);
                    println!(
                        "    {}",
                        "This is likely due to an unimplemented opcode or EVM feature".yellow()
                    );
                    println!("    {}", "The trace system is ready - once all opcodes are implemented, traces will show execution flow".dimmed());
                }
                (Exitcode::Exception as i32, (1, 0, 1))
            }
        };

        let test_result = TestResult {
            name: test_name.to_string(),
            exitcode,
            num_models: if exitcode == Exitcode::Counterexample as i32 {
                Some(1)
            } else {
                None
            },
            num_paths: Some(num_paths),
            num_bounded_loops: Some(0),
        };

        results.push(test_result);
    }

    Ok(results)
}

/// Check if returndata contains a Panic error
fn check_for_panic(returndata: &[u8], config: &Config) -> bool {
    // Panic selector is 0x4e487b71 (keccak256("Panic(uint256)")[:4])
    const PANIC_SELECTOR: [u8; 4] = [0x4e, 0x48, 0x7b, 0x71];

    if returndata.len() < 36 {
        return false;
    }

    // Check if selector matches Panic
    if &returndata[0..4] != PANIC_SELECTOR {
        return false;
    }

    // Extract panic code (next 32 bytes as uint256)
    let panic_code = if returndata.len() >= 36 {
        // Read last byte of the uint256 (most codes are < 256)
        returndata[35]
    } else {
        return false;
    };

    // Parse panic error codes from config
    // Default is "0x01" (assertion failure)
    let panic_codes: Vec<u8> = config
        .panic_error_codes
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim().strip_prefix("0x").unwrap_or(s.trim());
            u8::from_str_radix(trimmed, 16).ok()
        })
        .collect();

    // Check if panic code matches configured codes
    let matches = panic_codes.contains(&panic_code);

    if matches && config.verbose >= 2 {
        println!("    Panic code: 0x{:02x}", panic_code);
    }

    matches
}

/// Parse build output directory (matches Python parse_build_out)
fn parse_build_out(
    artifacts_path: &Path,
    config: &Config,
) -> Result<HashMap<String, HashMap<String, HashMap<String, (Value, String, Option<Value>)>>>> {
    let mut build_out: HashMap<
        String,
        HashMap<String, HashMap<String, (Value, String, Option<Value>)>>,
    > = HashMap::new();

    // Iterate through .sol directories
    for entry in fs::read_dir(artifacts_path)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let sol_dirname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if !sol_dirname.ends_with(".sol") {
            continue;
        }

        // Iterate through JSON files in this directory
        for json_entry in fs::read_dir(&path)? {
            let json_entry = json_entry?;
            let json_path = json_entry.path();

            let json_filename = json_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if !json_filename.ends_with(".json") || json_filename.starts_with('.') {
                continue;
            }

            // Read and parse JSON
            let json_content = fs::read_to_string(&json_path)?;
            let json_out: Value = serde_json::from_str(&json_content)?;

            // Extract contract name (remove .json extension)
            let contract_name = json_filename
                .strip_suffix(".json")
                .unwrap_or(json_filename)
                .split('.')
                .next()
                .unwrap_or(json_filename);

            // Get contract type from AST
            let ast = json_out.get("ast").context("Missing AST")?;
            let (contract_type, natspec) = get_contract_type_from_ast(ast, contract_name);

            if contract_type.is_none() {
                continue;
            }

            // Get compiler version
            let compiler_version = json_out
                .get("metadata")
                .and_then(|m| m.get("compiler"))
                .and_then(|c| c.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Store in nested map structure
            build_out
                .entry(compiler_version)
                .or_insert_with(HashMap::new)
                .entry(sol_dirname.to_string())
                .or_insert_with(HashMap::new)
                .insert(
                    contract_name.to_string(),
                    (json_out, contract_type.unwrap(), natspec),
                );
        }
    }

    Ok(build_out)
}

/// Extract contract type from AST (matches Python get_contract_type)
fn get_contract_type_from_ast(ast: &Value, contract_name: &str) -> (Option<String>, Option<Value>) {
    let nodes = match ast.get("nodes").and_then(|n| n.as_array()) {
        Some(n) => n,
        None => return (None, None),
    };

    for node in nodes {
        if let Some(node_type) = node.get("nodeType").and_then(|t| t.as_str()) {
            if node_type == "ContractDefinition" {
                if let Some(name) = node.get("name").and_then(|n| n.as_str()) {
                    if name == contract_name {
                        let kind = node
                            .get("contractKind")
                            .and_then(|k| k.as_str())
                            .unwrap_or("contract")
                            .to_string();

                        let natspec = node.get("documentation").cloned();

                        return (Some(kind), natspec);
                    }
                }
            }
        }
    }

    (None, None)
}

/// Build contract name matching regex
fn make_contract_regex(config: &Config) -> Result<Regex> {
    let pattern = if !config.contract.is_empty() {
        format!("^{}$", regex::escape(&config.contract))
    } else if !config.match_contract.is_empty() {
        config.match_contract.clone()
    } else {
        ".*".to_string()
    };

    Ok(Regex::new(&pattern)?)
}

/// Build test function matching regex
fn make_test_regex(config: &Config) -> Result<Regex> {
    let pattern = if !config.match_test.is_empty() {
        config.match_test.clone()
    } else {
        config.function.clone()
    };

    Ok(Regex::new(&pattern)?)
}

/// Run forge build command
fn run_forge_build(config: &Config) -> Result<()> {
    let mut cmd = Command::new("forge");
    cmd.arg("build")
        .arg("--ast")
        .arg("--root")
        .arg(&config.root)
        .arg("--extra-output")
        .arg("storageLayout")
        .arg("metadata");

    // Force rebuild if coverage is enabled
    if config.coverage_output.is_some() {
        cmd.arg("--force");
    }

    println!("{} {:?}", "Running:".cyan(), cmd);

    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!("Forge build failed with exit code: {:?}", status.code());
    }

    Ok(())
}

/// Print CBSE banner
fn print_banner() {
    println!(
        "\n{}",
        "╔═══════════════════════════════════════════╗".cyan()
    );
    println!("{}", "║  CBSE - Complete Blockchain Symbolic     ║".cyan());
    println!("{}", "║         Executor (Rust Edition)           ║".cyan());
    println!("{}", "╚═══════════════════════════════════════════╝".cyan());
    println!();
}

/// Print test summary
fn print_summary(
    total_found: usize,
    total_passed: usize,
    total_failed: usize,
    duration: std::time::Duration,
) {
    println!(
        "\n{} {} tests, {} {} {} {} ({}ms)",
        "Summary:".yellow().bold(),
        total_found,
        total_passed.to_string().green(),
        "passed".green(),
        total_failed.to_string().red(),
        "failed".red(),
        duration.as_millis()
    );
}

/// Test SSH connection to remote node
fn test_ssh_connection(config: &Config) -> Result<MainResult> {
    use cbse_remote;

    println!("{}", "Testing SSH connection...".cyan());

    if config.ssh_host.is_empty() {
        anyhow::bail!("--ssh-host is required for SSH connection test");
    }

    // Parse host (format: user@host or just host)
    let (username, hostname) = parse_ssh_host(&config.ssh_host)?;

    // Use explicit username if provided, otherwise use parsed username
    let final_username = config.ssh_user.as_ref().unwrap_or(&username);

    // Prompt for password
    let password = cbse_remote::prompt_password(&format!(
        "Enter SSH password for {}@{}: ",
        final_username, hostname
    ))?;

    // Test connection
    match cbse_remote::test_remote_connection(
        &hostname,
        config.ssh_port,
        final_username,
        &password,
        &config.ssh_remote_binary,
    ) {
        Ok(()) => {
            println!("{}", "✓ SSH connection successful!".green());
            println!(
                "{}",
                format!("  Remote binary: {}", config.ssh_remote_binary).dimmed()
            );
            Ok(MainResult {
                exitcode: 0,
                total_passed: 0,
                total_failed: 0,
                total_found: 0,
                duration: std::time::Duration::from_secs(0),
            })
        }
        Err(e) => {
            eprintln!("{}", format!("✗ SSH connection failed: {}", e).red());
            Ok(MainResult {
                exitcode: 1,
                total_passed: 0,
                total_failed: 0,
                total_found: 0,
                duration: std::time::Duration::from_secs(0),
            })
        }
    }
}

/// Run in SSH mode - compile locally, execute remotely
fn run_ssh_mode(config: &Config, start_time: Instant) -> Result<MainResult> {
    use cbse_remote::{JobArtifact, RemoteExecutor};

    println!("{}", "Running in SSH mode (remote execution)".cyan());

    if config.ssh_host.is_empty() {
        anyhow::bail!("--ssh-host is required for SSH execution");
    }

    // Parse host
    let (username, hostname) = parse_ssh_host(&config.ssh_host)?;
    let final_username = config.ssh_user.as_ref().unwrap_or(&username);

    // Prompt for password
    let password = cbse_remote::prompt_password(&format!(
        "Enter SSH password for {}@{}: ",
        final_username, hostname
    ))?;

    println!("{}", "Building contracts locally...".cyan());
    run_forge_build(config)?;

    // Load build artifacts
    let artifacts_path = config.root.join(&config.forge_build_out);
    if !artifacts_path.exists() {
        anyhow::bail!("Artifacts directory not found: {:?}", artifacts_path);
    }

    let build_out = parse_build_out(&artifacts_path, config)?;

    // Compile regex patterns
    let contract_regex = make_contract_regex(config)?;
    let test_regex = make_test_regex(config)?;

    // Collect contracts and tests to run
    let mut job_artifact = JobArtifact::new();
    job_artifact.set_config(config);

    for (_compiler_version, files_map) in &build_out {
        for (filename, contracts_map) in files_map {
            for (contract_name, (contract_json, contract_type, _natspec)) in contracts_map {
                if !contract_regex.is_match(contract_name) {
                    continue;
                }
                if contract_type != "contract" {
                    continue;
                }

                // Find test methods
                let method_identifiers = contract_json
                    .get("methodIdentifiers")
                    .and_then(|v| v.as_object())
                    .context("Missing methodIdentifiers")?;

                let test_functions: Vec<String> = method_identifiers
                    .keys()
                    .filter(|name| test_regex.is_match(name))
                    .cloned()
                    .collect();

                if test_functions.is_empty() {
                    continue;
                }

                // Extract DEPLOYED bytecode (not deployment bytecode) and ABI
                let bytecode = contract_json
                    .get("deployedBytecode")
                    .and_then(|v| v.get("object"))
                    .and_then(|v| v.as_str())
                    .context("Missing deployed bytecode")?;

                let abi = contract_json.get("abi").context("Missing ABI")?;

                job_artifact.add_contract(
                    contract_name.clone(),
                    bytecode.to_string(),
                    abi.clone(),
                    test_functions,
                );
            }
        }
    }

    if job_artifact.contracts.is_empty() {
        anyhow::bail!("No test contracts found");
    }

    println!(
        "{}",
        format!("Found {} test contracts", job_artifact.contracts.len()).cyan()
    );

    // Create remote executor and run
    let executor = RemoteExecutor::new(
        &hostname,
        config.ssh_port,
        &final_username,
        &password,
        &config.ssh_remote_workdir,
        &config.ssh_remote_binary,
    )?;

    println!("{}", "Uploading artifacts and executing remotely...".cyan());
    let result = executor.execute(&job_artifact)?;

    // Display results
    let mut total_found = 0;
    let mut total_passed = 0;
    let mut total_failed = 0;

    for test_result in &result.test_results {
        total_found += 1;
        if test_result.passed {
            total_passed += 1;
            println!("  {} {}", "✓".green(), test_result.name.cyan());
        } else {
            total_failed += 1;
            println!("  {} {}", "✗".red(), test_result.name.red());
            if let Some(error) = &test_result.error {
                println!("    Error: {}", error);
            }
        }
    }

    print_summary(
        total_found,
        total_passed,
        total_failed,
        start_time.elapsed(),
    );

    Ok(MainResult {
        exitcode: if total_failed == 0 { 0 } else { 1 },
        total_passed,
        total_failed,
        total_found,
        duration: start_time.elapsed(),
    })
}

/// Run in worker mode - execute from JSON artifact
fn run_worker_mode(config: &Config) -> Result<MainResult> {
    use cbse_remote::{JobArtifact, JobResult, TestResult as RemoteTestResult};

    let start_time = Instant::now();

    let input_path = config
        .input
        .as_ref()
        .context("--input is required in worker mode")?;

    let output_path = config
        .output
        .as_ref()
        .context("--output is required in worker mode")?;

    // Read job artifact
    let artifact_json = fs::read_to_string(input_path).context("Failed to read input artifact")?;

    let job_artifact: JobArtifact =
        serde_json::from_str(&artifact_json).context("Failed to parse job artifact")?;

    // Apply configuration from artifact
    let exec_config = &job_artifact.config;
    let verbose = exec_config.verbosity;

    // Print banner if verbose
    if verbose >= 1 {
        print_banner();
    }

    // Create Z3 context for symbolic execution
    let z3_config = z3::Config::new();
    let ctx = Z3Context::new(&z3_config);

    // Execute tests
    let mut test_results = Vec::new();
    let mut total_passed = 0;
    let mut total_failed = 0;

    for contract_data in &job_artifact.contracts {
        // Create SEVM instance
        let mut sevm = SEVM::new(&ctx);

        // Parse bytecode
        let bytecode_hex = contract_data
            .bytecode
            .strip_prefix("0x")
            .unwrap_or(&contract_data.bytecode);

        let contract = match Contract::from_hexcode(bytecode_hex, &ctx) {
            Ok(c) => c,
            Err(e) => {
                // If contract creation fails, mark all tests as failed
                for test_name in &contract_data.test_functions {
                    test_results.push(RemoteTestResult {
                        name: format!("{}::{}", contract_data.name, test_name),
                        passed: false,
                        error: Some(format!("Failed to create contract: {}", e)),
                        counterexample: None,
                        gas_used: 0,
                    });
                    total_failed += 1;
                }
                continue;
            }
        };

        // Deploy test contract at Foundry test address
        let test_address: [u8; 20] = [
            0x7F, 0xA9, 0x38, 0x5b, 0xE1, 0x02, 0xac, 0x3E, 0xAc, 0x29, 0x74, 0x83, 0xDd, 0x62,
            0x33, 0xD6, 0x2b, 0x3e, 0x14, 0x96,
        ];
        sevm.deploy_contract(test_address, contract);

        // Caller address (Foundry caller)
        let caller_address: [u8; 20] = [
            0x18, 0x04, 0xc8, 0xAB, 0x1F, 0x12, 0xE6, 0xbb, 0xf3, 0x89, 0x4d, 0x40, 0x83, 0xf3,
            0x3e, 0x07, 0x30, 0x9d, 0x1f, 0x38,
        ];

        // Get method identifiers from ABI
        let method_identifiers = contract_data
            .abi
            .as_object()
            .and_then(|obj| {
                // ABI is an array, we need to extract function signatures
                if let Some(arr) = contract_data.abi.as_array() {
                    let mut map = serde_json::Map::new();
                    for item in arr {
                        if let (Some("function"), Some(name)) = (
                            item.get("type").and_then(|t| t.as_str()),
                            item.get("name").and_then(|n| n.as_str()),
                        ) {
                            // Calculate selector
                            let signature = build_function_signature(item);
                            let selector = calculate_selector(&signature);
                            map.insert(name.to_string(), serde_json::Value::String(selector));
                        }
                    }
                    Some(map)
                } else {
                    None
                }
            })
            .unwrap_or_default();

        // Run each test function
        for test_name in &contract_data.test_functions {
            if verbose >= 1 {
                println!("  Executing {}", test_name);
            }

            // Get function selector
            let selector_str = if let Some(sel) = method_identifiers.get(test_name) {
                sel.as_str().unwrap_or("")
            } else {
                // Fallback: calculate selector from test name
                &calculate_selector(test_name)
            };

            let selector_bytes = match hex::decode(selector_str) {
                Ok(b) => b,
                Err(e) => {
                    if verbose >= 1 {
                        println!("    {} Failed to decode selector: {}", "✗".red(), e);
                    }
                    test_results.push(RemoteTestResult {
                        name: format!("{}::{}", contract_data.name, test_name),
                        passed: false,
                        error: Some(format!("Failed to decode selector: {}", e)),
                        counterexample: None,
                        gas_used: 0,
                    });
                    total_failed += 1;
                    continue;
                }
            };

            if exec_config.print_steps {
                println!("    Executing with selector: {}", selector_str);
            }

            // Execute the test function
            let exec_result = sevm.execute_call(
                test_address,
                caller_address,
                caller_address,
                0,
                selector_bytes,
                u64::MAX,
                false,
            );

            // Analyze results
            let (passed, error, gas) = match exec_result {
                Ok((success, returndata, gas_used, call_context)) => {
                    if verbose >= 2 {
                        println!(
                            "    Success: {}, Gas: {}, Return: {} bytes",
                            success,
                            gas_used,
                            returndata.len()
                        );
                    }

                    let has_panic = !returndata.is_empty()
                        && returndata.len() >= 4
                        && &returndata[0..4] == &[0x4e, 0x48, 0x7b, 0x71]; // Panic(uint256)

                    if success && !has_panic {
                        if verbose >= 1 {
                            println!("    {} Test passed", "✓".green());
                        }
                        (true, None, gas_used)
                    } else if has_panic {
                        let panic_msg = if returndata.len() >= 36 {
                            let panic_code = returndata[35];
                            if verbose >= 1 {
                                println!(
                                    "    {} Assertion failed (Panic 0x{:02x})",
                                    "✗".red(),
                                    panic_code
                                );
                            }
                            format!("Assertion failed (Panic 0x{:02x})", panic_code)
                        } else {
                            if verbose >= 1 {
                                println!("    {} Assertion failed", "✗".red());
                            }
                            "Assertion failed".to_string()
                        };

                        // Print trace if requested
                        if verbose >= 2 || exec_config.print_states {
                            println!("    {}", "Trace:".cyan());
                            let mapper = DeployAddressMapper::new();
                            let trace_events =
                                vec![TraceEvent::Sload, TraceEvent::Sstore, TraceEvent::Log];
                            let _ = render_trace(
                                &call_context,
                                &mapper,
                                &trace_events,
                                &mut io::stdout(),
                            );
                        }

                        (false, Some(panic_msg), gas_used)
                    } else {
                        if verbose >= 1 {
                            println!("    {} Execution reverted", "✗".red());
                        }

                        // Print trace for reverts if requested
                        if verbose >= 2 || exec_config.print_failed_states {
                            println!("    {}", "Trace:".cyan());
                            let mapper = DeployAddressMapper::new();
                            let trace_events =
                                vec![TraceEvent::Sload, TraceEvent::Sstore, TraceEvent::Log];
                            let _ = render_trace(
                                &call_context,
                                &mapper,
                                &trace_events,
                                &mut io::stdout(),
                            );
                        }

                        (false, Some("Execution reverted".to_string()), gas_used)
                    }
                }
                Err(e) => {
                    if verbose >= 1 {
                        println!("    {} Execution error: {:?}", "✗".red(), e);
                    }
                    (false, Some(format!("Execution error: {}", e)), 0)
                }
            };

            if passed {
                total_passed += 1;
            } else {
                total_failed += 1;
            }

            test_results.push(RemoteTestResult {
                name: format!("{}::{}", contract_data.name, test_name),
                passed,
                error,
                counterexample: None,
                gas_used: gas,
            });
        }
    }

    let execution_time_ms = start_time.elapsed().as_millis() as u64;

    // Write results
    let job_result = JobResult {
        status: if total_failed == 0 {
            "success".to_string()
        } else {
            "failed".to_string()
        },
        test_results,
        execution_time_ms,
        traces: Vec::new(),
        counterexamples: Vec::new(),
    };

    let result_json = serde_json::to_string_pretty(&job_result)?;
    fs::write(output_path, result_json)?;

    Ok(MainResult {
        exitcode: if total_failed == 0 { 0 } else { 1 },
        total_passed,
        total_failed,
        total_found: total_passed + total_failed,
        duration: start_time.elapsed(),
    })
}

/// Build function signature from ABI item
fn build_function_signature(abi_item: &Value) -> String {
    let name = abi_item.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let inputs = abi_item
        .get("inputs")
        .and_then(|i| i.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|input| input.get("type").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();

    format!("{}({})", name, inputs)
}

/// Calculate keccak256 selector for function signature
fn calculate_selector(signature: &str) -> String {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(signature.as_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[0..4])
}

/// Parse SSH host string (format: user@host or just host)
fn parse_ssh_host(host_str: &str) -> Result<(String, String)> {
    if let Some((user, host)) = host_str.split_once('@') {
        Ok((user.to_string(), host.to_string()))
    } else {
        // No @ sign, assume host only and use current user
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "root".to_string());
        Ok((username, host_str.to_string()))
    }
}

/// Print test summary
fn print_summary_old(
    total_found: usize,
    total_passed: usize,
    total_failed: usize,
    duration: std::time::Duration,
) {
    println!("\n{}", "═".repeat(60).cyan());
    println!("{}", "  Test Summary".bold());
    println!("{}", "═".repeat(60).cyan());
    println!("  Total tests: {}", total_found);
    println!("  Passed:      {}", total_passed.to_string().green());
    println!("  Failed:      {}", total_failed.to_string().red());
    println!("  Duration:    {:.2}s", duration.as_secs_f64());
    println!("{}", "═".repeat(60).cyan());
}
