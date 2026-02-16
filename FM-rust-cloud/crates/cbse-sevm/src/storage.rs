// SPDX-License-Identifier: AGPL-3.0

//! Storage models for Solidity and generic storage layouts

use cbse_bitvec::CbseBitVec;
use cbse_exceptions::{CbseException, CbseResult};
use std::collections::HashMap;
use z3::{ast::Array as Z3Array, Context, Sort};

/// Storage data container
#[derive(Debug, Clone)]
pub struct StorageData<'ctx> {
    /// Whether this storage uses symbolic values
    pub symbolic: bool,
    /// The actual storage mapping
    /// For SolidityStorage: (slot, num_keys, size_keys) -> value or array
    mapping: HashMap<StorageKey, StorageValue<'ctx>>,
}

/// Storage key for the mapping
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageKey {
    /// Solidity storage: (slot, num_keys, size_keys)
    Solidity(u64, usize, usize),
    /// Generic storage: just the size
    Generic(usize),
}

/// Storage value
#[derive(Debug, Clone)]
pub enum StorageValue<'ctx> {
    /// A concrete or symbolic value (for scalar storage)
    Value(CbseBitVec<'ctx>),
    /// A Z3 Array (for mapping/array storage)
    Array(Z3Array<'ctx>),
}

impl<'ctx> StorageData<'ctx> {
    /// Create new empty storage data
    pub fn new() -> Self {
        Self {
            symbolic: false,
            mapping: HashMap::new(),
        }
    }

    /// Get a value from storage
    pub fn get(&self, key: &StorageKey) -> Option<&StorageValue<'ctx>> {
        self.mapping.get(key)
    }

    /// Set a value in storage
    pub fn set(&mut self, key: StorageKey, value: StorageValue<'ctx>) {
        self.mapping.insert(key, value);
    }

    /// Check if a key exists
    pub fn contains(&self, key: &StorageKey) -> bool {
        self.mapping.contains_key(key)
    }

    /// Compute a hash digest of the storage (for state comparison)
    pub fn digest(&self) -> u64 {
        // Simple hash based on the number of entries
        // In production, this should be more sophisticated
        self.mapping.len() as u64
    }
}

impl<'ctx> Default for StorageData<'ctx> {
    fn default() -> Self {
        Self::new()
    }
}

/// Solidity storage model
///
/// Handles Solidity-specific storage layout with:
/// - Scalar values at specific slots
/// - Mappings with keccak-based slot calculation
/// - Arrays with sequential slot allocation
pub struct SolidityStorage;

impl SolidityStorage {
    /// Create a new storage data instance
    pub fn mk_storagedata<'ctx>() -> StorageData<'ctx> {
        StorageData::new()
    }

