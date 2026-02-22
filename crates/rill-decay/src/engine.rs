//! Decay engine implementing the [`DecayCalculator`] trait.
//!
//! Provides the production sigmoid-based decay computation with compound
//! decay over multiple blocks using binary exponentiation.
//! All arithmetic is integer-only with u128 intermediates for overflow safety.

use rill_core::constants::{
    BPS_PRECISION, DECAY_C_THRESHOLD_PPB, DECAY_K, DECAY_POOL_RELEASE_BPS, DECAY_PRECISION,
    DECAY_R_MAX_PPB,
};
use rill_core::error::DecayError;
use rill_core::traits::DecayCalculator;

use crate::sigmoid::{sigmoid_positive, SIGMOID_PRECISION};

/// The production decay calculator using sigmoid-based rates.
///
/// Implements [`DecayCalculator`] with:
/// - Sigmoid lookup table for decay rate computation
/// - Compound decay via binary exponentiation
/// - Zero decay below the concentration threshold
#[derive(Debug, Clone, Default)]
pub struct DecayEngine;

impl DecayEngine {
    /// Create a new DecayEngine.
    pub fn new() -> Self {
        Self
    }
}

/// Fixed-point exponentiation: computes `(base/precision)^exp` in fixed-point.
///
/// Uses binary exponentiation for O(log n) multiplications.
/// `base` and return value are in fixed-point with `precision` as denominator.
fn fixed_pow(base: u64, exp: u64, precision: u64) -> Result<u64, DecayError> {
    if exp == 0 {
        return Ok(precision); // (base/precision)^0 = 1.0
    }

    let p = precision as u128;
    let mut result: u128 = p;
    let mut b: u128 = base as u128;
    let mut e = exp;

    while e > 0 {
        if e & 1 == 1 {
            result = result
                .checked_mul(b)
                .ok_or(DecayError::ArithmeticOverflow)?
                / p;
        }
        e >>= 1;
        if e > 0 {
            b = b
                .checked_mul(b)
                .ok_or(DecayError::ArithmeticOverflow)?
                / p;
        }
    }

    Ok(result as u64)
}

impl DecayCalculator for DecayEngine {
    fn decay_rate_ppb(&self, concentration_ppb: u64) -> Result<u64, DecayError> {
        // Below or at threshold: no decay
        if concentration_ppb <= DECAY_C_THRESHOLD_PPB {
            return Ok(0);
        }

        // arg = k * (C - C_threshold)
        // arg_scaled = DECAY_K * (concentration_ppb - threshold_ppb)
        // The implicit denominator is CONCENTRATION_PRECISION.
        let diff = concentration_ppb - DECAY_C_THRESHOLD_PPB;
        let arg_scaled = (DECAY_K as u128)
            .checked_mul(diff as u128)
            .ok_or(DecayError::ArithmeticOverflow)?;

        // Evaluate sigmoid (returns value in [SIGMOID_TABLE[0], SIGMOID_PRECISION])
        let sigmoid_val = sigmoid_positive(arg_scaled);

        // rate = R_MAX * sigmoid_val / SIGMOID_PRECISION
        let rate = (DECAY_R_MAX_PPB as u128)
            .checked_mul(sigmoid_val as u128)
            .ok_or(DecayError::ArithmeticOverflow)?
            / SIGMOID_PRECISION as u128;

        Ok(rate as u64)
    }

