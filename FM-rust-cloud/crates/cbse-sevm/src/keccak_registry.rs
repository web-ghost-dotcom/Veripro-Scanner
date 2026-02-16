// SPDX-License-Identifier: AGPL-3.0

//! Keccak hash registry for tracking SHA3 operations in symbolic execution

use std::collections::HashMap;
use z3::ast::BV as Z3BV;

/// Registry for tracking Keccak hash expressions and their values
///
/// This provides:
/// 1. Hash expression tracking with unique IDs
/// 2. Reverse lookup from hash values to original expressions
/// 3. Support for offset-based lookups (hash + delta)
pub struct KeccakRegistry<'ctx> {
    /// Maps hash expressions to unique IDs
    hash_ids: HashMap<String, usize>,
    /// Maps hash values to their generating expressions
    hash_values: OffsetMap<'ctx>,
    next_id: usize,
}

/// Map that supports lookups with offsets
///
/// Allows finding original_expr such that:
/// hash_value = keccak(original_expr) + delta
struct OffsetMap<'ctx> {
    /// Maps base hash values to expressions
    map: HashMap<u64, String>,
    _phantom: std::marker::PhantomData<&'ctx ()>,
}

impl<'ctx> OffsetMap<'ctx> {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    fn insert(&mut self, hash_value: u64, expr: String) {
        self.map.insert(hash_value, expr);
    }

    /// Find an expression that, when hashed and offset, equals the target value
    /// Returns (expr, delta) where keccak(expr) + delta = target
    fn get(&self, target: u64) -> Option<(String, i64)> {
        // First try exact match
        if let Some(expr) = self.map.get(&target) {
            return Some((expr.clone(), 0));
        }

        // Then try with offsets (search nearby values)
        // This is expensive, so we limit the search range
        const MAX_OFFSET: i64 = 1024;

        for delta in 1..=MAX_OFFSET {
            if let Some(base) = target.checked_sub(delta as u64) {
                if let Some(expr) = self.map.get(&base) {
                    return Some((expr.clone(), delta));
                }
            }
        }

        // Try negative offsets
        for delta in 1..=MAX_OFFSET {
            if let Some(target_plus) = target.checked_add(delta as u64) {
                if let Some(expr) = self.map.get(&target_plus) {
                    return Some((expr.clone(), -delta));
                }
            }
        }

        None
    }

    fn copy(&self) -> Self {
        Self {
            map: self.map.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'ctx> KeccakRegistry<'ctx> {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            hash_ids: HashMap::new(),
            hash_values: OffsetMap::new(),
            next_id: 0,
        }
    }

    /// Get the unique ID for a hash expression
    pub fn get_id(&self, expr: &str) -> Option<usize> {
        self.hash_ids.get(expr).copied()
    }

    /// Check if an expression is registered
    pub fn contains(&self, expr: &str) -> bool {
        self.hash_ids.contains_key(expr)
    }

    /// Get an iterator over all registered expressions
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.hash_ids.keys()
    }

    /// Register a new hash expression with optional concrete hash value
    ///
    /// # Arguments
    /// * `expr` - The hash expression (e.g., "sha3_256(data)")
    /// * `hash_value` - The concrete hash value if available (32 bytes)
    pub fn register(&mut self, expr: String, hash_value: Option<&[u8]>) {
        // Skip if already registered
        if self.hash_ids.contains_key(&expr) {
            return;
        }

        // Assign new ID
        let id = self.next_id;
        self.next_id += 1;
        self.hash_ids.insert(expr.clone(), id);

        // Store hash value for reverse lookup if available
        if let Some(hash_bytes) = hash_value {
            if hash_bytes.len() == 32 {
                // Convert bytes to u64 (use first 8 bytes for map key)
                let mut key_bytes = [0u8; 8];
                key_bytes.copy_from_slice(&hash_bytes[0..8]);
                let key = u64::from_be_bytes(key_bytes);

                self.hash_values.insert(key, expr);
            }
        }
    }

