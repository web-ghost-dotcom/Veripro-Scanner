// SPDX-License-Identifier: AGPL-3.0

//! SMT solver integration and model parsing

use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Once;
use std::time::Duration;

/// Exit code for timeout
pub const EXIT_TIMEDOUT: i32 = 124;

/// SMT query result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SatResult {
    Sat,
    Unsat,
    Unknown,
    Error,
}

impl std::fmt::Display for SatResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SatResult::Sat => write!(f, "sat"),
            SatResult::Unsat => write!(f, "unsat"),
            SatResult::Unknown => write!(f, "unknown"),
            SatResult::Error => write!(f, "error"),
        }
    }
}

/// Model variable from SMT solver output
#[derive(Debug, Clone)]
pub struct ModelVariable {
    pub full_name: String,
    pub variable_name: String,
    pub solidity_type: String,
    pub smt_type: String,
    pub size_bits: usize,
    pub value: u128,
}

impl ModelVariable {
    pub fn new(
        full_name: String,
        variable_name: String,
        solidity_type: String,
        smt_type: String,
        size_bits: usize,
        value: u128,
    ) -> Self {
        Self {
            full_name,
            variable_name,
            solidity_type,
            smt_type,
            size_bits,
            value,
        }
    }
}

/// Model variables map
pub type ModelVariables = HashMap<String, ModelVariable>;

/// Potential model with validity flag
#[derive(Debug, Clone)]
pub struct PotentialModel {
    pub model: ModelVariables,
    pub is_valid: bool,
}

impl PotentialModel {
    pub fn new(model: ModelVariables, is_valid: bool) -> Self {
        Self { model, is_valid }
    }

    pub fn empty() -> Self {
        Self {
            model: HashMap::new(),
            is_valid: false,
        }
    }
}

impl std::fmt::Display for PotentialModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.model.is_empty() {
            return write!(f, "∅");
        }

        let mut vars: Vec<_> = self.model.values().collect();
        vars.sort_by(|a, b| a.full_name.cmp(&b.full_name));

        for var in vars {
            writeln!(f, "    {} = 0x{:x}", var.full_name, var.value)?;
        }
        Ok(())
    }
}

/// SMT query
#[derive(Debug, Clone)]
pub struct SMTQuery {
    pub smtlib: String,
    pub assertions: Vec<String>,
}

impl SMTQuery {
    pub fn new(smtlib: String, assertions: Vec<String>) -> Self {
        Self { smtlib, assertions }
    }
}

/// Solver output
#[derive(Debug, Clone)]
pub struct SolverOutput {
    pub result: SatResult,
    pub returncode: i32,
    pub path_id: usize,
    pub query_file: String,
    pub model: Option<PotentialModel>,
    pub unsat_core: Option<Vec<String>>,
    pub error: Option<String>,
}

impl SolverOutput {
    pub fn new(result: SatResult, returncode: i32, path_id: usize, query_file: String) -> Self {
        Self {
            result,
            returncode,
            path_id,
            query_file,
            model: None,
            unsat_core: None,
            error: None,
        }
    }

    pub fn from_result(
        stdout: &str,
        stderr: &str,
        returncode: i32,
        path_id: usize,
        query_file: String,
    ) -> Self {
        let first_line = stdout.lines().next().unwrap_or("");

        match first_line {
            "sat" => {
                let is_valid = is_model_valid(stdout);
                let model = PotentialModel::new(parse_model_str(stdout), is_valid);
                Self {
                    result: SatResult::Sat,
                    returncode,
                    path_id,
                    query_file,
                    model: Some(model),
                    unsat_core: None,
                    error: None,
                }
            }
            "unsat" => {
                let unsat_core = parse_unsat_core(stdout);
                Self {
                    result: SatResult::Unsat,
                    returncode,
                    path_id,
                    query_file,
                    model: None,
                    unsat_core,
                    error: None,
                }
            }
            "unknown" => Self {
                result: SatResult::Unknown,
                returncode,
                path_id,
                query_file,
                model: None,
                unsat_core: None,
                error: None,
            },
            _ => Self {
                result: SatResult::Error,
                returncode,
                path_id,
                query_file,
                model: None,
                unsat_core: None,
                error: Some(stderr.to_string()),
            },
        }
    }

    pub fn from_error(error: String, path_id: usize, query_file: String) -> Self {
        Self {
            result: SatResult::Error,
            returncode: -1,
            path_id,
            query_file,
            model: None,
            unsat_core: None,
            error: Some(error),
        }
    }
}

