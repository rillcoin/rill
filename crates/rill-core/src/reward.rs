//! Reward schedule and halving logic.
//!
//! The mining reward follows a halving schedule: the base reward starts at
//! [`INITIAL_REWARD`](crate::constants::INITIAL_REWARD) (50 RILL) and halves
//! every [`HALVING_INTERVAL`](crate::constants::HALVING_INTERVAL) (210,000)
//! blocks.
//!
//! Epochs:
//! - Epoch 0 (heights 0–209,999): 50 RILL per block
//! - Epoch 1 (heights 210,000–419,999): 25 RILL per block
//! - …
//! - Epoch 32 (heights 6,720,000–6,929,999): 1 rill per block
//! - Epoch 33+: 0 (reward exhausted)
//!
//! The total mining supply across all epochs is slightly less than
//! [`MAX_SUPPLY`](crate::constants::MAX_SUPPLY) due to integer truncation
//! in the halving arithmetic.

use crate::constants::{HALVING_INTERVAL, INITIAL_REWARD};

/// Compute the base mining reward (in rills) for a given block height.
///
/// Follows the halving schedule: `INITIAL_REWARD >> (height / HALVING_INTERVAL)`.
/// Returns 0 once the truncated reward reaches zero (epoch ≥ 33 with current
/// constants) or the epoch exceeds 63 (shift-overflow guard).
///
/// Note: Height 0 is the genesis block whose coinbase pays `DEV_FUND_PREMINE`,
/// not the regular mining reward. This function returns the *schedule* reward
/// regardless — the genesis block is a special case handled by the genesis module.
pub fn block_reward(height: u64) -> u64 {
    epoch_reward(halving_epoch(height))
}

/// The mining reward (in rills) for a given halving epoch.
///
/// `epoch_reward(0) == INITIAL_REWARD`, `epoch_reward(1) == INITIAL_REWARD / 2`,
/// etc. Returns 0 for epoch ≥ 64 (shift guard) or when truncation yields zero.
pub fn epoch_reward(epoch: u64) -> u64 {
    if epoch >= 64 {
        return 0;
    }
    INITIAL_REWARD >> epoch
}

/// Which halving epoch a block height falls in.
///
/// Epoch 0 spans heights `[0, HALVING_INTERVAL)`,
/// epoch 1 spans `[HALVING_INTERVAL, 2 * HALVING_INTERVAL)`, etc.
pub fn halving_epoch(height: u64) -> u64 {
    height / HALVING_INTERVAL
}

/// The first block height of a given halving epoch.
///
/// VULN-06 fix: Uses saturating_mul to prevent overflow for large epoch values.
pub fn epoch_start_height(epoch: u64) -> u64 {
    epoch.saturating_mul(HALVING_INTERVAL)
}

/// The height at which the next halving occurs after `height`.
///
/// Returns `None` if the current reward is already zero (no future halvings).
pub fn next_halving_height(height: u64) -> Option<u64> {
    let epoch = halving_epoch(height);
    if epoch_reward(epoch) == 0 {
        return None;
    }
    Some(epoch_start_height(epoch + 1))
}

/// Number of blocks remaining until the next halving from `height`.
///
/// Returns `None` if the current reward is already zero.
pub fn blocks_until_halving(height: u64) -> Option<u64> {
    next_halving_height(height).map(|next| next - height)
}

/// Cumulative mining rewards from height 0 through `height` (inclusive).
///
/// Uses the epoch structure for O(epochs) computation rather than iterating
/// every block.
///
/// Note: this returns the *schedule* total. The actual genesis coinbase
/// pays [`DEV_FUND_PREMINE`](crate::genesis::DEV_FUND_PREMINE) rather than
/// the schedule reward at height 0.
pub fn cumulative_reward(height: u64) -> u64 {
    let final_epoch = halving_epoch(height);
    let mut total: u64 = 0;

    for epoch in 0..=final_epoch {
        let reward = epoch_reward(epoch);
        if reward == 0 {
            break;
        }
        let start = epoch_start_height(epoch);
        let end = if epoch == final_epoch {
            height
        } else {
            epoch_start_height(epoch + 1) - 1
        };
        let blocks = end - start + 1;
        // VULN-06 fix: Use saturating_add to prevent overflow in pathological cases.
        // For current constants this is safe, but defensive programming is important.
        total = total.saturating_add(reward.saturating_mul(blocks));
    }

    total
}

