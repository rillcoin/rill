//! Difficulty adjustment algorithm.
//!
//! Adjusts the proof-of-work difficulty every block using a rolling window
//! of the most recent [`DIFFICULTY_WINDOW`](crate::constants::DIFFICULTY_WINDOW)
//! block intervals.
//!
//! The algorithm compares actual elapsed time against expected time and adjusts
//! the difficulty target proportionally. Per-window adjustments are clamped to
//! [`MAX_ADJUSTMENT_FACTOR`] (4×) to prevent wild swings from timestamp
//! manipulation or sudden hashrate changes.
//!
//! # Difficulty target semantics
//!
//! The `difficulty_target` field in [`BlockHeader`](crate::types::BlockHeader)
//! is a u64 where **higher = easier**. The PoW check interprets the first 8
//! bytes of the header hash as a little-endian u64 and requires it to be
//! ≤ `difficulty_target`. A target of [`MAX_TARGET`] (`u64::MAX`) accepts any
//! hash (genesis difficulty).
//!
//! # Window sizing
//!
//! At steady state the window contains `DIFFICULTY_WINDOW` intervals
//! (`DIFFICULTY_WINDOW + 1` timestamps). During the early chain (height <
//! `DIFFICULTY_WINDOW + 1`), all available blocks are used, giving a growing
//! window that smoothly transitions to the full size.

use crate::constants::{BLOCK_TIME_SECS, DIFFICULTY_WINDOW};

/// Maximum difficulty adjustment factor per window.
///
/// The target cannot change by more than this factor in a single adjustment.
/// Prevents extreme swings from timestamp manipulation or hashrate spikes.
pub const MAX_ADJUSTMENT_FACTOR: u64 = 4;

/// Minimum difficulty target (hardest possible difficulty).
///
/// A target of 1 requires the LE u64 of the first 8 hash bytes to be ≤ 1,
/// which is astronomically difficult.
pub const MIN_TARGET: u64 = 1;

/// Maximum (easiest) difficulty target. Used for the genesis block.
///
/// A target of `u64::MAX` accepts any hash.
pub const MAX_TARGET: u64 = u64::MAX;

/// Compute the next difficulty target from a window of recent timestamps.
///
/// `timestamps` must be ordered oldest to newest. `current_target` is the
/// difficulty target of the most recent block in the window.
///
/// Returns `current_target` unchanged if fewer than 2 timestamps are provided
/// (not enough data for adjustment). Otherwise computes:
///
/// 1. `actual_time = timestamps.last() - timestamps.first()`
/// 2. `expected_time = (timestamps.len() - 1) * BLOCK_TIME_SECS`
/// 3. Clamp actual time to `[expected / 4, expected * 4]`
/// 4. `new_target = current_target * clamped_actual / expected`
/// 5. Clamp result to `[MIN_TARGET, MAX_TARGET]`
pub fn next_target(timestamps: &[u64], current_target: u64) -> u64 {
    if timestamps.len() < 2 {
        return current_target;
    }

    let actual_time = timestamps[timestamps.len() - 1].saturating_sub(timestamps[0]);
    let intervals = (timestamps.len() - 1) as u64;
    let expected_time = intervals * BLOCK_TIME_SECS;

    // Guard: zero expected time can only happen if BLOCK_TIME_SECS is 0 (impossible
    // with current constants) or intervals is 0 (already guarded by len < 2 check).
    if expected_time == 0 {
        return current_target;
    }

    // Clamp actual time to prevent extreme adjustments (max 4× change).
    let min_time = expected_time / MAX_ADJUSTMENT_FACTOR;
    let max_time = expected_time.saturating_mul(MAX_ADJUSTMENT_FACTOR);
    let clamped = actual_time.max(min_time).min(max_time);

    // new_target = current_target * clamped / expected_time
    // Use u128 to avoid overflow. Max product:
    //   u64::MAX * (60 * 60 * 4) ≈ 2.6e23, well within u128 range.
    let result =
        (current_target as u128).saturating_mul(clamped as u128) / (expected_time as u128);

    // Clamp to valid u64 range.
    (result.min(MAX_TARGET as u128) as u64).max(MIN_TARGET)
}