    fn compute_decay(
        &self,
        nominal_value: u64,
        concentration_ppb: u64,
        blocks_held: u64,
    ) -> Result<u64, DecayError> {
        if blocks_held == 0 || nominal_value == 0 {
            return Ok(0);
        }

        let rate = self.decay_rate_ppb(concentration_ppb)?;
        if rate == 0 {
            return Ok(0);
        }

        // Ensure rate doesn't exceed precision (would mean >100% decay per block)
        if rate >= DECAY_PRECISION {
            return Ok(nominal_value);
        }

        // retention_per_block = (DECAY_PRECISION - rate) / DECAY_PRECISION
        let retention = DECAY_PRECISION - rate;

        // Compound over blocks_held: retention_total = (retention/PRECISION)^blocks_held
        let retention_total = fixed_pow(retention, blocks_held, DECAY_PRECISION)?;

        // effective = nominal * retention_total / PRECISION
        let effective = (nominal_value as u128)
            .checked_mul(retention_total as u128)
            .ok_or(DecayError::ArithmeticOverflow)?
            / DECAY_PRECISION as u128;

        // decay = nominal - effective
        Ok(nominal_value.saturating_sub(effective as u64))
    }

    fn compute_decay_with_conduct(
        &self,
        nominal_value: u64,
        concentration_ppb: u64,
        blocks_held: u64,
        conduct_multiplier_bps: u64,
    ) -> Result<u64, DecayError> {
        if blocks_held == 0 || nominal_value == 0 {
            return Ok(0);
        }

        let base_rate = self.decay_rate_ppb(concentration_ppb)?;
        if base_rate == 0 {
            return Ok(0);
        }

        // Apply conduct multiplier: adjusted_rate = base_rate * multiplier / BPS_PRECISION
        // Use u128 to prevent overflow (base_rate up to ~1.5B, multiplier up to 100,000).
        let adjusted_rate = (base_rate as u128)
            .checked_mul(conduct_multiplier_bps as u128)
            .ok_or(DecayError::ArithmeticOverflow)?
            / BPS_PRECISION as u128;

        // Cap at DECAY_PRECISION to prevent >100% decay per block (Undertow 10× safety).
        let adjusted_rate = adjusted_rate.min(DECAY_PRECISION as u128) as u64;

        if adjusted_rate >= DECAY_PRECISION {
            return Ok(nominal_value);
        }

        // retention_per_block = (DECAY_PRECISION - adjusted_rate) / DECAY_PRECISION
        let retention = DECAY_PRECISION - adjusted_rate;

        // Compound over blocks_held
        let retention_total = fixed_pow(retention, blocks_held, DECAY_PRECISION)?;

        // effective = nominal * retention_total / PRECISION
        let effective = (nominal_value as u128)
            .checked_mul(retention_total as u128)
            .ok_or(DecayError::ArithmeticOverflow)?
            / DECAY_PRECISION as u128;

        // Invariant: decay = nominal - effective
        Ok(nominal_value.saturating_sub(effective as u64))
    }

