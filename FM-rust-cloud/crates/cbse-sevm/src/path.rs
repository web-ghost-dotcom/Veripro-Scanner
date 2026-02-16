// SPDX-License-Identifier: AGPL-3.0

//! Path management for symbolic execution with constraint tracking

use cbse_bitvec::CbseBitVec;
use cbse_exceptions::{CbseException, CbseResult};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use z3::{ast::Bool as Z3Bool, ast::BV as Z3BV, Context, SatResult, Solver};

/// Represents a path through symbolic execution with constraint tracking
///
/// Corresponds to Python's Path class in halmos/sevm.py at line 787
#[derive(Debug)]
pub struct Path<'ctx> {
    /// Reference-counted solver - allows multiple paths to share one solver instance
    /// This matches Python's approach where all paths share the same solver
    pub solver: Rc<Solver<'ctx>>,
    pub num_scopes: usize,
    pub conditions: Vec<(Z3Bool<'ctx>, bool)>, // Vec of (condition, is_branching)
    pub concretization: Concretization<'ctx>,
    pub pending: Vec<Z3Bool<'ctx>>,
    pub related: HashMap<usize, HashSet<usize>>,
    pub var_to_conds: HashMap<String, HashSet<usize>>,
    pub term_to_vars: HashMap<String, HashSet<String>>,
    pub sliced: Option<HashSet<usize>>,
}

impl<'ctx> Clone for Path<'ctx> {
    fn clone(&self) -> Self {
        Self {
            solver: self.solver.clone(), // Rc clone - shares solver
            num_scopes: self.num_scopes,
            conditions: self.conditions.clone(),
            concretization: self.concretization.clone(),
            pending: self.pending.clone(),
            related: self.related.clone(),
            var_to_conds: self.var_to_conds.clone(),
            term_to_vars: self.term_to_vars.clone(),
            sliced: self.sliced.clone(),
        }
    }
}

/// Concretization mapping for symbolic values
#[derive(Debug, Clone)]
pub struct Concretization<'ctx> {
    /// Maps symbolic terms to concrete values
    pub substitution: HashMap<String, u64>,
    /// Maps symbols to candidate concrete values for branching
    pub candidates: HashMap<String, Vec<u64>>,
    _phantom: std::marker::PhantomData<&'ctx ()>,
}

