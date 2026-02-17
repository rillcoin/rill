//! Chain state storage interface and in-memory implementation.
//!
//! Provides the [`ChainStore`] trait for UTXO set management, block storage,
//! and chain tip tracking. The [`MemoryChainStore`] is suitable for testing;
//! the production node uses RocksDB (rill-node).
//!
//! Blocks passed to [`ChainStore::connect_block`] must already be validated.
//! The store only performs minimal sanity checks (height consistency, no
//! duplicate blocks).

use std::collections::HashMap;

use crate::error::{ChainStateError, RillError};
use crate::types::{Block, BlockHeader, Hash256, OutPoint, Transaction, UtxoEntry};

/// Result of connecting a block to the chain state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectBlockResult {
    /// Number of new UTXOs created by this block's transactions.
    pub utxos_created: usize,
    /// Number of UTXOs spent by this block's non-coinbase inputs.
    pub utxos_spent: usize,
}

/// Result of disconnecting the tip block from the chain state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisconnectBlockResult {
    /// Number of UTXOs restored (previously spent, now unspent again).
    pub utxos_restored: usize,
    /// Number of UTXOs removed (created by the disconnected block).
    pub utxos_removed: usize,
}

/// Undo data for reverting a connected block.
///
/// Stores the UTXOs consumed by the block's transactions so they can be
/// restored during chain reorganization.
#[derive(Clone, Debug)]
struct BlockUndo {
    /// Spent UTXOs in the order they were consumed.
    spent_utxos: Vec<(OutPoint, UtxoEntry)>,
}

/// Mutable chain state storage interface.
///
/// Provides UTXO set management, block storage, and chain tip tracking.
/// Assumes all blocks passed to [`connect_block`](ChainStore::connect_block)
/// have already been validated by the consensus layer.
///
/// Not thread-safe — callers should wrap in a `Mutex` or `RwLock` if
/// concurrent access is needed.
pub trait ChainStore: Send + Sync {
    /// Connect a validated block at the given height.
    ///
    /// Updates the UTXO set (spends inputs, creates outputs), stores the
    /// block and header, and advances the chain tip. Stores undo data
    /// for later disconnection.
    ///
    /// # Errors
    ///
    /// - [`ChainStateError::HeightMismatch`] if `height` is not the expected next height
    /// - [`ChainStateError::DuplicateBlock`] if the block hash already exists
    fn connect_block(&mut self, block: &Block, height: u64) -> Result<ConnectBlockResult, RillError>;

    /// Disconnect the current tip block, reverting UTXO changes.
    ///
    /// Uses stored undo data to restore spent UTXOs and remove created
    /// UTXOs. The chain tip moves to the previous block.
    ///
    /// # Errors
    ///
    /// - [`ChainStateError::EmptyChain`] if no blocks are connected
    /// - [`ChainStateError::BlockNotFound`] if the tip block is missing
    /// - [`ChainStateError::UndoDataMissing`] if undo data was not stored
    fn disconnect_tip(&mut self) -> Result<DisconnectBlockResult, RillError>;

    /// Look up a UTXO by outpoint. Returns `None` if spent or unknown.
    fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UtxoEntry>, RillError>;

    /// Check whether a UTXO exists and is unspent.
    ///
    /// Default implementation delegates to [`get_utxo`](Self::get_utxo).
    fn contains_utxo(&self, outpoint: &OutPoint) -> Result<bool, RillError> {
        Ok(self.get_utxo(outpoint)?.is_some())
    }

    /// Current chain tip as `(height, block_hash)`.
    ///
    /// Returns `(0, Hash256::ZERO)` if no blocks have been connected.
    fn chain_tip(&self) -> Result<(u64, Hash256), RillError>;

    /// Get a block header by its hash. Returns `None` if not found.
    fn get_block_header(&self, hash: &Hash256) -> Result<Option<BlockHeader>, RillError>;

    /// Get a full block by its hash. Returns `None` if not found.
    fn get_block(&self, hash: &Hash256) -> Result<Option<Block>, RillError>;

    /// Get the block hash at a given height. Returns `None` if height exceeds tip.
    fn get_block_hash(&self, height: u64) -> Result<Option<Hash256>, RillError>;

    /// Number of unspent transaction outputs in the set.
    fn utxo_count(&self) -> usize;

    /// Whether no blocks have been connected.
    fn is_empty(&self) -> bool;

    /// Iterate over all UTXOs. Used for balance queries and UTXO scanning.
    /// Default implementation returns empty vec (override for production).
    fn iter_utxos(&self) -> Result<Vec<(OutPoint, UtxoEntry)>, RillError> {
        Ok(Vec::new())
    }
}

/// In-memory chain state storage for testing.
///
/// Stores everything in `HashMap`s with no persistence. Not suitable for
/// production use (no crash recovery, unbounded memory growth).
pub struct MemoryChainStore {
    /// UTXO set: outpoint → entry.
    utxos: HashMap<OutPoint, UtxoEntry>,
    /// Full blocks by hash.
    blocks: HashMap<Hash256, Block>,
    /// Block headers by hash.
    headers: HashMap<Hash256, BlockHeader>,
    /// Height → block hash mapping.
    height_to_hash: HashMap<u64, Hash256>,
    /// Undo data by block hash (for disconnect_tip).
    undo_data: HashMap<Hash256, BlockUndo>,
    /// Current tip height.
    tip_height: u64,
    /// Current tip block hash. `Hash256::ZERO` means empty chain.
    tip_hash: Hash256,
}