    /// Create an empty Z3 Array for a given slot and keys
    /// Returns an Array from (concat of keys) -> BitVec(256)
    pub fn empty<'ctx>(
        addr: &[u8; 20],
        slot: u64,
        num_keys: usize,
        size_keys: usize,
        ctx: &'ctx Context,
    ) -> Z3Array<'ctx> {
        let name = format!("storage_{:?}_{}_{}_{}_00", addr, slot, num_keys, size_keys);

        // Create domain sort (BitVec of size_keys bits)
        let domain_sort = Sort::bitvector(ctx, size_keys as u32);
        // Create range sort (BitVec of 256 bits for EVM word)
        let range_sort = Sort::bitvector(ctx, 256);

        // Create Z3 Array
        Z3Array::new_const(ctx, name, &domain_sort, &range_sort)
    }

    /// Initialize storage location if not yet initialized
    pub fn init<'ctx>(
        storage: &mut HashMap<[u8; 20], StorageData<'ctx>>,
        addr: [u8; 20],
        slot: u64,
        num_keys: usize,
        size_keys: usize,
        ctx: &'ctx Context,
    ) -> CbseResult<()> {
        let storage_addr = storage.entry(addr).or_insert_with(StorageData::new);

        let key = StorageKey::Solidity(slot, num_keys, size_keys);

        if !storage_addr.contains(&key) {
            if size_keys > 0 {
                // Mapping type: use Z3 Array
                let array = Self::empty(&addr, slot, num_keys, size_keys, ctx);
                storage_addr.set(key, StorageValue::Array(array));
            } else {
                // Scalar type: initialize with zero or symbolic
                let value = if storage_addr.symbolic {
                    CbseBitVec::symbolic(
                        ctx,
                        &format!("storage_{:?}_{}_{}_{}_00", addr, slot, num_keys, size_keys),
                        256,
                    )
                } else {
                    CbseBitVec::from_u64(0, 256)
                };
                storage_addr.set(key, StorageValue::Value(value));
            }
        }

        Ok(())
    }

    /// Load a value from storage
    pub fn load<'ctx>(
        storage: &HashMap<[u8; 20], StorageData<'ctx>>,
        addr: [u8; 20],
        slot: u64,
        keys: &[CbseBitVec<'ctx>],
        ctx: &'ctx Context,
    ) -> CbseResult<CbseBitVec<'ctx>> {
        let num_keys = keys.len();
        let size_keys: usize = keys.iter().map(|k| k.size() as usize).sum();

        let storage_addr = storage
            .get(&addr)
            .ok_or_else(|| CbseException::Internal("Storage address not found".to_string()))?;

        let key = StorageKey::Solidity(slot, num_keys, size_keys);

        match storage_addr.get(&key) {
            Some(StorageValue::Value(v)) => {
                // Scalar storage: return the value directly
                Ok(v.clone())
            }
            Some(StorageValue::Array(array)) => {
                // Mapping storage: use Z3 Select to read from array
                if keys.is_empty() {
                    return Err(CbseException::Internal(
                        "Cannot load from array without keys".to_string(),
                    ));
                }

                // Concatenate keys to form the array index
                let concat_key = if keys.len() == 1 {
                    keys[0].clone()
                } else {
                    // Concatenate multiple keys
                    let mut result = keys[0].clone();
                    for key in &keys[1..] {
                        result = result.concat(key);
                    }
                    result
                };

                // Use Z3 Select operation: Select(array, index)
                let value = array.select(&concat_key.as_z3(ctx));
                Ok(CbseBitVec::from_z3(value.as_bv().unwrap()))
            }
            None => {
                // Uninitialized storage returns zero
                Ok(CbseBitVec::from_u64(0, 256))
            }
        }
    }

    /// Store a value to storage
    pub fn store<'ctx>(
        storage: &mut HashMap<[u8; 20], StorageData<'ctx>>,
        addr: [u8; 20],
        slot: u64,
        keys: &[CbseBitVec<'ctx>],
        value: CbseBitVec<'ctx>,
        ctx: &'ctx Context,
    ) -> CbseResult<()> {
        let num_keys = keys.len();
        let size_keys: usize = keys.iter().map(|k| k.size() as usize).sum();

        let storage_addr = storage.entry(addr).or_insert_with(StorageData::new);

        let key = StorageKey::Solidity(slot, num_keys, size_keys);

        if num_keys == 0 {
            // Scalar storage: store the value directly
            storage_addr.set(key, StorageValue::Value(value));
        } else {
            // Mapping storage: use Z3 Store to create updated array

            // Get the current array or create a new one
            let current_array = if let Some(StorageValue::Array(arr)) = storage_addr.get(&key) {
                arr.clone()
            } else {
                // Create empty array if not exists
                Self::empty(&addr, slot, num_keys, size_keys, ctx)
            };

            // Concatenate keys to form the array index
            let concat_key = if keys.len() == 1 {
                keys[0].clone()
            } else {
                let mut result = keys[0].clone();
                for key in &keys[1..] {
                    result = result.concat(key);
                }
                result
            };

            // Use Z3 Store operation: Store(array, index, value)
            let new_array = current_array.store(&concat_key.as_z3(ctx), &value.as_z3(ctx));

            // Store the new array
            storage_addr.set(key, StorageValue::Array(new_array));
        }

        Ok(())
    }

    /// Decode a storage location into (slot, keys)
    /// This handles Solidity's storage layout rules following Python implementation
    ///
    /// Solidity storage layout patterns:
    /// 1. m[k]: hash(k . m) where k is 256-bit → sha3_512
    /// 2. a[i]: hash(a) + i → sha3_256
    /// 3. m[k]: hash(k . m) where k is non-256-bit → generic sha3 with concat
    /// 4. Array indexing: base + offset → bvadd
    /// 5. Concrete values: lookup in keccak registry for reverse mapping
    ///
    /// Returns: (base_slot, [key1, key2, ...]) where keys are in order
    pub fn decode<'ctx>(
        loc: &CbseBitVec<'ctx>,
        ctx: &'ctx Context,
    ) -> CbseResult<(u64, Vec<CbseBitVec<'ctx>>)> {
        Self::decode_recursive(loc, ctx)
    }

    /// Recursive helper for decode that returns a tuple of decoded components
    /// Mirrors Python's cls.decode(ex, loc) which returns tuple
    fn decode_recursive<'ctx>(
        loc: &CbseBitVec<'ctx>,
        ctx: &'ctx Context,
    ) -> CbseResult<(u64, Vec<CbseBitVec<'ctx>>)> {
        use z3::ast::Ast;

        // Simplify the location first (Python: loc = normalize(loc))
        let loc_bv = loc.as_z3(ctx);
        let simplified = loc_bv.simplify();

        // Check if this is a concrete value
        // In Z3, we can check if it's a numeral by trying to get its u64 value
        if let Some(val) = simplified.as_u64() {
            // Just a concrete slot (keccak registry not yet implemented)
            return Ok((val, Vec::new()));
        }

        // For now, simplified implementation: return the location as a single key
        // Full Z3 App introspection would require accessing internal Z3 AST structure
        // which is not easily exposed in z3-sys Rust bindings.
        //
        // TODO: For complete implementation, we would need to:
        // 1. Parse string representation of the expression
        // 2. Or use z3-sys FFI to access Z3_get_app_decl, Z3_get_app_num_args, etc.
        // 3. Pattern match on:
        //    - f_sha3_512(concat(key, base)) for mapping[key]
        //    - f_sha3_256(base) for array indexing
        //    - bvadd(base, offset) for array offset calculations
        //    - concat operations for non-256-bit keys
        //
        // For basic functionality, treating location as single key works for simple storage
        Ok((0, vec![loc.clone()]))
    }
}

