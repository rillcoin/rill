//! UTXO lineage-based clustering for decay calculation.
//!
//! Clusters group UTXOs that share ownership lineage. The cluster balance
//! determines the concentration used for decay rate computation.
//!
//! Rules:
//! 1. All outputs from a single transaction belong to the same cluster.
//! 2. Mixed inputs from different clusters cause a deterministic merge.
//! 3. Lineage weakens over time: 50% after [`LINEAGE_HALF_LIFE`] blocks,
//!    full reset after [`LINEAGE_FULL_RESET`] blocks.

use rill_core::constants::{CONCENTRATION_PRECISION, LINEAGE_FULL_RESET, LINEAGE_HALF_LIFE};
use rill_core::types::Hash256;

/// Determine the cluster ID for outputs of a transaction.
///
/// - Coinbase (empty `input_cluster_ids`): new cluster derived from `txid`.
/// - Single input cluster: outputs inherit that cluster.
/// - Multiple input clusters: deterministic merge via BLAKE3 hash of sorted IDs.
pub fn determine_output_cluster(input_cluster_ids: &[Hash256], txid: &Hash256) -> Hash256 {
    if input_cluster_ids.is_empty() {
        return *txid;
    }

    // Deduplicate and sort for determinism
    let mut unique: Vec<Hash256> = input_cluster_ids.to_vec();
    unique.sort();
    unique.dedup();

    if unique.len() == 1 {
        return unique[0];
    }

    // Multiple clusters: merge by hashing sorted cluster IDs
    let mut hasher = blake3::Hasher::new();
    for id in &unique {
        hasher.update(id.as_bytes());
    }
    Hash256(hasher.finalize().into())
}

/// Compute the lineage factor for a UTXO held for `blocks_held` blocks.
///
/// Returns a value in `[0, CONCENTRATION_PRECISION]` representing the fraction
/// of the original cluster association that remains:
/// - At 0 blocks: full association ([`CONCENTRATION_PRECISION`])
/// - At [`LINEAGE_HALF_LIFE`] blocks: half association
/// - At [`LINEAGE_FULL_RESET`] blocks and beyond: no association (0)
///
/// Uses piecewise linear decay:
/// - `[0, HALF_LIFE]`: linearly from 1.0 to 0.5
/// - `[HALF_LIFE, FULL_RESET]`: linearly from 0.5 to 0.0
pub fn lineage_factor(blocks_held: u64) -> u64 {
    if blocks_held == 0 {
        return CONCENTRATION_PRECISION;
    }
    if blocks_held >= LINEAGE_FULL_RESET {
        return 0;
    }

    let half = CONCENTRATION_PRECISION / 2;

    if blocks_held <= LINEAGE_HALF_LIFE {
        // Linear from CONCENTRATION_PRECISION to half over HALF_LIFE blocks
        CONCENTRATION_PRECISION - half * blocks_held / LINEAGE_HALF_LIFE
    } else {
        // Linear from half to 0 over (FULL_RESET - HALF_LIFE) blocks
        let remaining = LINEAGE_FULL_RESET - blocks_held;
        let range = LINEAGE_FULL_RESET - LINEAGE_HALF_LIFE;
        half * remaining / range
    }
}