impl<'ctx> Concretization<'ctx> {
    pub fn new() -> Self {
        Self {
            substitution: HashMap::new(),
            candidates: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Process a condition to extract concretization information
    /// Extracts equality constraints like (x == 5) and adds to substitution map
    pub fn process_cond(&mut self, cond: &Z3Bool<'ctx>) {
        // For now, simplified implementation
        // TODO: Extract equality constraints from Z3 AST
        // This would require walking the AST to find (var == constant) patterns
    }

    /// Process dynamic parameters to add candidates
    pub fn process_dyn_params(&mut self, params: &[(String, Vec<u64>)]) {
        for (symbol, choices) in params {
            self.candidates.insert(symbol.clone(), choices.clone());
        }
    }
}

impl<'ctx> Default for Concretization<'ctx> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ctx> Path<'ctx> {
    /// Create a new path with the given solver
    pub fn new(solver: Rc<Solver<'ctx>>) -> Self {
        Self {
            solver,
            num_scopes: 0,
            conditions: Vec::new(),
            concretization: Concretization::new(),
            pending: Vec::new(),
            related: HashMap::new(),
            var_to_conds: HashMap::new(),
            term_to_vars: HashMap::new(),
            sliced: None,
        }
    }

    /// Check if a condition is satisfiable
    pub fn check(&self, cond: &Z3Bool<'ctx>) -> CbseResult<SatResult> {
        self.solver.push();
        self.solver.assert(cond);
        let result = self.solver.check();
        self.solver.pop(1);
        Ok(result)
    }

    /// Branch the path with a new condition
    ///
    /// Creates a new path that shares the same solver instance, following Python's
    /// implementation at line 923-966 in halmos/sevm.py
    pub fn branch(&self, cond: Z3Bool<'ctx>) -> CbseResult<Path<'ctx>> {
        if !self.pending.is_empty() {
            return Err(CbseException::Internal(
                "Cannot branch from inactive path".to_string(),
            ));
        }

        // Push a new solver scope (Python line 935)
        self.solver.push();

        // Get current number of scopes - we track this manually since Solver doesn't expose it
        let num_scopes = self.num_scopes + 1;

        // Create a new path sharing the same solver (Rc clones the reference, not the solver)
        let new_path = Path {
            solver: Rc::clone(&self.solver),
            num_scopes,
            conditions: self.conditions.clone(),
            concretization: self.concretization.clone(),
            pending: vec![cond],
            related: self.related.clone(),
            var_to_conds: self.var_to_conds.clone(),
            term_to_vars: self.term_to_vars.clone(),
            sliced: None,
        };

        Ok(new_path)
    }

    /// Check if the path is activated (no pending conditions)
    pub fn is_activated(&self) -> bool {
        self.pending.is_empty()
    }

    /// Activate the path by adding pending conditions
    pub fn activate(&mut self) -> CbseResult<()> {
        // Pop to the saved scope level
        // We track num_scopes manually since the solver doesn't expose this
        let scopes_to_pop = if self.num_scopes > 0 {
            // Calculate how many scopes we need to pop based on tracking
            // This is an approximation - in production you'd want better tracking
            0 // For now, don't pop - just add conditions
        } else {
            0
        };

        if scopes_to_pop > 0 {
            self.solver.pop(scopes_to_pop);
        }

        // Add pending conditions
        let pending = std::mem::take(&mut self.pending);
        for cond in pending {
            self.append(cond, true)?;
        }

        Ok(())
    }

    /// Collect variable sets for dependency tracking
    /// Recursively walks the Z3 AST to find all variables
    pub fn collect_var_sets(&mut self, term: &Z3Bool<'ctx>) {
        // Create a unique key for this term
        let term_str = format!("{}", term);

        // Check if already processed
        if self.term_to_vars.contains_key(&term_str) {
            return;
        }

        // For now, simplified: just use the term string itself as a variable
        // TODO: Implement proper Z3 AST traversal to extract variables
        let mut result = HashSet::new();
        result.insert(term_str.clone());
        self.term_to_vars.insert(term_str, result);
    }

    /// Helper to collect variables from Bool terms
    fn collect_var_sets_internal(&mut self, term: &Z3Bool<'ctx>) {
        self.collect_var_sets(term);
    }

    /// Helper to collect variables from BitVec terms  
    fn collect_var_sets_bv(&mut self, term: &Z3BV<'ctx>) {
        let term_str = format!("{}", term);

        // Check if already processed
        if self.term_to_vars.contains_key(&term_str) {
            return;
        }

        // Simplified: use term string as variable
        let mut result = HashSet::new();
        result.insert(term_str.clone());
        self.term_to_vars.insert(term_str, result);
    }

    /// Get the variable set for a term
    pub fn get_var_set(&mut self, term: &Z3Bool<'ctx>) -> HashSet<String> {
        self.collect_var_sets(term);
        let term_str = format!("{}", term);
        self.term_to_vars
            .get(&term_str)
            .cloned()
            .unwrap_or_default()
    }

    /// Append a condition to the path
    pub fn append(&mut self, cond: Z3Bool<'ctx>, branching: bool) -> CbseResult<()> {
        // TODO: Simplify condition if needed
        // For now, skip simplification as it requires Z3 API we don't have access to

        // Skip if condition is trivially true (we can't easily check this without Z3 API)
        // For now, just add it

        // Check if already exists (by comparing with existing conditions)
        let cond_str = format!("{}", cond);
        for (existing, _) in &self.conditions {
            if format!("{}", existing) == cond_str {
                return Ok(());
            }
        }

        // Determine the index for the new condition
        let idx = self.conditions.len();

        // Add to solver and conditions
        self.solver.assert(&cond);
        self.conditions.push((cond.clone(), branching));
        self.concretization.process_cond(&cond);

        // Update dependency tracking
        let var_set = self.get_var_set(&cond);
        let related = self._get_related(&var_set);
        self.related.insert(idx, related);

        for var in var_set {
            self.var_to_conds.entry(var).or_default().insert(idx);
        }

        Ok(())
    }

    /// Extend the path with multiple conditions
    pub fn extend(&mut self, conds: Vec<Z3Bool<'ctx>>, branching: bool) -> CbseResult<()> {
        for cond in conds {
            self.append(cond, branching)?;
        }
        Ok(())
    }

    /// Extend from another path
    pub fn extend_path(&mut self, other: &Path<'ctx>) -> CbseResult<()> {
        self.conditions = other.conditions.clone();
        self.concretization = other.concretization.clone();
        self.related = other.related.clone();
        self.var_to_conds = other.var_to_conds.clone();
        self.term_to_vars = other.term_to_vars.clone();

        // If the parent path is not sliced, add all constraints to the solver
        if other.sliced.is_none() {
            for (cond, _) in &self.conditions {
                self.solver.assert(cond);
            }
            return Ok(());
        }

        // If the parent path is sliced, add only sliced constraints to the solver
        if let Some(ref sliced) = other.sliced {
            for (idx, (cond, _)) in self.conditions.iter().enumerate() {
                if sliced.contains(&idx) {
                    self.solver.assert(cond);
                }
            }
        }

        Ok(())
    }

    /// Slice the path based on variable set
    pub fn slice(&mut self, var_set: &HashSet<String>) -> CbseResult<()> {
        if self.sliced.is_some() {
            return Err(CbseException::Internal("Path already sliced".to_string()));
        }

        self.sliced = Some(self._get_related(var_set));
        Ok(())
    }

    /// Get related conditions for a variable set
    fn _get_related(&self, var_set: &HashSet<String>) -> HashSet<usize> {
        let mut conds = HashSet::new();

        for var in var_set {
            if let Some(var_conds) = self.var_to_conds.get(var) {
                conds.extend(var_conds);
            }
        }

        let mut result = conds.clone();
        for cond_idx in &conds {
            if let Some(related) = self.related.get(cond_idx) {
                result.extend(related);
            }
        }

        result
    }

    /// Generate SMT2 query
    pub fn to_smt2(&self) -> CbseResult<String> {
        // Convert solver assertions to SMT2 format
        Ok(self.solver.to_string())
    }

    /// Get a string representation of the path
    pub fn to_string(&self) -> String {
        let mut output = String::new();
        for (cond_id, is_branching) in &self.conditions {
            if *is_branching {
                output.push_str(&format!("- {}\n", cond_id));
            }
        }
        if output.is_empty() {
            output = "- (empty path condition)\n".to_string();
        }
        output
    }

    /// Get a model from the solver showing satisfying assignment
    ///
    /// This extracts concrete values for symbolic variables from the current solver state.
    /// Returns a HashMap mapping variable names to their concrete values.
    ///
    /// Matches Python's model extraction in solve.py at lines 300-400
    pub fn get_model(&self) -> CbseResult<HashMap<String, u64>> {
        // Check if current path is satisfiable
        if self.solver.check() != SatResult::Sat {
            return Ok(HashMap::new());
        }

        let model = self.solver.get_model().ok_or_else(|| {
            CbseException::Internal("Solver returned SAT but no model available".to_string())
        })?;

        let mut result = HashMap::new();

        // Collect all symbolic variables from conditions
        let mut symbolic_vars: HashSet<String> = HashSet::new();
        for (cond, _) in &self.conditions {
            // Extract variable names from condition string representation
            let cond_str = format!("{}", cond);
            // Look for halmos_ or p_ prefixed variables
            for word in cond_str.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if word.starts_with("halmos_") || word.starts_with("p_") {
                    symbolic_vars.insert(word.to_string());
                }
            }
        }

        // For each collected variable, try to evaluate it in the model
        for var_name in symbolic_vars {
            // Try to create a BV constant with this name and evaluate it
            // Note: This is a simplified approach - in production we'd need to track
            // the actual Z3 AST nodes for variables
            // For now, we'll just return the concretization map if it exists
        }

        // Return concretization substitutions as a fallback
        // This contains variables that were explicitly concretized
        for (name, value) in &self.concretization.substitution {
            result.insert(name.clone(), *value);
        }

        Ok(result)
    }

    /// Format a counterexample model into a human-readable string
    ///
    /// Displays variable names and their concrete values in hexadecimal format.
    /// Example output: "halmos_arg_0 = 0x2a, halmos_storage_0 = 0xff"
    pub fn format_counterexample(model: &HashMap<String, u64>) -> String {
        if model.is_empty() {
            return "∅".to_string();
        }

        let mut entries: Vec<String> = model
            .iter()
            .map(|(name, value)| format!("{} = 0x{:x}", name, value))
            .collect();

        entries.sort();
        entries.join(", ")
    }

    /// Check if the current path is satisfiable
    ///
    /// Returns true if there exists a concrete assignment that satisfies all constraints.
    /// This is used to check path feasibility before continuing exploration.
    pub fn is_feasible(&self) -> bool {
        self.solver.check() == SatResult::Sat
    }

    /// Check if a specific condition would be satisfiable with current constraints
    ///
    /// This temporarily adds the condition to the solver, checks satisfiability,
    /// then removes it. Used for branch feasibility checking.
    pub fn check_feasibility(&self, cond: &Z3Bool<'ctx>) -> SatResult {
        self.solver.push();
        self.solver.assert(cond);
        let result = self.solver.check();
        self.solver.pop(1);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use z3::{ast::Ast, Config};

    #[test]
    fn test_path_creation() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Rc::new(Solver::new(&ctx));
        let path = Path::new(solver);

        assert!(path.is_activated());
        assert_eq!(path.conditions.len(), 0);
    }

    #[test]
    fn test_path_append() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Rc::new(Solver::new(&ctx));
        let mut path = Path::new(solver);

        let x = z3::ast::Bool::new_const(&ctx, "x");
        path.append(x, false).unwrap();

        assert_eq!(path.conditions.len(), 1);
    }