/// Compute the expected difficulty target for the block at `height`.
///
/// `parent_target` is the difficulty target of the block at `height - 1`.
/// `get_timestamp` returns the timestamp of the block at a given height
/// (must be valid for all heights in the selected window).
///
/// Returns [`MAX_TARGET`] for heights 0 and 1 (genesis and first mined block,
/// before sufficient data exists). For height ≥ 2, computes the adjustment
/// using all available timestamps (up to `DIFFICULTY_WINDOW + 1`).
pub fn target_for_height(
    height: u64,
    parent_target: u64,
    get_timestamp: impl Fn(u64) -> u64,
) -> u64 {
    target_for_height_with_initial(height, parent_target, get_timestamp, MAX_TARGET)
}

/// Like [`target_for_height`] but allows overriding the initial (genesis) target.
///
/// Use [`TESTNET_INITIAL_TARGET`](crate::constants::TESTNET_INITIAL_TARGET) for
/// testnet deployments to prevent instant-mining of early blocks.
pub fn target_for_height_with_initial(
    height: u64,
    parent_target: u64,
    get_timestamp: impl Fn(u64) -> u64,
    initial_target: u64,
) -> u64 {
    if height <= 1 {
        return initial_target;
    }

    // We want DIFFICULTY_WINDOW intervals = DIFFICULTY_WINDOW + 1 timestamps.
    // Early chain: use all available blocks (minimum 2 timestamps = 1 interval).
    let num_timestamps = height.min(DIFFICULTY_WINDOW + 1);
    let start = height - num_timestamps;

    let timestamps: Vec<u64> = (start..height).map(&get_timestamp).collect();

    next_target(&timestamps, parent_target)
}

/// Expected total time for the full difficulty window (in seconds).
///
/// Equals `DIFFICULTY_WINDOW * BLOCK_TIME_SECS`.
pub const fn expected_window_time() -> u64 {
    DIFFICULTY_WINDOW * BLOCK_TIME_SECS
}

