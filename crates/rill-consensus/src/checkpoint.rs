//! Header checkpoint verification.
//!
//! Provides functions to verify that blocks at checkpoint heights match the
//! expected hash, and to reject reorgs that would unwind past a checkpoint.
//!
//! # Attack vectors
//!
//! - **Long-range rewrite:** Without checkpoints an attacker with sufficient
//!   hash power could rewrite arbitrarily deep history. Checkpoints pin known-
//!   good blocks so that reorgs below the last checkpoint are rejected outright.
//!
//! - **Checkpoint spoofing:** The checkpoint list is compiled into the binary.
//!   An attacker would need to distribute a modified binary to exploit this,
//!   which is outside our threat model.
//!
//! # Usage
//!
//! The node layer should call [`check_checkpoint`] (or
//! [`check_checkpoint_with`] for testing) when connecting a new block whose
//! height is known. It should call [`is_below_checkpoint`] before accepting a
//! reorg that would disconnect blocks at or below the last checkpoint height.
//!
//! The [`SyncManager`](crate::engine) does **not** call these functions
//! directly because it does not track block heights. The node is responsible
//! for invoking checkpoint validation during sync header processing.

use rill_core::constants::CHECKPOINTS;
use rill_core::error::BlockError;
use rill_core::types::Hash256;

/// Verify that a block at the given `height` has the expected checkpoint hash.
///
/// If `height` matches a checkpoint height, the block hash must match exactly.
/// If there is no checkpoint at `height`, the function succeeds unconditionally.
///
/// # Errors
///
/// Returns [`BlockError::CheckpointMismatch`] when the hash does not match
/// the checkpoint at the given height.
pub fn check_checkpoint(height: u64, hash: &Hash256) -> Result<(), BlockError> {
    check_checkpoint_with(CHECKPOINTS, height, hash)
}

/// Like [`check_checkpoint`] but takes an explicit checkpoint list.
///
/// This is the testable core: production code passes [`CHECKPOINTS`], while
/// tests can supply their own list.
pub fn check_checkpoint_with(
    checkpoints: &[(u64, [u8; 32])],
    height: u64,
    hash: &Hash256,
) -> Result<(), BlockError> {
    for &(cp_height, cp_hash) in checkpoints {
        if cp_height == height {
            if hash.0 != cp_hash {
                return Err(BlockError::CheckpointMismatch);
            }
            return Ok(());
        }
    }
    Ok(())
}

/// Return the height of the most recent checkpoint, or 0 if there are none.
pub fn last_checkpoint_height() -> u64 {
    last_checkpoint_height_with(CHECKPOINTS)
}

/// Like [`last_checkpoint_height`] but with an explicit checkpoint list.
pub fn last_checkpoint_height_with(checkpoints: &[(u64, [u8; 32])]) -> u64 {
    checkpoints.iter().map(|(h, _)| *h).max().unwrap_or(0)
}

/// Returns `true` if `height` is at or below the last checkpoint height.
///
/// The node should reject any reorg that would disconnect blocks at or below
/// this height, because those blocks are pinned by a checkpoint.
pub fn is_below_checkpoint(height: u64) -> bool {
    is_below_checkpoint_with(CHECKPOINTS, height)
}

/// Like [`is_below_checkpoint`] but with an explicit checkpoint list.
pub fn is_below_checkpoint_with(checkpoints: &[(u64, [u8; 32])], height: u64) -> bool {
    let last = last_checkpoint_height_with(checkpoints);
    last > 0 && height <= last
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test-only checkpoint list with two entries.
    const TEST_CHECKPOINTS: &[(u64, [u8; 32])] = &[
        (10, [0xAA; 32]),
        (50, [0xBB; 32]),
    ];

    // ------------------------------------------------------------------
    // check_checkpoint_with
    // ------------------------------------------------------------------

    #[test]
    fn checkpoint_passes_for_matching_hash() {
        let hash = Hash256([0xAA; 32]);
        assert!(check_checkpoint_with(TEST_CHECKPOINTS, 10, &hash).is_ok());

        let hash2 = Hash256([0xBB; 32]);
        assert!(check_checkpoint_with(TEST_CHECKPOINTS, 50, &hash2).is_ok());
    }

    #[test]
    fn checkpoint_fails_for_wrong_hash() {
        let wrong = Hash256([0xFF; 32]);
        let err = check_checkpoint_with(TEST_CHECKPOINTS, 10, &wrong).unwrap_err();
        assert_eq!(err, BlockError::CheckpointMismatch);

        let also_wrong = Hash256([0x00; 32]);
        let err2 = check_checkpoint_with(TEST_CHECKPOINTS, 50, &also_wrong).unwrap_err();
        assert_eq!(err2, BlockError::CheckpointMismatch);
    }

    #[test]
    fn no_checkpoint_at_height_passes() {
        // Heights 0, 5, 11, 49, 100 have no checkpoint -- any hash is fine.
        let arbitrary = Hash256([0xDE; 32]);
        for height in [0, 5, 11, 49, 100, u64::MAX] {
            assert!(
                check_checkpoint_with(TEST_CHECKPOINTS, height, &arbitrary).is_ok(),
                "height {height} should pass with no checkpoint"
            );
        }

        // Also verify against the real (empty) CHECKPOINTS constant.
        assert!(check_checkpoint(42, &arbitrary).is_ok());
    }

    // ------------------------------------------------------------------
    // last_checkpoint_height
    // ------------------------------------------------------------------

    #[test]
    fn last_checkpoint_height_empty() {
        // The production constant is empty, so last_checkpoint_height returns 0.
        assert_eq!(last_checkpoint_height(), 0);

        // Explicit empty list also returns 0.
        assert_eq!(last_checkpoint_height_with(&[]), 0);
    }

    #[test]
    fn last_checkpoint_height_with_entries() {
        assert_eq!(last_checkpoint_height_with(TEST_CHECKPOINTS), 50);

        let single: &[(u64, [u8; 32])] = &[(999, [0x01; 32])];
        assert_eq!(last_checkpoint_height_with(single), 999);
    }

    // ------------------------------------------------------------------
    // is_below_checkpoint
    // ------------------------------------------------------------------

    #[test]
    fn is_below_checkpoint_works() {
        // With TEST_CHECKPOINTS, last checkpoint is at height 50.
        assert!(is_below_checkpoint_with(TEST_CHECKPOINTS, 0));
        assert!(is_below_checkpoint_with(TEST_CHECKPOINTS, 10));
        assert!(is_below_checkpoint_with(TEST_CHECKPOINTS, 50));
        assert!(!is_below_checkpoint_with(TEST_CHECKPOINTS, 51));
        assert!(!is_below_checkpoint_with(TEST_CHECKPOINTS, 100));

        // With empty checkpoints (production), nothing is below a checkpoint.
        assert!(!is_below_checkpoint(0));
        assert!(!is_below_checkpoint(u64::MAX));
    }
}
