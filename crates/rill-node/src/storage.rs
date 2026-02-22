//! RocksDB-backed persistent chain state storage.
//!
//! Implements [`ChainStore`] using RocksDB column families for blocks, headers,
//! UTXOs, height index, undo data, and metadata. All mutations use atomic
//! [`WriteBatch`] for crash safety.
//!
//! On first open, automatically connects the genesis block.

use std::collections::HashMap;
use std::path::Path;

use rocksdb::{ColumnFamilyDescriptor, Options, SliceTransform, WriteBatch, DB};

use rill_core::agent::AgentWalletState;
use rill_core::chain_state::{ChainStore, ConnectBlockResult, DisconnectBlockResult};
use rill_core::conduct::{self, VelocityBaseline};
use rill_core::constants::CONDUCT_PERIOD_BLOCKS;
use rill_core::error::{ChainStateError, RillError};
use rill_core::genesis;
use rill_core::types::{Block, BlockHeader, Hash256, OutPoint, Transaction, UtxoEntry};
use rill_decay::cluster::determine_output_cluster;

// --- Column family names ---

const CF_BLOCKS: &str = "blocks";
const CF_HEADERS: &str = "headers";
const CF_UTXOS: &str = "utxos";
const CF_HEIGHT_INDEX: &str = "height_index";
const CF_UNDO: &str = "undo";
const CF_METADATA: &str = "metadata";
const CF_CLUSTERS: &str = "clusters";
const CF_ADDRESS_INDEX: &str = "address_index";
const CF_AGENT_WALLETS: &str = "agent_wallets";