/// Parse constant value from SMT output
pub fn parse_const_value(value: &str) -> Result<u128, String> {
    if value.starts_with("#b") {
        // Binary: #b1010
        u128::from_str_radix(&value[2..], 2)
            .map_err(|e| format!("Failed to parse binary value: {}", e))
    } else if value.starts_with("#x") {
        // Hex: #xFF
        u128::from_str_radix(&value[2..], 16)
            .map_err(|e| format!("Failed to parse hex value: {}", e))
    } else if value.starts_with("bv") {
        // Decimal: bv42
        value[2..]
            .parse()
            .map_err(|e| format!("Failed to parse bv value: {}", e))
    } else if value.contains("bv") {
        // Pattern: (_ bv123 256)
        for token in value.split_whitespace() {
            if token.starts_with("bv") {
                return token[2..]
                    .parse()
                    .map_err(|e| format!("Failed to parse bv token: {}", e));
            }
        }
        Err(format!("No bv token found in: {}", value))
    } else {
        Err(format!("Unknown value format: {}", value))
    }
}

/// Get halmos variable regex pattern
fn halmos_var_pattern() -> &'static Regex {
    static INIT: Once = Once::new();
    static mut PATTERN: Option<Regex> = None;

    unsafe {
        INIT.call_once(|| {
            PATTERN = Some(
                Regex::new(
                    r"(?x)
                    \(\s*define-fun\s+               # Match \(define-fun
                    \|?((?:halmos_|p_)[^\s|]+)\|?\s+  # Capture halmos_\* or p_\*
                    \(\)\s+\(_\s+([^\s]+)\s+          # Capture SMT type
                    (\d+)\)\s+                       # Capture bit-width
                    (                                # Value group
                        \#b[01]+                     # Binary
                        |\#x[0-9a-fA-F]+             # Hex
                        |\(_\s+bv\d+\s+\d+\)         # Decimal
                    )
                    ",
                )
                .unwrap(),
            );
        });
        PATTERN.as_ref().unwrap()
    }
}

/// Parse model variables from SMT output
pub fn parse_model_str(smtlib_str: &str) -> ModelVariables {
    let mut model_variables = HashMap::new();
    let pattern = halmos_var_pattern();

    for captures in pattern.captures_iter(smtlib_str) {
        let full_name = captures[1].trim().to_string();
        let smt_type = format!("{} {}", &captures[2], &captures[3]);
        let size_bits: usize = captures[3].parse().unwrap_or(0);
        let value = parse_const_value(&captures[4]).unwrap_or(0);

        // Extract variable name and type from full name
        // Format: halmos_varname_type or p_varname_type
        let parts: Vec<&str> = full_name.split('_').collect();
        let variable_name = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            full_name.clone()
        };
        let solidity_type = if parts.len() > 2 {
            parts[2].to_string()
        } else {
            "unknown".to_string()
        };

        let var = ModelVariable::new(
            full_name.clone(),
            variable_name,
            solidity_type,
            smt_type,
            size_bits,
            value,
        );

        model_variables.insert(full_name, var);
    }

    model_variables
}

/// Parse model from file
pub fn parse_model_file(file_path: &str) -> Result<ModelVariables, std::io::Error> {
    let content = fs::read_to_string(file_path)?;
    Ok(parse_model_str(&content))
}

/// Parse unsat core from solver output
pub fn parse_unsat_core(output: &str) -> Option<Vec<String>> {
    // Pattern: unsat\n(optional error line)\n(<id1> <id2> ...)
    let re = Regex::new(r"unsat\s*(?:\(\s*error\s+[^)]*\)\s*)?\(\s*((?:<[0-9]+>\s*)*)\)").ok()?;

    if let Some(captures) = re.captures(output) {
        let ids_str = &captures[1];
        let ids: Vec<String> = ids_str
            .split_whitespace()
            .filter_map(|s| {
                if s.starts_with('<') && s.ends_with('>') {
                    Some(s[1..s.len() - 1].to_string())
                } else {
                    None
                }
            })
            .collect();
        Some(ids)
    } else {
        None
    }
}

/// Check if model is valid (no f_evm_* symbols)
pub fn is_model_valid(solver_stdout: &str) -> bool {
    !solver_stdout.contains("f_evm_")
}

