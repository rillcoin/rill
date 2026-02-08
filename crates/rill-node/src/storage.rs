//! RocksDB-backed persistent chain state storage.
//!
//! Implements [`ChainStore`] using RocksDB column families for blocks, headers,
//! UTXOs, height index, undo data, and metadata. All mutations use atomic
//! [`WriteBatch`] for crash safety.
//!
//! On first open, automatically connects the genesis block.

use std::path::Path;

use rocksdb::{ColumnFamilyDescriptor, Options, WriteBatch, DB};

use rill_core::chain_state::{ChainStore, ConnectBlockResult, DisconnectBlockResult};
use rill_core::error::{ChainStateError, RillError};
use rill_core::genesis;
use rill_core::types::{Block, BlockHeader, Hash256, OutPoint, Transaction, UtxoEntry};

// --- Column family names ---

const CF_BLOCKS: &str = "blocks";
const CF_HEADERS: &str = "headers";
const CF_UTXOS: &str = "utxos";
const CF_HEIGHT_INDEX: &str = "height_index";
const CF_UNDO: &str = "undo";
const CF_METADATA: &str = "metadata";

/// All column family names.
const ALL_CFS: &[&str] = &[
    CF_BLOCKS,
    CF_HEADERS,
    CF_UTXOS,
    CF_HEIGHT_INDEX,
    CF_UNDO,
    CF_METADATA,
];

// --- Metadata keys ---

const META_TIP_HEIGHT: &[u8] = b"tip_height";
const META_TIP_HASH: &[u8] = b"tip_hash";
const META_CIRCULATING_SUPPLY: &[u8] = b"circulating_supply";
const META_DECAY_POOL_BALANCE: &[u8] = b"decay_pool_balance";
const META_UTXO_COUNT: &[u8] = b"utxo_count";

/// Undo data for reverting a connected block.
///
/// Stores the UTXOs consumed by the block's transactions so they can be
/// restored during chain reorganization.
#[derive(bincode::Encode, bincode::Decode)]
struct BlockUndo {
    /// Spent UTXOs in the order they were consumed.
    spent_utxos: Vec<(OutPoint, UtxoEntry)>,
}

/// RocksDB-backed persistent chain state storage.
///
/// Stores blocks, headers, UTXOs, height index, undo data, and aggregate
/// metadata in separate column families. All mutations are atomic via
/// [`WriteBatch`].
///
/// On first open, automatically connects the genesis block.
pub struct RocksStore {
    db: DB,
}

impl RocksStore {
    /// Open or create a RocksDB database at the given path.
    ///
    /// Creates all column families if they don't exist. If the database is
    /// empty (no tip), automatically connects the genesis block.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, RillError> {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        let cf_descriptors: Vec<ColumnFamilyDescriptor> = ALL_CFS
            .iter()
            .map(|name| ColumnFamilyDescriptor::new(*name, Options::default()))
            .collect();

        let db = DB::open_cf_descriptors(&db_opts, path.as_ref(), cf_descriptors)
            .map_err(|e| RillError::Storage(e.to_string()))?;

        let mut store = Self { db };

        // Auto-connect genesis if the chain is empty.
        if store.is_empty() {
            let genesis = genesis::genesis_block();
            store.connect_block(genesis, 0)?;
        }