impl MemoryChainStore {
    /// Create a new empty chain store.
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            blocks: HashMap::new(),
            headers: HashMap::new(),
            height_to_hash: HashMap::new(),
            undo_data: HashMap::new(),
            tip_height: 0,
            tip_hash: Hash256::ZERO,
        }
    }

    /// Number of full blocks stored.
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Number of undo records stored.
    pub fn undo_count(&self) -> usize {
        self.undo_data.len()
    }

    /// Process a transaction's inputs: remove spent UTXOs, record undo data.
    ///
    /// Coinbase transactions are skipped (no real inputs to spend).
    /// Returns the number of UTXOs spent, or an error if a UTXO is missing.
    ///
    /// VULN-02 fix: This now returns an error if any non-coinbase input's UTXO
    /// is not found, preventing phantom spends during reorgs.
    fn spend_inputs(
        &mut self,
        tx: &Transaction,
        undo: &mut BlockUndo,
    ) -> Result<usize, RillError> {
        if tx.is_coinbase() {
            return Ok(0);
        }
        let mut spent = 0;
        for input in &tx.inputs {
            let entry = self.utxos.remove(&input.previous_output).ok_or_else(|| {
                RillError::ChainState(ChainStateError::MissingUtxo(
                    input.previous_output.to_string(),
                ))
            })?;
            undo.spent_utxos.push((input.previous_output.clone(), entry));
            spent += 1;
        }
        Ok(spent)
    }

    /// Process a transaction's outputs: create new UTXOs.
    ///
    /// Returns the number of UTXOs created, or an error if txid
    /// computation fails.
    fn create_outputs(
        &mut self,
        tx: &Transaction,
        height: u64,
    ) -> Result<usize, RillError> {
        let txid = tx.txid().map_err(RillError::from)?;
        let is_coinbase = tx.is_coinbase();
        let mut created = 0;
        for (index, output) in tx.outputs.iter().enumerate() {
            let outpoint = OutPoint {
                txid,
                index: index as u64,
            };
            let entry = UtxoEntry {
                output: output.clone(),
                block_height: height,
                is_coinbase,
                cluster_id: Hash256::ZERO, // Phase 1: no clustering
            };
            self.utxos.insert(outpoint, entry);
            created += 1;
        }
        Ok(created)
    }
}

impl Default for MemoryChainStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainStore for MemoryChainStore {
    fn connect_block(&mut self, block: &Block, height: u64) -> Result<ConnectBlockResult, RillError> {
        // Validate height consistency.
        if self.tip_hash == Hash256::ZERO {
            if height != 0 {
                return Err(ChainStateError::HeightMismatch {
                    expected: 0,
                    got: height,
                }.into());
            }
        } else if height != self.tip_height + 1 {
            return Err(ChainStateError::HeightMismatch {
                expected: self.tip_height + 1,
                got: height,
            }.into());
        }

        let block_hash = block.header.hash();

        // Reject duplicate blocks.
        if self.blocks.contains_key(&block_hash) {
            return Err(ChainStateError::DuplicateBlock(block_hash.to_string()).into());
        }

        let mut undo = BlockUndo { spent_utxos: Vec::new() };
        let mut total_spent = 0;
        let mut total_created = 0;

        // Process transactions: spend inputs, then create outputs.
        for tx in &block.transactions {
            total_spent += self.spend_inputs(tx, &mut undo)?;
            total_created += self.create_outputs(tx, height)?;
        }

        // Store block, header, height mapping, undo data.
        self.headers.insert(block_hash, block.header.clone());
        self.blocks.insert(block_hash, block.clone());
        self.height_to_hash.insert(height, block_hash);
        self.undo_data.insert(block_hash, undo);

        // Update tip.
        self.tip_height = height;
        self.tip_hash = block_hash;

        Ok(ConnectBlockResult {
            utxos_created: total_created,
            utxos_spent: total_spent,
        })
    }

    fn disconnect_tip(&mut self) -> Result<DisconnectBlockResult, RillError> {
        if self.tip_hash == Hash256::ZERO {
            return Err(ChainStateError::EmptyChain.into());
        }

        let tip_hash = self.tip_hash;
        let tip_height = self.tip_height;

        // Get the tip block.
        let block = self.blocks.get(&tip_hash)
            .cloned()
            .ok_or_else(|| ChainStateError::BlockNotFound(tip_hash.to_string()))?;

        // Get undo data.
        let undo = self.undo_data.remove(&tip_hash)
            .ok_or_else(|| ChainStateError::UndoDataMissing(tip_hash.to_string()))?;

        // Remove UTXOs created by this block (reverse transaction order).
        let mut total_removed = 0;
        for tx in block.transactions.iter().rev() {
            let txid = tx.txid().map_err(RillError::from)?;
            for (index, _) in tx.outputs.iter().enumerate() {
                let outpoint = OutPoint {
                    txid,
                    index: index as u64,
                };
                if self.utxos.remove(&outpoint).is_some() {
                    total_removed += 1;
                }
            }
        }

        // Restore spent UTXOs from undo data.
        let total_restored = undo.spent_utxos.len();
        for (outpoint, entry) in undo.spent_utxos {
            self.utxos.insert(outpoint, entry);
        }

        // Remove block from height index.
        self.height_to_hash.remove(&tip_height);

        // Update tip.
        if tip_height == 0 {
            // Disconnected genesis — back to empty chain.
            self.tip_height = 0;
            self.tip_hash = Hash256::ZERO;
        } else {
            self.tip_height = tip_height - 1;
            self.tip_hash = block.header.prev_hash;
        }

        Ok(DisconnectBlockResult {
            utxos_restored: total_restored,
            utxos_removed: total_removed,
        })
    }

    fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UtxoEntry>, RillError> {
        Ok(self.utxos.get(outpoint).cloned())
    }

    fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
        Ok((self.tip_height, self.tip_hash))
    }

    fn get_block_header(&self, hash: &Hash256) -> Result<Option<BlockHeader>, RillError> {
        Ok(self.headers.get(hash).cloned())
    }

    fn get_block(&self, hash: &Hash256) -> Result<Option<Block>, RillError> {
        Ok(self.blocks.get(hash).cloned())
    }

    fn get_block_hash(&self, height: u64) -> Result<Option<Hash256>, RillError> {
        Ok(self.height_to_hash.get(&height).copied())
    }

    fn utxo_count(&self) -> usize {
        self.utxos.len()
    }

    fn is_empty(&self) -> bool {
        self.tip_hash == Hash256::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::COIN;
    use crate::error::ChainStateError;
    use crate::merkle;
    use crate::types::{TxInput, TxOutput};

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Create a coinbase transaction paying to the given pubkey hash.
    fn make_coinbase(value: u64, pubkey_hash: Hash256) -> Transaction {
        Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value,
                pubkey_hash,
            }],
            lock_time: 0,
        }
    }

    /// Create a coinbase with unique data to produce a unique txid.
    ///
    /// Sets `lock_time: height` matching the production consensus engine so that
    /// coinbases at different heights always have distinct txids.
    fn make_coinbase_unique(value: u64, pubkey_hash: Hash256, height: u64) -> Transaction {
        Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: height.to_le_bytes().to_vec(),
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value,
                pubkey_hash,
            }],
            lock_time: height,
        }
    }

    /// Create a regular transaction spending the given outpoint.
    fn make_tx(outpoints: &[OutPoint], output_value: u64, pubkey_hash: Hash256) -> Transaction {
        Transaction {
            version: 1,
            inputs: outpoints.iter().map(|op| TxInput {
                previous_output: op.clone(),
                signature: vec![0; 64],
                public_key: vec![0; 32],
            }).collect(),
            outputs: vec![TxOutput {
                value: output_value,
                pubkey_hash,
            }],
            lock_time: 0,
        }
    }

    /// Create a regular transaction with multiple outputs.
    fn make_tx_multi_out(
        outpoints: &[OutPoint],
        outputs: &[(u64, Hash256)],
    ) -> Transaction {
        Transaction {
            version: 1,
            inputs: outpoints.iter().map(|op| TxInput {
                previous_output: op.clone(),
                signature: vec![0; 64],
                public_key: vec![0; 32],
            }).collect(),
            outputs: outputs.iter().map(|(value, pkh)| TxOutput {
                value: *value,
                pubkey_hash: *pkh,
            }).collect(),
            lock_time: 0,
        }
    }

    /// Build a block from transactions, computing the merkle root.
    fn make_block(prev_hash: Hash256, timestamp: u64, txs: Vec<Transaction>) -> Block {
        let txids: Vec<Hash256> = txs.iter().map(|tx| tx.txid().unwrap()).collect();
        Block {
            header: BlockHeader {
                version: 1,
                prev_hash,
                merkle_root: merkle::merkle_root(&txids),
                timestamp,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: txs,
        }
    }

    /// Simple pubkey hash derived from a seed byte.
    fn pkh(seed: u8) -> Hash256 {
        Hash256([seed; 32])
    }

    // ------------------------------------------------------------------
    // Empty store
    // ------------------------------------------------------------------

    #[test]
    fn new_store_is_empty() {
        let store = MemoryChainStore::new();
        assert!(store.is_empty());
        assert_eq!(store.utxo_count(), 0);
        assert_eq!(store.block_count(), 0);
        assert_eq!(store.undo_count(), 0);
    }

    #[test]
    fn default_store_is_empty() {
        let store = MemoryChainStore::default();
        assert!(store.is_empty());
    }

    #[test]
    fn empty_store_chain_tip() {
        let store = MemoryChainStore::new();
        let (height, hash) = store.chain_tip().unwrap();
        assert_eq!(height, 0);
        assert_eq!(hash, Hash256::ZERO);
    }

    #[test]
    fn empty_store_get_utxo_returns_none() {
        let store = MemoryChainStore::new();
        let op = OutPoint { txid: Hash256([1; 32]), index: 0 };
        assert_eq!(store.get_utxo(&op).unwrap(), None);
    }

    #[test]
    fn empty_store_contains_utxo_returns_false() {
        let store = MemoryChainStore::new();
        let op = OutPoint { txid: Hash256([1; 32]), index: 0 };
        assert!(!store.contains_utxo(&op).unwrap());
    }

    #[test]
    fn empty_store_get_block_returns_none() {
        let store = MemoryChainStore::new();
        assert_eq!(store.get_block(&Hash256([1; 32])).unwrap(), None);
    }

    #[test]
    fn empty_store_get_block_header_returns_none() {
        let store = MemoryChainStore::new();
        assert_eq!(store.get_block_header(&Hash256([1; 32])).unwrap(), None);
    }

    #[test]
    fn empty_store_get_block_hash_returns_none() {
        let store = MemoryChainStore::new();
        assert_eq!(store.get_block_hash(0).unwrap(), None);
    }

    // ------------------------------------------------------------------
    // Connect genesis block
    // ------------------------------------------------------------------

    #[test]
    fn connect_genesis_block() {
        let mut store = MemoryChainStore::new();
        let coinbase = make_coinbase(50 * COIN, pkh(0xAA));
        let block = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);
        let block_hash = block.header.hash();

        let result = store.connect_block(&block, 0).unwrap();
        assert_eq!(result.utxos_created, 1);
        assert_eq!(result.utxos_spent, 0);

        assert!(!store.is_empty());
        assert_eq!(store.utxo_count(), 1);
        assert_eq!(store.block_count(), 1);
        assert_eq!(store.undo_count(), 1);

        let (height, hash) = store.chain_tip().unwrap();
        assert_eq!(height, 0);
        assert_eq!(hash, block_hash);
    }

    #[test]
    fn connect_genesis_creates_utxos() {
        let mut store = MemoryChainStore::new();
        let coinbase = make_coinbase(50 * COIN, pkh(0xAA));
        let coinbase_txid = coinbase.txid().unwrap();
        let block = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);

        store.connect_block(&block, 0).unwrap();

        let utxo = store.get_utxo(&OutPoint { txid: coinbase_txid, index: 0 }).unwrap();
        assert!(utxo.is_some());
        let entry = utxo.unwrap();
        assert_eq!(entry.output.value, 50 * COIN);
        assert_eq!(entry.output.pubkey_hash, pkh(0xAA));
        assert_eq!(entry.block_height, 0);
        assert!(entry.is_coinbase);
    }

    #[test]
    fn connect_genesis_stores_block() {
        let mut store = MemoryChainStore::new();
        let coinbase = make_coinbase(50 * COIN, pkh(0xAA));
        let block = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);
        let block_hash = block.header.hash();

        store.connect_block(&block, 0).unwrap();

        let stored = store.get_block(&block_hash).unwrap().unwrap();
        assert_eq!(stored, block);

        let header = store.get_block_header(&block_hash).unwrap().unwrap();
        assert_eq!(header, block.header);

        let hash_at_0 = store.get_block_hash(0).unwrap().unwrap();
        assert_eq!(hash_at_0, block_hash);
    }

    #[test]
    fn connect_genesis_rejects_wrong_height() {
        let mut store = MemoryChainStore::new();
        let coinbase = make_coinbase(50 * COIN, pkh(0xAA));
        let block = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);

        let err = store.connect_block(&block, 1).unwrap_err();
        let chain_err: ChainStateError = match err {
            RillError::ChainState(e) => e,
            _ => panic!("expected ChainStateError"),
        };
        assert_eq!(chain_err, ChainStateError::HeightMismatch { expected: 0, got: 1 });
    }

    // ------------------------------------------------------------------
    // Connect multiple blocks
    // ------------------------------------------------------------------

    #[test]
    fn connect_two_blocks() {
        let mut store = MemoryChainStore::new();

        // Block 0: coinbase creates 50 RILL.
        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        // Block 1: coinbase creates 50 RILL.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(hash0, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();
        let result = store.connect_block(&block1, 1).unwrap();
        assert_eq!(result.utxos_created, 1);
        assert_eq!(result.utxos_spent, 0);

        let (height, hash) = store.chain_tip().unwrap();
        assert_eq!(height, 1);
        assert_eq!(hash, hash1);
        assert_eq!(store.utxo_count(), 2);
        assert_eq!(store.block_count(), 2);
    }

    #[test]
    fn connect_block_with_spending_tx() {
        let mut store = MemoryChainStore::new();

        // Block 0: coinbase creates 50 RILL to pkh(0xAA).
        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let cb0_txid = cb0.txid().unwrap();
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        // Block 1: coinbase + tx spending block 0 coinbase output.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let spend_tx = make_tx(
            &[OutPoint { txid: cb0_txid, index: 0 }],
            49 * COIN,
            pkh(0xCC),
        );
        let block1 = make_block(hash0, 1_000_060, vec![cb1, spend_tx]);
        let result = store.connect_block(&block1, 1).unwrap();

        assert_eq!(result.utxos_created, 2); // coinbase + spend output
        assert_eq!(result.utxos_spent, 1);   // spent the block 0 coinbase

        // Original UTXO is now spent.
        assert_eq!(store.get_utxo(&OutPoint { txid: cb0_txid, index: 0 }).unwrap(), None);
        // New UTXO exists.
        assert_eq!(store.utxo_count(), 2); // block 1 coinbase + spend output
    }

    #[test]
    fn connect_block_rejects_wrong_height() {
        let mut store = MemoryChainStore::new();
        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        store.connect_block(&block0, 0).unwrap();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(block0.header.hash(), 1_000_060, vec![cb1]);

        // Try connecting at height 5 instead of 1.
        let err = store.connect_block(&block1, 5).unwrap_err();
        let chain_err: ChainStateError = match err {
            RillError::ChainState(e) => e,
            _ => panic!("expected ChainStateError"),
        };
        assert_eq!(chain_err, ChainStateError::HeightMismatch { expected: 1, got: 5 });
    }

    #[test]
    fn connect_block_rejects_duplicate() {
        let mut store = MemoryChainStore::new();
        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0.clone()]);
        store.connect_block(&block0, 0).unwrap();

        // Try to connect the same block again at height 1.
        // Different height but same hash — should be rejected.
        let err = store.connect_block(&block0, 1).unwrap_err();
        let chain_err: ChainStateError = match err {
            RillError::ChainState(e) => e,
            _ => panic!("expected ChainStateError"),
        };
        assert!(matches!(chain_err, ChainStateError::DuplicateBlock(_)));
    }

    // ------------------------------------------------------------------
    // Multi-output transactions
    // ------------------------------------------------------------------

    #[test]
    fn connect_block_multi_output_coinbase() {
        let mut store = MemoryChainStore::new();
        let coinbase = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![
                TxOutput { value: 30 * COIN, pubkey_hash: pkh(0xAA) },
                TxOutput { value: 20 * COIN, pubkey_hash: pkh(0xBB) },
            ],
            lock_time: 0,
        };
        let cb_txid = coinbase.txid().unwrap();
        let block = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);

        let result = store.connect_block(&block, 0).unwrap();
        assert_eq!(result.utxos_created, 2);

        let utxo0 = store.get_utxo(&OutPoint { txid: cb_txid, index: 0 }).unwrap().unwrap();
        assert_eq!(utxo0.output.value, 30 * COIN);
        let utxo1 = store.get_utxo(&OutPoint { txid: cb_txid, index: 1 }).unwrap().unwrap();
        assert_eq!(utxo1.output.value, 20 * COIN);
    }

    #[test]
    fn connect_block_multi_output_regular_tx() {
        let mut store = MemoryChainStore::new();

        // Block 0: coinbase with 50 RILL.
        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let cb0_txid = cb0.txid().unwrap();
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        // Block 1: tx with 2 outputs.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let spend = make_tx_multi_out(
            &[OutPoint { txid: cb0_txid, index: 0 }],
            &[(30 * COIN, pkh(0xCC)), (19 * COIN, pkh(0xDD))],
        );
        let spend_txid = spend.txid().unwrap();
        let block1 = make_block(hash0, 1_000_060, vec![cb1, spend]);
        store.connect_block(&block1, 1).unwrap();

        // 2 UTXOs from spend + 1 from coinbase = 3 total (block 0 coinbase spent).
        assert_eq!(store.utxo_count(), 3);

        let out0 = store.get_utxo(&OutPoint { txid: spend_txid, index: 0 }).unwrap().unwrap();
        assert_eq!(out0.output.value, 30 * COIN);
        assert!(!out0.is_coinbase);

        let out1 = store.get_utxo(&OutPoint { txid: spend_txid, index: 1 }).unwrap().unwrap();
        assert_eq!(out1.output.value, 19 * COIN);
    }

    // ------------------------------------------------------------------
    // Disconnect tip
    // ------------------------------------------------------------------

    #[test]
    fn disconnect_tip_empty_chain_errors() {
        let mut store = MemoryChainStore::new();
        let err = store.disconnect_tip().unwrap_err();
        let chain_err: ChainStateError = match err {
            RillError::ChainState(e) => e,
            _ => panic!("expected ChainStateError"),
        };
        assert_eq!(chain_err, ChainStateError::EmptyChain);
    }

    #[test]
    fn disconnect_genesis_returns_to_empty() {
        let mut store = MemoryChainStore::new();
        let coinbase = make_coinbase(50 * COIN, pkh(0xAA));
        let block = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);
        store.connect_block(&block, 0).unwrap();

        let result = store.disconnect_tip().unwrap();
        assert_eq!(result.utxos_removed, 1);
        assert_eq!(result.utxos_restored, 0);

        assert!(store.is_empty());
        assert_eq!(store.utxo_count(), 0);
        let (height, hash) = store.chain_tip().unwrap();
        assert_eq!(height, 0);
        assert_eq!(hash, Hash256::ZERO);
    }

    #[test]
    fn disconnect_restores_spent_utxos() {
        let mut store = MemoryChainStore::new();

        // Block 0: coinbase.
        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let cb0_txid = cb0.txid().unwrap();
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        // Block 1: spends the coinbase.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let spend = make_tx(
            &[OutPoint { txid: cb0_txid, index: 0 }],
            49 * COIN,
            pkh(0xCC),
        );
        let block1 = make_block(hash0, 1_000_060, vec![cb1, spend]);
        store.connect_block(&block1, 1).unwrap();

        // Verify coinbase 0 is spent.
        assert_eq!(store.get_utxo(&OutPoint { txid: cb0_txid, index: 0 }).unwrap(), None);

        // Disconnect block 1.
        let result = store.disconnect_tip().unwrap();
        assert_eq!(result.utxos_removed, 2);   // coinbase 1 + spend output
        assert_eq!(result.utxos_restored, 1);   // coinbase 0 restored

        // Coinbase 0 UTXO is back.
        let restored = store.get_utxo(&OutPoint { txid: cb0_txid, index: 0 }).unwrap().unwrap();
        assert_eq!(restored.output.value, 50 * COIN);
        assert!(restored.is_coinbase);
        assert_eq!(restored.block_height, 0);

        // Tip is back to block 0.
        let (height, hash) = store.chain_tip().unwrap();
        assert_eq!(height, 0);
        assert_eq!(hash, hash0);
        assert_eq!(store.utxo_count(), 1);
    }

    #[test]
    fn disconnect_removes_height_mapping() {
        let mut store = MemoryChainStore::new();

        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(hash0, 1_000_060, vec![cb1]);
        store.connect_block(&block1, 1).unwrap();

        assert!(store.get_block_hash(1).unwrap().is_some());

        store.disconnect_tip().unwrap();
        assert_eq!(store.get_block_hash(1).unwrap(), None);
        // Block 0 hash still available.
        assert_eq!(store.get_block_hash(0).unwrap(), Some(hash0));
    }

    #[test]
    fn disconnect_undo_data_removed() {
        let mut store = MemoryChainStore::new();
        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        store.connect_block(&block0, 0).unwrap();
        assert_eq!(store.undo_count(), 1);

        store.disconnect_tip().unwrap();
        assert_eq!(store.undo_count(), 0);
    }

    // ------------------------------------------------------------------
    // Connect-disconnect roundtrip
    // ------------------------------------------------------------------

    #[test]
    fn connect_disconnect_roundtrip_three_blocks() {
        let mut store = MemoryChainStore::new();

        // Connect 3 blocks.
        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(hash0, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        let cb2 = make_coinbase_unique(50 * COIN, pkh(0xCC), 2);
        let block2 = make_block(hash1, 1_000_120, vec![cb2]);
        store.connect_block(&block2, 2).unwrap();

        assert_eq!(store.utxo_count(), 3);
        assert_eq!(store.block_count(), 3);

        // Disconnect all 3.
        store.disconnect_tip().unwrap();
        assert_eq!(store.chain_tip().unwrap(), (1, hash1));
        assert_eq!(store.utxo_count(), 2);

        store.disconnect_tip().unwrap();
        assert_eq!(store.chain_tip().unwrap(), (0, hash0));
        assert_eq!(store.utxo_count(), 1);

        store.disconnect_tip().unwrap();
        assert!(store.is_empty());
        assert_eq!(store.utxo_count(), 0);
    }

    #[test]
    fn connect_disconnect_reconnect() {
        let mut store = MemoryChainStore::new();

        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let cb0_txid = cb0.txid().unwrap();
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(hash0, 1_000_060, vec![cb1]);
        store.connect_block(&block1, 1).unwrap();

        // Disconnect block 1.
        store.disconnect_tip().unwrap();

        // Connect a different block 1.
        let cb1_alt = make_coinbase_unique(50 * COIN, pkh(0xDD), 100);
        let spend_alt = make_tx(
            &[OutPoint { txid: cb0_txid, index: 0 }],
            48 * COIN,
            pkh(0xEE),
        );
        let block1_alt = make_block(hash0, 1_000_061, vec![cb1_alt, spend_alt]);
        let result = store.connect_block(&block1_alt, 1).unwrap();

        assert_eq!(result.utxos_created, 2);
        assert_eq!(result.utxos_spent, 1);
        // coinbase_alt + spend_alt output, block 0 coinbase spent
        assert_eq!(store.utxo_count(), 2);
    }

    // ------------------------------------------------------------------
    // Block and header lookups
    // ------------------------------------------------------------------

    #[test]
    fn get_block_after_connect() {
        let mut store = MemoryChainStore::new();
        let cb0 = make_coinbase(50 * COIN, pkh(0xAA));
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        assert_eq!(store.get_block(&hash0).unwrap(), Some(block0.clone()));
        assert_eq!(store.get_block(&Hash256::ZERO).unwrap(), None);
    }

    #[test]
    fn get_block_header_after_connect() {
        let mut store = MemoryChainStore::new();
        let cb0 = make_coinbase(50 * COIN, pkh(0xAA));
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        assert_eq!(store.get_block_header(&hash0).unwrap(), Some(block0.header));
    }

    #[test]
    fn get_block_hash_multiple_heights() {
        let mut store = MemoryChainStore::new();

        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(hash0, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        assert_eq!(store.get_block_hash(0).unwrap(), Some(hash0));
        assert_eq!(store.get_block_hash(1).unwrap(), Some(hash1));
        assert_eq!(store.get_block_hash(2).unwrap(), None);
    }

    // ------------------------------------------------------------------
    // UTXO queries
    // ------------------------------------------------------------------

    #[test]
    fn contains_utxo_after_connect() {
        let mut store = MemoryChainStore::new();
        let coinbase = make_coinbase(50 * COIN, pkh(0xAA));
        let cb_txid = coinbase.txid().unwrap();
        let block = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);
        store.connect_block(&block, 0).unwrap();

        let op = OutPoint { txid: cb_txid, index: 0 };
        assert!(store.contains_utxo(&op).unwrap());
        assert!(!store.contains_utxo(&OutPoint { txid: cb_txid, index: 1 }).unwrap());
    }

    #[test]
    fn utxo_entry_fields_correct() {
        let mut store = MemoryChainStore::new();
        let coinbase = make_coinbase(50 * COIN, pkh(0xAA));
        let cb_txid = coinbase.txid().unwrap();
        let block = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);
        store.connect_block(&block, 0).unwrap();

        let entry = store.get_utxo(&OutPoint { txid: cb_txid, index: 0 }).unwrap().unwrap();
        assert_eq!(entry.output.value, 50 * COIN);
        assert_eq!(entry.output.pubkey_hash, pkh(0xAA));
        assert_eq!(entry.block_height, 0);
        assert!(entry.is_coinbase);
        assert_eq!(entry.cluster_id, Hash256::ZERO);
    }

    #[test]
    fn regular_tx_utxo_not_coinbase() {
        let mut store = MemoryChainStore::new();

        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let cb0_txid = cb0.txid().unwrap();
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let spend = make_tx(
            &[OutPoint { txid: cb0_txid, index: 0 }],
            49 * COIN,
            pkh(0xCC),
        );
        let spend_txid = spend.txid().unwrap();
        let block1 = make_block(hash0, 1_000_060, vec![cb1, spend]);
        store.connect_block(&block1, 1).unwrap();

        let entry = store.get_utxo(&OutPoint { txid: spend_txid, index: 0 }).unwrap().unwrap();
        assert!(!entry.is_coinbase);
        assert_eq!(entry.block_height, 1);
    }

    // ------------------------------------------------------------------
    // Blocks still accessible after disconnect
    // ------------------------------------------------------------------

    #[test]
    fn blocks_persist_after_disconnect() {
        let mut store = MemoryChainStore::new();
        let coinbase = make_coinbase(50 * COIN, pkh(0xAA));
        let block = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);
        let hash = block.header.hash();
        store.connect_block(&block, 0).unwrap();
        store.disconnect_tip().unwrap();

        // Block data still retrievable by hash (for reorgs/history).
        assert!(store.get_block(&hash).unwrap().is_some());
        assert!(store.get_block_header(&hash).unwrap().is_some());
    }

    // ------------------------------------------------------------------
    // Trait object compatibility
    // ------------------------------------------------------------------

    #[test]
    fn chain_store_dyn_compatible() {
        let mut store = MemoryChainStore::new();
        let coinbase = make_coinbase(50 * COIN, pkh(0xAA));
        let block = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);
        store.connect_block(&block, 0).unwrap();

        let dyn_store: &dyn ChainStore = &store;
        assert!(!dyn_store.is_empty());
        assert_eq!(dyn_store.utxo_count(), 1);
        assert!(dyn_store.chain_tip().is_ok());
    }

    fn _assert_dyn_compatible(_cs: &dyn ChainStore) {
        let _ = _cs.chain_tip();
    }

    // ------------------------------------------------------------------
    // Result types
    // ------------------------------------------------------------------

    #[test]
    fn connect_result_debug() {
        let r = ConnectBlockResult { utxos_created: 3, utxos_spent: 1 };
        let debug = format!("{r:?}");
        assert!(debug.contains("utxos_created: 3"));
        assert!(debug.contains("utxos_spent: 1"));
    }

    #[test]
    fn disconnect_result_debug() {
        let r = DisconnectBlockResult { utxos_restored: 2, utxos_removed: 4 };
        let debug = format!("{r:?}");
        assert!(debug.contains("utxos_restored: 2"));
        assert!(debug.contains("utxos_removed: 4"));
    }

    #[test]
    fn connect_result_eq() {
        let a = ConnectBlockResult { utxos_created: 1, utxos_spent: 2 };
        let b = ConnectBlockResult { utxos_created: 1, utxos_spent: 2 };
        assert_eq!(a, b);
    }

    #[test]
    fn disconnect_result_eq() {
        let a = DisconnectBlockResult { utxos_restored: 3, utxos_removed: 4 };
        let b = DisconnectBlockResult { utxos_restored: 3, utxos_removed: 4 };
        assert_eq!(a, b);
    }

    #[test]
    fn connect_result_clone() {
        let r = ConnectBlockResult { utxos_created: 5, utxos_spent: 2 };
        let c = r.clone();
        assert_eq!(r, c);
    }

    // ------------------------------------------------------------------
    // Error display
    // ------------------------------------------------------------------

    #[test]
    fn error_variants_display() {
        let errors: Vec<ChainStateError> = vec![
            ChainStateError::EmptyChain,
            ChainStateError::BlockNotFound("abc".into()),
            ChainStateError::UndoDataMissing("def".into()),
            ChainStateError::HeightMismatch { expected: 1, got: 5 },
            ChainStateError::DuplicateBlock("ghi".into()),
        ];
        for e in &errors {
            assert!(!format!("{e}").is_empty());
        }
    }

    #[test]
    fn error_eq() {
        assert_eq!(ChainStateError::EmptyChain, ChainStateError::EmptyChain);
        assert_ne!(
            ChainStateError::HeightMismatch { expected: 0, got: 1 },
            ChainStateError::HeightMismatch { expected: 0, got: 2 },
        );
    }

    // ------------------------------------------------------------------
    // Edge cases
    // ------------------------------------------------------------------

    #[test]
    fn connect_coinbase_only_blocks_accumulate_utxos() {
        let mut store = MemoryChainStore::new();
        let mut prev_hash = Hash256::ZERO;

        for h in 0..10 {
            let cb = make_coinbase_unique(50 * COIN, pkh(h as u8), h);
            let block = make_block(prev_hash, 1_000_000 + h * 60, vec![cb]);
            prev_hash = block.header.hash();
            store.connect_block(&block, h).unwrap();
        }

        assert_eq!(store.utxo_count(), 10);
        assert_eq!(store.block_count(), 10);
        assert_eq!(store.undo_count(), 10);
        let (height, _) = store.chain_tip().unwrap();
        assert_eq!(height, 9);
    }

    #[test]
    fn disconnect_all_blocks_returns_to_empty() {
        let mut store = MemoryChainStore::new();
        let mut prev_hash = Hash256::ZERO;

        for h in 0..5 {
            let cb = make_coinbase_unique(50 * COIN, pkh(h as u8), h);
            let block = make_block(prev_hash, 1_000_000 + h * 60, vec![cb]);
            prev_hash = block.header.hash();
            store.connect_block(&block, h).unwrap();
        }

        for _ in 0..5 {
            store.disconnect_tip().unwrap();
        }

        assert!(store.is_empty());
        assert_eq!(store.utxo_count(), 0);
        assert_eq!(store.undo_count(), 0);
    }

    #[test]
    fn spending_chain_utxo_tracking() {
        let mut store = MemoryChainStore::new();

        // Block 0: coinbase creates 50 RILL to A.
        let cb0 = make_coinbase_unique(50 * COIN, pkh(0xAA), 0);
        let cb0_txid = cb0.txid().unwrap();
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![cb0]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();

        // Block 1: A→B (49 RILL), coinbase to X.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0x11), 1);
        let tx_a_to_b = make_tx(
            &[OutPoint { txid: cb0_txid, index: 0 }],
            49 * COIN,
            pkh(0xBB),
        );
        let tx_ab_txid = tx_a_to_b.txid().unwrap();
        let block1 = make_block(hash0, 1_000_060, vec![cb1, tx_a_to_b]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        // Block 2: B→C (48 RILL), coinbase to Y.
        let cb2 = make_coinbase_unique(50 * COIN, pkh(0x22), 2);
        let tx_b_to_c = make_tx(
            &[OutPoint { txid: tx_ab_txid, index: 0 }],
            48 * COIN,
            pkh(0xCC),
        );
        let tx_bc_txid = tx_b_to_c.txid().unwrap();
        let block2 = make_block(hash1, 1_000_120, vec![cb2, tx_b_to_c]);
        store.connect_block(&block2, 2).unwrap();

        // UTXO set: cb1, cb2, tx_b_to_c output. cb0 and tx_a_to_b spent.
        assert_eq!(store.utxo_count(), 3);
        assert!(store.get_utxo(&OutPoint { txid: cb0_txid, index: 0 }).unwrap().is_none());
        assert!(store.get_utxo(&OutPoint { txid: tx_ab_txid, index: 0 }).unwrap().is_none());
        assert!(store.get_utxo(&OutPoint { txid: tx_bc_txid, index: 0 }).unwrap().is_some());

        // Disconnect block 2: B's UTXO restored, C's removed.
        store.disconnect_tip().unwrap();
        assert_eq!(store.utxo_count(), 2);
        assert!(store.get_utxo(&OutPoint { txid: tx_ab_txid, index: 0 }).unwrap().is_some());
        assert!(store.get_utxo(&OutPoint { txid: tx_bc_txid, index: 0 }).unwrap().is_none());

        // Disconnect block 1: A's UTXO restored.
        store.disconnect_tip().unwrap();
        assert_eq!(store.utxo_count(), 1);
        assert!(store.get_utxo(&OutPoint { txid: cb0_txid, index: 0 }).unwrap().is_some());
    }

    #[test]
    fn multi_input_spending() {
        let mut store = MemoryChainStore::new();

        // Block 0: coinbase with 2 outputs.
        let coinbase = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![
                TxOutput { value: 30 * COIN, pubkey_hash: pkh(0xAA) },
                TxOutput { value: 20 * COIN, pubkey_hash: pkh(0xBB) },
            ],
            lock_time: 0,
        };
        let cb_txid = coinbase.txid().unwrap();
        let block0 = make_block(Hash256::ZERO, 1_000_000, vec![coinbase]);
        let hash0 = block0.header.hash();
        store.connect_block(&block0, 0).unwrap();
        assert_eq!(store.utxo_count(), 2);

        // Block 1: spends both outputs from coinbase.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0x11), 1);
        let merge_tx = make_tx(
            &[
                OutPoint { txid: cb_txid, index: 0 },
                OutPoint { txid: cb_txid, index: 1 },
            ],
            49 * COIN,
            pkh(0xCC),
        );
        let block1 = make_block(hash0, 1_000_060, vec![cb1, merge_tx]);
        let result = store.connect_block(&block1, 1).unwrap();
        assert_eq!(result.utxos_spent, 2);
        assert_eq!(result.utxos_created, 2);
        assert_eq!(store.utxo_count(), 2); // cb1 + merge output

        // Both original UTXOs spent.
        assert!(store.get_utxo(&OutPoint { txid: cb_txid, index: 0 }).unwrap().is_none());
        assert!(store.get_utxo(&OutPoint { txid: cb_txid, index: 1 }).unwrap().is_none());

        // Disconnect: both restored.
        store.disconnect_tip().unwrap();
        assert_eq!(store.utxo_count(), 2);
        assert_eq!(
            store.get_utxo(&OutPoint { txid: cb_txid, index: 0 }).unwrap().unwrap().output.value,
            30 * COIN,
        );
        assert_eq!(
            store.get_utxo(&OutPoint { txid: cb_txid, index: 1 }).unwrap().unwrap().output.value,
            20 * COIN,
        );
    }
}