/// Dump SMT query to file
pub fn dump_query(query: &SMTQuery, path: &Path, cache_solver: bool) -> Result<(), std::io::Error> {
    let mut content = String::new();

    if cache_solver {
        content.push_str("(set-option :produce-unsat-cores true)\n");
        content.push_str("(set-logic QF_AUFBV)\n");
        content.push_str(&query.smtlib);
        content.push('\n');

        // Add named assertions
        for assert_id in &query.assertions {
            content.push_str(&format!(
                "(assert (! |{}| :named <{}>))\n",
                assert_id, assert_id
            ));
        }

        content.push_str("(check-sat)\n");
        content.push_str("(get-model)\n");
        content.push_str("(get-unsat-core)\n");
    } else {
        content.push_str("(set-logic QF_AUFBV)\n");
        content.push_str(&query.smtlib);
        content.push_str("\n(check-sat)\n(get-model)\n");
    }

    fs::write(path, content)?;
    Ok(())
}

/// Solve SMT query with external solver
pub fn solve_external(
    solver_command: &[String],
    query_file: &Path,
    timeout: Option<Duration>,
    path_id: usize,
) -> SolverOutput {
    let query_file_str = query_file.to_string_lossy().to_string();

    let mut cmd = Command::new(&solver_command[0]);
    cmd.args(&solver_command[1..])
        .arg(query_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let result = if let Some(timeout_duration) = timeout {
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return SolverOutput::from_error(
                    format!("Failed to spawn solver: {}", e),
                    path_id,
                    query_file_str,
                )
            }
        };

        match wait_timeout::ChildExt::wait_timeout(&mut child, timeout_duration) {
            Ok(Some(status)) => {
                let output = child
                    .wait_with_output()
                    .unwrap_or_else(|_| std::process::Output {
                        status,
                        stdout: vec![],
                        stderr: vec![],
                    });
                Ok((output, false))
            }
            Ok(None) => {
                // Timeout
                Err("timeout")
            }
            Err(e) => {
                return SolverOutput::from_error(
                    format!("Wait error: {}", e),
                    path_id,
                    query_file_str,
                )
            }
        }
    } else {
        cmd.output()
            .map(|output| (output, false))
            .map_err(|_| "failed")
    };

    match result {
        Ok((output, _)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let returncode = output.status.code().unwrap_or(-1);

            // Save stdout
            if let Ok(mut file) = fs::File::create(format!("{}.out", query_file_str)) {
                let _ = file.write_all(stdout.as_bytes());
            }

            // Save stderr if present
            if !stderr.is_empty() {
                if let Ok(mut file) = fs::File::create(format!("{}.err", query_file_str)) {
                    let _ = file.write_all(stderr.as_bytes());
                }
            }

            SolverOutput::from_result(&stdout, &stderr, returncode, path_id, query_file_str)
        }
        Err("timeout") => SolverOutput {
            result: SatResult::Unknown,
            returncode: EXIT_TIMEDOUT,
            path_id,
            query_file: query_file_str,
            model: None,
            unsat_core: None,
            error: Some("Solver timeout".to_string()),
        },
        Err(e) => SolverOutput::from_error(e.to_string(), path_id, query_file_str),
    }
}

/// Check if query contains unsat core
pub fn check_unsat_cores(query: &SMTQuery, unsat_cores: &[Vec<String>]) -> bool {
    for core in unsat_cores {
        if core.iter().all(|id| query.assertions.contains(id)) {
            return true;
        }
    }
    false
}