        Ok(store)
    }

    /// Current circulating supply in rills.
    pub fn circulating_supply(&self) -> Result<u64, RillError> {
        self.get_meta_u64(META_CIRCULATING_SUPPLY)
    }

    /// Current decay pool balance in rills.
    pub fn decay_pool_balance(&self) -> Result<u64, RillError> {
        self.get_meta_u64(META_DECAY_POOL_BALANCE)
    }

    /// Cluster balance for a given cluster ID.
    ///
    /// Phase 1: always returns 0 (cluster tracking deferred to Phase 2).
    pub fn cluster_balance(&self, _cluster_id: &Hash256) -> Result<u64, RillError> {
        Ok(0)
    }

    /// Flush all in-memory buffers to disk.
    pub fn flush(&self) -> Result<(), RillError> {
        self.db
            .flush()
            .map_err(|e| RillError::Storage(e.to_string()))
    }

    // --- Internal helpers ---

    /// Get a u64 from the metadata column family.
    fn get_meta_u64(&self, key: &[u8]) -> Result<u64, RillError> {
        let cf = self.cf_handle(CF_METADATA)?;
        match self
            .db
            .get_cf(&cf, key)
            .map_err(|e| RillError::Storage(e.to_string()))?
        {
            Some(bytes) if bytes.len() == 8 => {
                Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
            }
            Some(_) => Err(RillError::Storage("invalid metadata value length".into())),
            None => Ok(0),
        }
    }

    /// Get a column family handle.
    fn cf_handle(&self, name: &str) -> Result<&rocksdb::ColumnFamily, RillError> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| RillError::Storage(format!("missing column family: {name}")))
    }

    /// Encode an OutPoint as a bincode key.
    fn encode_outpoint(outpoint: &OutPoint) -> Result<Vec<u8>, RillError> {
        bincode::encode_to_vec(outpoint, bincode::config::standard())
            .map_err(|e| RillError::Storage(e.to_string()))
    }

    /// Encode a height as big-endian bytes for ordered iteration.
    fn height_key(height: u64) -> [u8; 8] {
        height.to_be_bytes()
    }

    /// Compute the total coinbase output value for a block.
    fn coinbase_value(block: &Block) -> u64 {
        block
            .coinbase()
            .map(|cb| cb.outputs.iter().map(|o| o.value).sum())
            .unwrap_or(0)
    }

    /// Process a transaction's inputs: mark outpoints for deletion, collect undo data.
    ///
    /// Returns the number of UTXOs spent.
    fn collect_spent_utxos(
        &self,
        tx: &Transaction,
        undo: &mut BlockUndo,
    ) -> Result<usize, RillError> {
        if tx.is_coinbase() {
            return Ok(0);
        }

        let cf_utxos = self.cf_handle(CF_UTXOS)?;
        let mut spent = 0;

        for input in &tx.inputs {
            let key = Self::encode_outpoint(&input.previous_output)?;
            if let Some(data) = self
                .db
                .get_cf(&cf_utxos, &key)
                .map_err(|e| RillError::Storage(e.to_string()))?
            {
                let (entry, _): (UtxoEntry, _) =
                    bincode::decode_from_slice(&data, bincode::config::standard())
                        .map_err(|e| RillError::Storage(e.to_string()))?;
                undo.spent_utxos
                    .push((input.previous_output.clone(), entry));
                spent += 1;
            }
        }

        Ok(spent)
    }
}