/// The number of timestamp entries used in a full difficulty window.
///
/// Equals `DIFFICULTY_WINDOW + 1` (one more than the number of intervals).
pub const fn full_window_size() -> u64 {
    DIFFICULTY_WINDOW + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{BLOCK_TIME_SECS, DIFFICULTY_WINDOW};

    // ------------------------------------------------------------------
    // Helper: build evenly-spaced timestamps
    // ------------------------------------------------------------------

    /// Generate `count` timestamps starting at `start`, spaced by `interval` seconds.
    fn spaced_timestamps(start: u64, count: usize, interval: u64) -> Vec<u64> {
        (0..count).map(|i| start + i as u64 * interval).collect()
    }

    // ------------------------------------------------------------------
    // next_target — edge cases
    // ------------------------------------------------------------------

    #[test]
    fn next_target_empty_returns_current() {
        assert_eq!(next_target(&[], 1000), 1000);
    }

    #[test]
    fn next_target_single_returns_current() {
        assert_eq!(next_target(&[100], 1000), 1000);
    }

    // ------------------------------------------------------------------
    // next_target — on-target timing
    // ------------------------------------------------------------------

    #[test]
    fn on_target_returns_same_difficulty() {
        // 61 timestamps with exactly 60-second spacing → 60 intervals, 3600s actual = expected
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS);
        let target = 1_000_000_u64;
        assert_eq!(next_target(&ts, target), target);
    }

    #[test]
    fn on_target_small_window() {
        // 2 timestamps, 1 interval at exact pace
        let ts = vec![100, 100 + BLOCK_TIME_SECS];
        let target = 500_000_u64;
        assert_eq!(next_target(&ts, target), target);
    }

    // ------------------------------------------------------------------
    // next_target — slow blocks → easier (higher target)
    // ------------------------------------------------------------------

    #[test]
    fn slow_blocks_increase_target() {
        // Blocks twice as slow as expected
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS * 2);
        let target = 1_000_000_u64;
        let new = next_target(&ts, target);
        assert_eq!(new, target * 2);
    }

    #[test]
    fn slow_blocks_three_times() {
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS * 3);
        let target = 1_000_000_u64;
        let new = next_target(&ts, target);
        assert_eq!(new, target * 3);
    }

    // ------------------------------------------------------------------
    // next_target — fast blocks → harder (lower target)
    // ------------------------------------------------------------------

    #[test]
    fn fast_blocks_decrease_target() {
        // Blocks twice as fast
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS / 2);
        let target = 1_000_000_u64;
        let new = next_target(&ts, target);
        assert_eq!(new, target / 2);
    }

    #[test]
    fn fast_blocks_three_times() {
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS / 3);
        let target = 1_200_000_u64; // divisible by 3
        let new = next_target(&ts, target);
        assert_eq!(new, target / 3);
    }

    // ------------------------------------------------------------------
    // next_target — clamping
    // ------------------------------------------------------------------

    #[test]
    fn clamps_max_increase_to_4x() {
        // Blocks 10x slower than expected → clamped to 4x increase
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS * 10);
        let target = 1_000_000_u64;
        let new = next_target(&ts, target);
        assert_eq!(new, target * MAX_ADJUSTMENT_FACTOR);
    }

    #[test]
    fn clamps_max_decrease_to_quarter() {
        // All same timestamp (instant blocks) → clamped to 1/4 decrease
        let ts = vec![1_000_000; 61];
        let target = 1_000_000_u64;
        let new = next_target(&ts, target);
        assert_eq!(new, target / MAX_ADJUSTMENT_FACTOR);
    }

    #[test]
    fn clamp_at_exact_4x_boundary() {
        // Exactly 4x slower → not clamped, equals 4x
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS * 4);
        let target = 1_000_000_u64;
        let new = next_target(&ts, target);
        assert_eq!(new, target * 4);
    }

    #[test]
    fn clamp_at_exact_quarter_boundary() {
        // Exactly 4x faster → not clamped, equals 1/4
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS / 4);
        let target = 1_000_000_u64;
        let new = next_target(&ts, target);
        assert_eq!(new, target / 4);
    }

    // ------------------------------------------------------------------
    // next_target — bounds
    // ------------------------------------------------------------------

    #[test]
    fn result_never_below_min_target() {
        // Very fast blocks with very low current target
        let ts = vec![1_000_000; 61]; // instant
        let new = next_target(&ts, 1); // 1 / 4 = 0, clamped to MIN_TARGET
        assert_eq!(new, MIN_TARGET);
    }

    #[test]
    fn result_never_above_max_target() {
        // Very slow blocks with very high current target
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS * 10);
        let new = next_target(&ts, u64::MAX);
        assert_eq!(new, MAX_TARGET);
    }

    #[test]
    fn max_target_with_on_target_stays_max() {
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS);
        assert_eq!(next_target(&ts, MAX_TARGET), MAX_TARGET);
    }

    #[test]
    fn min_target_with_on_target_stays_min() {
        let ts = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS);
        assert_eq!(next_target(&ts, MIN_TARGET), MIN_TARGET);
    }

    // ------------------------------------------------------------------
    // next_target — partial windows
    // ------------------------------------------------------------------

    #[test]
    fn two_timestamps_on_target() {
        let ts = vec![1000, 1060]; // 1 interval, 60s
        assert_eq!(next_target(&ts, 500_000), 500_000);
    }

    #[test]
    fn two_timestamps_slow() {
        let ts = vec![1000, 1120]; // 1 interval, 120s (2x slow)
        assert_eq!(next_target(&ts, 500_000), 1_000_000);
    }

    #[test]
    fn two_timestamps_fast() {
        let ts = vec![1000, 1030]; // 1 interval, 30s (2x fast)
        assert_eq!(next_target(&ts, 500_000), 250_000);
    }

    #[test]
    fn ten_timestamps_on_target() {
        // 10 timestamps, 9 intervals, each 60s → 540s actual = expected
        let ts = spaced_timestamps(1000, 10, BLOCK_TIME_SECS);
        assert_eq!(next_target(&ts, 1_000_000), 1_000_000);
    }

    // ------------------------------------------------------------------
    // next_target — proportional adjustment
    // ------------------------------------------------------------------

    #[test]
    fn proportional_increase_half_speed() {
        // 50% slower: each interval 90s instead of 60s
        let ts = spaced_timestamps(1_000_000, 61, 90);
        let target = 2_000_000_u64;
        let new = next_target(&ts, target);
        // actual = 60 * 90 = 5400, expected = 60 * 60 = 3600
        // new = 2_000_000 * 5400 / 3600 = 3_000_000
        assert_eq!(new, 3_000_000);
    }

    #[test]
    fn proportional_decrease_twice_speed() {
        // 50% faster: each interval 30s instead of 60s
        let ts = spaced_timestamps(1_000_000, 61, 30);
        let target = 2_000_000_u64;
        let new = next_target(&ts, target);
        // actual = 60 * 30 = 1800, expected = 60 * 60 = 3600
        // new = 2_000_000 * 1800 / 3600 = 1_000_000
        assert_eq!(new, 1_000_000);
    }

    // ------------------------------------------------------------------
    // next_target — non-uniform spacing
    // ------------------------------------------------------------------

    #[test]
    fn non_uniform_spacing_uses_total_time() {
        // Only the total span matters, not individual intervals
        // Total span = 3600 (same as expected for 60 intervals)
        let mut ts: Vec<u64> = Vec::new();
        ts.push(1_000_000);
        // 59 blocks at 1s intervals
        for i in 1..60 {
            ts.push(1_000_000 + i);
        }
        // Last block jumps to make total span exactly 3600
        ts.push(1_000_000 + 3600);
        assert_eq!(ts.len(), 61);

        let target = 1_000_000_u64;
        assert_eq!(next_target(&ts, target), target);
    }

    // ------------------------------------------------------------------
    // target_for_height
    // ------------------------------------------------------------------

    #[test]
    fn height_zero_returns_max() {
        assert_eq!(target_for_height(0, 1000, |_| 0), MAX_TARGET);
    }

    #[test]
    fn height_one_returns_max() {
        assert_eq!(target_for_height(1, 1000, |_| 0), MAX_TARGET);
    }

    #[test]
    fn height_two_uses_two_timestamps() {
        // Heights 0 and 1, 1 interval
        let target = 500_000_u64;
        let new = target_for_height(2, target, |h| match h {
            0 => 1000,
            1 => 1060, // exactly on target
            _ => panic!("unexpected height {h}"),
        });
        assert_eq!(new, target);
    }

    #[test]
    fn height_two_slow_blocks() {
        let target = 500_000_u64;
        let new = target_for_height(2, target, |h| match h {
            0 => 1000,
            1 => 1120, // 2x slow
            _ => panic!("unexpected height {h}"),
        });
        assert_eq!(new, 1_000_000);
    }

    #[test]
    fn height_grows_window_progressively() {
        // At height 5, window = min(5, 61) = 5 timestamps → 4 intervals
        let target = 1_000_000_u64;
        let new = target_for_height(5, target, |h| {
            assert!(h < 5, "should not request height >= 5");
            // On-target: each block 60s apart, starting at t=1000
            1000 + h * BLOCK_TIME_SECS
        });
        assert_eq!(new, target);
    }

    #[test]
    fn height_uses_full_window_at_steady_state() {
        // At height 100, window = min(100, 61) = 61 timestamps from heights [39..100)
        let target = 1_000_000_u64;
        let new = target_for_height(100, target, |h| {
            assert!(h >= 39 && h < 100, "height {h} out of expected window");
            1000 + h * BLOCK_TIME_SECS
        });
        assert_eq!(new, target);
    }

    #[test]
    fn height_exactly_at_window_boundary() {
        // At height 61, window = min(61, 61) = 61 timestamps from heights [0..61)
        let target = 1_000_000_u64;
        let new = target_for_height(61, target, |h| {
            assert!(h < 61, "height {h} out of expected window");
            1000 + h * BLOCK_TIME_SECS
        });
        assert_eq!(new, target);
    }

    #[test]
    fn height_just_past_window_boundary() {
        // At height 62, window = min(62, 61) = 61 timestamps from heights [1..62)
        let target = 1_000_000_u64;
        let new = target_for_height(62, target, |h| {
            assert!(h >= 1 && h < 62, "height {h} out of expected window");
            1000 + h * BLOCK_TIME_SECS
        });
        assert_eq!(new, target);
    }

    #[test]
    fn target_for_height_slow_steady_state() {
        // Blocks are 2x slow in the full window
        let target = 1_000_000_u64;
        let new = target_for_height(100, target, |h| {
            1000 + h * BLOCK_TIME_SECS * 2
        });
        assert_eq!(new, target * 2);
    }

    #[test]
    fn target_for_height_fast_steady_state() {
        // Blocks are 2x fast in the full window
        let target = 1_000_000_u64;
        let new = target_for_height(100, target, |h| {
            1000 + h * BLOCK_TIME_SECS / 2
        });
        assert_eq!(new, target / 2);
    }

    // ------------------------------------------------------------------
    // Constants and helpers
    // ------------------------------------------------------------------

    #[test]
    fn expected_window_time_value() {
        assert_eq!(expected_window_time(), DIFFICULTY_WINDOW * BLOCK_TIME_SECS);
        assert_eq!(expected_window_time(), 3600);
    }

    #[test]
    fn full_window_size_value() {
        assert_eq!(full_window_size(), DIFFICULTY_WINDOW + 1);
        assert_eq!(full_window_size(), 61);
    }

    #[test]
    fn max_adjustment_factor_value() {
        assert_eq!(MAX_ADJUSTMENT_FACTOR, 4);
    }

    #[test]
    fn min_target_value() {
        assert_eq!(MIN_TARGET, 1);
    }

    #[test]
    fn max_target_value() {
        assert_eq!(MAX_TARGET, u64::MAX);
    }

    // ------------------------------------------------------------------
    // Convergence / stability
    // ------------------------------------------------------------------

    #[test]
    fn converges_to_target_block_time() {
        // Simulate difficulty adjustment over multiple windows.
        // Start with MAX_TARGET and blocks at 30s (2x fast).
        // After one adjustment, target should halve.
        let mut target = MAX_TARGET / 2; // start at half-max to avoid overflow
        let ts_fast = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS / 2);
        target = next_target(&ts_fast, target);
        assert_eq!(target, MAX_TARGET / 4);

        // Now simulate on-target blocks — difficulty should stay
        let ts_on = spaced_timestamps(2_000_000, 61, BLOCK_TIME_SECS);
        let stable = next_target(&ts_on, target);
        assert_eq!(stable, target);
    }

    #[test]
    fn repeated_on_target_is_stable() {
        let mut target = 5_000_000_u64;
        for round in 0..10 {
            let ts = spaced_timestamps(1_000_000 + round * 10_000, 61, BLOCK_TIME_SECS);
            target = next_target(&ts, target);
        }
        assert_eq!(target, 5_000_000);
    }

    #[test]
    fn oscillation_dampened_by_clamp() {
        // Even with wild swings, clamp limits each adjustment to 4x
        let target = 1_000_000_u64;

        // Extremely fast window (all same timestamp)
        let ts_instant = vec![1_000_000; 61];
        let after_fast = next_target(&ts_instant, target);
        assert_eq!(after_fast, target / 4);

        // Then extremely slow window
        let ts_slow = spaced_timestamps(2_000_000, 61, BLOCK_TIME_SECS * 100);
        let after_slow = next_target(&ts_slow, after_fast);
        assert_eq!(after_slow, after_fast * 4); // = target (back to original)
    }

    // ------------------------------------------------------------------
    // Integer precision
    // ------------------------------------------------------------------

    #[test]
    fn u128_intermediate_handles_large_target() {
        // MAX_TARGET * 4 would overflow u64 but u128 handles it
        let ts_slow = spaced_timestamps(1_000_000, 61, BLOCK_TIME_SECS * 4);
        let new = next_target(&ts_slow, MAX_TARGET);
        // MAX_TARGET * 4 / 1 overflows u64 but clamped to MAX_TARGET
        assert_eq!(new, MAX_TARGET);
    }

    #[test]
    fn small_target_rounding() {
        // Target = 3, blocks 2x slow → 3 * 2 = 6
        let ts = spaced_timestamps(1000, 61, BLOCK_TIME_SECS * 2);
        assert_eq!(next_target(&ts, 3), 6);
    }

    #[test]
    fn small_target_truncation() {
        // Target = 5, blocks 2x fast → 5 / 2 = 2 (integer truncation)
        let ts = spaced_timestamps(1000, 61, BLOCK_TIME_SECS / 2);
        assert_eq!(next_target(&ts, 5), 2);
    }

    #[test]
    fn target_one_fast_blocks_stays_at_min() {
        // Target = 1, fast blocks → 1 / 4 = 0, clamped to MIN_TARGET = 1
        let ts = vec![1_000_000; 61];
        assert_eq!(next_target(&ts, 1), MIN_TARGET);
    }
}
