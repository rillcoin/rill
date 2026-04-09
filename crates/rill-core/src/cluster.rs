//! UTXO cluster identification for concentration decay.
//!
//! Clusters group UTXOs that share ownership lineage. All outputs from a
//! single transaction belong to the same cluster. When a transaction spends
//! inputs from multiple clusters, the clusters merge deterministically.
//!
//! This module lives in `rill-core` so that both `rill-decay` (for lineage
//! calculations) and `MemoryChainStore` (for test/validation) can assign
//! cluster IDs without circular dependencies.

use crate::types::Hash256;

/// Determine the cluster ID for outputs of a transaction.
///
/// - Coinbase (empty `input_cluster_ids`): new cluster derived from `txid`.
/// - Single input cluster: outputs inherit that cluster.
/// - Multiple input clusters: deterministic merge via BLAKE3 hash of sorted IDs.
///
/// # Examples
///
/// ```
/// use rill_core::cluster::determine_output_cluster;
/// use rill_core::types::Hash256;
///
/// // Coinbase: cluster = txid
/// let txid = Hash256([0xAA; 32]);
/// assert_eq!(determine_output_cluster(&[], &txid), txid);
///
/// // Single cluster: inherited
/// let cluster = Hash256([0xBB; 32]);
/// assert_eq!(determine_output_cluster(&[cluster], &txid), cluster);
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hash(val: u8) -> Hash256 {
        Hash256([val; 32])
    }

    #[test]
    fn coinbase_creates_new_cluster() {
        let txid = test_hash(0xAA);
        assert_eq!(determine_output_cluster(&[], &txid), txid);
    }

    #[test]
    fn single_input_inherits_cluster() {
        let cluster_id = test_hash(0xBB);
        let txid = test_hash(0xCC);
        assert_eq!(determine_output_cluster(&[cluster_id], &txid), cluster_id);
    }

    #[test]
    fn same_cluster_inputs_no_merge() {
        let c = test_hash(0xBB);
        let txid = test_hash(0xCC);
        assert_eq!(determine_output_cluster(&[c, c, c], &txid), c);
    }

    #[test]
    fn different_clusters_merge_deterministically() {
        let c1 = test_hash(0x11);
        let c2 = test_hash(0x22);
        let txid = test_hash(0xCC);
        let r1 = determine_output_cluster(&[c1, c2], &txid);
        let r2 = determine_output_cluster(&[c2, c1], &txid);
        assert_eq!(r1, r2, "merge must be order-independent");
        assert_ne!(r1, c1);
        assert_ne!(r1, c2);
    }
}