impl ChainStore for RocksStore {
    fn connect_block(
        &mut self,
        block: &Block,
        height: u64,
    ) -> Result<ConnectBlockResult, RillError> {
        // Validate height consistency.
        let (tip_height, tip_hash) = self.chain_tip()?;
        if tip_hash == Hash256::ZERO {
            if height != 0 {
                return Err(ChainStateError::HeightMismatch {
                    expected: 0,
                    got: height,
                }
                .into());
            }
        } else if height != tip_height + 1 {
            return Err(ChainStateError::HeightMismatch {
                expected: tip_height + 1,
                got: height,
            }
            .into());
        }

        let block_hash = block.header.hash();

        // Reject duplicate blocks.
        let cf_blocks = self.cf_handle(CF_BLOCKS)?;
        if self
            .db
            .get_cf(&cf_blocks, block_hash.as_bytes())
            .map_err(|e| RillError::Storage(e.to_string()))?
            .is_some()
        {
            return Err(ChainStateError::DuplicateBlock(block_hash.to_string()).into());
        }

        // Collect spent UTXOs for undo data.
        let mut undo = BlockUndo {
            spent_utxos: Vec::new(),
        };
        let mut total_spent = 0;
        for tx in &block.transactions {
            total_spent += self.collect_spent_utxos(tx, &mut undo)?;
        }

        // Build an atomic WriteBatch.
        let mut batch = WriteBatch::default();

        let cf_blocks = self.cf_handle(CF_BLOCKS)?;
        let cf_headers = self.cf_handle(CF_HEADERS)?;
        let cf_utxos = self.cf_handle(CF_UTXOS)?;
        let cf_height = self.cf_handle(CF_HEIGHT_INDEX)?;
        let cf_undo = self.cf_handle(CF_UNDO)?;
        let cf_meta = self.cf_handle(CF_METADATA)?;

        // Delete spent UTXOs.
        for (outpoint, _) in &undo.spent_utxos {
            let key = Self::encode_outpoint(outpoint)?;
            batch.delete_cf(cf_utxos, &key);
        }

        // Create new UTXOs.
        let mut total_created = 0u64;
        for tx in &block.transactions {
            let txid = tx.txid().map_err(RillError::from)?;
            let is_coinbase = tx.is_coinbase();
            for (index, output) in tx.outputs.iter().enumerate() {
                let outpoint = OutPoint {
                    txid,
                    index: index as u64,
                };
                let entry = UtxoEntry {
                    output: output.clone(),
                    block_height: height,
                    is_coinbase,
                    cluster_id: Hash256::ZERO,
                };
                let key = Self::encode_outpoint(&outpoint)?;
                let value = bincode::encode_to_vec(&entry, bincode::config::standard())
                    .map_err(|e| RillError::Storage(e.to_string()))?;
                batch.put_cf(cf_utxos, &key, &value);
                total_created += 1;
            }
        }

        // Store block and header.
        let block_bytes = bincode::encode_to_vec(block, bincode::config::standard())
            .map_err(|e| RillError::Storage(e.to_string()))?;
        let header_bytes =
            bincode::encode_to_vec(&block.header, bincode::config::standard())
                .map_err(|e| RillError::Storage(e.to_string()))?;
        batch.put_cf(cf_blocks, block_hash.as_bytes(), &block_bytes);
        batch.put_cf(cf_headers, block_hash.as_bytes(), &header_bytes);

        // Height index.
        batch.put_cf(cf_height, Self::height_key(height), block_hash.as_bytes());

        // Undo data.
        let undo_bytes = bincode::encode_to_vec(&undo, bincode::config::standard())
            .map_err(|e| RillError::Storage(e.to_string()))?;
        batch.put_cf(cf_undo, block_hash.as_bytes(), &undo_bytes);

        // Update metadata.
        batch.put_cf(cf_meta, META_TIP_HEIGHT, height.to_le_bytes());
        batch.put_cf(cf_meta, META_TIP_HASH, block_hash.as_bytes());

        // Update UTXO count.
        let current_utxo_count = self.get_meta_u64(META_UTXO_COUNT)?;
        let new_utxo_count = current_utxo_count + total_created - total_spent as u64;
        batch.put_cf(cf_meta, META_UTXO_COUNT, new_utxo_count.to_le_bytes());

        // Update circulating supply: add coinbase value.
        let current_supply = self.get_meta_u64(META_CIRCULATING_SUPPLY)?;
        let coinbase_val = Self::coinbase_value(block);
        let new_supply = current_supply.saturating_add(coinbase_val);
        batch.put_cf(
            cf_meta,
            META_CIRCULATING_SUPPLY,
            new_supply.to_le_bytes(),
        );

        // Write atomically.
        self.db
            .write(batch)
            .map_err(|e| RillError::Storage(e.to_string()))?;

        Ok(ConnectBlockResult {
            utxos_created: total_created as usize,
            utxos_spent: total_spent,
        })
    }

