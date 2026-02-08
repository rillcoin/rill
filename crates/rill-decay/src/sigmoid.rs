//! Fixed-point sigmoid lookup table for decay rate computation.
//!
//! All computation uses integer arithmetic only. The sigmoid function is
//! evaluated via a precomputed lookup table with linear interpolation.
//!
//! The table covers `sigmoid(x)` for `x = 0.0, 0.5, 1.0, ..., 8.0` (17 entries).
//! For negative inputs, callers should use symmetry: `sigmoid(-x) = 1 - sigmoid(x)`.
//! Beyond `x = 8`, the sigmoid saturates at the table maximum (~0.9997).

use rill_core::constants::CONCENTRATION_PRECISION;

/// Precision of sigmoid output values (parts-per-billion).
pub const SIGMOID_PRECISION: u64 = 1_000_000_000;

/// Step size between table entries in scaled input units.
///
/// Each step represents 0.5 in the real sigmoid input.
/// Since the input is scaled by [`CONCENTRATION_PRECISION`] (10^9),
/// a step of 0.5 = 500_000_000 in scaled units.
const TABLE_STEP: u64 = CONCENTRATION_PRECISION / 2;

/// Precomputed `sigmoid(x) * SIGMOID_PRECISION` for `x = 0.0, 0.5, 1.0, ..., 8.0`.
///
/// 17 entries covering the positive half of the sigmoid curve.
/// Values computed from `sigmoid(x) = 1 / (1 + e^(-x))` and rounded to nearest integer.
const SIGMOID_TABLE: [u64; 17] = [
    500_000_000, // sigmoid(0.0) = 0.5000000000
    622_459_331, // sigmoid(0.5) = 0.6224593312
    731_058_579, // sigmoid(1.0) = 0.7310585786
    817_574_476, // sigmoid(1.5) = 0.8175744762
    880_797_078, // sigmoid(2.0) = 0.8807970780
    924_141_820, // sigmoid(2.5) = 0.9241418200
    952_574_127, // sigmoid(3.0) = 0.9525741268
    970_687_769, // sigmoid(3.5) = 0.9706877692
    982_013_790, // sigmoid(4.0) = 0.9820137900
    989_013_057, // sigmoid(4.5) = 0.9890130574
    993_307_149, // sigmoid(5.0) = 0.9933071491
    995_929_862, // sigmoid(5.5) = 0.9959298623
    997_527_377, // sigmoid(6.0) = 0.9975273768
    998_498_883, // sigmoid(6.5) = 0.9984988832
    999_088_949, // sigmoid(7.0) = 0.9990889488
    999_447_221, // sigmoid(7.5) = 0.9994472213
    999_664_650, // sigmoid(8.0) = 0.9996646499
];

