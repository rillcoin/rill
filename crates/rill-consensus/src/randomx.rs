//! RandomX proof-of-work integration.
//!
//! Provides [`RandomXValidator`] (light-mode VM for block validation) and
//! [`RandomXMiner`] (full-mode VM for mining). Both auto-rotate the RandomX
//! key when crossing a key block interval boundary.
//!
//! # Key rotation
//!
//! The RandomX VM is seeded with a block hash that changes every
//! [`RANDOMX_KEY_BLOCK_INTERVAL`] blocks. This prevents precomputation attacks
//! where an adversary computes dataset tables far in advance.
//!
//! # Security considerations
//!
//! - **Attack vector: stale key.** If `update_key_if_needed` is not called
//!   before validation, an attacker could submit blocks mined against an old
//!   key. Always call `update_key_if_needed` before `hash`.
//! - **Attack vector: empty input.** RandomX rejects empty inputs. The
//!   `header_bytes()` method always returns 96 bytes, so this cannot happen
//!   for well-formed block headers. Callers providing raw bytes must ensure
//!   non-empty input.
//! - **Attack vector: key block hash unavailable.** If the chain does not have
//!   the key block hash (e.g., during initial sync), `update_key_if_needed`
//!   returns an error. Callers must handle this gracefully.

use std::sync::Mutex;

use randomx_rs::{RandomXCache, RandomXDataset, RandomXFlag, RandomXVM};
use rill_core::constants::RANDOMX_KEY_BLOCK_INTERVAL;
use rill_core::types::Hash256;

/// Compute the key block height for a given block height.
///
/// The key block is the block whose hash seeds the RandomX VM.
/// For heights < RANDOMX_KEY_BLOCK_INTERVAL, uses height 0 (genesis).
pub fn key_block_height(height: u64) -> u64 {
    (height / RANDOMX_KEY_BLOCK_INTERVAL) * RANDOMX_KEY_BLOCK_INTERVAL
}

/// Light-mode RandomX validator for block verification.
///
/// Uses a ~256 MB cache (no dataset). Suitable for full nodes that validate
/// blocks but do not need to mine.
pub struct RandomXValidator {
    vm: Mutex<RandomXVM>,
    current_key_height: Mutex<u64>,
    flags: RandomXFlag,
}

impl RandomXValidator {
    /// Create a new validator seeded with the given key hash.
    ///
    /// `key_height` is the height of the block whose hash is used as key.
    /// `key_hash` is that block's header hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the RandomX cache or VM allocation fails.
    pub fn new(key_height: u64, key_hash: &Hash256) -> Result<Self, String> {
        let flags = RandomXFlag::get_recommended_flags();
        let cache = RandomXCache::new(flags, key_hash.as_bytes())
            .map_err(|e| format!("RandomX cache init failed: {e}"))?;
        let vm = RandomXVM::new(flags, Some(cache), None)
            .map_err(|e| format!("RandomX VM init failed: {e}"))?;
        Ok(Self {
            vm: Mutex::new(vm),
            current_key_height: Mutex::new(key_height),
            flags,
        })
    }

    /// Update the RandomX key if the block height crosses a key interval boundary.
    ///
    /// `height` is the height of the block being validated.
    /// `get_hash` returns the block hash at a given height, or `None` if not found.
    ///
    /// # Errors
    ///
    /// Returns an error if the key block hash is not found or if RandomX
    /// reinitialization fails.
    pub fn update_key_if_needed(
        &self,
        height: u64,
        get_hash: impl Fn(u64) -> Option<Hash256>,
    ) -> Result<(), String> {
        let needed_key_height = key_block_height(height);
        let mut current = self.current_key_height.lock().unwrap();
        if *current == needed_key_height {
            return Ok(());
        }

        let key_hash = get_hash(needed_key_height)
            .ok_or_else(|| format!("key block hash not found at height {needed_key_height}"))?;

        let new_cache = RandomXCache::new(self.flags, key_hash.as_bytes())
            .map_err(|e| format!("RandomX cache reinit failed: {e}"))?;

        let mut vm = self.vm.lock().unwrap();
        vm.reinit_cache(new_cache)
            .map_err(|e| format!("RandomX VM reinit failed: {e}"))?;

        *current = needed_key_height;
        Ok(())
    }

