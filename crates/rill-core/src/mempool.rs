//! In-memory pool of unconfirmed transactions (mempool).
//!
//! The mempool stores validated transactions awaiting inclusion in blocks.
//! It provides:
//! - O(1) lookup by txid
//! - O(1) conflict detection via spent-outpoint index
//! - O(log n) fee-rate-ordered selection for block templates
//! - Size-limited storage with lowest-fee-rate eviction
//!
//! Transactions must be validated by the caller before insertion (using
//! [`validate_transaction`](crate::validation::validate_transaction)).
//! The mempool only checks for duplicates and input conflicts.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::constants::MIN_TX_FEE;
use crate::error::MempoolError;
use crate::types::{Block, Hash256, OutPoint, Transaction};

/// Default maximum number of transactions in the mempool.
pub const DEFAULT_MAX_COUNT: usize = 5_000;

/// Default maximum total serialized bytes in the mempool (5 MiB).
pub const DEFAULT_MAX_BYTES: usize = 5 * 1024 * 1024;

/// Fee rate precision multiplier.
///
/// Fee rate is stored as `fee * FEE_RATE_PRECISION / size`, giving
/// milli-rills per byte for fine-grained ordering.
const FEE_RATE_PRECISION: u128 = 1_000;

/// Compute fee rate in milli-rills per byte.
///
/// Uses u128 intermediate to prevent overflow for large fees.
fn compute_fee_rate(fee: u64, size: usize) -> u64 {
    if size == 0 {
        return u64::MAX;
    }
    let rate = (fee as u128) * FEE_RATE_PRECISION / (size as u128);
    rate.min(u64::MAX as u128) as u64
}

/// A transaction stored in the mempool with precomputed metadata.
#[derive(Debug, Clone)]
pub struct MempoolEntry {
    /// The unconfirmed transaction.
    pub tx: Transaction,
    /// Precomputed transaction ID.
    pub txid: Hash256,
    /// Transaction fee in rills (`total_input - total_output`).
    pub fee: u64,
    /// Serialized size in bytes.
    pub size: usize,
    /// Fee rate in milli-rills per byte.
    fee_rate: u64,
}

impl MempoolEntry {
    /// Fee rate in milli-rills per byte.
    pub fn fee_rate(&self) -> u64 {
        self.fee_rate
    }
}

/// In-memory pool of unconfirmed transactions.
///
/// Stores pre-validated transactions indexed by txid and spent outpoints.
/// Maintains a fee-rate-ordered index for efficient block template selection
/// and lowest-priority eviction.
///
/// Not thread-safe — callers should wrap in a `Mutex` or `RwLock` if
/// concurrent access is needed.
pub struct Mempool {
    /// Primary storage: txid → entry.
    entries: HashMap<Hash256, MempoolEntry>,
    /// Spent outpoint → txid of the pool transaction that spends it.
    by_outpoint: HashMap<OutPoint, Hash256>,
    /// Fee-rate-ordered index: `(fee_rate, txid)`.
    /// Ascending order: lowest fee rate first (for eviction), iterate in
    /// reverse for highest-first (block template selection).
    by_fee_rate: BTreeSet<(u64, Hash256)>,
    /// Maximum transaction count.
    max_count: usize,
    /// Maximum total serialized bytes.
    max_bytes: usize,
    /// Current total serialized bytes in the pool.
    total_bytes: usize,
}