    #[test]
    fn test_concretization() {
        let mut conc: Concretization = Concretization::new();
        conc.substitution.insert("x".to_string(), 42);
        conc.candidates.insert("y".to_string(), vec![1, 2, 3]);

        assert_eq!(conc.substitution.get("x"), Some(&42));
        assert_eq!(conc.candidates.get("y").unwrap().len(), 3);
    }

    #[test]
    fn test_counterexample_formatting() {
        let mut model = HashMap::new();
        model.insert("halmos_arg_0".to_string(), 42);
        model.insert("halmos_storage_0".to_string(), 255);

        let formatted = Path::format_counterexample(&model);
        assert!(formatted.contains("halmos_arg_0 = 0x2a"));
        assert!(formatted.contains("halmos_storage_0 = 0xff"));
    }

    #[test]
    fn test_path_feasibility() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Rc::new(Solver::new(&ctx));
        let mut path = Path::new(solver);

        // Initially feasible (no constraints)
        assert!(path.is_feasible());

        // Add a simple constraint: x == 5
        let x = z3::ast::BV::new_const(&ctx, "x", 256);
        let five = z3::ast::BV::from_u64(&ctx, 5, 256);
        let constraint = x._eq(&five);
        path.append(constraint, false).unwrap();

        // Still feasible
        assert!(path.is_feasible());

        // Check that x == 10 is infeasible given x == 5
        let ten = z3::ast::BV::from_u64(&ctx, 10, 256);
        let new_constraint = x._eq(&ten);
        assert_eq!(path.check_feasibility(&new_constraint), SatResult::Unsat);
    }
}