    fn disconnect_tip(&mut self) -> Result<DisconnectBlockResult, RillError> {
        let (tip_height, tip_hash) = self.chain_tip()?;
        if tip_hash == Hash256::ZERO {
            return Err(ChainStateError::EmptyChain.into());
        }

        // Get the tip block.
        let block = self
            .get_block(&tip_hash)?
            .ok_or_else(|| ChainStateError::BlockNotFound(tip_hash.to_string()))?;

        // Get undo data.
        let cf_undo = self.cf_handle(CF_UNDO)?;
        let undo_bytes = self
            .db
            .get_cf(&cf_undo, tip_hash.as_bytes())
            .map_err(|e| RillError::Storage(e.to_string()))?
            .ok_or_else(|| ChainStateError::UndoDataMissing(tip_hash.to_string()))?;
        let (undo, _): (BlockUndo, _) =
            bincode::decode_from_slice(&undo_bytes, bincode::config::standard())
                .map_err(|e| RillError::Storage(e.to_string()))?;

        let mut batch = WriteBatch::default();

        let cf_utxos = self.cf_handle(CF_UTXOS)?;
        let cf_height = self.cf_handle(CF_HEIGHT_INDEX)?;
        let cf_undo = self.cf_handle(CF_UNDO)?;
        let cf_meta = self.cf_handle(CF_METADATA)?;

        // Remove UTXOs created by this block.
        let mut total_removed = 0u64;
        for tx in block.transactions.iter().rev() {
            let txid = tx.txid().map_err(RillError::from)?;
            for (index, _) in tx.outputs.iter().enumerate() {
                let outpoint = OutPoint {
                    txid,
                    index: index as u64,
                };
                let key = Self::encode_outpoint(&outpoint)?;
                // Check if it exists before counting.
                if self
                    .db
                    .get_cf(&cf_utxos, &key)
                    .map_err(|e| RillError::Storage(e.to_string()))?
                    .is_some()
                {
                    batch.delete_cf(cf_utxos, &key);
                    total_removed += 1;
                }
            }
        }

        // Restore spent UTXOs from undo data.
        let total_restored = undo.spent_utxos.len();
        for (outpoint, entry) in &undo.spent_utxos {
            let key = Self::encode_outpoint(outpoint)?;
            let value = bincode::encode_to_vec(entry, bincode::config::standard())
                .map_err(|e| RillError::Storage(e.to_string()))?;
            batch.put_cf(cf_utxos, &key, &value);
        }

        // Remove undo data and height index entry.
        batch.delete_cf(cf_undo, tip_hash.as_bytes());
        batch.delete_cf(cf_height, Self::height_key(tip_height));

        // Update tip metadata.
        if tip_height == 0 {
            // Disconnected genesis — back to empty chain.
            batch.put_cf(cf_meta, META_TIP_HEIGHT, 0u64.to_le_bytes());
            batch.put_cf(cf_meta, META_TIP_HASH, Hash256::ZERO.as_bytes());
        } else {
            batch.put_cf(
                cf_meta,
                META_TIP_HEIGHT,
                (tip_height - 1).to_le_bytes(),
            );
            batch.put_cf(
                cf_meta,
                META_TIP_HASH,
                block.header.prev_hash.as_bytes(),
            );
        }

        // Update UTXO count.
        let current_utxo_count = self.get_meta_u64(META_UTXO_COUNT)?;
        let new_utxo_count =
            current_utxo_count + total_restored as u64 - total_removed;
        batch.put_cf(cf_meta, META_UTXO_COUNT, new_utxo_count.to_le_bytes());

        // Update circulating supply: subtract coinbase value.
        let current_supply = self.get_meta_u64(META_CIRCULATING_SUPPLY)?;
        let coinbase_val = Self::coinbase_value(&block);
        let new_supply = current_supply.saturating_sub(coinbase_val);
        batch.put_cf(
            cf_meta,
            META_CIRCULATING_SUPPLY,
            new_supply.to_le_bytes(),
        );

        // Write atomically.
        self.db
            .write(batch)
            .map_err(|e| RillError::Storage(e.to_string()))?;

        Ok(DisconnectBlockResult {
            utxos_restored: total_restored,
            utxos_removed: total_removed as usize,
        })
    }

    fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UtxoEntry>, RillError> {
        let cf = self.cf_handle(CF_UTXOS)?;
        let key = Self::encode_outpoint(outpoint)?;
        match self
            .db
            .get_cf(&cf, &key)
            .map_err(|e| RillError::Storage(e.to_string()))?
        {
            Some(data) => {
                let (entry, _): (UtxoEntry, _) =
                    bincode::decode_from_slice(&data, bincode::config::standard())
                        .map_err(|e| RillError::Storage(e.to_string()))?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
        let cf = self.cf_handle(CF_METADATA)?;
        let hash = match self
            .db
            .get_cf(&cf, META_TIP_HASH)
            .map_err(|e| RillError::Storage(e.to_string()))?
        {
            Some(bytes) if bytes.len() == 32 => {
                Hash256(bytes.try_into().unwrap())
            }
            _ => return Ok((0, Hash256::ZERO)),
        };

        if hash == Hash256::ZERO {
            return Ok((0, Hash256::ZERO));
        }

        let height = self.get_meta_u64(META_TIP_HEIGHT)?;
        Ok((height, hash))
    }

    fn get_block_header(
        &self,
        hash: &Hash256,
    ) -> Result<Option<BlockHeader>, RillError> {
        let cf = self.cf_handle(CF_HEADERS)?;
        match self
            .db
            .get_cf(&cf, hash.as_bytes())
            .map_err(|e| RillError::Storage(e.to_string()))?
        {
            Some(data) => {
                let (header, _): (BlockHeader, _) =
                    bincode::decode_from_slice(&data, bincode::config::standard())
                        .map_err(|e| RillError::Storage(e.to_string()))?;
                Ok(Some(header))
            }
            None => Ok(None),
        }
    }

    fn get_block(&self, hash: &Hash256) -> Result<Option<Block>, RillError> {
        let cf = self.cf_handle(CF_BLOCKS)?;
        match self
            .db
            .get_cf(&cf, hash.as_bytes())
            .map_err(|e| RillError::Storage(e.to_string()))?
        {
            Some(data) => {
                let (block, _): (Block, _) =
                    bincode::decode_from_slice(&data, bincode::config::standard())
                        .map_err(|e| RillError::Storage(e.to_string()))?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    fn get_block_hash(&self, height: u64) -> Result<Option<Hash256>, RillError> {
        let cf = self.cf_handle(CF_HEIGHT_INDEX)?;
        match self
            .db
            .get_cf(&cf, Self::height_key(height))
            .map_err(|e| RillError::Storage(e.to_string()))?
        {
            Some(bytes) if bytes.len() == 32 => {
                Ok(Some(Hash256(bytes.try_into().unwrap())))
            }
            _ => Ok(None),
        }
    }

    fn utxo_count(&self) -> usize {
        self.get_meta_u64(META_UTXO_COUNT).unwrap_or(0) as usize
    }

    fn is_empty(&self) -> bool {
        match self.chain_tip() {
            Ok((_, hash)) => hash == Hash256::ZERO,
            Err(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rill_core::constants::COIN;
    use rill_core::genesis::{self, DEV_FUND_PREMINE};
    use rill_core::merkle;
    use rill_core::types::{TxInput, TxOutput};

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Create a temporary RocksStore.
    fn temp_store() -> (RocksStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = RocksStore::open(dir.path().join("chaindata")).unwrap();
        (store, dir)
    }

    /// Create a coinbase with unique data to produce a unique txid.
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
            lock_time: 0,
        }
    }

    /// Create a regular transaction spending the given outpoints.
    fn make_tx(outpoints: &[OutPoint], output_value: u64, pubkey_hash: Hash256) -> Transaction {
        Transaction {
            version: 1,
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
                pubkey_hash,
            }],
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
    // Genesis auto-init
    // ------------------------------------------------------------------

    #[test]
    fn open_auto_connects_genesis() {
        let (store, _dir) = temp_store();
        assert!(!store.is_empty());

        let (height, hash) = store.chain_tip().unwrap();
        assert_eq!(height, 0);
        assert_eq!(hash, genesis::genesis_hash());
    }

    #[test]
    fn genesis_block_stored() {
        let (store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let block = store.get_block(&genesis_hash).unwrap().unwrap();
        assert_eq!(block, *genesis::genesis_block());

        let header = store.get_block_header(&genesis_hash).unwrap().unwrap();
        assert_eq!(header, genesis::genesis_block().header);
    }

    #[test]
    fn genesis_creates_utxo() {
        let (store, _dir) = temp_store();
        assert_eq!(store.utxo_count(), 1);

        let coinbase_txid = genesis::genesis_coinbase_txid();
        let utxo = store
            .get_utxo(&OutPoint {
                txid: coinbase_txid,
                index: 0,
            })
            .unwrap();
        assert!(utxo.is_some());
        let entry = utxo.unwrap();
        assert_eq!(entry.output.value, DEV_FUND_PREMINE);
        assert!(entry.is_coinbase);
    }

    #[test]
    fn genesis_supply_tracked() {
        let (store, _dir) = temp_store();
        assert_eq!(store.circulating_supply().unwrap(), DEV_FUND_PREMINE);
    }

    #[test]
    fn genesis_height_index() {
        let (store, _dir) = temp_store();
        let hash = store.get_block_hash(0).unwrap().unwrap();
        assert_eq!(hash, genesis::genesis_hash());
        assert!(store.get_block_hash(1).unwrap().is_none());
    }

    // ------------------------------------------------------------------
    // Connect additional blocks
    // ------------------------------------------------------------------

    #[test]
    fn connect_block_after_genesis() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();

        let result = store.connect_block(&block1, 1).unwrap();
        assert_eq!(result.utxos_created, 1);
        assert_eq!(result.utxos_spent, 0);

        let (height, hash) = store.chain_tip().unwrap();
        assert_eq!(height, 1);
        assert_eq!(hash, hash1);
        assert_eq!(store.utxo_count(), 2); // genesis + block1
    }

    #[test]
    fn connect_block_with_spending_tx() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();
        let coinbase_txid = genesis::genesis_coinbase_txid();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let spend_tx = make_tx(
            &[OutPoint {
                txid: coinbase_txid,
                index: 0,
            }],
            DEV_FUND_PREMINE - COIN,
            pkh(0xCC),
        );
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1, spend_tx]);

        let result = store.connect_block(&block1, 1).unwrap();
        assert_eq!(result.utxos_created, 2);
        assert_eq!(result.utxos_spent, 1);

        // Original UTXO is now spent.
        assert!(store
            .get_utxo(&OutPoint {
                txid: coinbase_txid,
                index: 0
            })
            .unwrap()
            .is_none());
        assert_eq!(store.utxo_count(), 2);
    }

    #[test]
    fn connect_block_rejects_wrong_height() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);

        let err = store.connect_block(&block1, 5).unwrap_err();
        assert!(
            matches!(err, RillError::ChainState(ChainStateError::HeightMismatch { .. })),
            "expected HeightMismatch, got: {err:?}"
        );
    }

    #[test]
    fn connect_block_rejects_duplicate() {
        let (mut store, _dir) = temp_store();

        // Try to connect genesis again at height 1.
        let genesis = genesis::genesis_block();
        let err = store.connect_block(genesis, 1).unwrap_err();
        assert!(
            matches!(err, RillError::ChainState(ChainStateError::DuplicateBlock(_))),
            "expected DuplicateBlock, got: {err:?}"
        );
    }

    #[test]
    fn supply_tracks_across_blocks() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let initial_supply = store.circulating_supply().unwrap();
        assert_eq!(initial_supply, DEV_FUND_PREMINE);

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        store.connect_block(&block1, 1).unwrap();

        let new_supply = store.circulating_supply().unwrap();
        assert_eq!(new_supply, DEV_FUND_PREMINE + 50 * COIN);
    }