    /// Compute the RandomX hash for the given input bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the RandomX hash computation fails or returns
    /// an unexpected number of bytes.
    pub fn hash(&self, input: &[u8]) -> Result<Hash256, String> {
        let vm = self.vm.lock().unwrap();
        let result = vm
            .calculate_hash(input)
            .map_err(|e| format!("RandomX hash failed: {e}"))?;
        if result.len() != 32 {
            return Err(format!(
                "RandomX hash returned {} bytes, expected 32",
                result.len()
            ));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Ok(Hash256(hash))
    }
}

/// Full-mode RandomX miner using the ~2 GB dataset for fast hashing.
///
/// The full dataset is precomputed from the cache and allows much faster
/// hash computation, making this suitable for mining operations.
pub struct RandomXMiner {
    vm: Mutex<RandomXVM>,
    current_key_height: Mutex<u64>,
    flags: RandomXFlag,
}

impl RandomXMiner {
    /// Create a new full-mode miner seeded with the given key hash.
    ///
    /// This allocates ~2 GB of memory for the RandomX dataset.
    ///
    /// # Errors
    ///
    /// Returns an error if the RandomX cache, dataset, or VM allocation fails.
    pub fn new(key_height: u64, key_hash: &Hash256) -> Result<Self, String> {
        let flags = RandomXFlag::get_recommended_flags() | RandomXFlag::FLAG_FULL_MEM;
        let cache = RandomXCache::new(flags, key_hash.as_bytes())
            .map_err(|e| format!("RandomX cache init failed: {e}"))?;
        let dataset = RandomXDataset::new(flags, cache, 0)
            .map_err(|e| format!("RandomX dataset init failed: {e}"))?;
        let vm = RandomXVM::new(flags, None, Some(dataset))
            .map_err(|e| format!("RandomX VM init failed: {e}"))?;
        Ok(Self {
            vm: Mutex::new(vm),
            current_key_height: Mutex::new(key_height),
            flags,
        })
    }

    /// Update the key if needed, same as validator.
    ///
    /// # Errors
    ///
    /// Returns an error if the key block hash is not found or if RandomX
    /// reinitialization fails.
    pub fn update_key_if_needed(
        &self,
        height: u64,
        get_hash: impl Fn(u64) -> Option<Hash256>,
    ) -> Result<(), String> {
        let needed_key_height = key_block_height(height);
        let mut current = self.current_key_height.lock().unwrap();
        if *current == needed_key_height {
            return Ok(());
        }

        let key_hash = get_hash(needed_key_height)
            .ok_or_else(|| format!("key block hash not found at height {needed_key_height}"))?;

        let cache = RandomXCache::new(self.flags, key_hash.as_bytes())
            .map_err(|e| format!("RandomX cache reinit failed: {e}"))?;
        let new_dataset = RandomXDataset::new(self.flags, cache, 0)
            .map_err(|e| format!("RandomX dataset reinit failed: {e}"))?;

        let mut vm = self.vm.lock().unwrap();
        vm.reinit_dataset(new_dataset)
            .map_err(|e| format!("RandomX VM reinit failed: {e}"))?;

        *current = needed_key_height;
        Ok(())
    }

    /// Compute the RandomX hash for the given input bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the RandomX hash computation fails or returns
    /// an unexpected number of bytes.
    pub fn hash(&self, input: &[u8]) -> Result<Hash256, String> {
        let vm = self.vm.lock().unwrap();
        let result = vm
            .calculate_hash(input)
            .map_err(|e| format!("RandomX hash failed: {e}"))?;
        if result.len() != 32 {
            return Err(format!(
                "RandomX hash returned {} bytes, expected 32",
                result.len()
            ));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Ok(Hash256(hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_block_height_genesis() {
        assert_eq!(key_block_height(0), 0);
        assert_eq!(key_block_height(1), 0);
        assert_eq!(key_block_height(2047), 0);
    }

    #[test]
    fn key_block_height_first_rotation() {
        assert_eq!(key_block_height(2048), 2048);
        assert_eq!(key_block_height(2049), 2048);
        assert_eq!(key_block_height(4095), 2048);
    }

    #[test]
    fn key_block_height_later_rotations() {
        assert_eq!(key_block_height(4096), 4096);
        assert_eq!(key_block_height(10000), 8192);
    }

    #[test]
    fn validator_hash_deterministic() {
        let key_hash = Hash256([0xAA; 32]);
        let validator = RandomXValidator::new(0, &key_hash).unwrap();
        let input = b"test input data";
        let h1 = validator.hash(input).unwrap();
        let h2 = validator.hash(input).unwrap();
        assert_eq!(h1, h2);
        assert!(!h1.is_zero());
    }

    #[test]
    fn validator_different_input_different_hash() {
        let key_hash = Hash256([0xAA; 32]);
        let validator = RandomXValidator::new(0, &key_hash).unwrap();
        let h1 = validator.hash(b"input1").unwrap();
        let h2 = validator.hash(b"input2").unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn validator_key_rotation() {
        let key0 = Hash256([0xAA; 32]);
        let key1 = Hash256([0xBB; 32]);
        let validator = RandomXValidator::new(0, &key0).unwrap();

        // Hash with key0
        let h_before = validator.hash(b"test").unwrap();

        // Rotate key
        validator
            .update_key_if_needed(2048, |h| {
                if h == 2048 {
                    Some(key1)
                } else {
                    Some(key0)
                }
            })
            .unwrap();

        // Hash with key1 -- should be different
        let h_after = validator.hash(b"test").unwrap();
        assert_ne!(h_before, h_after);
    }

    #[test]
    fn validator_no_rotation_same_interval() {
        let key0 = Hash256([0xAA; 32]);
        let validator = RandomXValidator::new(0, &key0).unwrap();

        // update_key_if_needed for same interval should be a no-op
        validator
            .update_key_if_needed(100, |_| Some(key0))
            .unwrap();

        let h = validator.hash(b"test").unwrap();
        assert!(!h.is_zero());
    }
}