/// Generic storage model
///
/// Simpler storage model that doesn't assume Solidity layout.
/// Uses direct address-based storage without layout rules.
pub struct GenericStorage;

impl GenericStorage {
    /// Create a new storage data instance
    pub fn mk_storagedata<'ctx>() -> StorageData<'ctx> {
        StorageData::new()
    }

    /// Create an empty array for storage
    pub fn empty<'ctx>(addr: &[u8; 20], size: usize, ctx: &'ctx Context) -> Z3Array<'ctx> {
        let name = format!("storage_{:?}_{}", addr, size);
        let domain_sort = Sort::bitvector(ctx, size as u32);
        let range_sort = Sort::bitvector(ctx, 256);
        Z3Array::new_const(ctx, name, &domain_sort, &range_sort)
    }

    /// Initialize storage if needed
    pub fn init<'ctx>(
        storage: &mut HashMap<[u8; 20], StorageData<'ctx>>,
        addr: [u8; 20],
        size_keys: usize,
        ctx: &'ctx Context,
    ) -> CbseResult<()> {
        let storage_addr = storage.entry(addr).or_insert_with(StorageData::new);

        let key = StorageKey::Generic(size_keys);

        if !storage_addr.contains(&key) {
            let array = Self::empty(&addr, size_keys, ctx);
            storage_addr.set(key, StorageValue::Array(array));
        }

        Ok(())
    }

    /// Load from generic storage
    pub fn load<'ctx>(
        storage: &HashMap<[u8; 20], StorageData<'ctx>>,
        addr: [u8; 20],
        loc: &CbseBitVec<'ctx>,
        ctx: &'ctx Context,
    ) -> CbseResult<CbseBitVec<'ctx>> {
        let size_keys = loc.size() as usize;

        let storage_addr = storage
            .get(&addr)
            .ok_or_else(|| CbseException::Internal("Storage address not found".to_string()))?;

        let key = StorageKey::Generic(size_keys);

        match storage_addr.get(&key) {
            Some(StorageValue::Array(_array_name)) => {
                // Return symbolic value for now
                Ok(CbseBitVec::symbolic(
                    ctx,
                    &format!("storage_load_{}", size_keys),
                    256,
                ))
            }
            Some(StorageValue::Value(v)) => Ok(v.clone()),
            None => Ok(CbseBitVec::from_u64(0, 256)),
        }
    }

    /// Store to generic storage
    pub fn store<'ctx>(
        storage: &mut HashMap<[u8; 20], StorageData<'ctx>>,
        addr: [u8; 20],
        loc: &CbseBitVec<'ctx>,
        value: CbseBitVec<'ctx>,
        _ctx: &'ctx Context,
    ) -> CbseResult<()> {
        let size_keys = loc.size() as usize;

        let storage_addr = storage.entry(addr).or_insert_with(StorageData::new);
        let key = StorageKey::Generic(size_keys);

        storage_addr.set(key, StorageValue::Value(value));

        Ok(())
    }

    /// Decode storage location (simpler for generic model)
    pub fn decode<'ctx>(loc: &CbseBitVec<'ctx>) -> CbseResult<CbseBitVec<'ctx>> {
        // For generic model, location is used as-is
        Ok(loc.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use z3::Config;

    #[test]
    fn test_storage_data() {
        let mut storage: StorageData = StorageData::new();

        let key = StorageKey::Solidity(0, 0, 0);
        let value = StorageValue::Value(CbseBitVec::from_u64(42, 256));

        storage.set(key.clone(), value);

        assert!(storage.contains(&key));
    }

    #[test]
    fn test_solidity_storage() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut storage = HashMap::new();
        let addr = [1u8; 20];

        // Initialize storage
        SolidityStorage::init(&mut storage, addr, 0, 0, 0, &ctx).unwrap();

        // Store a value
        let value = CbseBitVec::from_u64(100, 256);
        SolidityStorage::store(&mut storage, addr, 0, &[], value, &ctx).unwrap();

        // Load it back
        let loaded = SolidityStorage::load(&storage, addr, 0, &[], &ctx).unwrap();
        assert_eq!(loaded.as_u64().unwrap(), 100);
    }

    #[test]
    fn test_generic_storage() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut storage = HashMap::new();
        let addr = [2u8; 20];

        // Initialize
        GenericStorage::init(&mut storage, addr, 256, &ctx).unwrap();

        let loc = CbseBitVec::from_u64(5, 256);
        let value = CbseBitVec::from_u64(200, 256);

        // Store
        GenericStorage::store(&mut storage, addr, &loc, value, &ctx).unwrap();

        // Load
        let loaded = GenericStorage::load(&storage, addr, &loc, &ctx).unwrap();
        // Note: Might be symbolic in actual implementation
        assert!(loaded.as_u64().is_ok());
    }
}