impl Mempool {
    /// Create a new mempool with the given size limits.
    pub fn new(max_count: usize, max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            by_outpoint: HashMap::new(),
            by_fee_rate: BTreeSet::new(),
            max_count,
            max_bytes,
            total_bytes: 0,
        }
    }

    /// Create a new mempool with default size limits.
    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_MAX_COUNT, DEFAULT_MAX_BYTES)
    }

    /// Insert a validated transaction into the mempool.
    ///
    /// The transaction must have been validated by the caller (structural +
    /// contextual). The mempool checks for duplicates, input conflicts,
    /// and size limits.
    ///
    /// `fee` is the transaction fee in rills (from
    /// [`ValidatedTransaction::fee`](crate::validation::ValidatedTransaction::fee)).
    ///
    /// Returns the txid on success. If the pool is full, attempts to evict
    /// the lowest-fee-rate entry (only if the new transaction has a strictly
    /// higher fee rate). Eviction continues until space is available or the
    /// new transaction's fee rate is not higher than the lowest entry.
    pub fn insert(&mut self, tx: Transaction, fee: u64) -> Result<Hash256, MempoolError> {
        if fee < MIN_TX_FEE {
            return Err(MempoolError::FeeTooLow { fee, minimum: MIN_TX_FEE });
        }

        // Compute txid and size from a single serialization.
        let encoded = bincode::encode_to_vec(&tx, bincode::config::standard())
            .map_err(|e| MempoolError::Internal(e.to_string()))?;
        let txid = Hash256(blake3::hash(&encoded).into());
        let size = encoded.len();

        if self.entries.contains_key(&txid) {
            return Err(MempoolError::AlreadyExists(txid.to_string()));
        }

        // Check for input conflicts with existing pool entries.
        for input in &tx.inputs {
            if let Some(conflicting) = self.by_outpoint.get(&input.previous_output) {
                return Err(MempoolError::Conflict {
                    new_txid: txid.to_string(),
                    existing_txid: conflicting.to_string(),
                    outpoint: input.previous_output.to_string(),
                });
            }
        }

        let fee_rate = compute_fee_rate(fee, size);

        // Evict lowest-fee-rate entries if the pool is full.
        while (self.entries.len() >= self.max_count || self.total_bytes + size > self.max_bytes)
            && !self.entries.is_empty()
        {
            if let Some(&(lowest_rate, lowest_txid)) = self.by_fee_rate.iter().next() {
                if lowest_rate >= fee_rate {
                    return Err(MempoolError::PoolFull);
                }
                self.remove_entry(lowest_txid);
            } else {
                break;
            }
        }

        // Final capacity check.
        if self.entries.len() >= self.max_count || self.total_bytes + size > self.max_bytes {
            return Err(MempoolError::PoolFull);
        }

        // Insert into all indices.
        for input in &tx.inputs {
            self.by_outpoint
                .insert(input.previous_output.clone(), txid);
        }
        self.by_fee_rate.insert((fee_rate, txid));
        self.total_bytes += size;
        self.entries.insert(
            txid,
            MempoolEntry {
                tx,
                txid,
                fee,
                size,
                fee_rate,
            },
        );

        Ok(txid)
    }

    /// Remove a transaction from the mempool by txid.
    ///
    /// Returns the removed entry, or `None` if not found.
    pub fn remove(&mut self, txid: &Hash256) -> Option<MempoolEntry> {
        self.remove_entry(*txid)
    }

    /// Internal: remove an entry and clean up all indices.
    fn remove_entry(&mut self, txid: Hash256) -> Option<MempoolEntry> {
        let entry = self.entries.remove(&txid)?;
        for input in &entry.tx.inputs {
            self.by_outpoint.remove(&input.previous_output);
        }
        self.by_fee_rate.remove(&(entry.fee_rate, txid));
        self.total_bytes -= entry.size;
        Some(entry)
    }

    /// Check if a transaction with the given txid is in the pool.
    pub fn contains(&self, txid: &Hash256) -> bool {
        self.entries.contains_key(txid)
    }

    /// Get a mempool entry by txid.
    pub fn get(&self, txid: &Hash256) -> Option<&MempoolEntry> {
        self.entries.get(txid)
    }

    /// Check whether any of a transaction's inputs conflict with pool entries.
    ///
    /// Returns `true` if any input outpoint is already spent by a pool
    /// transaction.
    pub fn has_conflict(&self, tx: &Transaction) -> bool {
        tx.inputs
            .iter()
            .any(|input| self.by_outpoint.contains_key(&input.previous_output))
    }

    /// Get the txids of pool entries that conflict with the given transaction.
    ///
    /// Returns a deduplicated list of txids whose inputs overlap with `tx`.
    pub fn conflicting_txids(&self, tx: &Transaction) -> Vec<Hash256> {
        let mut seen = HashSet::new();
        tx.inputs
            .iter()
            .filter_map(|input| self.by_outpoint.get(&input.previous_output).copied())
            .filter(|txid| seen.insert(*txid))
            .collect()
    }

    /// Select transactions for a block template, ordered by fee rate (highest first).
    ///
    /// Greedily fills up to `max_block_bytes` of serialized transaction data,
    /// skipping individual transactions that are too large for the remaining
    /// space (smaller transactions may still fit).
    pub fn select_transactions(&self, max_block_bytes: usize) -> Vec<&MempoolEntry> {
        let mut selected = Vec::new();
        let mut remaining = max_block_bytes;

        for (_, txid) in self.by_fee_rate.iter().rev() {
            if remaining == 0 {
                break;
            }
            if let Some(entry) = self.entries.get(txid) {
                if entry.size <= remaining {
                    selected.push(entry);
                    remaining -= entry.size;
                }
            }
        }

        selected
    }

    /// Remove transactions confirmed in a block and any that conflict.
    ///
    /// Call this when a new block is accepted into the chain. Removes:
    /// 1. Transactions whose txids appear in the block
    /// 2. Pool transactions whose inputs are now spent by block transactions
    pub fn remove_confirmed_block(&mut self, block: &Block) {
        let mut confirmed_txids = HashSet::new();
        let mut spent = HashSet::new();

        for tx in &block.transactions {
            if let Ok(txid) = tx.txid() {
                confirmed_txids.insert(txid);
            }
            for input in &tx.inputs {
                if !input.previous_output.is_null() {
                    spent.insert(input.previous_output.clone());
                }
            }
        }

        // Remove confirmed transactions.
        for txid in &confirmed_txids {
            self.remove_entry(*txid);
        }

        // Remove pool transactions that conflict with the block's spent outpoints.
        let conflicting: Vec<Hash256> = spent
            .iter()
            .filter_map(|op| self.by_outpoint.get(op).copied())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        for txid in conflicting {
            self.remove_entry(txid);
        }
    }

    /// Number of transactions in the pool.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total serialized bytes of all transactions in the pool.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Maximum transaction count limit.
    pub fn max_count(&self) -> usize {
        self.max_count
    }

    /// Maximum total bytes limit.
    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    /// Total fees of all transactions in the pool.
    pub fn total_fees(&self) -> u64 {
        self.entries.values().map(|e| e.fee).sum()
    }

    /// Iterate over all entries (arbitrary order).
    pub fn iter(&self) -> impl Iterator<Item = &MempoolEntry> {
        self.entries.values()
    }

    /// Collect all txids in the pool.
    pub fn txids(&self) -> Vec<Hash256> {
        self.entries.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::COIN;
    use crate::merkle;
    use crate::types::{BlockHeader, TxInput, TxOutput, TxType};

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Create a test transaction spending the given outpoints.
    fn make_tx(outpoints: &[OutPoint], output_value: u64, lock_time: u64) -> Transaction {
        Transaction {
            version: 1,
            tx_type: TxType::default(),
            inputs: outpoints
                .iter()
                .map(|op| TxInput {
                    previous_output: op.clone(),
                    signature: vec![0; 64],
                    public_key: vec![0; 32],
                })
                .collect(),
            outputs: vec![TxOutput {
                value: output_value,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time,
        }
    }

    /// Create an outpoint with a unique txid derived from `seed`.
    fn outpoint(seed: u8, index: u64) -> OutPoint {
        OutPoint {
            txid: Hash256([seed; 32]),
            index,
        }
    }

    /// Compute the serialized size of a transaction.
    fn tx_size(tx: &Transaction) -> usize {
        bincode::encode_to_vec(tx, bincode::config::standard())
            .unwrap()
            .len()
    }

    // ------------------------------------------------------------------
    // Basic operations
    // ------------------------------------------------------------------

    #[test]
    fn new_mempool_is_empty() {
        let pool = Mempool::new(100, 100_000);
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
        assert_eq!(pool.total_bytes(), 0);
        assert_eq!(pool.total_fees(), 0);
    }

    #[test]
    fn with_defaults_creates_pool() {
        let pool = Mempool::with_defaults();
        assert_eq!(pool.max_count(), DEFAULT_MAX_COUNT);
        assert_eq!(pool.max_bytes(), DEFAULT_MAX_BYTES);
        assert!(pool.is_empty());
    }

    #[test]
    fn insert_and_get() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let fee = 1 * COIN;

        let txid = pool.insert(tx.clone(), fee).unwrap();
        assert!(!txid.is_zero());

        let entry = pool.get(&txid).unwrap();
        assert_eq!(entry.txid, txid);
        assert_eq!(entry.fee, fee);
        assert_eq!(entry.tx, tx);
        assert!(entry.size > 0);
    }

    #[test]
    fn insert_updates_counts() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let size = tx_size(&tx);

        pool.insert(tx, COIN).unwrap();

        assert_eq!(pool.len(), 1);
        assert!(!pool.is_empty());
        assert_eq!(pool.total_bytes(), size);
        assert_eq!(pool.total_fees(), COIN);
    }

    #[test]
    fn contains_after_insert() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);

        let txid = pool.insert(tx, COIN).unwrap();
        assert!(pool.contains(&txid));
        assert!(!pool.contains(&Hash256::ZERO));
    }

    #[test]
    fn remove_returns_entry() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let txid = pool.insert(tx.clone(), COIN).unwrap();

        let entry = pool.remove(&txid).unwrap();
        assert_eq!(entry.txid, txid);
        assert_eq!(entry.tx, tx);
        assert!(pool.is_empty());
        assert_eq!(pool.total_bytes(), 0);
    }

    #[test]
    fn remove_unknown_returns_none() {
        let mut pool = Mempool::new(100, 100_000);
        assert!(pool.remove(&Hash256::ZERO).is_none());
    }

    #[test]
    fn remove_cleans_outpoint_index() {
        let mut pool = Mempool::new(100, 100_000);
        let op = outpoint(1, 0);
        let tx = make_tx(&[op.clone()], 49 * COIN, 0);
        let txid = pool.insert(tx, COIN).unwrap();

        // Outpoint should be tracked while in pool.
        let conflict_tx = make_tx(&[op.clone()], 48 * COIN, 0);
        assert!(pool.has_conflict(&conflict_tx));

        pool.remove(&txid);

        // After removal, the outpoint should be free.
        assert!(!pool.has_conflict(&conflict_tx));
    }

    #[test]
    fn txids_returns_all() {
        let mut pool = Mempool::new(100, 100_000);
        let txid1 = pool
            .insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), COIN)
            .unwrap();
        let txid2 = pool
            .insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), 2 * COIN)
            .unwrap();

        let mut txids = pool.txids();
        txids.sort();
        let mut expected = vec![txid1, txid2];
        expected.sort();
        assert_eq!(txids, expected);
    }

    #[test]
    fn iter_yields_all_entries() {
        let mut pool = Mempool::new(100, 100_000);
        pool.insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), COIN)
            .unwrap();
        pool.insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), 2 * COIN)
            .unwrap();

        let entries: Vec<_> = pool.iter().collect();
        assert_eq!(entries.len(), 2);
    }

    // ------------------------------------------------------------------
    // Duplicates
    // ------------------------------------------------------------------

    #[test]
    fn rejects_duplicate_txid() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);

        pool.insert(tx.clone(), COIN).unwrap();
        let err = pool.insert(tx, COIN).unwrap_err();
        assert!(matches!(err, MempoolError::AlreadyExists(_)));
    }

    // ------------------------------------------------------------------
    // Conflicts
    // ------------------------------------------------------------------

    #[test]
    fn rejects_conflicting_outpoint() {
        let mut pool = Mempool::new(100, 100_000);
        let op = outpoint(1, 0);

        pool.insert(make_tx(&[op.clone()], 49 * COIN, 0), COIN)
            .unwrap();

        // Different tx spending the same outpoint.
        let tx2 = make_tx(&[op], 48 * COIN, 1);
        let err = pool.insert(tx2, 2 * COIN).unwrap_err();
        assert!(matches!(err, MempoolError::Conflict { .. }));
    }

    #[test]
    fn has_conflict_true() {
        let mut pool = Mempool::new(100, 100_000);
        let op = outpoint(1, 0);
        pool.insert(make_tx(&[op.clone()], 49 * COIN, 0), COIN)
            .unwrap();

        let tx2 = make_tx(&[op], 48 * COIN, 1);
        assert!(pool.has_conflict(&tx2));
    }

    #[test]
    fn has_conflict_false() {
        let mut pool = Mempool::new(100, 100_000);
        pool.insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), COIN)
            .unwrap();

        let tx2 = make_tx(&[outpoint(2, 0)], 48 * COIN, 0);
        assert!(!pool.has_conflict(&tx2));
    }

    #[test]
    fn has_conflict_empty_pool() {
        let pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        assert!(!pool.has_conflict(&tx));
    }

    #[test]
    fn conflicting_txids_returns_correct() {
        let mut pool = Mempool::new(100, 100_000);
        let op = outpoint(1, 0);
        let txid = pool
            .insert(make_tx(&[op.clone()], 49 * COIN, 0), COIN)
            .unwrap();

        let tx2 = make_tx(&[op], 48 * COIN, 1);
        assert_eq!(pool.conflicting_txids(&tx2), vec![txid]);
    }

    #[test]
    fn conflicting_txids_empty_when_no_conflict() {
        let mut pool = Mempool::new(100, 100_000);
        pool.insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), COIN)
            .unwrap();

        let tx2 = make_tx(&[outpoint(2, 0)], 48 * COIN, 0);
        assert!(pool.conflicting_txids(&tx2).is_empty());
    }

    #[test]
    fn conflicting_txids_deduplicates() {
        let mut pool = Mempool::new(100, 100_000);
        let op1 = outpoint(1, 0);
        let op2 = outpoint(1, 1);
        // One pool tx spends both outpoints.
        let txid = pool
            .insert(make_tx(&[op1.clone(), op2.clone()], 49 * COIN, 0), COIN)
            .unwrap();

        // New tx also spends both outpoints → should return the same txid once.
        let tx2 = make_tx(&[op1, op2], 48 * COIN, 1);
        let conflicts = pool.conflicting_txids(&tx2);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0], txid);
    }

    // ------------------------------------------------------------------
    // Size limits and eviction
    // ------------------------------------------------------------------

    #[test]
    fn respects_max_count() {
        let mut pool = Mempool::new(2, 1_000_000);

        pool.insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), 1 * COIN)
            .unwrap();
        pool.insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), 2 * COIN)
            .unwrap();
        assert_eq!(pool.len(), 2);

        // Third insert with higher fee rate should evict the lowest.
        pool.insert(make_tx(&[outpoint(3, 0)], 47 * COIN, 0), 3 * COIN)
            .unwrap();
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn evicts_lowest_fee_rate() {
        let mut pool = Mempool::new(2, 1_000_000);

        let txid_low = pool
            .insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), 1_000) // low fee
            .unwrap();
        let txid_high = pool
            .insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), 10_000) // high fee
            .unwrap();

        // Insert with medium fee → should evict the low-fee tx.
        let txid_med = pool
            .insert(make_tx(&[outpoint(3, 0)], 47 * COIN, 0), 5_000) // medium fee
            .unwrap();

        assert!(!pool.contains(&txid_low));
        assert!(pool.contains(&txid_high));
        assert!(pool.contains(&txid_med));
    }

    #[test]
    fn rejects_when_fee_too_low_for_eviction() {
        let mut pool = Mempool::new(2, 1_000_000);

        pool.insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), 5_000)
            .unwrap();
        pool.insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), 10_000)
            .unwrap();

        // Insert with lower fee rate than the lowest in pool → rejected.
        let err = pool
            .insert(make_tx(&[outpoint(3, 0)], 47 * COIN, 0), 1_000)
            .unwrap_err();
        assert!(matches!(err, MempoolError::PoolFull));
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn respects_max_bytes() {
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let size = tx_size(&tx);
        // Allow exactly one transaction.
        let mut pool = Mempool::new(100, size);

        pool.insert(tx, COIN).unwrap();
        assert_eq!(pool.len(), 1);

        // Second tx doesn't fit by byte limit. Higher fee should evict.
        pool.insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), 2 * COIN)
            .unwrap();
        assert_eq!(pool.len(), 1);
    }

    // ------------------------------------------------------------------
    // Fee rate computation
    // ------------------------------------------------------------------

    #[test]
    fn fee_rate_accessor() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let fee = 1_000;

        let txid = pool.insert(tx, fee).unwrap();
        let entry = pool.get(&txid).unwrap();

        let expected_rate = compute_fee_rate(fee, entry.size);
        assert_eq!(entry.fee_rate(), expected_rate);
        assert!(entry.fee_rate() > 0);
    }

    #[test]
    fn fee_rate_zero_fee() {
        assert_eq!(compute_fee_rate(0, 100), 0);
    }

    #[test]
    fn fee_rate_zero_size() {
        assert_eq!(compute_fee_rate(1000, 0), u64::MAX);
    }

    #[test]
    fn fee_rate_precision() {
        // 999 rills / 1000 bytes = 0.999 rills/byte
        // With FEE_RATE_PRECISION = 1000: 999 * 1000 / 1000 = 999 milli-rills/byte
        assert_eq!(compute_fee_rate(999, 1000), 999);
        // Without precision this would truncate to 0.
    }

    // ------------------------------------------------------------------
    // select_transactions
    // ------------------------------------------------------------------

    #[test]
    fn select_empty_pool() {
        let pool = Mempool::new(100, 100_000);
        assert!(pool.select_transactions(100_000).is_empty());
    }

    #[test]
    fn select_returns_highest_fee_rate_first() {
        let mut pool = Mempool::new(100, 1_000_000);

        let txid_low = pool
            .insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), 1_000)
            .unwrap();
        let txid_high = pool
            .insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), 10_000)
            .unwrap();
        let txid_med = pool
            .insert(make_tx(&[outpoint(3, 0)], 47 * COIN, 0), 5_000)
            .unwrap();

        let selected = pool.select_transactions(1_000_000);
        assert_eq!(selected.len(), 3);
        assert_eq!(selected[0].txid, txid_high);
        assert_eq!(selected[1].txid, txid_med);
        assert_eq!(selected[2].txid, txid_low);
    }

    #[test]
    fn select_respects_size_budget() {
        let mut pool = Mempool::new(100, 1_000_000);

        let tx1 = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let size1 = tx_size(&tx1);
        pool.insert(tx1, 10_000).unwrap();
        pool.insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), 5_000)
            .unwrap();

        // Budget for exactly one transaction.
        let selected = pool.select_transactions(size1);
        assert_eq!(selected.len(), 1);
    }

    #[test]
    fn select_zero_budget() {
        let mut pool = Mempool::new(100, 100_000);
        pool.insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), COIN)
            .unwrap();

        assert!(pool.select_transactions(0).is_empty());
    }

    // ------------------------------------------------------------------
    // remove_confirmed_block
    // ------------------------------------------------------------------

    #[test]
    fn remove_confirmed_removes_block_txids() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let txid = pool.insert(tx.clone(), COIN).unwrap();

        // Unrelated pool tx should survive.
        let txid_other = pool
            .insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), COIN)
            .unwrap();

        // Build a block containing the first tx.
        let coinbase = Transaction {
            version: 1,
            tx_type: TxType::default(),
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        let block_txs = vec![coinbase, tx];
        let txids: Vec<Hash256> = block_txs.iter().map(|t| t.txid().unwrap()).collect();
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash: Hash256::ZERO,
                merkle_root: merkle::merkle_root(&txids),
                timestamp: 0,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: block_txs,
        };

        pool.remove_confirmed_block(&block);

        assert!(!pool.contains(&txid));
        assert!(pool.contains(&txid_other));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn remove_confirmed_removes_conflicting_txs() {
        let mut pool = Mempool::new(100, 100_000);
        let op = outpoint(1, 0);
        // Pool has tx spending outpoint(1,0).
        let pool_txid = pool
            .insert(make_tx(&[op.clone()], 49 * COIN, 0), COIN)
            .unwrap();

        // Block contains a *different* tx spending the same outpoint.
        let block_tx = make_tx(&[op], 48 * COIN, 99);
        let coinbase = Transaction {
            version: 1,
            tx_type: TxType::default(),
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        let block_txs = vec![coinbase, block_tx];
        let txids: Vec<Hash256> = block_txs.iter().map(|t| t.txid().unwrap()).collect();
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash: Hash256::ZERO,
                merkle_root: merkle::merkle_root(&txids),
                timestamp: 0,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: block_txs,
        };

        pool.remove_confirmed_block(&block);

        // Pool tx is gone because its input was spent in the block.
        assert!(!pool.contains(&pool_txid));
        assert!(pool.is_empty());
    }

    #[test]
    fn remove_confirmed_unrelated_survives() {
        let mut pool = Mempool::new(100, 100_000);
        let txid_survivor = pool
            .insert(make_tx(&[outpoint(99, 0)], 49 * COIN, 0), COIN)
            .unwrap();

        let coinbase = Transaction {
            version: 1,
            tx_type: TxType::default(),
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        let block_txs = vec![coinbase];
        let txids: Vec<Hash256> = block_txs.iter().map(|t| t.txid().unwrap()).collect();
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash: Hash256::ZERO,
                merkle_root: merkle::merkle_root(&txids),
                timestamp: 0,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: block_txs,
        };

        pool.remove_confirmed_block(&block);
        assert!(pool.contains(&txid_survivor));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn remove_confirmed_empty_pool_noop() {
        let mut pool = Mempool::new(100, 100_000);
        let coinbase = Transaction {
            version: 1,
            tx_type: TxType::default(),
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        let block_txs = vec![coinbase];
        let txids: Vec<Hash256> = block_txs.iter().map(|t| t.txid().unwrap()).collect();
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash: Hash256::ZERO,
                merkle_root: merkle::merkle_root(&txids),
                timestamp: 0,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: block_txs,
        };

        pool.remove_confirmed_block(&block);
        assert!(pool.is_empty());
    }

    // ------------------------------------------------------------------
    // Total fees
    // ------------------------------------------------------------------

    #[test]
    fn total_fees_sums_correctly() {
        let mut pool = Mempool::new(100, 100_000);
        pool.insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), 1_000)
            .unwrap();
        pool.insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), 2_000)
            .unwrap();

        assert_eq!(pool.total_fees(), 3_000);
    }

    #[test]
    fn total_fees_after_remove() {
        let mut pool = Mempool::new(100, 100_000);
        let txid = pool
            .insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), 1_000)
            .unwrap();
        pool.insert(make_tx(&[outpoint(2, 0)], 48 * COIN, 0), 2_000)
            .unwrap();

        pool.remove(&txid);
        assert_eq!(pool.total_fees(), 2_000);
    }

    // ------------------------------------------------------------------
    // Total bytes tracking
    // ------------------------------------------------------------------

    #[test]
    fn total_bytes_tracks_insert_remove() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let size = tx_size(&tx);

        let txid = pool.insert(tx, COIN).unwrap();
        assert_eq!(pool.total_bytes(), size);

        pool.remove(&txid);
        assert_eq!(pool.total_bytes(), 0);
    }

    #[test]
    fn total_bytes_multi() {
        let mut pool = Mempool::new(100, 100_000);
        let tx1 = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let tx2 = make_tx(&[outpoint(2, 0)], 48 * COIN, 0);
        let s1 = tx_size(&tx1);
        let s2 = tx_size(&tx2);

        pool.insert(tx1, COIN).unwrap();
        pool.insert(tx2, COIN).unwrap();
        assert_eq!(pool.total_bytes(), s1 + s2);
    }

    // ------------------------------------------------------------------
    // Error display
    // ------------------------------------------------------------------

    #[test]
    fn error_variants_display() {
        let errors: Vec<MempoolError> = vec![
            MempoolError::AlreadyExists("abc".into()),
            MempoolError::Conflict {
                new_txid: "new".into(),
                existing_txid: "old".into(),
                outpoint: "op:0".into(),
            },
            MempoolError::PoolFull,
            MempoolError::FeeTooLow { fee: 100, minimum: 1000 },
            MempoolError::Internal("oops".into()),
        ];
        for e in &errors {
            assert!(!format!("{e}").is_empty());
        }
    }

    // ------------------------------------------------------------------
    // MempoolEntry
    // ------------------------------------------------------------------

    #[test]
    fn entry_clone() {
        let mut pool = Mempool::new(100, 100_000);
        let txid = pool
            .insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), COIN)
            .unwrap();
        let entry = pool.get(&txid).unwrap();
        let cloned = entry.clone();
        assert_eq!(cloned.txid, entry.txid);
        assert_eq!(cloned.fee, entry.fee);
    }

    #[test]
    fn entry_debug() {
        let mut pool = Mempool::new(100, 100_000);
        let txid = pool
            .insert(make_tx(&[outpoint(1, 0)], 49 * COIN, 0), COIN)
            .unwrap();
        let entry = pool.get(&txid).unwrap();
        let debug = format!("{entry:?}");
        assert!(debug.contains("fee"));
    }

    // ------------------------------------------------------------------
    // Min fee enforcement
    // ------------------------------------------------------------------

    #[test]
    fn rejects_zero_fee() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let err = pool.insert(tx, 0).unwrap_err();
        assert!(matches!(err, MempoolError::FeeTooLow { fee: 0, minimum: 1000 }));
    }

    #[test]
    fn rejects_fee_below_minimum() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        let err = pool.insert(tx, 999).unwrap_err();
        assert!(matches!(err, MempoolError::FeeTooLow { fee: 999, minimum: 1000 }));
    }

    #[test]
    fn accepts_fee_at_minimum() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        assert!(pool.insert(tx, 1000).is_ok());
    }

    #[test]
    fn accepts_fee_above_minimum() {
        let mut pool = Mempool::new(100, 100_000);
        let tx = make_tx(&[outpoint(1, 0)], 49 * COIN, 0);
        assert!(pool.insert(tx, 5000).is_ok());
    }

    #[test]
    fn fee_too_low_error_display() {
        let e = MempoolError::FeeTooLow { fee: 500, minimum: 1000 };
        assert_eq!(e.to_string(), "fee too low: 500 < minimum 1000");
    }

    // ------------------------------------------------------------------
    // Multiple outpoints per tx
    // ------------------------------------------------------------------

    #[test]
    fn multi_input_tx_tracks_all_outpoints() {
        let mut pool = Mempool::new(100, 100_000);
        let op1 = outpoint(1, 0);
        let op2 = outpoint(2, 0);
        pool.insert(make_tx(&[op1.clone(), op2.clone()], 49 * COIN, 0), COIN)
            .unwrap();

        // Both outpoints should be tracked.
        assert!(pool.has_conflict(&make_tx(&[op1], 40 * COIN, 1)));
        assert!(pool.has_conflict(&make_tx(&[op2], 40 * COIN, 2)));
    }

    #[test]
    fn multi_input_tx_removal_frees_all_outpoints() {
        let mut pool = Mempool::new(100, 100_000);
        let op1 = outpoint(1, 0);
        let op2 = outpoint(2, 0);
        let txid = pool
            .insert(make_tx(&[op1.clone(), op2.clone()], 49 * COIN, 0), COIN)
            .unwrap();

        pool.remove(&txid);

        assert!(!pool.has_conflict(&make_tx(&[op1], 40 * COIN, 1)));
        assert!(!pool.has_conflict(&make_tx(&[op2], 40 * COIN, 2)));
    }
}