    fn decay_pool_release(&self, pool_balance: u64) -> Result<u64, DecayError> {
        pool_balance
            .checked_mul(DECAY_POOL_RELEASE_BPS)
            .map(|v| v / BPS_PRECISION)
            .ok_or(DecayError::ArithmeticOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rill_core::constants::{COIN, CONCENTRATION_PRECISION, MAX_SUPPLY};

    fn engine() -> DecayEngine {
        DecayEngine::new()
    }

    // --- decay_rate_ppb ---

    #[test]
    fn rate_zero_below_threshold() {
        let e = engine();
        assert_eq!(e.decay_rate_ppb(0).unwrap(), 0);
        assert_eq!(e.decay_rate_ppb(DECAY_C_THRESHOLD_PPB).unwrap(), 0);
        assert_eq!(e.decay_rate_ppb(DECAY_C_THRESHOLD_PPB / 2).unwrap(), 0);
    }

    #[test]
    fn rate_nonzero_above_threshold() {
        let e = engine();
        let rate = e.decay_rate_ppb(DECAY_C_THRESHOLD_PPB + 1).unwrap();
        assert!(rate > 0);
    }

    #[test]
    fn rate_increases_with_concentration() {
        let e = engine();
        let r1 = e
            .decay_rate_ppb(DECAY_C_THRESHOLD_PPB + 100_000)
            .unwrap();
        let r2 = e
            .decay_rate_ppb(DECAY_C_THRESHOLD_PPB + 500_000)
            .unwrap();
        let r3 = e
            .decay_rate_ppb(DECAY_C_THRESHOLD_PPB + 1_000_000)
            .unwrap();
        assert!(r1 < r2, "rate should increase: {r1} < {r2}");
        assert!(r2 < r3, "rate should increase: {r2} < {r3}");
    }

    #[test]
    fn rate_bounded_by_r_max() {
        let e = engine();
        let rate = e.decay_rate_ppb(CONCENTRATION_PRECISION).unwrap();
        assert!(
            rate <= DECAY_R_MAX_PPB,
            "rate {rate} exceeds R_MAX {DECAY_R_MAX_PPB}"
        );
    }

    #[test]
    fn rate_at_threshold_boundary() {
        let e = engine();
        // Just above threshold: sigmoid(0+eps) ≈ 0.5, rate ≈ R_MAX * 0.5 = 750M
        let rate = e.decay_rate_ppb(DECAY_C_THRESHOLD_PPB + 1).unwrap();
        assert!(
            rate >= 740_000_000 && rate <= 760_000_000,
            "rate at threshold boundary: {rate}"
        );
    }

    #[test]
    fn rate_at_high_concentration() {
        let e = engine();
        // 1% of supply: concentration_ppb = 10_000_000
        // arg = 2000 * 9_000_000 = 18e9, sigmoid(18) ≈ 1.0
        let rate = e.decay_rate_ppb(10_000_000).unwrap();
        assert!(
            rate > DECAY_R_MAX_PPB * 99 / 100,
            "rate at 1% should be near R_MAX: {rate}"
        );
    }

    // --- compute_decay ---

    #[test]
    fn decay_zero_blocks() {
        let e = engine();
        assert_eq!(
            e.compute_decay(1000 * COIN, DECAY_C_THRESHOLD_PPB + 100_000, 0)
                .unwrap(),
            0
        );
    }

    #[test]
    fn decay_zero_value() {
        let e = engine();
        assert_eq!(
            e.compute_decay(0, DECAY_C_THRESHOLD_PPB + 100_000, 100)
                .unwrap(),
            0
        );
    }

    #[test]
    fn decay_below_threshold_is_zero() {
        let e = engine();
        assert_eq!(e.compute_decay(1000 * COIN, 0, 1000).unwrap(), 0);
        assert_eq!(
            e.compute_decay(1000 * COIN, DECAY_C_THRESHOLD_PPB, 1000)
                .unwrap(),
            0
        );
    }

    #[test]
    fn decay_increases_with_blocks() {
        let e = engine();
        let conc = DECAY_C_THRESHOLD_PPB + 500_000;
        let value = 1000 * COIN;
        let d1 = e.compute_decay(value, conc, 1).unwrap();
        let d10 = e.compute_decay(value, conc, 10).unwrap();
        let d100 = e.compute_decay(value, conc, 100).unwrap();

        assert!(d1 < d10, "decay should increase: {d1} < {d10}");
        assert!(d10 < d100, "decay should increase: {d10} < {d100}");
    }

    #[test]
    fn decay_never_exceeds_nominal() {
        let e = engine();
        let value = 1000 * COIN;
        let conc = CONCENTRATION_PRECISION;
        let decay = e.compute_decay(value, conc, 1_000_000).unwrap();
        assert!(decay <= value, "decay {decay} exceeds nominal {value}");
    }

    #[test]
    fn decay_compound_less_than_linear() {
        let e = engine();
        let conc = DECAY_C_THRESHOLD_PPB + 500_000;
        let value = 1000 * COIN;
        let d1 = e.compute_decay(value, conc, 1).unwrap();
        let d10 = e.compute_decay(value, conc, 10).unwrap();
        assert!(
            d10 < d1 * 10,
            "compound should be less than linear: {d10} < {}",
            d1 * 10
        );
    }

    #[test]
    fn decay_large_blocks_approaches_total() {
        let e = engine();
        let value = 1000 * COIN;
        let conc = DECAY_C_THRESHOLD_PPB + 1_000_000;
        // After many blocks at high concentration, almost all value decays
        let decay = e.compute_decay(value, conc, 100_000).unwrap();
        assert_eq!(decay, value, "after 100K blocks, decay should equal nominal");
    }

    // --- effective_value (default impl from trait) ---

    #[test]
    fn effective_value_no_decay_below_threshold() {
        let e = engine();
        let value = 500 * COIN;
        let effective = e.effective_value(value, 0, 1000).unwrap();
        assert_eq!(effective, value);
    }

    #[test]
    fn effective_value_decreases_above_threshold() {
        let e = engine();
        let value = 500 * COIN;
        let conc = DECAY_C_THRESHOLD_PPB + 500_000;
        let effective = e.effective_value(value, conc, 10).unwrap();
        assert!(
            effective < value,
            "effective {effective} should be < nominal {value}"
        );
        assert!(effective > 0, "effective should not be zero after 10 blocks");
    }

    #[test]
    fn effective_plus_decay_equals_nominal() {
        let e = engine();
        let value = 1000 * COIN;
        let conc = DECAY_C_THRESHOLD_PPB + 500_000;
        let blocks = 50;
        let decay = e.compute_decay(value, conc, blocks).unwrap();
        let effective = e.effective_value(value, conc, blocks).unwrap();
        assert_eq!(effective + decay, value);
    }

    // --- decay_pool_release ---

    #[test]
    fn pool_release_percentage() {
        let e = engine();
        let pool = 10_000 * COIN;
        let release = e.decay_pool_release(pool).unwrap();
        assert_eq!(release, pool * DECAY_POOL_RELEASE_BPS / BPS_PRECISION);
        assert_eq!(release, 100 * COIN);
    }

    #[test]
    fn pool_release_zero_balance() {
        let e = engine();
        assert_eq!(e.decay_pool_release(0).unwrap(), 0);
    }

    #[test]
    fn pool_release_small_balance() {
        let e = engine();
        // 99 rills: 99 * 100 / 10000 = 0 (rounds down)
        assert_eq!(e.decay_pool_release(99).unwrap(), 0);
        // 100 rills: 100 * 100 / 10000 = 1
        assert_eq!(e.decay_pool_release(100).unwrap(), 1);
    }

    // --- fixed_pow ---

    #[test]
    fn fixed_pow_zero_exponent() {
        assert_eq!(
            fixed_pow(5_000_000_000, 0, DECAY_PRECISION).unwrap(),
            DECAY_PRECISION
        );
    }

    #[test]
    fn fixed_pow_one_exponent() {
        let base = 8_500_000_000;
        let result = fixed_pow(base, 1, DECAY_PRECISION).unwrap();
        assert_eq!(result, base);
    }

    #[test]
    fn fixed_pow_squares_correctly() {
        // 0.8^2 = 0.64
        let result = fixed_pow(8_000_000_000, 2, DECAY_PRECISION).unwrap();
        assert_eq!(result, 6_400_000_000);
    }

    #[test]
    fn fixed_pow_cubes_correctly() {
        // 0.9^3 = 0.729
        let result = fixed_pow(9_000_000_000, 3, DECAY_PRECISION).unwrap();
        assert_eq!(result, 7_290_000_000);
    }

    #[test]
    fn fixed_pow_large_exponent() {
        // 0.9999^10000 ≈ e^(-1) ≈ 0.3679
        let result = fixed_pow(9_999_000_000, 10_000, DECAY_PRECISION).unwrap();
        assert!(
            result > 3_600_000_000 && result < 3_800_000_000,
            "0.9999^10000 = {result}, expected ~3_679_000_000"
        );
    }

    #[test]
    fn fixed_pow_full_precision() {
        // 1.0^anything = 1.0
        let result = fixed_pow(DECAY_PRECISION, 1_000_000, DECAY_PRECISION).unwrap();
        assert_eq!(result, DECAY_PRECISION);
    }

    #[test]
    fn fixed_pow_zero_base() {
        let result = fixed_pow(0, 100, DECAY_PRECISION).unwrap();
        assert_eq!(result, 0);
    }

    // --- dyn compatibility ---

    #[test]
    fn engine_is_object_safe() {
        let e = engine();
        let dyn_e: &dyn DecayCalculator = &e;
        assert_eq!(dyn_e.decay_rate_ppb(0).unwrap(), 0);
    }

    // --- compute_decay_with_conduct ---

    #[test]
    fn conduct_1x_matches_base() {
        let e = engine();
        let value = 1000 * COIN;
        let conc = DECAY_C_THRESHOLD_PPB + 500_000;
        let blocks = 50;
        let base = e.compute_decay(value, conc, blocks).unwrap();
        let conducted = e
            .compute_decay_with_conduct(value, conc, blocks, BPS_PRECISION)
            .unwrap();
        assert_eq!(base, conducted, "1.0× multiplier should match base decay");
    }

    #[test]
    fn conduct_1_5x_decays_more() {
        let e = engine();
        let value = 1000 * COIN;
        let conc = DECAY_C_THRESHOLD_PPB + 500_000;
        let blocks = 50;
        let base = e.compute_decay(value, conc, blocks).unwrap();
        let conducted = e
            .compute_decay_with_conduct(value, conc, blocks, 15_000)
            .unwrap();
        assert!(
            conducted > base,
            "1.5× multiplier should decay more: {conducted} > {base}"
        );
    }

    #[test]
    fn conduct_0_5x_decays_less() {
        let e = engine();
        let value = 1000 * COIN;
        let conc = DECAY_C_THRESHOLD_PPB + 500_000;
        let blocks = 50;
        let base = e.compute_decay(value, conc, blocks).unwrap();
        let conducted = e
            .compute_decay_with_conduct(value, conc, blocks, 5_000)
            .unwrap();
        assert!(
            conducted < base,
            "0.5× multiplier should decay less: {conducted} < {base}"
        );
    }

    #[test]
    fn conduct_2x_decays_more_than_1_5x() {
        let e = engine();
        let value = 1000 * COIN;
        let conc = DECAY_C_THRESHOLD_PPB + 500_000;
        let blocks = 50;
        let d_1_5 = e
            .compute_decay_with_conduct(value, conc, blocks, 15_000)
            .unwrap();
        let d_2_0 = e
            .compute_decay_with_conduct(value, conc, blocks, 20_000)
            .unwrap();
        assert!(d_2_0 > d_1_5, "2.0× > 1.5×: {d_2_0} > {d_1_5}");
    }

    #[test]
    fn conduct_10x_undertow_capped() {
        let e = engine();
        let value = 1000 * COIN;
        let conc = DECAY_C_THRESHOLD_PPB + 500_000;
        let blocks = 50;
        // 10× = Undertow. Rate should be capped at DECAY_PRECISION.
        let conducted = e
            .compute_decay_with_conduct(value, conc, blocks, 100_000)
            .unwrap();
        assert!(conducted <= value, "decay must not exceed nominal");
    }

    #[test]
    fn conduct_invariant_effective_plus_decay() {
        let e = engine();
        let value = 1000 * COIN;
        let conc = DECAY_C_THRESHOLD_PPB + 500_000;
        let blocks = 50;

        for multiplier in [5_000, 7_500, 10_000, 15_000, 20_000, 25_000, 30_000, 100_000] {
            let decay = e
                .compute_decay_with_conduct(value, conc, blocks, multiplier)
                .unwrap();
            let effective = value.saturating_sub(decay);
            assert_eq!(
                effective + decay, value,
                "invariant broken for multiplier {multiplier}"
            );
        }
    }

    #[test]
    fn conduct_zero_blocks_is_zero() {
        let e = engine();
        let conducted = e
            .compute_decay_with_conduct(1000 * COIN, DECAY_C_THRESHOLD_PPB + 500_000, 0, 15_000)
            .unwrap();
        assert_eq!(conducted, 0);
    }

    #[test]
    fn conduct_below_threshold_is_zero() {
        let e = engine();
        let conducted = e
            .compute_decay_with_conduct(1000 * COIN, 0, 100, 15_000)
            .unwrap();
        assert_eq!(conducted, 0);
    }

    // --- proptest ---

    proptest! {
        #[test]
        fn rate_monotonic(
            a in (DECAY_C_THRESHOLD_PPB + 1)..CONCENTRATION_PRECISION,
            b in (DECAY_C_THRESHOLD_PPB + 1)..CONCENTRATION_PRECISION,
        ) {
            let e = engine();
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let ra = e.decay_rate_ppb(lo).unwrap();
            let rb = e.decay_rate_ppb(hi).unwrap();
            prop_assert!(ra <= rb, "rate not monotonic: r({})={} > r({})={}", lo, ra, hi, rb);
        }

        #[test]
        fn rate_bounded(conc in 0u64..=CONCENTRATION_PRECISION) {
            let e = engine();
            let rate = e.decay_rate_ppb(conc).unwrap();
            prop_assert!(rate <= DECAY_R_MAX_PPB);
        }

        #[test]
        fn decay_never_exceeds_nominal_prop(
            value in 1u64..=MAX_SUPPLY,
            conc in 0u64..=CONCENTRATION_PRECISION,
            blocks in 0u64..=10_000_000,
        ) {
            let e = engine();
            let decay = e.compute_decay(value, conc, blocks).unwrap();
            prop_assert!(decay <= value, "decay {} > nominal {}", decay, value);
        }

        #[test]
        fn effective_plus_decay_invariant(
            value in 0u64..=MAX_SUPPLY,
            conc in 0u64..=CONCENTRATION_PRECISION,
            blocks in 0u64..=10_000_000,
        ) {
            let e = engine();
            let decay = e.compute_decay(value, conc, blocks).unwrap();
            let effective = e.effective_value(value, conc, blocks).unwrap();
            prop_assert_eq!(effective + decay, value);
        }

        #[test]
        fn pool_release_bounded(balance in 0u64..=MAX_SUPPLY) {
            let e = engine();
            let release = e.decay_pool_release(balance).unwrap();
            prop_assert!(release <= balance);
        }

        #[test]
        fn conduct_decay_never_exceeds_nominal(
            value in 1u64..=MAX_SUPPLY,
            conc in 0u64..=CONCENTRATION_PRECISION,
            blocks in 0u64..=1_000_000,
            multiplier in 5_000u64..=100_000u64,
        ) {
            let e = engine();
            let decay = e.compute_decay_with_conduct(value, conc, blocks, multiplier).unwrap();
            prop_assert!(decay <= value, "conduct decay {} > nominal {}", decay, value);
        }

        #[test]
        fn conduct_higher_multiplier_more_decay(
            value in 1u64..=(MAX_SUPPLY / 100),
            conc in (DECAY_C_THRESHOLD_PPB + 100_000)..CONCENTRATION_PRECISION,
            blocks in 1u64..=1_000u64,
        ) {
            let e = engine();
            let d_lo = e.compute_decay_with_conduct(value, conc, blocks, 5_000).unwrap();
            let d_hi = e.compute_decay_with_conduct(value, conc, blocks, 20_000).unwrap();
            prop_assert!(d_hi >= d_lo, "higher multiplier should decay >= lower: {} >= {}", d_hi, d_lo);
        }
    }
}