/// Refine query by replacing f_evm_* abstractions
pub fn refine_query(query: &SMTQuery) -> SMTQuery {
    let mut smtlib = query.smtlib.clone();

    // Replace f_evm_bvmul abstraction
    let bvmul_re = Regex::new(
        r"\(declare-fun f_evm_(bvmul)_([0-9]+) \(\(_ BitVec ([0-9]+)\) \(_ BitVec ([0-9]+)\)\) \(_ BitVec ([0-9]+)\)\)"
    ).unwrap();
    smtlib = bvmul_re
        .replace_all(
            &smtlib,
            r"(define-fun f_evm_$1_$2 ((x (_ BitVec $2)) (y (_ BitVec $2))) (_ BitVec $2) ($1 x y))",
        )
        .to_string();

    // Replace f_evm_bvudiv, bvurem, bvsdiv, bvsrem abstractions
    let div_re = Regex::new(
        r"\(declare-fun f_evm_(bvudiv|bvurem|bvsdiv|bvsrem)_([0-9]+) \(\(_ BitVec ([0-9]+)\) \(_ BitVec ([0-9]+)\)\) \(_ BitVec ([0-9]+)\)\)"
    ).unwrap();
    smtlib = div_re
        .replace_all(
            &smtlib,
            r"(define-fun f_evm_$1_$2 ((x (_ BitVec $2)) (y (_ BitVec $2))) (_ BitVec $2) (ite (= y (_ bv0 $2)) (_ bv0 $2) ($1 x y)))",
        )
        .to_string();

    SMTQuery::new(smtlib, query.assertions.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_const_value_binary() {
        assert_eq!(parse_const_value("#b1010").unwrap(), 10);
        assert_eq!(parse_const_value("#b11111111").unwrap(), 255);
    }

    #[test]
    fn test_parse_const_value_hex() {
        assert_eq!(parse_const_value("#xFF").unwrap(), 255);
        assert_eq!(parse_const_value("#x10").unwrap(), 16);
    }

    #[test]
    fn test_parse_const_value_decimal() {
        assert_eq!(parse_const_value("bv42").unwrap(), 42);
        assert_eq!(parse_const_value("(_ bv123 256)").unwrap(), 123);
    }

    #[test]
    fn test_model_variable_creation() {
        let var = ModelVariable::new(
            "halmos_x_uint256".to_string(),
            "x".to_string(),
            "uint256".to_string(),
            "BitVec 256".to_string(),
            256,
            42,
        );
        assert_eq!(var.value, 42);
        assert_eq!(var.size_bits, 256);
    }

    #[test]
    fn test_potential_model_display_empty() {
        let model = PotentialModel::empty();
        assert_eq!(format!("{}", model), "∅");
    }

    #[test]
    fn test_sat_result_display() {
        assert_eq!(format!("{}", SatResult::Sat), "sat");
        assert_eq!(format!("{}", SatResult::Unsat), "unsat");
        assert_eq!(format!("{}", SatResult::Unknown), "unknown");
    }

    #[test]
    fn test_smt_query_creation() {
        let query = SMTQuery::new("(assert true)".to_string(), vec!["1".to_string()]);
        assert_eq!(query.smtlib, "(assert true)");
        assert_eq!(query.assertions.len(), 1);
    }

    #[test]
    fn test_is_model_valid() {
        assert!(is_model_valid("sat\n(model x 42)"));
        assert!(!is_model_valid("sat\nf_evm_bvmul_256"));
    }

    #[test]
    fn test_check_unsat_cores_found() {
        let query = SMTQuery::new(String::new(), vec!["1".to_string(), "2".to_string()]);
        let cores = vec![vec!["1".to_string()]];
        assert!(check_unsat_cores(&query, &cores));
    }

    #[test]
    fn test_check_unsat_cores_not_found() {
        let query = SMTQuery::new(String::new(), vec!["1".to_string()]);
        let cores = vec![vec!["3".to_string()]];
        assert!(!check_unsat_cores(&query, &cores));
    }

    #[test]
    fn test_refine_query_bvmul() {
        let query = SMTQuery::new(
            "(declare-fun f_evm_bvmul_256 ((_ BitVec 256) (_ BitVec 256)) (_ BitVec 256))"
                .to_string(),
            vec![],
        );
        let refined = refine_query(&query);
        assert!(refined.smtlib.contains("define-fun"));
        assert!(refined.smtlib.contains("bvmul"));
    }

    #[test]
    fn test_parse_unsat_core() {
        let output = "unsat\n(<123> <456> <789>)";
        let core = parse_unsat_core(output).unwrap();
        assert_eq!(core, vec!["123", "456", "789"]);
    }

    #[test]
    fn test_solver_output_from_sat() {
        let stdout = "sat\n(model (define-fun x () (_ BitVec 32) #x0000002a))";
        let output = SolverOutput::from_result(stdout, "", 0, 1, "test.smt2".to_string());
        assert_eq!(output.result, SatResult::Sat);
        assert!(output.model.is_some());
    }

    #[test]
    fn test_solver_output_from_unsat() {
        let stdout = "unsat";
        let output = SolverOutput::from_result(stdout, "", 0, 1, "test.smt2".to_string());
        assert_eq!(output.result, SatResult::Unsat);
        assert!(output.model.is_none());
    }

    #[test]
    fn test_solver_output_from_error() {
        let output = SolverOutput::from_error("test error".to_string(), 1, "test.smt2".to_string());
        assert_eq!(output.result, SatResult::Error);
        assert!(output.error.is_some());
    }
}