/// Total mining supply across all halving epochs.
///
/// Sum of `epoch_reward(e) * HALVING_INTERVAL` for all epochs with non-zero
/// reward. Due to integer truncation, this is slightly less than
/// `2 * INITIAL_REWARD * HALVING_INTERVAL` (= `MAX_SUPPLY`).
///
/// VULN-06 fix: Uses saturating arithmetic.
pub fn total_mining_supply() -> u64 {
    let mut total: u64 = 0;
    for epoch in 0..64u64 {
        let reward = epoch_reward(epoch);
        if reward == 0 {
            break;
        }
        total = total.saturating_add(reward.saturating_mul(HALVING_INTERVAL));
    }
    total
}

/// The last halving epoch with a non-zero mining reward.
///
/// Epochs after this one have zero reward.
pub fn last_reward_epoch() -> u64 {
    for epoch in (0..64u64).rev() {
        if epoch_reward(epoch) > 0 {
            return epoch;
        }
    }
    0
}

/// The last block height that receives a non-zero mining reward.
///
/// All heights after this receive zero reward.
pub fn last_reward_height() -> u64 {
    epoch_start_height(last_reward_epoch() + 1) - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{COIN, HALVING_INTERVAL, INITIAL_REWARD, MAX_SUPPLY};
    use crate::genesis::DEV_FUND_PREMINE;

    // ------------------------------------------------------------------
    // block_reward
    // ------------------------------------------------------------------

    #[test]
    fn reward_at_height_zero() {
        assert_eq!(block_reward(0), INITIAL_REWARD);
    }

    #[test]
    fn reward_at_end_of_first_epoch() {
        assert_eq!(block_reward(HALVING_INTERVAL - 1), INITIAL_REWARD);
    }

    #[test]
    fn reward_halves_at_interval() {
        assert_eq!(block_reward(HALVING_INTERVAL), INITIAL_REWARD / 2);
    }

    #[test]
    fn reward_second_halving() {
        assert_eq!(block_reward(2 * HALVING_INTERVAL), INITIAL_REWARD / 4);
    }

    #[test]
    fn reward_third_halving() {
        assert_eq!(block_reward(3 * HALVING_INTERVAL), INITIAL_REWARD / 8);
    }

    #[test]
    fn reward_epoch_32_is_one_rill() {
        // INITIAL_REWARD (5_000_000_000) >> 32 = 1
        assert_eq!(block_reward(32 * HALVING_INTERVAL), 1);
    }

    #[test]
    fn reward_epoch_33_is_zero() {
        assert_eq!(block_reward(33 * HALVING_INTERVAL), 0);
    }

    #[test]
    fn reward_epoch_64_is_zero() {
        assert_eq!(block_reward(64 * HALVING_INTERVAL), 0);
    }

    #[test]
    fn reward_very_large_height() {
        assert_eq!(block_reward(u64::MAX), 0);
    }

    #[test]
    fn reward_matches_mock_block_producer_logic() {
        // Consistency with MockBlockProducer in traits.rs
        for h in [
            0,
            1,
            100,
            HALVING_INTERVAL - 1,
            HALVING_INTERVAL,
            2 * HALVING_INTERVAL,
            10 * HALVING_INTERVAL,
            33 * HALVING_INTERVAL,
        ] {
            let halvings = h / HALVING_INTERVAL;
            let expected = if halvings >= 64 {
                0
            } else {
                INITIAL_REWARD >> halvings
            };
            assert_eq!(block_reward(h), expected, "mismatch at height {h}");
        }
    }

    // ------------------------------------------------------------------
    // epoch_reward
    // ------------------------------------------------------------------

    #[test]
    fn epoch_reward_zero() {
        assert_eq!(epoch_reward(0), INITIAL_REWARD);
    }

    #[test]
    fn epoch_reward_one() {
        assert_eq!(epoch_reward(1), INITIAL_REWARD / 2);
    }

    #[test]
    fn epoch_reward_strictly_decreasing() {
        let mut prev = epoch_reward(0);
        for e in 1..=32u64 {
            let r = epoch_reward(e);
            assert!(r < prev, "epoch {e} not less than epoch {}", e - 1);
            prev = r;
        }
    }

    #[test]
    fn epoch_reward_32_is_one() {
        assert_eq!(epoch_reward(32), 1);
    }

    #[test]
    fn epoch_reward_33_is_zero() {
        assert_eq!(epoch_reward(33), 0);
    }

    // ------------------------------------------------------------------
    // halving_epoch
    // ------------------------------------------------------------------

    #[test]
    fn epoch_of_height_zero() {
        assert_eq!(halving_epoch(0), 0);
    }

    #[test]
    fn epoch_of_last_block_in_first_epoch() {
        assert_eq!(halving_epoch(HALVING_INTERVAL - 1), 0);
    }

    #[test]
    fn epoch_of_first_block_in_second_epoch() {
        assert_eq!(halving_epoch(HALVING_INTERVAL), 1);
    }

    #[test]
    fn epoch_of_mid_second_epoch() {
        assert_eq!(halving_epoch(HALVING_INTERVAL + 100_000), 1);
    }

    // ------------------------------------------------------------------
    // epoch_start_height
    // ------------------------------------------------------------------

    #[test]
    fn epoch_start_zero() {
        assert_eq!(epoch_start_height(0), 0);
    }

    #[test]
    fn epoch_start_one() {
        assert_eq!(epoch_start_height(1), HALVING_INTERVAL);
    }

    #[test]
    fn epoch_start_ten() {
        assert_eq!(epoch_start_height(10), 10 * HALVING_INTERVAL);
    }

    // ------------------------------------------------------------------
    // next_halving_height
    // ------------------------------------------------------------------

    #[test]
    fn next_halving_from_zero() {
        assert_eq!(next_halving_height(0), Some(HALVING_INTERVAL));
    }

    #[test]
    fn next_halving_from_mid_epoch() {
        assert_eq!(next_halving_height(100_000), Some(HALVING_INTERVAL));
    }

    #[test]
    fn next_halving_from_boundary() {
        assert_eq!(
            next_halving_height(HALVING_INTERVAL),
            Some(2 * HALVING_INTERVAL)
        );
    }

    #[test]
    fn next_halving_from_just_before_boundary() {
        assert_eq!(
            next_halving_height(HALVING_INTERVAL - 1),
            Some(HALVING_INTERVAL)
        );
    }

    #[test]
    fn next_halving_none_when_reward_zero() {
        assert_eq!(next_halving_height(33 * HALVING_INTERVAL), None);
    }

    // ------------------------------------------------------------------
    // blocks_until_halving
    // ------------------------------------------------------------------

    #[test]
    fn blocks_until_from_zero() {
        assert_eq!(blocks_until_halving(0), Some(HALVING_INTERVAL));
    }

    #[test]
    fn blocks_until_from_one() {
        assert_eq!(blocks_until_halving(1), Some(HALVING_INTERVAL - 1));
    }

    #[test]
    fn blocks_until_from_boundary() {
        assert_eq!(blocks_until_halving(HALVING_INTERVAL), Some(HALVING_INTERVAL));
    }

    #[test]
    fn blocks_until_just_before_boundary() {
        assert_eq!(blocks_until_halving(HALVING_INTERVAL - 1), Some(1));
    }

    #[test]
    fn blocks_until_none_past_last() {
        assert_eq!(blocks_until_halving(33 * HALVING_INTERVAL), None);
    }

    // ------------------------------------------------------------------
    // cumulative_reward
    // ------------------------------------------------------------------

    #[test]
    fn cumulative_at_zero() {
        assert_eq!(cumulative_reward(0), INITIAL_REWARD);
    }

    #[test]
    fn cumulative_at_one() {
        assert_eq!(cumulative_reward(1), 2 * INITIAL_REWARD);
    }

    #[test]
    fn cumulative_end_of_epoch_zero() {
        assert_eq!(
            cumulative_reward(HALVING_INTERVAL - 1),
            INITIAL_REWARD * HALVING_INTERVAL
        );
    }

    #[test]
    fn cumulative_start_of_epoch_one() {
        let epoch0 = INITIAL_REWARD * HALVING_INTERVAL;
        let epoch1_first = INITIAL_REWARD / 2;
        assert_eq!(cumulative_reward(HALVING_INTERVAL), epoch0 + epoch1_first);
    }

    #[test]
    fn cumulative_end_of_epoch_one() {
        let epoch0 = INITIAL_REWARD * HALVING_INTERVAL;
        let epoch1 = (INITIAL_REWARD / 2) * HALVING_INTERVAL;
        assert_eq!(
            cumulative_reward(2 * HALVING_INTERVAL - 1),
            epoch0 + epoch1
        );
    }

    #[test]
    fn cumulative_at_last_reward_equals_total() {
        assert_eq!(cumulative_reward(last_reward_height()), total_mining_supply());
    }

    #[test]
    fn cumulative_past_all_rewards_equals_total() {
        assert_eq!(
            cumulative_reward(last_reward_height() + 1_000_000),
            total_mining_supply()
        );
    }

    #[test]
    fn cumulative_is_monotonically_nondecreasing() {
        let heights = [
            0,
            1,
            100,
            HALVING_INTERVAL - 1,
            HALVING_INTERVAL,
            2 * HALVING_INTERVAL,
            33 * HALVING_INTERVAL,
            34 * HALVING_INTERVAL,
        ];
        let mut prev = 0u64;
        for h in heights {
            let c = cumulative_reward(h);
            assert!(c >= prev, "cumulative not monotonic at height {h}");
            prev = c;
        }
    }

    // ------------------------------------------------------------------
    // total_mining_supply
    // ------------------------------------------------------------------

    #[test]
    fn total_mining_supply_positive() {
        assert!(total_mining_supply() > 0);
    }

    #[test]
    fn total_mining_supply_less_than_max() {
        assert!(total_mining_supply() < MAX_SUPPLY);
    }

    #[test]
    fn total_mining_supply_close_to_max() {
        // Difference due to integer truncation is less than 1 RILL.
        let diff = MAX_SUPPLY - total_mining_supply();
        assert!(
            diff < COIN,
            "mining supply too far from MAX_SUPPLY: diff = {diff} rills"
        );
    }

    #[test]
    fn total_mining_supply_deterministic() {
        assert_eq!(total_mining_supply(), total_mining_supply());
    }

    #[test]
    fn total_mining_supply_epoch_by_epoch() {
        let mut manual: u64 = 0;
        for epoch in 0..64u64 {
            let r = epoch_reward(epoch);
            if r == 0 {
                break;
            }
            manual += r * HALVING_INTERVAL;
        }
        assert_eq!(total_mining_supply(), manual);
    }

    // ------------------------------------------------------------------
    // last_reward_epoch / last_reward_height
    // ------------------------------------------------------------------

    #[test]
    fn last_epoch_is_32() {
        assert_eq!(last_reward_epoch(), 32);
    }

    #[test]
    fn last_epoch_reward_is_one() {
        assert_eq!(epoch_reward(last_reward_epoch()), 1);
    }

    #[test]
    fn epoch_after_last_is_zero() {
        assert_eq!(epoch_reward(last_reward_epoch() + 1), 0);
    }

    #[test]
    fn last_height_value() {
        assert_eq!(last_reward_height(), 33 * HALVING_INTERVAL - 1);
    }

    #[test]
    fn last_height_has_nonzero_reward() {
        assert!(block_reward(last_reward_height()) > 0);
    }

    #[test]
    fn height_after_last_is_zero_reward() {
        assert_eq!(block_reward(last_reward_height() + 1), 0);
    }

    // ------------------------------------------------------------------
    // Consistency checks
    // ------------------------------------------------------------------

    #[test]
    fn block_reward_equals_epoch_reward() {
        for h in [
            0,
            1,
            HALVING_INTERVAL - 1,
            HALVING_INTERVAL,
            5 * HALVING_INTERVAL + 42,
            32 * HALVING_INTERVAL,
            33 * HALVING_INTERVAL,
        ] {
            assert_eq!(
                block_reward(h),
                epoch_reward(halving_epoch(h)),
                "inconsistency at height {h}"
            );
        }
    }

    #[test]
    fn initial_reward_is_50_rill() {
        assert_eq!(INITIAL_REWARD, 50 * COIN);
    }

    #[test]
    fn halving_interval_is_210k() {
        assert_eq!(HALVING_INTERVAL, 210_000);
    }

    #[test]
    fn total_supply_with_premine() {
        let mining = total_mining_supply();
        // Mining alone is approximately MAX_SUPPLY (Bitcoin-like schedule).
        // With the 5% dev fund premine, the effective total exceeds MAX_SUPPLY.
        // MAX_SUPPLY represents the mining-supply cap; the premine is additional.
        assert!(mining < MAX_SUPPLY);
        assert!(mining + DEV_FUND_PREMINE > MAX_SUPPLY);
    }
}