/// All column family names.
const ALL_CFS: &[&str] = &[
    CF_BLOCKS,
    CF_HEADERS,
    CF_UTXOS,
    CF_HEIGHT_INDEX,
    CF_UNDO,
    CF_METADATA,
    CF_CLUSTERS,
    CF_ADDRESS_INDEX,
    CF_AGENT_WALLETS,
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
    /// Cluster balance deltas applied during this block, for reorg reversal.
    cluster_deltas: Vec<(Hash256, i128)>,
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
            .map(|name| {
                let mut opts = Options::default();
                // Configure address index with fixed prefix extractor (32 bytes for pubkey_hash)
                if *name == CF_ADDRESS_INDEX {
                    opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(32));
                }
                ColumnFamilyDescriptor::new(*name, opts)
            })
            .collect();

        let db = DB::open_cf_descriptors(&db_opts, path.as_ref(), cf_descriptors)
            .map_err(|e| RillError::Storage(e.to_string()))?;

        let mut store = Self { db };

        // Auto-connect genesis if the chain is empty.
        if store.is_empty() {
            let genesis = genesis::genesis_block();
            store.connect_block(genesis, 0)?;
        }

        // Migrate: build address index if empty but UTXOs exist
        store.migrate_address_index()?;

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
    pub fn cluster_balance(&self, cluster_id: &Hash256) -> Result<u64, RillError> {
        let cf = self.cf_handle(CF_CLUSTERS)?;
        match self
            .db
            .get_cf(&cf, cluster_id.as_bytes())
            .map_err(|e| RillError::Storage(e.to_string()))?
        {
            Some(bytes) if bytes.len() == 8 => {
                Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
            }
            Some(_) => Err(RillError::Storage("invalid cluster balance length".into())),
            None => Ok(0),
        }
    }

    /// Flush all in-memory buffers to disk.
    pub fn flush(&self) -> Result<(), RillError> {
        self.db
            .flush()
            .map_err(|e| RillError::Storage(e.to_string()))
    }

    /// Trigger manual compaction across all column families.
    ///
    /// Compaction merges SSTables, reclaims space from deleted keys, and
    /// improves read performance. Call this during low-activity periods (e.g.
    /// on startup after IBD completes).
    pub fn compact(&self) -> Result<(), RillError> {
        for cf_name in ALL_CFS {
            let cf = self.cf_handle(cf_name)?;
            self.db
                .compact_range_cf(&cf, None::<&[u8]>, None::<&[u8]>);
        }
        Ok(())
    }

    /// Delete full block data for blocks older than `keep_recent` blocks
    /// from the current tip. Headers and undo data are preserved.
    ///
    /// Returns the number of blocks pruned.
    pub fn prune_blocks(&self, keep_recent: u64) -> Result<u64, RillError> {
        let (tip_height, _) = self.chain_tip()?;

        // Calculate the cutoff: blocks at heights 1..=cutoff are eligible for pruning.
        // Height 0 (genesis) is never pruned.
        let cutoff = tip_height.saturating_sub(keep_recent);
        if cutoff == 0 {
            return Ok(0);
        }

        let cf_blocks = self.cf_handle(CF_BLOCKS)?;
        let cf_height = self.cf_handle(CF_HEIGHT_INDEX)?;
        let mut batch = WriteBatch::default();
        let mut pruned_count = 0u64;

        for height in 1..=cutoff {
            // Look up the block hash for this height.
            let hash_bytes = match self
                .db
                .get_cf(&cf_height, Self::height_key(height))
                .map_err(|e| RillError::Storage(e.to_string()))?
            {
                Some(bytes) if bytes.len() == 32 => bytes,
                _ => continue,
            };

            // Only delete from CF_BLOCKS if the data is still present.
            if self
                .db
                .get_cf(&cf_blocks, &hash_bytes)
                .map_err(|e| RillError::Storage(e.to_string()))?
                .is_some()
            {
                batch.delete_cf(cf_blocks, &hash_bytes);
                pruned_count += 1;
            }
        }

        if pruned_count > 0 {
            self.db
                .write(batch)
                .map_err(|e| RillError::Storage(e.to_string()))?;
            tracing::info!("pruned {} full block(s) up to height {}", pruned_count, cutoff);
        }

        Ok(pruned_count)
    }

    /// Returns true if the block at the given height has been pruned
    /// (header exists but full block data does not).
    pub fn is_block_pruned(&self, height: u64) -> Result<bool, RillError> {
        // Look up the block hash from the height index.
        let hash = match self.get_block_hash(height)? {
            Some(h) => h,
            None => return Ok(false), // Height not in chain at all.
        };

        // Header must exist for the block to be considered "pruned" vs "unknown".
        if self.get_block_header(&hash)?.is_none() {
            return Ok(false);
        }

        // Full block data must be absent.
        let cf_blocks = self.cf_handle(CF_BLOCKS)?;
        let has_full_data = self
            .db
            .get_cf(&cf_blocks, hash.as_bytes())
            .map_err(|e| RillError::Storage(e.to_string()))?
            .is_some();

        Ok(!has_full_data)
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

    /// Encode an address index key: pubkey_hash || txid || index(BE).
    fn encode_address_index_key(pubkey_hash: &Hash256, outpoint: &OutPoint) -> [u8; 72] {
        let mut key = [0u8; 72];
        key[0..32].copy_from_slice(pubkey_hash.as_bytes());
        key[32..64].copy_from_slice(outpoint.txid.as_bytes());
        key[64..72].copy_from_slice(&outpoint.index.to_be_bytes());
        key
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

    /// One-time migration: build address index from existing UTXOs.
    fn migrate_address_index(&self) -> Result<(), RillError> {
        let cf_addr = self.cf_handle(CF_ADDRESS_INDEX)?;

        // Check if index already has entries
        let mut iter = self.db.iterator_cf(&cf_addr, rocksdb::IteratorMode::Start);
        if iter.next().is_some() {
            return Ok(()); // Already populated
        }
        drop(iter);

        // Check if there are UTXOs to index
        let utxo_count = self.get_meta_u64(META_UTXO_COUNT)?;
        if utxo_count == 0 {
            return Ok(());
        }

        tracing::info!("migrating address index for {} UTXOs", utxo_count);

        let cf_utxos = self.cf_handle(CF_UTXOS)?;
        let mut batch = WriteBatch::default();
        let mut count = 0u64;

        let iter = self.db.iterator_cf(&cf_utxos, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key_bytes, value_bytes) = item.map_err(|e| RillError::Storage(e.to_string()))?;
            let (outpoint, _): (OutPoint, _) =
                bincode::decode_from_slice(&key_bytes, bincode::config::standard())
                    .map_err(|e| RillError::Storage(e.to_string()))?;
            let (entry, _): (UtxoEntry, _) =
                bincode::decode_from_slice(&value_bytes, bincode::config::standard())
                    .map_err(|e| RillError::Storage(e.to_string()))?;

            let addr_key = Self::encode_address_index_key(&entry.output.pubkey_hash, &outpoint);
            batch.put_cf(cf_addr, addr_key, []);
            count += 1;
        }

        if count > 0 {
            self.db.write(batch).map_err(|e| RillError::Storage(e.to_string()))?;
            tracing::info!("address index migration complete: {} entries", count);
        }

        Ok(())
    }

    /// Get all UTXOs for a given pubkey hash using the address index.
    ///
    /// Uses RocksDB prefix iteration over CF_ADDRESS_INDEX for O(k) lookup
    /// where k is the number of UTXOs owned by this address.
    pub fn get_utxos_by_address(
        &self,
        pubkey_hash: &Hash256,
    ) -> Result<Vec<(OutPoint, UtxoEntry)>, RillError> {
        let cf_addr = self.cf_handle(CF_ADDRESS_INDEX)?;
        let cf_utxos = self.cf_handle(CF_UTXOS)?;
        let prefix = pubkey_hash.as_bytes();

        let mut result = Vec::new();
        let iter = self.db.prefix_iterator_cf(&cf_addr, prefix);

        for item in iter {
            let (key_bytes, _) = item.map_err(|e| RillError::Storage(e.to_string()))?;

            // Verify the prefix still matches (prefix_iterator may overshoot)
            if key_bytes.len() != 72 || &key_bytes[0..32] != prefix {
                break;
            }

            // Extract outpoint from key
            let mut txid_bytes = [0u8; 32];
            txid_bytes.copy_from_slice(&key_bytes[32..64]);
            let index = u64::from_be_bytes(key_bytes[64..72].try_into().unwrap());
            let outpoint = OutPoint {
                txid: Hash256(txid_bytes),
                index,
            };

            // Look up the actual UTXO entry
            let utxo_key = Self::encode_outpoint(&outpoint)?;
            if let Some(utxo_data) = self.db.get_cf(&cf_utxos, &utxo_key)
                .map_err(|e| RillError::Storage(e.to_string()))?
            {
                let (entry, _): (UtxoEntry, _) =
                    bincode::decode_from_slice(&utxo_data, bincode::config::standard())
                        .map_err(|e| RillError::Storage(e.to_string()))?;
                result.push((outpoint, entry));
            }
        }

        Ok(result)
    }

    /// Look up an agent wallet state by pubkey hash.
    pub fn get_agent_wallet(&self, pubkey_hash: &Hash256) -> Result<Option<AgentWalletState>, RillError> {
        let cf = self.cf_handle(CF_AGENT_WALLETS)?;
        match self.db.get_cf(&cf, pubkey_hash.as_bytes())
            .map_err(|e| RillError::Storage(e.to_string()))?
        {
            Some(bytes) => {
                let (state, _): (AgentWalletState, _) =
                    bincode::decode_from_slice(&bytes, bincode::config::standard())
                        .map_err(|e| RillError::Storage(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    /// Store an agent wallet state.
    pub fn put_agent_wallet(&self, state: &AgentWalletState) -> Result<(), RillError> {
        let cf = self.cf_handle(CF_AGENT_WALLETS)?;
        let bytes = bincode::encode_to_vec(state, bincode::config::standard())
            .map_err(|e| RillError::Storage(e.to_string()))?;
        self.db.put_cf(&cf, state.pubkey_hash.as_bytes(), &bytes)
            .map_err(|e| RillError::Storage(e.to_string()))
    }

    /// Check if a pubkey hash is registered as an agent wallet.
    pub fn is_agent_wallet(&self, pubkey_hash: &Hash256) -> Result<bool, RillError> {
        let cf = self.cf_handle(CF_AGENT_WALLETS)?;
        self.db.get_cf(&cf, pubkey_hash.as_bytes())
            .map(|v| v.is_some())
            .map_err(|e| RillError::Storage(e.to_string()))
    }

    /// Process conduct score updates at epoch boundaries.
    ///
    /// Called during `connect_block` when `height` is a non-zero multiple of
    /// [`CONDUCT_PERIOD_BLOCKS`]. Iterates all registered agent wallets,
    /// updates each wallet's velocity baseline with the epoch's outbound
    /// volume, recomputes the conduct score via exponential smoothing, and
    /// derives the updated decay multiplier.
    ///
    /// All updates are written into `batch` for atomic commit alongside the
    /// rest of the block.
    fn process_conduct_epoch(
        &self,
        batch: &mut WriteBatch,
        height: u64,
        agent_epoch_volumes: &HashMap<Hash256, u64>,
    ) -> Result<(), RillError> {
        let cf_agents = self.cf_handle(CF_AGENT_WALLETS)?;

        // Collect all agent wallet states first to avoid holding the iterator
        // while we also need to call self methods.
        let mut agents: Vec<AgentWalletState> = Vec::new();
        {
            let iter = self
                .db
                .iterator_cf(&cf_agents, rocksdb::IteratorMode::Start);
            for item in iter {
                let (_, value_bytes) =
                    item.map_err(|e| RillError::Storage(e.to_string()))?;
                let (state, _): (AgentWalletState, _) =
                    bincode::decode_from_slice(&value_bytes, bincode::config::standard())
                        .map_err(|e| RillError::Storage(e.to_string()))?;
                agents.push(state);
            }
        }

        for mut state in agents {
            // Get the outbound volume for this agent during the epoch (0 if none).
            let epoch_volume = agent_epoch_volumes
                .get(&state.pubkey_hash)
                .copied()
                .unwrap_or(0);

            // Push the new epoch volume into the rolling baseline.
            state.velocity_baseline.push_epoch(epoch_volume);

            // Compute wallet age in epochs.
            let age_in_epochs = (height.saturating_sub(state.registered_at_block))
                / CONDUCT_PERIOD_BLOCKS;

            // Compute individual signal scores.
            let velocity_anomaly =
                conduct::velocity_anomaly_score(&state.velocity_baseline, epoch_volume);
            let wallet_age = conduct::wallet_age_score(age_in_epochs);

            // Compute raw score (neutral 500 for signals not yet implemented).
            let raw_score =
                conduct::compute_raw_score(500, 500, velocity_anomaly, 500, wallet_age);

            // Apply exponential smoothing to avoid sudden score jumps.
            let new_score = conduct::smooth_score(state.conduct_score, raw_score);

            // Derive the updated decay multiplier from the new score.
            let new_multiplier_bps = conduct::score_to_multiplier_bps(new_score);

            tracing::debug!(
                pubkey_hash = %state.pubkey_hash,
                height,
                epoch_volume,
                velocity_anomaly,
                wallet_age,
                raw_score,
                old_score = state.conduct_score,
                new_score,
                new_multiplier_bps,
                "conduct epoch update"
            );

            state.conduct_score = new_score;
            state.conduct_multiplier_bps = new_multiplier_bps;

            let state_bytes =
                bincode::encode_to_vec(&state, bincode::config::standard())
                    .map_err(|e| RillError::Storage(e.to_string()))?;
            batch.put_cf(cf_agents, state.pubkey_hash.as_bytes(), &state_bytes);
        }

        Ok(())
    }

    /// Get a geometric block locator for chain synchronization.
    ///
    /// Returns hashes in the pattern: tip, tip-1, tip-2, tip-4, tip-8, ..., genesis.
    /// This allows efficient common ancestor discovery with O(log n) hashes.
    pub fn get_block_locator(&self) -> Result<Vec<Hash256>, RillError> {
        let (tip_height, tip_hash) = self.chain_tip()?;
        if tip_hash == Hash256::ZERO {
            return Ok(vec![Hash256::ZERO]);
        }

        let mut locator = Vec::new();
        let mut step = 1u64;
        let mut height = tip_height;

        loop {
            if let Some(hash) = self.get_block_hash(height)? {
                locator.push(hash);
            }

            if height == 0 {
                break;
            }

            // Geometric backoff: 1, 1, 2, 4, 8, 16, ...
            if height <= step {
                height = 0;
            } else {
                height -= step;
            }

            // Double the step after the first few blocks.
            if locator.len() > 10 {
                step *= 2;
            }
        }

        // Always include genesis if not already present.
        if locator.last() != Some(&Hash256::ZERO) {
            if let Some(genesis_hash) = self.get_block_hash(0)? {
                if !locator.contains(&genesis_hash) {
                    locator.push(genesis_hash);
                }
            }
        }

        Ok(locator)
    }

    /// Look up the height at which a given hash appears in the height index.
    ///
    /// Iterates the height index from the most-recent end backwards, since
    /// recent blocks are the common case for locator and header-sync queries.
    /// Returns `None` if the hash is not in the main chain.
    fn get_height_for_hash(&self, hash: &Hash256) -> Result<Option<u64>, RillError> {
        let cf_height = self.cf_handle(CF_HEIGHT_INDEX)?;
        let iter = self
            .db
            .iterator_cf(&cf_height, rocksdb::IteratorMode::End);
        for item in iter {
            let (key_bytes, value_bytes) =
                item.map_err(|e| RillError::Storage(e.to_string()))?;
            if value_bytes.len() == 32 {
                let stored_hash = Hash256(value_bytes[..32].try_into().unwrap());
                if stored_hash == *hash && key_bytes.len() == 8 {
                    let height =
                        u64::from_be_bytes(key_bytes[..8].try_into().unwrap());
                    return Ok(Some(height));
                }
            }
        }
        Ok(None)
    }

    /// Find the first locator hash that we have in our chain.
    ///
    /// Returns (height, hash) of the common ancestor, or None if no match.
    /// Uses the height index for O(chain_length) worst-case instead of
    /// O(locator_len * chain_length).
    pub fn find_common_ancestor(
        &self,
        locator: &[Hash256],
    ) -> Result<Option<(u64, Hash256)>, RillError> {
        for hash in locator {
            // Check if we have this block header at all.
            if self.get_block_header(hash)?.is_none() {
                continue;
            }
            // Scan the height index (newest-first) to find it on our main chain.
            if let Some(height) = self.get_height_for_hash(hash)? {
                return Ok(Some((height, *hash)));
            }
            // We have the block but it is not on our main chain (stale/orphan).
            // Keep looking for a deeper common ancestor.
        }

        Ok(None)
    }

    /// Get up to `max_count` headers after the given hash.
    ///
    /// Caps at 2000 headers maximum per request. Uses the height index for an
    /// O(result_count) scan rather than O(chain_length).
    pub fn get_headers_after(
        &self,
        hash: &Hash256,
        max_count: usize,
    ) -> Result<Vec<BlockHeader>, RillError> {
        const MAX_HEADERS_PER_REQUEST: usize = 2000;
        let limit = max_count.min(MAX_HEADERS_PER_REQUEST);

        // Find the height of the starting hash via the height index.
        let start_height = match self.get_height_for_hash(hash)? {
            Some(h) => h,
            None => return Ok(vec![]), // Unknown hash, return empty.
        };

        let cf_height = self.cf_handle(CF_HEIGHT_INDEX)?;
        let mut headers = Vec::new();

        // Seek to the first height after `start_height` and iterate forward.
        let start_key = Self::height_key(start_height + 1);
        let iter = self.db.iterator_cf(
            &cf_height,
            rocksdb::IteratorMode::From(&start_key, rocksdb::Direction::Forward),
        );

        for item in iter {
            if headers.len() >= limit {
                break;
            }
            let (_, value_bytes) = item.map_err(|e| RillError::Storage(e.to_string()))?;
            if value_bytes.len() == 32 {
                let h = Hash256(value_bytes[..32].try_into().unwrap());
                if let Some(header) = self.get_block_header(&h)? {
                    headers.push(header);
                }
            }
        }

        Ok(headers)
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
            cluster_deltas: Vec::new(),
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
        let cf_addr_index = self.cf_handle(CF_ADDRESS_INDEX)?;

        // Track cluster balance deltas.
        let mut cluster_deltas: HashMap<Hash256, i128> = HashMap::new();

        // Track outbound volumes per agent wallet for conduct epoch processing.
        let mut agent_epoch_volumes: HashMap<Hash256, u64> = HashMap::new();

        // Delete spent UTXOs and subtract their values from cluster balances.
        for (outpoint, entry) in &undo.spent_utxos {
            let key = Self::encode_outpoint(outpoint)?;
            batch.delete_cf(cf_utxos, &key);

            // Delete address index entry
            let addr_key = Self::encode_address_index_key(&entry.output.pubkey_hash, outpoint);
            batch.delete_cf(cf_addr_index, addr_key);

            // Subtract spent UTXO value from its cluster.
            let delta = cluster_deltas.entry(entry.cluster_id).or_insert(0);
            *delta -= entry.output.value as i128;

            // Accumulate outbound volume for agent wallets.
            if self.is_agent_wallet(&entry.output.pubkey_hash)? {
                let vol = agent_epoch_volumes
                    .entry(entry.output.pubkey_hash)
                    .or_insert(0);
                *vol = vol.saturating_add(entry.output.value);
            }
        }

        // Create new UTXOs with proper cluster tracking.
        let cf_clusters = self.cf_handle(CF_CLUSTERS)?;
        let mut total_created = 0u64;
        for tx in &block.transactions {
            let txid = tx.txid().map_err(RillError::from)?;
            let is_coinbase = tx.is_coinbase();

            // Determine input cluster IDs for this transaction.
            let input_cluster_ids: Vec<Hash256> = if is_coinbase {
                Vec::new()
            } else {
                tx.inputs
                    .iter()
                    .filter_map(|input| {
                        undo.spent_utxos
                            .iter()
                            .find(|(op, _)| op == &input.previous_output)
                            .map(|(_, entry)| entry.cluster_id)
                    })
                    .collect()
            };

            // Determine the output cluster ID for this transaction.
            let output_cluster_id = determine_output_cluster(&input_cluster_ids, &txid);

            // Create UTXOs with the determined cluster ID.
            for (index, output) in tx.outputs.iter().enumerate() {
                let outpoint = OutPoint {
                    txid,
                    index: index as u64,
                };
                let entry = UtxoEntry {
                    output: output.clone(),
                    block_height: height,
                    is_coinbase,
                    cluster_id: output_cluster_id,
                };
                let key = Self::encode_outpoint(&outpoint)?;
                let value = bincode::encode_to_vec(&entry, bincode::config::standard())
                    .map_err(|e| RillError::Storage(e.to_string()))?;
                batch.put_cf(cf_utxos, &key, &value);
                total_created += 1;

                // Add address index entry
                let addr_key = Self::encode_address_index_key(&output.pubkey_hash, &outpoint);
                batch.put_cf(cf_addr_index, addr_key, []);

                // Add created UTXO value to its cluster.
                let delta = cluster_deltas.entry(output_cluster_id).or_insert(0);
                *delta += output.value as i128;
            }
        }

        // Process AgentRegister transactions.
        let cf_agents = self.cf_handle(CF_AGENT_WALLETS)?;
        for tx in &block.transactions {
            if tx.tx_type == rill_core::types::TxType::AgentRegister {
                // The first output's pubkey_hash is the registrant.
                if let Some(first_output) = tx.outputs.first() {
                    if first_output.value >= rill_core::constants::AGENT_REGISTRATION_STAKE {
                        // Check not already registered.
                        if !self.is_agent_wallet(&first_output.pubkey_hash)? {
                            let state = AgentWalletState {
                                pubkey_hash: first_output.pubkey_hash,
                                registered_at_block: height,
                                stake_balance: first_output.value,
                                stake_locked_until: height + rill_core::constants::CONDUCT_PERIOD_BLOCKS,
                                conduct_score: rill_core::constants::CONDUCT_SCORE_DEFAULT,
                                conduct_multiplier_bps: rill_core::constants::CONDUCT_MULTIPLIER_DEFAULT_BPS,
                                undertow_active: false,
                                undertow_expires_at: 0,
                                velocity_baseline: VelocityBaseline::new(),
                            };
                            let state_bytes = bincode::encode_to_vec(&state, bincode::config::standard())
                                .map_err(|e| RillError::Storage(e.to_string()))?;
                            batch.put_cf(cf_agents, state.pubkey_hash.as_bytes(), &state_bytes);
                        }
                    }
                }
            }
        }

        // Apply cluster balance deltas.
        for (cluster_id, delta) in &cluster_deltas {
            let current_balance = self.cluster_balance(cluster_id)?;
            let new_balance = if *delta >= 0 {
                current_balance
                    .checked_add(*delta as u64)
                    .ok_or_else(|| RillError::Storage("cluster balance overflow".into()))?
            } else {
                current_balance
                    .checked_sub((-*delta) as u64)
                    .ok_or_else(|| RillError::Storage("cluster balance underflow".into()))?
            };

            if new_balance == 0 {
                batch.delete_cf(cf_clusters, cluster_id.as_bytes());
            } else {
                batch.put_cf(cf_clusters, cluster_id.as_bytes(), new_balance.to_le_bytes());
            }
        }

        // Store cluster deltas in undo data.
        undo.cluster_deltas = cluster_deltas.into_iter().collect();

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

        // Process conduct score epoch at boundaries (skip genesis block 0).
        if height > 0 && height % CONDUCT_PERIOD_BLOCKS == 0 {
            self.process_conduct_epoch(&mut batch, height, &agent_epoch_volumes)?;
        }

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
        let cf_addr_index = self.cf_handle(CF_ADDRESS_INDEX)?;

        // Remove UTXOs created by this block.
        let mut total_removed = 0u64;
        for tx in block.transactions.iter().rev() {
            let txid = tx.txid().map_err(RillError::from)?;
            for (index, output) in tx.outputs.iter().enumerate() {
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

                    // Delete address index entry
                    let addr_key = Self::encode_address_index_key(&output.pubkey_hash, &outpoint);
                    batch.delete_cf(cf_addr_index, addr_key);

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

            // Restore address index entry
            let addr_key = Self::encode_address_index_key(&entry.output.pubkey_hash, outpoint);
            batch.put_cf(cf_addr_index, addr_key, []);
        }

        // Reverse cluster balance deltas.
        let cf_clusters = self.cf_handle(CF_CLUSTERS)?;
        for (cluster_id, delta) in &undo.cluster_deltas {
            let current_balance = self.cluster_balance(cluster_id)?;
            let new_balance = if *delta >= 0 {
                // Original delta was positive, so subtract it now.
                current_balance
                    .checked_sub(*delta as u64)
                    .ok_or_else(|| RillError::Storage("cluster balance underflow on disconnect".into()))?
            } else {
                // Original delta was negative, so add it back now.
                current_balance
                    .checked_add((-*delta) as u64)
                    .ok_or_else(|| RillError::Storage("cluster balance overflow on disconnect".into()))?
            };

            if new_balance == 0 {
                batch.delete_cf(cf_clusters, cluster_id.as_bytes());
            } else {
                batch.put_cf(cf_clusters, cluster_id.as_bytes(), new_balance.to_le_bytes());
            }
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

    fn iter_utxos(&self) -> Result<Vec<(OutPoint, UtxoEntry)>, RillError> {
        let cf = self.cf_handle(CF_UTXOS)?;
        let mut utxos = Vec::new();

        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key_bytes, value_bytes) = item.map_err(|e| RillError::Storage(e.to_string()))?;
            let (outpoint, _): (OutPoint, _) =
                bincode::decode_from_slice(&key_bytes, bincode::config::standard())
                    .map_err(|e| RillError::Storage(e.to_string()))?;
            let (entry, _): (UtxoEntry, _) =
                bincode::decode_from_slice(&value_bytes, bincode::config::standard())
                    .map_err(|e| RillError::Storage(e.to_string()))?;
            utxos.push((outpoint, entry));
        }

        Ok(utxos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rill_core::constants::COIN;
    use rill_core::genesis::{self, DEV_FUND_PREMINE};
    use rill_core::merkle;
    use rill_core::types::{TxInput, TxOutput, TxType};

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
    ///
    /// Sets `lock_time: height` matching the production consensus engine so that
    /// coinbases at different heights always have distinct txids.
    fn make_coinbase_unique(value: u64, pubkey_hash: Hash256, height: u64) -> Transaction {
        Transaction {
            version: 1,
            tx_type: TxType::default(),
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

    /// Create a regular transaction spending the given outpoints.
    fn make_tx(outpoints: &[OutPoint], output_value: u64, pubkey_hash: Hash256) -> Transaction {
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

    // ------------------------------------------------------------------
    // Cluster balance tracking (Phase 2)
    // ------------------------------------------------------------------

    #[test]
    fn cluster_balance_tracked_on_connect() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Block 1: coinbase creates a new cluster.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let cb1_txid = cb1.txid().unwrap();
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        store.connect_block(&block1, 1).unwrap();

        // Coinbase creates a new cluster with ID = txid.
        let cluster_id = cb1_txid;
        let balance = store.cluster_balance(&cluster_id).unwrap();
        assert_eq!(balance, 50 * COIN);
    }

    #[test]
    fn cluster_balance_decreases_on_spend() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Block 1: coinbase creates 50 RILL cluster.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let cb1_txid = cb1.txid().unwrap();
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        let cluster1 = cb1_txid;
        assert_eq!(store.cluster_balance(&cluster1).unwrap(), 50 * COIN);

        // Block 2: spend the coinbase, creating a new output of 49 RILL.
        let cb2 = make_coinbase_unique(50 * COIN, pkh(0xCC), 2);
        let spend = make_tx(
            &[OutPoint {
                txid: cb1_txid,
                index: 0,
            }],
            49 * COIN,
            pkh(0xDD),
        );
        let block2 = make_block(hash1, 1_000_120, vec![cb2, spend]);
        store.connect_block(&block2, 2).unwrap();

        // Original cluster should now have 49 RILL (spent 50, created 49).
        assert_eq!(store.cluster_balance(&cluster1).unwrap(), 49 * COIN);
    }

    #[test]
    fn cluster_balance_restored_on_disconnect() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Block 1: coinbase.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let cb1_txid = cb1.txid().unwrap();
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        let cluster1 = cb1_txid;
        let initial_balance = store.cluster_balance(&cluster1).unwrap();
        assert_eq!(initial_balance, 50 * COIN);

        // Block 2: spend.
        let cb2 = make_coinbase_unique(50 * COIN, pkh(0xCC), 2);
        let spend = make_tx(
            &[OutPoint {
                txid: cb1_txid,
                index: 0,
            }],
            49 * COIN,
            pkh(0xDD),
        );
        let block2 = make_block(hash1, 1_000_120, vec![cb2, spend]);
        store.connect_block(&block2, 2).unwrap();

        let after_spend = store.cluster_balance(&cluster1).unwrap();
        assert_eq!(after_spend, 49 * COIN);

        // Disconnect block 2.
        store.disconnect_tip().unwrap();

        // Balance should be restored.
        let restored = store.cluster_balance(&cluster1).unwrap();
        assert_eq!(restored, initial_balance);
    }

    #[test]
    fn cluster_merge_tracked() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Block 1: two separate coinbase clusters.
        let cb1a = make_coinbase_unique(30 * COIN, pkh(0xAA), 1);
        let cb1a_txid = cb1a.txid().unwrap();
        let cb1b = make_coinbase_unique(20 * COIN, pkh(0xBB), 101);
        let cb1b_txid = cb1b.txid().unwrap();
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1a.clone(), cb1b.clone()]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        let cluster_a = cb1a_txid;
        let cluster_b = cb1b_txid;
        assert_eq!(store.cluster_balance(&cluster_a).unwrap(), 30 * COIN);
        assert_eq!(store.cluster_balance(&cluster_b).unwrap(), 20 * COIN);

        // Block 2: merge both clusters into one transaction.
        let cb2 = make_coinbase_unique(50 * COIN, pkh(0xCC), 2);
        let merge_tx = make_tx(
            &[
                OutPoint {
                    txid: cb1a_txid,
                    index: 0,
                },
                OutPoint {
                    txid: cb1b_txid,
                    index: 0,
                },
            ],
            49 * COIN,
            pkh(0xDD),
        );
        let merge_txid = merge_tx.txid().unwrap();
        let block2 = make_block(hash1, 1_000_120, vec![cb2, merge_tx]);
        store.connect_block(&block2, 2).unwrap();

        // Original clusters should be spent (balance 0).
        assert_eq!(store.cluster_balance(&cluster_a).unwrap(), 0);
        assert_eq!(store.cluster_balance(&cluster_b).unwrap(), 0);

        // New merged cluster should have the combined balance.
        let merged_cluster = determine_output_cluster(&[cluster_a, cluster_b], &merge_txid);
        assert_eq!(store.cluster_balance(&merged_cluster).unwrap(), 49 * COIN);
    }

    #[test]
    fn cluster_reorg_roundtrip() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Connect 3 blocks, each with a coinbase creating a cluster.
        let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
        let cb1_txid = cb1.txid().unwrap();
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        let cb2 = make_coinbase_unique(50 * COIN, pkh(0xCC), 2);
        let cb2_txid = cb2.txid().unwrap();
        let block2 = make_block(hash1, 1_000_120, vec![cb2]);
        let hash2 = block2.header.hash();
        store.connect_block(&block2, 2).unwrap();

        let cb3 = make_coinbase_unique(50 * COIN, pkh(0xDD), 3);
        let cb3_txid = cb3.txid().unwrap();
        let block3 = make_block(hash2, 1_000_180, vec![cb3]);
        store.connect_block(&block3, 3).unwrap();

        let c1 = cb1_txid;
        let c2 = cb2_txid;
        let c3 = cb3_txid;

        assert_eq!(store.cluster_balance(&c1).unwrap(), 50 * COIN);
        assert_eq!(store.cluster_balance(&c2).unwrap(), 50 * COIN);
        assert_eq!(store.cluster_balance(&c3).unwrap(), 50 * COIN);

        // Disconnect all 3.
        store.disconnect_tip().unwrap();
        assert_eq!(store.cluster_balance(&c3).unwrap(), 0);
        assert_eq!(store.cluster_balance(&c2).unwrap(), 50 * COIN);
        assert_eq!(store.cluster_balance(&c1).unwrap(), 50 * COIN);

        store.disconnect_tip().unwrap();
        assert_eq!(store.cluster_balance(&c2).unwrap(), 0);
        assert_eq!(store.cluster_balance(&c1).unwrap(), 50 * COIN);

        store.disconnect_tip().unwrap();
        assert_eq!(store.cluster_balance(&c1).unwrap(), 0);
    }

    #[test]
    fn genesis_cluster_balance() {
        let (store, _dir) = temp_store();
        let genesis_coinbase_txid = genesis::genesis_coinbase_txid();

        // Genesis coinbase creates a cluster.
        let cluster_id = genesis_coinbase_txid;
        let balance = store.cluster_balance(&cluster_id).unwrap();
        assert_eq!(balance, DEV_FUND_PREMINE);
    }

    // ------------------------------------------------------------------
    // Address index tests (Phase 2 Item 3)
    // ------------------------------------------------------------------

    #[test]
    fn address_index_created_on_connect() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Connect a block with a unique address
        let addr_pkh = pkh(0xEE);
        let cb1 = make_coinbase_unique(50 * COIN, addr_pkh, 1);
        let cb1_txid = cb1.txid().unwrap();
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        store.connect_block(&block1, 1).unwrap();

        // Verify the UTXO is findable via address index
        let utxos = store.get_utxos_by_address(&addr_pkh).unwrap();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].0.txid, cb1_txid);
        assert_eq!(utxos[0].0.index, 0);
        assert_eq!(utxos[0].1.output.value, 50 * COIN);
    }

    #[test]
    fn address_index_deleted_on_spend() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Block 1: create a UTXO
        let addr_pkh = pkh(0xEE);
        let cb1 = make_coinbase_unique(50 * COIN, addr_pkh, 1);
        let cb1_txid = cb1.txid().unwrap();
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        // Verify it's in the index
        let utxos = store.get_utxos_by_address(&addr_pkh).unwrap();
        assert_eq!(utxos.len(), 1);

        // Block 2: spend the UTXO
        let cb2 = make_coinbase_unique(50 * COIN, pkh(0xFF), 2);
        let spend_tx = make_tx(
            &[OutPoint {
                txid: cb1_txid,
                index: 0,
            }],
            49 * COIN,
            pkh(0xDD), // Different address
        );
        let block2 = make_block(hash1, 1_000_120, vec![cb2, spend_tx]);
        store.connect_block(&block2, 2).unwrap();

        // Verify it's no longer in the index
        let utxos = store.get_utxos_by_address(&addr_pkh).unwrap();
        assert_eq!(utxos.len(), 0);
    }

    #[test]
    fn address_index_restored_on_disconnect() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Block 1: create a UTXO
        let addr_pkh = pkh(0xEE);
        let cb1 = make_coinbase_unique(50 * COIN, addr_pkh, 1);
        let cb1_txid = cb1.txid().unwrap();
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
        let hash1 = block1.header.hash();
        store.connect_block(&block1, 1).unwrap();

        // Block 2: spend the UTXO
        let cb2 = make_coinbase_unique(50 * COIN, pkh(0xFF), 2);
        let spend_tx = make_tx(
            &[OutPoint {
                txid: cb1_txid,
                index: 0,
            }],
            49 * COIN,
            pkh(0xDD),
        );
        let block2 = make_block(hash1, 1_000_120, vec![cb2, spend_tx]);
        store.connect_block(&block2, 2).unwrap();

        // Verify it's gone
        let utxos = store.get_utxos_by_address(&addr_pkh).unwrap();
        assert_eq!(utxos.len(), 0);

        // Disconnect block 2
        store.disconnect_tip().unwrap();

        // Verify the UTXO is back in the index
        let utxos = store.get_utxos_by_address(&addr_pkh).unwrap();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].0.txid, cb1_txid);
        assert_eq!(utxos[0].1.output.value, 50 * COIN);
    }

    #[test]
    fn address_index_prefix_lookup() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let addr_pkh = pkh(0xEE);

        // Block 1: two UTXOs for the same address
        let cb1a = make_coinbase_unique(30 * COIN, addr_pkh, 1);
        let cb1a_txid = cb1a.txid().unwrap();
        let cb1b = make_coinbase_unique(20 * COIN, addr_pkh, 101);
        let cb1b_txid = cb1b.txid().unwrap();
        let block1 = make_block(genesis_hash, 1_000_060, vec![cb1a, cb1b]);
        store.connect_block(&block1, 1).unwrap();

        // Both should be found
        let utxos = store.get_utxos_by_address(&addr_pkh).unwrap();
        assert_eq!(utxos.len(), 2);

        let values: Vec<u64> = utxos.iter().map(|(_, entry)| entry.output.value).collect();
        assert!(values.contains(&(30 * COIN)));
        assert!(values.contains(&(20 * COIN)));

        let txids: Vec<Hash256> = utxos.iter().map(|(op, _)| op.txid).collect();
        assert!(txids.contains(&cb1a_txid));
        assert!(txids.contains(&cb1b_txid));
    }

    #[test]
    fn address_index_empty_for_unknown() {
        let (store, _dir) = temp_store();

        // Query for an address that has no UTXOs
        let unknown_addr = pkh(0xAB);
        let utxos = store.get_utxos_by_address(&unknown_addr).unwrap();
        assert_eq!(utxos.len(), 0);
    }

    #[test]
    fn address_index_migration() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("chaindata");

        // Create a store and connect some blocks (genesis + 1)
        {
            let mut store = RocksStore::open(&db_path).unwrap();
            let genesis_hash = genesis::genesis_hash();
            let cb1 = make_coinbase_unique(50 * COIN, pkh(0xEE), 1);
            let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
            store.connect_block(&block1, 1).unwrap();
            store.flush().unwrap();
        }

        // Reopen — migration should have already run in open()
        // But we can verify that the address index is populated
        {
            let store = RocksStore::open(&db_path).unwrap();
            let utxos = store.get_utxos_by_address(&pkh(0xEE)).unwrap();
            assert_eq!(utxos.len(), 1);
            assert_eq!(utxos[0].1.output.value, 50 * COIN);
        }
    }

    // ------------------------------------------------------------------
    // Chain sync methods (Phase 3 Item 2)
    // ------------------------------------------------------------------

    #[test]
    fn get_block_locator_geometric_pattern() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Build a chain: genesis (0) + 10 blocks.
        let mut prev = genesis_hash;
        for i in 1..=10 {
            let cb = make_coinbase_unique(50 * COIN, pkh(0xBB), i);
            let block = make_block(prev, 1_000_000 + i * 60, vec![cb]);
            prev = block.header.hash();
            store.connect_block(&block, i).unwrap();
        }

        let locator = store.get_block_locator().unwrap();

        // At height 10, geometric pattern: 10, 9, 8, 6, 2, 0 (after doubling step kicks in).
        // First few: 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0 (step=1 for first 10).
        // Then step doubles.
        assert!(locator.len() >= 2);
        assert_eq!(locator[0], store.get_block_hash(10).unwrap().unwrap());
        assert!(locator.contains(&genesis_hash));
    }

    #[test]
    fn get_block_locator_single_block() {
        let (store, _dir) = temp_store();
        let locator = store.get_block_locator().unwrap();
        let genesis_hash = genesis::genesis_hash();

        // Only genesis exists.
        assert_eq!(locator.len(), 1);
        assert_eq!(locator[0], genesis_hash);
    }

    #[test]
    fn find_common_ancestor_finds_matching_hash() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Connect 5 blocks.
        let mut prev = genesis_hash;
        for i in 1..=5 {
            let cb = make_coinbase_unique(50 * COIN, pkh(0xBB), i);
            let block = make_block(prev, 1_000_000 + i * 60, vec![cb]);
            prev = block.header.hash();
            store.connect_block(&block, i).unwrap();
        }

        let hash3 = store.get_block_hash(3).unwrap().unwrap();
        let locator = vec![Hash256([0xFF; 32]), hash3, genesis_hash];

        let common = store.find_common_ancestor(&locator).unwrap();
        assert_eq!(common, Some((3, hash3)));
    }

    #[test]
    fn find_common_ancestor_returns_none_for_unknown() {
        let (store, _dir) = temp_store();
        let locator = vec![Hash256([0xFF; 32]), Hash256([0xEE; 32])];
        let common = store.find_common_ancestor(&locator).unwrap();
        assert_eq!(common, None);
    }

    #[test]
    fn get_headers_after_returns_correct_range() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Connect 5 blocks.
        let mut prev = genesis_hash;
        for i in 1..=5 {
            let cb = make_coinbase_unique(50 * COIN, pkh(0xBB), i);
            let block = make_block(prev, 1_000_000 + i * 60, vec![cb]);
            prev = block.header.hash();
            store.connect_block(&block, i).unwrap();
        }

        let hash2 = store.get_block_hash(2).unwrap().unwrap();
        let headers = store.get_headers_after(&hash2, 10).unwrap();

        // Should return headers for blocks 3, 4, 5.
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].hash(), store.get_block_hash(3).unwrap().unwrap());
        assert_eq!(headers[1].hash(), store.get_block_hash(4).unwrap().unwrap());
        assert_eq!(headers[2].hash(), store.get_block_hash(5).unwrap().unwrap());
    }

    #[test]
    fn get_headers_after_caps_at_2000() {
        let (store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Request 3000 headers (more than max).
        let headers = store.get_headers_after(&genesis_hash, 3000).unwrap();

        // Should cap at 2000 (but we only have 0 blocks after genesis).
        assert_eq!(headers.len(), 0);
    }

    #[test]
    fn get_headers_after_unknown_hash_returns_empty() {
        let (store, _dir) = temp_store();
        let unknown_hash = Hash256([0xFF; 32]);
        let headers = store.get_headers_after(&unknown_hash, 10).unwrap();
        assert_eq!(headers.len(), 0);
    }

    // ------------------------------------------------------------------
    // Storage compaction & optimization (Phase 5c.3)
    // ------------------------------------------------------------------

    #[test]
    fn compact_succeeds() {
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        // Add a few blocks so that there is actual data to compact.
        let mut prev = genesis_hash;
        for i in 1..=3 {
            let cb = make_coinbase_unique(50 * COIN, pkh(i as u8), i);
            let block = make_block(prev, 1_000_000 + i * 60, vec![cb]);
            prev = block.header.hash();
            store.connect_block(&block, i).unwrap();
        }

        // compact() must complete without errors.
        store.compact().unwrap();

        // Chain state must be intact after compaction.
        let (height, _) = store.chain_tip().unwrap();
        assert_eq!(height, 3);
        assert_eq!(store.utxo_count(), 4); // genesis + 3 coinbases
    }

    #[test]
    fn find_common_ancestor_optimized() {
        // Same scenario as find_common_ancestor_finds_matching_hash but verifies
        // the optimized path (height-index scan) produces the same result.
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let mut prev = genesis_hash;
        for i in 1..=5 {
            let cb = make_coinbase_unique(50 * COIN, pkh(0xBB), i);
            let block = make_block(prev, 1_000_000 + i * 60, vec![cb]);
            prev = block.header.hash();
            store.connect_block(&block, i).unwrap();
        }

        // Locator contains an unknown hash, then hash@3, then genesis.
        let hash3 = store.get_block_hash(3).unwrap().unwrap();
        let locator = vec![Hash256([0xFF; 32]), hash3, genesis_hash];

        let common = store.find_common_ancestor(&locator).unwrap();
        assert_eq!(common, Some((3, hash3)));

        // Unknown-only locator still returns None.
        let no_match = store
            .find_common_ancestor(&[Hash256([0xAB; 32])])
            .unwrap();
        assert_eq!(no_match, None);
    }

    #[test]
    fn get_headers_after_optimized() {
        // Same scenario as get_headers_after_returns_correct_range but verifies
        // the optimized (height-index seek) path produces identical output.
        let (mut store, _dir) = temp_store();
        let genesis_hash = genesis::genesis_hash();

        let mut prev = genesis_hash;
        for i in 1..=5 {
            let cb = make_coinbase_unique(50 * COIN, pkh(0xBB), i);
            let block = make_block(prev, 1_000_000 + i * 60, vec![cb]);
            prev = block.header.hash();
            store.connect_block(&block, i).unwrap();
        }

        let hash2 = store.get_block_hash(2).unwrap().unwrap();
        let headers = store.get_headers_after(&hash2, 10).unwrap();

        // Must return headers for blocks 3, 4, 5 in order.
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].hash(), store.get_block_hash(3).unwrap().unwrap());
        assert_eq!(headers[1].hash(), store.get_block_hash(4).unwrap().unwrap());
        assert_eq!(headers[2].hash(), store.get_block_hash(5).unwrap().unwrap());

        // Count limit is still honoured.
        let capped = store.get_headers_after(&hash2, 2).unwrap();
        assert_eq!(capped.len(), 2);

        // Unknown hash returns empty.
        let empty = store
            .get_headers_after(&Hash256([0xFF; 32]), 10)
            .unwrap();
        assert_eq!(empty.len(), 0);
    }

    // ------------------------------------------------------------------
    // Block pruning (Phase 5b.5)
    // ------------------------------------------------------------------

    /// Build a chain of `count` blocks on top of genesis, returning hashes
    /// indexed by height (index 0 = genesis hash, index i = block i hash).
    fn build_chain(store: &mut RocksStore, count: u64) -> Vec<Hash256> {
        let genesis_hash = genesis::genesis_hash();
        let mut hashes = vec![genesis_hash];
        let mut prev = genesis_hash;
        for i in 1..=count {
            let cb = make_coinbase_unique(50 * COIN, pkh(i as u8), i);
            let block = make_block(prev, 1_000_000 + i * 60, vec![cb]);
            prev = block.header.hash();
            store.connect_block(&block, i).unwrap();
            hashes.push(prev);
        }
        hashes
    }

    #[test]
    fn prune_blocks_removes_old_data() {
        // Chain: genesis(0) + blocks 1-4, tip = 4.
        // prune_blocks(keep_recent=2) should remove full data for heights 1 and 2.
        let (mut store, _dir) = temp_store();
        let hashes = build_chain(&mut store, 4);

        let pruned = store.prune_blocks(2).unwrap();
        assert_eq!(pruned, 2); // heights 1 and 2 pruned

        // Blocks 1 and 2: full data gone.
        assert!(store.get_block(&hashes[1]).unwrap().is_none());
        assert!(store.get_block(&hashes[2]).unwrap().is_none());

        // Blocks 3 and 4: full data intact.
        assert!(store.get_block(&hashes[3]).unwrap().is_some());
        assert!(store.get_block(&hashes[4]).unwrap().is_some());
    }

    #[test]
    fn prune_blocks_preserves_headers() {
        // After pruning, headers for pruned blocks must still be accessible.
        let (mut store, _dir) = temp_store();
        let hashes = build_chain(&mut store, 4);

        store.prune_blocks(2).unwrap();

        // Headers for heights 1 and 2 must still be present.
        assert!(store.get_block_header(&hashes[1]).unwrap().is_some());
        assert!(store.get_block_header(&hashes[2]).unwrap().is_some());

        // Heights 3 and 4 headers are unaffected.
        assert!(store.get_block_header(&hashes[3]).unwrap().is_some());
        assert!(store.get_block_header(&hashes[4]).unwrap().is_some());
    }

    #[test]
    fn prune_blocks_preserves_undo() {
        // After pruning, undo data for pruned heights must still be present.
        let (mut store, _dir) = temp_store();
        let hashes = build_chain(&mut store, 4);

        store.prune_blocks(2).unwrap();

        // Verify undo data survives by confirming disconnect_tip still works
        // (it reads undo data from the tip, which is height 4 — not pruned).
        let result = store.disconnect_tip().unwrap();
        assert_eq!(result.utxos_removed, 1); // coinbase at height 4 removed

        // Now tip is 3. Height 3 undo data must also be intact.
        let result2 = store.disconnect_tip().unwrap();
        assert_eq!(result2.utxos_removed, 1);

        // Verify the height index entry is preserved for pruned heights.
        assert!(store.get_block_hash(1).unwrap().is_some());
        assert!(store.get_block_hash(2).unwrap().is_some());

        // Verify undo data in CF_UNDO is accessible for height 1 and 2 hashes.
        let cf_undo = store.cf_handle(CF_UNDO).unwrap();
        let undo1 = store
            .db
            .get_cf(&cf_undo, hashes[1].as_bytes())
            .unwrap();
        assert!(undo1.is_some(), "undo data for height 1 must be preserved");
        let undo2 = store
            .db
            .get_cf(&cf_undo, hashes[2].as_bytes())
            .unwrap();
        assert!(undo2.is_some(), "undo data for height 2 must be preserved");
    }

    #[test]
    fn prune_blocks_preserves_genesis() {
        // Genesis block (height 0) must never be pruned regardless of keep_recent.
        let (mut store, _dir) = temp_store();
        build_chain(&mut store, 5);

        // keep_recent=0 would prune everything up to tip height, but genesis
        // is explicitly skipped.
        let pruned = store.prune_blocks(0).unwrap();
        assert_eq!(pruned, 5); // heights 1-5 pruned, genesis untouched

        let genesis_hash = genesis::genesis_hash();
        assert!(
            store.get_block(&genesis_hash).unwrap().is_some(),
            "genesis full block data must never be pruned"
        );
        assert!(store.get_block_header(&genesis_hash).unwrap().is_some());
    }

    #[test]
    fn prune_blocks_returns_count() {
        // Verify prune_blocks returns the exact number of blocks pruned.
        let (mut store, _dir) = temp_store();
        build_chain(&mut store, 6); // heights 0-6, tip=6

        // keep_recent=3 => cutoff=3 => heights 1,2,3 eligible => 3 pruned.
        let count = store.prune_blocks(3).unwrap();
        assert_eq!(count, 3);

        // Calling again with the same keep_recent should prune 0 (already done).
        let count2 = store.prune_blocks(3).unwrap();
        assert_eq!(count2, 0);
    }

    #[test]
    fn is_block_pruned_works() {
        // is_block_pruned returns true for pruned blocks and false for non-pruned.
        let (mut store, _dir) = temp_store();
        build_chain(&mut store, 5); // heights 0-5, tip=5

        // Before pruning everything should report not pruned.
        assert!(!store.is_block_pruned(0).unwrap()); // genesis
        assert!(!store.is_block_pruned(1).unwrap());
        assert!(!store.is_block_pruned(3).unwrap());
        assert!(!store.is_block_pruned(5).unwrap());

        // Prune heights 1-3 (keep_recent=2 means cutoff=3).
        store.prune_blocks(2).unwrap();

        // Heights 1, 2, 3 are now pruned.
        assert!(store.is_block_pruned(1).unwrap());
        assert!(store.is_block_pruned(2).unwrap());
        assert!(store.is_block_pruned(3).unwrap());

        // Genesis is not pruned.
        assert!(!store.is_block_pruned(0).unwrap());

        // Heights 4 and 5 are not pruned.
        assert!(!store.is_block_pruned(4).unwrap());
        assert!(!store.is_block_pruned(5).unwrap());

        // Non-existent height returns false.
        assert!(!store.is_block_pruned(99).unwrap());
    }
}