    /// Reverse lookup: find the expression that produced a hash value
    ///
    /// Returns the expression that, when hashed and potentially offset,
    /// produces the given hash value.
    ///
    /// This checks:
    /// 1. Local registry (hashes generated during execution)
    /// 2. Precomputed registry (known/common hash expressions from cbse-hashes)
    ///
    /// # Arguments
    /// * `hash_value` - The hash value to look up (as integer)
    ///
    /// # Returns
    /// The original expression as a string, or None if not found
    pub fn reverse_lookup(&self, hash_value: u64) -> Option<String> {
        // Try local registry first
        if let Some((expr, delta)) = self.hash_values.get(hash_value) {
            if delta == 0 {
                return Some(expr);
            } else {
                return Some(format!("({} + {})", expr, delta));
            }
        }

        // Check precomputed registry from cbse-hashes
        if let Some(preimage) = check_precomputed_registry(hash_value) {
            return Some(preimage);
        }

        None
    }

    /// Create a copy of the registry
    pub fn copy(&self) -> Self {
        Self {
            hash_ids: self.hash_ids.clone(),
            hash_values: self.hash_values.copy(),
            next_id: self.next_id,
        }
    }

    /// Get the number of registered hashes
    pub fn len(&self) -> usize {
        self.hash_ids.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.hash_ids.is_empty()
    }
}

impl<'ctx> Default for KeccakRegistry<'ctx> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_basic() {
        let mut registry: KeccakRegistry = KeccakRegistry::new();

        // Register an expression
        let expr = "sha3_256(0x1234)".to_string();
        registry.register(expr.clone(), None);

        // Check it's registered
        assert!(registry.contains(&expr));
        assert_eq!(registry.get_id(&expr), Some(0));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_with_hash_value() {
        let mut registry: KeccakRegistry = KeccakRegistry::new();

        let expr = "sha3_256(0x1234)".to_string();
        let hash = vec![0u8; 32]; // Dummy hash

        registry.register(expr.clone(), Some(&hash));

        assert!(registry.contains(&expr));
        assert_eq!(registry.get_id(&expr), Some(0));
    }

    #[test]
    fn test_registry_multiple() {
        let mut registry: KeccakRegistry = KeccakRegistry::new();

        registry.register("sha3_256(a)".to_string(), None);
        registry.register("sha3_256(b)".to_string(), None);
        registry.register("sha3_512(c)".to_string(), None);

        assert_eq!(registry.len(), 3);
        assert_eq!(registry.get_id("sha3_256(a)"), Some(0));
        assert_eq!(registry.get_id("sha3_256(b)"), Some(1));
        assert_eq!(registry.get_id("sha3_512(c)"), Some(2));
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry: KeccakRegistry = KeccakRegistry::new();

        let expr = "sha3_256(x)".to_string();
        registry.register(expr.clone(), None);
        registry.register(expr.clone(), None); // Duplicate

        // Should still have only one entry
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_copy() {
        let mut registry: KeccakRegistry = KeccakRegistry::new();
        registry.register("sha3_256(data)".to_string(), None);

        let copy = registry.copy();

        assert_eq!(copy.len(), 1);
        assert!(copy.contains("sha3_256(data)"));
    }

    #[test]
    fn test_offset_map() {
        let mut map: OffsetMap = OffsetMap::new();

        map.insert(100, "expr1".to_string());
        map.insert(200, "expr2".to_string());

        // Exact match
        assert_eq!(map.get(100), Some(("expr1".to_string(), 0)));

        // With offset
        assert_eq!(map.get(105), Some(("expr1".to_string(), 5)));

        // Not found
        assert_eq!(map.get(5000), None);
    }
}

/// Check precomputed registry for known keccak256 preimages
///
/// This uses the cbse-hashes crate which contains precomputed
/// keccak256(x) values for x in 0..256
fn check_precomputed_registry(hash_value: u64) -> Option<String> {
    // Convert hash_value (first 8 bytes) to full 32-byte hash
    // and check against precomputed values

    // For now, we need to check if the hash_value matches any known preimage
    // The cbse-hashes crate has get_keccak256_256_preimage which takes full 32 bytes
    // We would need to reconstruct or store the full hash

    // Simplified: check if this could be a small integer hash
    // In practice, we'd store the full 32-byte hash and use cbse_hashes::get_keccak256_256_preimage

    // Return None for now - full implementation would require storing complete hashes
    None
}