/// Compute the effective cluster balance considering lineage weakening.
///
/// The UTXO's contribution to its cluster balance is reduced by the
/// lineage factor, simulating the weakening of ownership association over time.
///
/// Returns `nominal_balance * lineage_factor(blocks_held) / CONCENTRATION_PRECISION`.
pub fn lineage_adjusted_balance(nominal_balance: u64, blocks_held: u64) -> u64 {
    let factor = lineage_factor(blocks_held);
    (nominal_balance as u128 * factor as u128 / CONCENTRATION_PRECISION as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rill_core::constants::{COIN, MAX_SUPPLY};

    fn test_hash(val: u8) -> Hash256 {
        Hash256([val; 32])
    }

    // --- determine_output_cluster ---

    #[test]
    fn coinbase_creates_new_cluster() {
        let txid = test_hash(0xAA);
        let cluster = determine_output_cluster(&[], &txid);
        assert_eq!(cluster, txid);
    }

    #[test]
    fn single_input_inherits_cluster() {
        let cluster_id = test_hash(0xBB);
        let txid = test_hash(0xCC);
        let result = determine_output_cluster(&[cluster_id], &txid);
        assert_eq!(result, cluster_id);
    }

    #[test]
    fn same_cluster_inputs_no_merge() {
        let cluster_id = test_hash(0xBB);
        let txid = test_hash(0xCC);
        let result = determine_output_cluster(&[cluster_id, cluster_id, cluster_id], &txid);
        assert_eq!(result, cluster_id);
    }

    #[test]
    fn different_clusters_merge() {
        let c1 = test_hash(0x11);
        let c2 = test_hash(0x22);
        let txid = test_hash(0xCC);
        let result = determine_output_cluster(&[c1, c2], &txid);
        assert_ne!(result, c1);
        assert_ne!(result, c2);
        assert_ne!(result, txid);
    }

    #[test]
    fn merge_is_deterministic() {
        let c1 = test_hash(0x11);
        let c2 = test_hash(0x22);
        let txid = test_hash(0xCC);
        let r1 = determine_output_cluster(&[c1, c2], &txid);
        let r2 = determine_output_cluster(&[c1, c2], &txid);
        assert_eq!(r1, r2);
    }

    #[test]
    fn merge_order_independent() {
        let c1 = test_hash(0x11);
        let c2 = test_hash(0x22);
        let txid = test_hash(0xCC);
        let r1 = determine_output_cluster(&[c1, c2], &txid);
        let r2 = determine_output_cluster(&[c2, c1], &txid);
        assert_eq!(r1, r2, "merge must be order-independent");
    }

    #[test]
    fn merge_three_clusters() {
        let c1 = test_hash(0x11);
        let c2 = test_hash(0x22);
        let c3 = test_hash(0x33);
        let txid = test_hash(0xCC);
        let result = determine_output_cluster(&[c1, c2, c3], &txid);
        assert_ne!(result, c1);
        assert_ne!(result, c2);
        assert_ne!(result, c3);
    }

    #[test]
    fn merge_three_any_order() {
        let c1 = test_hash(0x11);
        let c2 = test_hash(0x22);
        let c3 = test_hash(0x33);
        let txid = test_hash(0xCC);
        let r1 = determine_output_cluster(&[c1, c2, c3], &txid);
        let r2 = determine_output_cluster(&[c3, c1, c2], &txid);
        let r3 = determine_output_cluster(&[c2, c3, c1], &txid);
        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
    }

    #[test]
    fn merge_with_duplicates() {
        let c1 = test_hash(0x11);
        let c2 = test_hash(0x22);
        let txid = test_hash(0xCC);
        let r1 = determine_output_cluster(&[c1, c2], &txid);
        let r2 = determine_output_cluster(&[c1, c1, c2, c2], &txid);
        assert_eq!(r1, r2, "duplicates should not affect merge result");
    }

    // --- lineage_factor ---

    #[test]
    fn lineage_factor_at_zero() {
        assert_eq!(lineage_factor(0), CONCENTRATION_PRECISION);
    }

    #[test]
    fn lineage_factor_at_half_life() {
        assert_eq!(lineage_factor(LINEAGE_HALF_LIFE), CONCENTRATION_PRECISION / 2);
    }

    #[test]
    fn lineage_factor_at_full_reset() {
        assert_eq!(lineage_factor(LINEAGE_FULL_RESET), 0);
    }

    #[test]
    fn lineage_factor_beyond_full_reset() {
        assert_eq!(lineage_factor(LINEAGE_FULL_RESET + 1), 0);
        assert_eq!(lineage_factor(u64::MAX), 0);
    }

    #[test]
    fn lineage_factor_monotonically_decreasing() {
        let step = LINEAGE_FULL_RESET / 100;
        let mut prev = lineage_factor(0);
        for i in 1..=100 {
            let blocks = i * step;
            let current = lineage_factor(blocks);
            assert!(
                current <= prev,
                "not monotonic at blocks={blocks}: {current} > {prev}"
            );
            prev = current;
        }
    }

    #[test]
    fn lineage_factor_continuous_at_half_life() {
        let before = lineage_factor(LINEAGE_HALF_LIFE - 1);
        let at = lineage_factor(LINEAGE_HALF_LIFE);
        let after = lineage_factor(LINEAGE_HALF_LIFE + 1);
        assert!(before >= at);
        assert!(at >= after);
        // Small step should produce small change
        let delta = before - after;
        assert!(
            delta < CONCENTRATION_PRECISION / 1000,
            "discontinuity at half-life: delta={delta}"
        );
    }

    // --- lineage_adjusted_balance ---

    #[test]
    fn adjusted_balance_at_zero_blocks() {
        let balance = 1000 * COIN;
        assert_eq!(lineage_adjusted_balance(balance, 0), balance);
    }

    #[test]
    fn adjusted_balance_at_half_life() {
        let balance = 1000 * COIN;
        let adjusted = lineage_adjusted_balance(balance, LINEAGE_HALF_LIFE);
        assert_eq!(adjusted, balance / 2);
    }

    #[test]
    fn adjusted_balance_at_full_reset() {
        let balance = 1000 * COIN;
        assert_eq!(lineage_adjusted_balance(balance, LINEAGE_FULL_RESET), 0);
    }

    #[test]
    fn adjusted_balance_zero_nominal() {
        assert_eq!(lineage_adjusted_balance(0, 100), 0);
    }

    // --- proptest ---

    proptest! {
        #[test]
        fn cluster_merge_order_independent(
            a in prop::array::uniform32(0u8..),
            b in prop::array::uniform32(0u8..),
            t in prop::array::uniform32(0u8..),
        ) {
            let c1 = Hash256(a);
            let c2 = Hash256(b);
            let txid = Hash256(t);
            prop_assume!(c1 != c2);
            let r1 = determine_output_cluster(&[c1, c2], &txid);
            let r2 = determine_output_cluster(&[c2, c1], &txid);
            prop_assert_eq!(r1, r2);
        }

        #[test]
        fn cluster_deterministic(
            inputs in prop::collection::vec(prop::array::uniform32(0u8..), 0..5),
            t in prop::array::uniform32(0u8..),
        ) {
            let cluster_ids: Vec<Hash256> = inputs.iter().map(|b| Hash256(*b)).collect();
            let txid = Hash256(t);
            let r1 = determine_output_cluster(&cluster_ids, &txid);
            let r2 = determine_output_cluster(&cluster_ids, &txid);
            prop_assert_eq!(r1, r2);
        }

        #[test]
        fn lineage_factor_bounded(blocks in 0u64..=u64::MAX) {
            let factor = lineage_factor(blocks);
            prop_assert!(factor <= CONCENTRATION_PRECISION);
        }

        #[test]
        fn lineage_factor_monotonic(
            a in 0u64..LINEAGE_FULL_RESET,
            b in 0u64..LINEAGE_FULL_RESET,
        ) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            prop_assert!(
                lineage_factor(lo) >= lineage_factor(hi),
                "not monotonic: f({})={} < f({})={}",
                lo, lineage_factor(lo), hi, lineage_factor(hi)
            );
        }

        #[test]
        fn adjusted_balance_bounded(
            balance in 0u64..=MAX_SUPPLY,
            blocks in 0u64..=LINEAGE_FULL_RESET * 2,
        ) {
            let adjusted = lineage_adjusted_balance(balance, blocks);
            prop_assert!(adjusted <= balance);
        }
    }
}
