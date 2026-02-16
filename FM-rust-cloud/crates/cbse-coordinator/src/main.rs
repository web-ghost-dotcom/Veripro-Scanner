use axum::{
    routing::{get, post},
    Json, Router,
};
use cbse_protocol::VerificationAttestation;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::os::unix::fs::symlink;
use std::process::Command;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/verify", post(start_verification))
        .layer(cors);

    // Use port 3001 and listen on all interfaces for external access
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("Coordinator listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct VerificationRequest {
    contract_source: String,
    spec_source: String,
    contract_name: String,
}

#[derive(Serialize)]
struct VerificationResponse {
    job_id: String,
    status: String,
    message: String,
    attestation: Option<VerificationAttestation>,
}

/// Extract test contract name from spec source
fn extract_test_contract_name(spec_source: &str) -> Option<String> {
    // Look for "contract SomeName is Test" or similar patterns
    let re = Regex::new(r"contract\s+(\w+)\s+is\s+").ok()?;
    re.captures(spec_source)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

/// Fix common import path issues in spec files
/// Since both contract and spec are in the same src/ directory,
/// we need to convert paths like "../src/Contract.sol" to "./Contract.sol"
fn fix_import_paths(spec_source: &str, contract_name: &str) -> String {
    let mut fixed = spec_source.to_string();
    let replacement = format!("\"./{}.sol\"", contract_name);

    println!("Fixing import paths for contract: {}", contract_name);

    // Pattern 1: "../src/ContractName.sol" -> "./ContractName.sol"
    let pattern1 = format!("\"\\.\\./src/{}\\.sol\"", regex::escape(contract_name));
    if let Ok(re) = Regex::new(&pattern1) {
        let before = fixed.clone();
        fixed = re.replace_all(&fixed, replacement.as_str()).to_string();
        if before != fixed {
            println!("  Fixed pattern1: ../src/{}.sol", contract_name);
        }
    }

    // Pattern 2: "src/ContractName.sol" -> "./ContractName.sol"
    let pattern2 = format!("\"src/{}\\.sol\"", regex::escape(contract_name));
    if let Ok(re) = Regex::new(&pattern2) {
        let before = fixed.clone();
        fixed = re.replace_all(&fixed, replacement.as_str()).to_string();
        if before != fixed {
            println!("  Fixed pattern2: src/{}.sol", contract_name);
        }
    }

    // Pattern 3: "../ContractName.sol" -> "./ContractName.sol"
    let pattern3 = format!("\"\\.\\./{}\\.sol\"", regex::escape(contract_name));
    if let Ok(re) = Regex::new(&pattern3) {
        let before = fixed.clone();
        fixed = re.replace_all(&fixed, replacement.as_str()).to_string();
        if before != fixed {
            println!("  Fixed pattern3: ../{}.sol", contract_name);
        }
    }

    // Generic: any "../src/*.sol" -> "./*.sol"
    if let Ok(re) = Regex::new("\"\\.\\./src/(\\w+)\\.sol\"") {
        let before = fixed.clone();
        fixed = re.replace_all(&fixed, "\"./$1.sol\"").to_string();
        if before != fixed {
            println!("  Fixed generic ../src/*.sol pattern");
        }
    }

    // Generic: any "src/*.sol" -> "./*.sol"
    if let Ok(re) = Regex::new("\"src/(\\w+)\\.sol\"") {
        let before = fixed.clone();
        fixed = re.replace_all(&fixed, "\"./$1.sol\"").to_string();
        if before != fixed {
            println!("  Fixed generic src/*.sol pattern");
        }
    }

    fixed
}

/// Find forge-std library path
fn find_forge_std_path() -> Option<String> {
    // Check environment variable first
    if let Ok(path) = std::env::var("FORGE_STD_PATH") {
        println!("Checking FORGE_STD_PATH: {}", path);
        if std::path::Path::new(&path).exists() {
            println!("Found forge-std at FORGE_STD_PATH: {}", path);
            return Some(path);
        }
    }

    let home = std::env::var("HOME").unwrap_or_default();

    // Check common locations - use absolute paths where possible
    let common_paths: Vec<String> = vec![
        // VeriPro project locations (absolute paths)
        format!(
            "{}/Desktop/CODE/VeriPro_openclaw/test-contract/lib/forge-std",
            home
        ),
        format!(
            "{}/Desktop/CODE/VeriPro_openclaw/smart-contracts/lib/forge-std",
            home
        ),
        // Relative to project (for when running from FM-rust-cloud)
        "../smart-contracts/lib/forge-std".to_string(),
        "../../smart-contracts/lib/forge-std".to_string(),
        "../test-contract/lib/forge-std".to_string(),
        "../../test-contract/lib/forge-std".to_string(),
        // Home directory foundry
        format!("{}/.foundry/forge-std", home),
        format!("{}/.foundry/cache/forge-std", home),
    ];

    for path in &common_paths {
        let full_path = if path.starts_with('/') || path.starts_with('~') {
            path.clone()
        } else {
            // Resolve relative to current exe or cwd
            std::env::current_dir()
                .ok()
                .map(|cwd| cwd.join(path).to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone())
        };

        if std::path::Path::new(&full_path).exists() {
            println!("Found forge-std at: {}", full_path);
            return Some(full_path);
        }
    }

    println!(
        "Could not find forge-std in any of {} locations",
        common_paths.len()
    );

    None
}

async fn start_verification(
    Json(payload): Json<VerificationRequest>,
) -> Json<VerificationResponse> {
    println!(
        "Received job request for contract: {}",
        payload.contract_name
    );
    let job_id = Uuid::new_v4().to_string();

    // Check if spec_source is empty or doesn't contain tests
    if payload.spec_source.trim().is_empty() {
        return error_response(
            job_id,
            "No specification provided. Please add a specification file with test functions."
                .to_string(),
        );
    }

    // Extract test contract name from spec
    let test_contract_name = extract_test_contract_name(&payload.spec_source);

    if test_contract_name.is_none() {
        return error_response(
            job_id,
            "Could not find a test contract in the specification. Ensure your spec has 'contract YourTestName is Test { ... }'".to_string()
        );
    }

    let test_contract_name = test_contract_name.unwrap();
    println!("Found test contract: {}", test_contract_name);

    // Debug: Show first 200 chars of each source
    println!(
        "Contract source (first 200 chars): {}",
        payload
            .contract_source
            .chars()
            .take(200)
            .collect::<String>()
    );
    println!(
        "Spec source (first 200 chars): {}",
        payload.spec_source.chars().take(200).collect::<String>()
    );

    // 1. Setup Wrapper Directory Structure
    let work_dir = format!("/tmp/cbse-jobs/{}", job_id);
    let src_dir = format!("{}/src", work_dir);
    let lib_dir = format!("{}/lib", work_dir);

    if let Err(e) = fs::create_dir_all(&src_dir) {
        return error_response(job_id, format!("Failed to create src dir: {}", e));
    }
    if let Err(e) = fs::create_dir_all(&lib_dir) {
        return error_response(job_id, format!("Failed to create lib dir: {}", e));
    }

    // 2. Setup forge-std library
    let forge_std_link = format!("{}/forge-std", lib_dir);

    // Try to find and symlink forge-std
    if let Some(forge_std_path) = find_forge_std_path() {
        // Create symlink to forge-std
        if let Err(e) = symlink(&forge_std_path, &forge_std_link) {
            println!(
                "Warning: Failed to symlink forge-std: {} - will try forge install",
                e
            );
        } else {
            println!("Symlinked forge-std from {}", forge_std_path);
        }
    }

    // If symlink doesn't exist, run forge install
    if !std::path::Path::new(&forge_std_link).exists() {
        println!("Installing forge-std via forge install...");
        let install_output = Command::new("forge")
            .arg("install")
            .arg("foundry-rs/forge-std")
            .arg("--no-git")
            .arg("--no-commit")
            .current_dir(&work_dir)
            .output();

        match install_output {
            Ok(out) => {
                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    println!("forge install warning: {}", stderr);
                    // Don't fail - forge build might still work with remappings
                }
            }
            Err(e) => {
                println!("Warning: forge install failed: {}", e);
            }
        }
    }

    // 3. Write Contract and Spec
    let contract_path = format!("{}/{}.sol", src_dir, payload.contract_name);
    let spec_path = format!("{}/{}.t.sol", src_dir, test_contract_name);

    if let Err(e) = fs::write(&contract_path, &payload.contract_source) {
        return error_response(job_id, format!("Failed to write contract: {}", e));
    }

    // Fix import paths in spec before writing
    // The AI might generate paths like "../src/Contract.sol" but both files are in src/
    let fixed_spec = fix_import_paths(&payload.spec_source, &payload.contract_name);

    if let Err(e) = fs::write(&spec_path, &fixed_spec) {
        return error_response(job_id, format!("Failed to write spec: {}", e));
    }

    // 4. Write foundry.toml with remappings
    let toml_content = r#"
[profile.default]
src = 'src'
out = 'out'
libs = ['lib']
test = 'src'

# Remappings for forge-std
remappings = ['forge-std/=lib/forge-std/src/']
"#;
    let toml_path = format!("{}/foundry.toml", work_dir);
    if let Err(e) = fs::write(&toml_path, toml_content) {
        return error_response(job_id, format!("Failed to write foundry.toml: {}", e));
    }

    // 5. Run 'forge build'
    println!("Compiling in {}...", work_dir);
    let build_output = Command::new("forge")
        .arg("build")
        .current_dir(&work_dir)
        .output();

    match build_output {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                println!("Forge Build Failed: {}", stderr);
                return error_response(job_id, format!("Forge build failed: {}", stderr));
            }
        }
        Err(e) => {
            return error_response(job_id, format!("Failed to execute forge build: {}", e));
        }
    }

    // 6. Setup cbse command
    let cbse_path =
        std::env::var("CBSE_BINARY").unwrap_or_else(|_| "target/debug/cbse".to_string());

    // MVP: Hardcoded key for "Prover #1"
    let prover_key = std::env::var("PROVER_PRIVATE_KEY").unwrap_or_else(|_| {
        "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()
    }); // Anvil default account #0

    println!(
        "Running prover on {} with test contract {}",
        work_dir, test_contract_name
    );

    // 7. Run 'cbse' - use the test contract name for --match-contract
    // Also override --function to match Foundry-style test_ functions in addition to
    // the default check_/invariant_ functions
    let output = Command::new(cbse_path)
        .arg("--root")
        .arg(&work_dir)
        .arg("--match-contract")
        .arg(&test_contract_name)
        .arg("--function")
        .arg("(test|check|invariant)_") // Match test_, check_, and invariant_ functions
        .arg("--prover-mode")
        .arg("--private-key")
        .arg(&prover_key)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);

                if let Some(json_line) = stdout.lines().last() {
                    if let Ok(attestation) =
                        serde_json::from_str::<VerificationAttestation>(json_line)
                    {
                        return Json(VerificationResponse {
                            job_id,
                            status: "Success".to_string(),
                            message: "Verification completed successfully.".to_string(),
                            attestation: Some(attestation),
                        });
                    } else {
                        println!("Last line not JSON: {}", json_line);
                    }
                }

                println!("Stdout: {}", stdout);
                Json(VerificationResponse {
                    job_id,
                    status: "Error".to_string(),
                    message: "Prover finished but output parse failed.".to_string(),
                    attestation: None,
                })
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                println!("Prover Failed: {}", stderr);
                Json(VerificationResponse {
                    job_id,
                    status: "Failure".to_string(),
                    message: format!("Verification failed: {}", stderr),
                    attestation: None,
                })
            }
        }
        Err(e) => {
            println!("Failed to spawn prover: {}", e);
            Json(VerificationResponse {
                job_id,
                status: "SystemError".to_string(),
                message: format!("Failed to spawn prover process: {}", e),
                attestation: None,
            })
        }
    }
}

fn error_response(job_id: String, message: String) -> Json<VerificationResponse> {
    Json(VerificationResponse {
        job_id,
        status: "Failure".to_string(),
        message,
        attestation: None,
    })
}