/// Compute `sigmoid(x) * SIGMOID_PRECISION` using the lookup table with linear interpolation.
///
/// Input `x_scaled` represents the sigmoid argument multiplied by
/// [`CONCENTRATION_PRECISION`]. For example, `x_scaled = 1_000_000_000`
/// corresponds to `sigmoid(1.0)`.
///
/// Only handles non-negative inputs. For negative inputs, callers
/// should use the symmetry property: `sigmoid(-x) = SIGMOID_PRECISION - sigmoid(x)`.
///
/// Returns a value in `[SIGMOID_TABLE[0], SIGMOID_TABLE[last]]`.
pub fn sigmoid_positive(x_scaled: u128) -> u64 {
    let step = TABLE_STEP as u128;
    let index = (x_scaled / step) as usize;

    if index >= SIGMOID_TABLE.len() - 1 {
        return SIGMOID_TABLE[SIGMOID_TABLE.len() - 1];
    }

    let frac = (x_scaled % step) as u64;
    let lo = SIGMOID_TABLE[index];
    let hi = SIGMOID_TABLE[index + 1];
    let diff = hi - lo;

    // Linear interpolation: lo + diff * frac / TABLE_STEP
    // Max diff ≈ 122M, max frac ≈ 500M, product ≈ 6.1e16 — fits u128.
    lo + (diff as u128 * frac as u128 / step) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn sigmoid_at_zero() {
        assert_eq!(sigmoid_positive(0), 500_000_000);
    }

    #[test]
    fn sigmoid_at_half() {
        // x=0.5 → x_scaled = 500_000_000
        assert_eq!(sigmoid_positive(500_000_000), 622_459_331);
    }

    #[test]
    fn sigmoid_at_one() {
        // x=1.0 → x_scaled = 1_000_000_000
        assert_eq!(sigmoid_positive(1_000_000_000), 731_058_579);
    }

    #[test]
    fn sigmoid_at_two() {
        // x=2.0 → x_scaled = 2_000_000_000
        assert_eq!(sigmoid_positive(2_000_000_000), 880_797_078);
    }

    #[test]
    fn sigmoid_at_table_max() {
        // x=8.0 → x_scaled = 8_000_000_000
        assert_eq!(sigmoid_positive(8_000_000_000), 999_664_650);
    }

    #[test]
    fn sigmoid_beyond_table_saturates() {
        assert_eq!(sigmoid_positive(100_000_000_000), 999_664_650);
        assert_eq!(sigmoid_positive(u128::MAX / 2), 999_664_650);
    }

    #[test]
    fn sigmoid_monotonically_increasing_at_steps() {
        for i in 1..SIGMOID_TABLE.len() {
            assert!(
                SIGMOID_TABLE[i] > SIGMOID_TABLE[i - 1],
                "table not monotonic at index {i}"
            );
        }
    }

    #[test]
    fn sigmoid_interpolation_midpoint() {
        // Midpoint between table[0] (x=0) and table[1] (x=0.5)
        // x=0.25 → x_scaled = 250_000_000
        let val = sigmoid_positive(250_000_000);
        let expected = SIGMOID_TABLE[0] + (SIGMOID_TABLE[1] - SIGMOID_TABLE[0]) / 2;
        assert_eq!(val, expected);
    }

    #[test]
    fn sigmoid_interpolation_quarter() {
        // Quarter point between table[0] and table[1]
        // x=0.125 → x_scaled = 125_000_000
        let val = sigmoid_positive(125_000_000);
        let expected = SIGMOID_TABLE[0] + (SIGMOID_TABLE[1] - SIGMOID_TABLE[0]) / 4;
        assert_eq!(val, expected);
    }

    #[test]
    fn sigmoid_all_table_entries_exact() {
        for (i, &expected) in SIGMOID_TABLE.iter().enumerate() {
            let x_scaled = i as u128 * TABLE_STEP as u128;
            assert_eq!(
                sigmoid_positive(x_scaled),
                expected,
                "mismatch at table index {i}"
            );
        }
    }

    #[test]
    fn sigmoid_symmetry_property() {
        // sigmoid(-x) = 1 - sigmoid(x)
        let sig_pos = sigmoid_positive(1_000_000_000);
        let sig_neg = SIGMOID_PRECISION - sig_pos;
        assert_eq!(sig_neg, SIGMOID_PRECISION - 731_058_579);
    }

    #[test]
    fn sigmoid_table_values_within_bounds() {
        for (i, &val) in SIGMOID_TABLE.iter().enumerate() {
            assert!(val >= 500_000_000, "sigmoid below 0.5 at index {i}");
            assert!(val <= SIGMOID_PRECISION, "sigmoid above 1.0 at index {i}");
        }
    }

    // --- proptest ---

    proptest! {
        #[test]
        fn sigmoid_always_in_bounds(x in 0u128..20_000_000_000u128) {
            let result = sigmoid_positive(x);
            prop_assert!(result >= SIGMOID_TABLE[0]);
            prop_assert!(result <= *SIGMOID_TABLE.last().unwrap());
        }

        #[test]
        fn sigmoid_monotonic(
            a in 0u128..10_000_000_000u128,
            b in 0u128..10_000_000_000u128,
        ) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            prop_assert!(
                sigmoid_positive(lo) <= sigmoid_positive(hi),
                "sigmoid not monotonic: f({}) = {} > f({}) = {}",
                lo, sigmoid_positive(lo), hi, sigmoid_positive(hi)
            );
        }

        #[test]
        fn sigmoid_deterministic(x in 0u128..20_000_000_000u128) {
            let r1 = sigmoid_positive(x);
            let r2 = sigmoid_positive(x);
            prop_assert_eq!(r1, r2);
        }
    }
}