    // ------------------------------------------------------------------
    // Disconnect tip
    // ------------------------------------------------------------------

    #[test]
    fn disconnect_tip_reverts_to_genesis() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        store.connect_block(&block1, 1).unwrap();

        let result = store.disconnect_tip().unwrap();
        assert_eq!(result.utxos_removed, 1);
        assert_eq!(result.utxos_restored, 0);

        let (height, hash) = store.chain_tip().unwrap();
        assert_eq!(height, 0);
        assert_eq!(hash, genesis_hash);
        assert_eq!(store.utxo_count(), 1); // genesis only
    }

    #[test]
    fn disconnect_restores_spent_utxos() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();
        let coinbase_txid = genesis::genesis_coinbase_txid();

        // Block 1: spends genesis coinbase.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let spend = make_tx(
            &[OutPoint {
                txid: coinbase_txid,
                index: 0,
            }],
            DEV_FUND_PREMINE - COIN,
            pkh(0xCC),
        );
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1, spend]);
        store.connect_block(&block1, 1).unwrap();

        // Verify genesis UTXO is spent.
        assert!(store
            .get_utxo(&OutPoint {
                txid: coinbase_txid,
                index: 0
            })
            .unwrap()
            .is_none());

        // Disconnect block 1.
        let result = store.disconnect_tip().unwrap();
        assert_eq!(result.utxos_removed, 2);
        assert_eq!(result.utxos_restored, 1);

        // Genesis UTXO is back.
        let restored = store
            .get_utxo(&OutPoint {
                txid: coinbase_txid,
                index: 0,
            })
            .unwrap()
            .unwrap();
        assert_eq!(restored.output.value, DEV_FUND_PREMINE);
        assert!(restored.is_coinbase);
    }

    #[test]
    fn disconnect_supply_reverts() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        store.connect_block(&block1, 1).unwrap();
        assert_eq!(
            store.circulating_supply().unwrap(),
            DEV_FUND_PREMINE + 50 * COIN
        );

        store.disconnect_tip().unwrap();
        assert_eq!(store.circulating_supply().unwrap(), DEV_FUND_PREMINE);
    }

    #[test]
    fn disconnect_removes_height_mapping() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        store.connect_block(&block1, 1).unwrap();
        assert!(store.get_block_hash(1).unwrap().is_some());

        store.disconnect_tip().unwrap();
        assert!(store.get_block_hash(1).unwrap().is_none());
        // Genesis still accessible.
        assert_eq!(
            store.get_block_hash(0).unwrap(),
            Some(genesis::genesis_hash())
        );
    }

    #[test]
    fn disconnect_empty_chain_errors() {
        let (mut store, _dir) = temp_store();
        // Disconnect genesis.
        store.disconnect_tip().unwrap();
        // Now chain is empty.
        let err = store.disconnect_tip().unwrap_err();
        assert!(matches!(
            err,
            RillError::ChainState(ChainStateError::EmptyChain)
        ));
    }

    // ------------------------------------------------------------------
    // Persistence across reopen
    // ------------------------------------------------------------------

    #[test]
    fn persistence_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("chaindata");
        let genesis_hash = genesis::genesis_hash();

        // Open, connect a block, close.
        {
            let mut store = RocksStore::open(&db_path).unwrap();
            let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
            let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
            store.connect_block(&block1, 1).unwrap();
            store.flush().unwrap();
        }

        // Reopen — data should be there.
        {
            let store = RocksStore::open(&db_path).unwrap();
            let (height, _) = store.chain_tip().unwrap();
            assert_eq!(height, 1);
            assert_eq!(store.utxo_count(), 2);
            assert_eq!(
                store.circulating_supply().unwrap(),
                DEV_FUND_PREMINE + 50 * COIN
            );
        }
    }

    // ------------------------------------------------------------------
    // Connect-disconnect roundtrip
    // ------------------------------------------------------------------

    #[test]
    fn connect_disconnect_roundtrip() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Connect 3 blocks.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        let cb2 = make_coinbase_unique(50 * COIN, pkh(0xCC), 2);
        let block2 = make_block(hash1, 1_000_120, vec![cb2]);
        let hash2 = block2.header.hash();
        store.connect_block(&block2, 2).unwrap();

        let cb3 = make_coinbase_unique(50 * COIN, pkh(0xDD), 3);
        let block3 = make_block(hash2, 1_000_180, vec![cb3]);
        store.connect_block(&block3, 3).unwrap();

        assert_eq!(store.utxo_count(), 4); // genesis + 3 coinbases
        assert_eq!(
            store.circulating_supply().unwrap(),
            DEV_FUND_PREMINE + 150 * COIN
        );

        // Disconnect all 3.
        store.disconnect_tip().unwrap();
        assert_eq!(store.chain_tip().unwrap(), (2, hash2));

        store.disconnect_tip().unwrap();
        assert_eq!(store.chain_tip().unwrap(), (1, hash1));

        store.disconnect_tip().unwrap();
        assert_eq!(store.chain_tip().unwrap(), (0, genesis_hash));
        assert_eq!(store.utxo_count(), 1);
        assert_eq!(store.circulating_supply().unwrap(), DEV_FUND_PREMINE);
    }

    // ------------------------------------------------------------------
    // Miscellaneous
    // ------------------------------------------------------------------

    #[test]
    fn decay_pool_balance_default_zero() {
        let (store, _dir) = temp_store();
        assert_eq!(store.decay_pool_balance().unwrap(), 0);
    }

    #[test]
    fn cluster_balance_always_zero_phase1() {
        let (store, _dir) = temp_store();
        assert_eq!(store.cluster_balance(&Hash256([0xFF; 32])).unwrap(), 0);
    }

    #[test]
    fn get_utxo_nonexistent() {
        let (store, _dir) = temp_store();
        let op = OutPoint {
            txid: Hash256([0xFF; 32]),
            index: 0,
        };
        assert!(store.get_utxo(&op).unwrap().is_none());
    }

    #[test]
    fn get_block_nonexistent() {
        let (store, _dir) = temp_store();
        assert!(store.get_block(&Hash256([0xFF; 32])).unwrap().is_none());
    }

    #[test]
    fn get_block_header_nonexistent() {
        let (store, _dir) = temp_store();
        assert!(store
            .get_block_header(&Hash256([0xFF; 32]))
            .unwrap()
            .is_none());
    }

    #[test]
    fn contains_utxo_after_genesis() {
        let (store, _dir) = temp_store();
        let coinbase_txid = genesis::genesis_coinbase_txid();
        assert!(store
            .contains_utxo(&OutPoint {
                txid: coinbase_txid,
                index: 0
            })
            .unwrap());
        assert!(!store
            .contains_utxo(&OutPoint {
                txid: coinbase_txid,
                index: 1
            })
            .unwrap());
    }

    #[test]
    fn blocks_persist_after_disconnect() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        store.disconnect_tip().unwrap();

        // Block data still retrievable by hash.
        assert!(store.get_block(&hash1).unwrap().is_some());
        assert!(store.get_block_header(&hash1).unwrap().is_some());
    }
}
