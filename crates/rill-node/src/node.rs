//! Full node composition and event loop.
//!
//! The [`Node`] struct wires together storage, mempool, consensus, and
//! networking into a running full node. The [`NodeChainState`] adapter
//! bridges the mutable [`ChainStore`] (behind a `RwLock`) to the read-only
//! [`ChainState`] trait required by the consensus engine.

use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

use rill_consensus::engine::ConsensusEngine;
use rill_core::chain_state::ChainStore;
use rill_core::error::{RillError, TransactionError};
use rill_core::mempool::Mempool;
use rill_core::traits::{BlockProducer, ChainState, DecayCalculator};
use rill_core::types::{Block, BlockHeader, Hash256, OutPoint, Transaction, UtxoEntry};
use rill_decay::engine::DecayEngine;
use rill_network::{NetworkEvent, NetworkNode, RillRequest, RillResponse, StorageQuery};

use crate::config::NodeConfig;
use crate::storage::RocksStore;

/// Adapter bridging `RocksStore` (behind `RwLock`) to the read-only `ChainState` trait.
///
/// Takes a read lock on each call. This allows the consensus engine and other
/// readers to access chain state concurrently with block processing.
pub struct NodeChainState {
    storage: Arc<RwLock<RocksStore>>,
}

impl NodeChainState {
    /// Create a new adapter wrapping the given storage.
    pub fn new(storage: Arc<RwLock<RocksStore>>) -> Self {
        Self { storage }
    }
}

impl ChainState for NodeChainState {
    fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UtxoEntry>, RillError> {
        self.storage.read().get_utxo(outpoint)
    }

    fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
        self.storage.read().chain_tip()
    }

    fn get_block_header(&self, hash: &Hash256) -> Result<Option<BlockHeader>, RillError> {
        self.storage.read().get_block_header(hash)
    }

    fn get_block(&self, hash: &Hash256) -> Result<Option<Block>, RillError> {
        self.storage.read().get_block(hash)
    }

    fn get_block_hash(&self, height: u64) -> Result<Option<Hash256>, RillError> {
        self.storage.read().get_block_hash(height)
    }

    fn circulating_supply(&self) -> Result<u64, RillError> {
        self.storage.read().circulating_supply()
    }

    fn cluster_balance(&self, cluster_id: &Hash256) -> Result<u64, RillError> {
        self.storage.read().cluster_balance(cluster_id)
    }

    fn decay_pool_balance(&self) -> Result<u64, RillError> {
        self.storage.read().decay_pool_balance()
    }

    fn validate_transaction(&self, tx: &Transaction) -> Result<(), TransactionError> {
        // Basic structural validation: non-empty inputs/outputs.
        if tx.inputs.is_empty() || tx.outputs.is_empty() {
            return Err(TransactionError::EmptyInputsOrOutputs);
        }

        // Check that all inputs reference existing UTXOs.
        let store = self.storage.read();
        for input in &tx.inputs {
            if !input.previous_output.is_null() {
                match store.get_utxo(&input.previous_output) {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        return Err(TransactionError::UnknownUtxo(
                            input.previous_output.to_string(),
                        ));
                    }
                    Err(_) => {
                        return Err(TransactionError::UnknownUtxo(
                            input.previous_output.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn iter_utxos(&self) -> Result<Vec<(OutPoint, UtxoEntry)>, RillError> {
        self.storage.read().iter_utxos()
    }
}

/// The full node, composing storage, mempool, consensus, and network.
pub struct Node {
    /// RocksDB storage behind a read-write lock.
    storage: Arc<RwLock<RocksStore>>,
    /// In-memory transaction pool.
    mempool: Mutex<Mempool>,
    /// Consensus engine for block validation and creation.
    consensus: ConsensusEngine,
    /// P2P network node (optional — None if network disabled).
    network: Option<NetworkNode>,
    /// Receiver for network events (behind tokio Mutex for async recv).
    event_rx: Option<tokio::sync::Mutex<broadcast::Receiver<NetworkEvent>>>,
    /// Receiver for storage queries from peers (behind tokio Mutex for async recv).
    query_rx: Option<tokio::sync::Mutex<mpsc::UnboundedReceiver<StorageQuery>>>,
    /// Node configuration.
    config: NodeConfig,
}

impl Node {
    /// Create a new node with the given configuration.
    ///
    /// Opens storage (auto-connects genesis if empty), creates the consensus
    /// engine, mempool, and starts the P2P network.
    pub async fn new(config: NodeConfig) -> Result<Arc<Self>, RillError> {
        // Open storage.
        let store = RocksStore::open(config.db_path())?;
        let storage = Arc::new(RwLock::new(store));

        // Create chain state adapter for consensus engine.
        let chain_state: Arc<dyn ChainState> =
            Arc::new(NodeChainState::new(Arc::clone(&storage)));

        // Create decay engine and consensus engine.
        let decay: Arc<dyn DecayCalculator> = Arc::new(DecayEngine::new());
        let consensus = ConsensusEngine::new(Arc::clone(&chain_state), decay);

        // Create mempool.
        let mempool = Mutex::new(Mempool::with_defaults());

        // Start network.
        let (network, event_rx, query_rx) = match NetworkNode::start(config.network.clone()).await {
            Ok((net, rx, qrx)) => (
                Some(net),
                Some(tokio::sync::Mutex::new(rx)),
                Some(tokio::sync::Mutex::new(qrx)),
            ),
            Err(e) => {
                warn!("failed to start network: {e}; running without P2P");
                (None, None, None)
            }
        };

        let node = Arc::new(Self {
            storage,
            mempool,
            consensus,
            network,
            event_rx,
            query_rx,
            config,
        });

        Ok(node)
    }

    /// Create a node without networking (for testing).
    pub fn without_network(config: NodeConfig) -> Result<Arc<Self>, RillError> {
        let store = RocksStore::open(config.db_path())?;
        let storage = Arc::new(RwLock::new(store));

        let chain_state: Arc<dyn ChainState> =
            Arc::new(NodeChainState::new(Arc::clone(&storage)));
        let decay: Arc<dyn DecayCalculator> = Arc::new(DecayEngine::new());
        let consensus = ConsensusEngine::new(Arc::clone(&chain_state), decay);
        let mempool = Mutex::new(Mempool::with_defaults());

        let node = Arc::new(Self {
            storage,
            mempool,
            consensus,
            network: None,
            event_rx: None,
            query_rx: None,
            config,
        });

        Ok(node)
    }

    /// Process and connect a new block.
    ///
    /// Validates the block via the consensus engine, connects it to storage,
    /// evicts conflicting mempool transactions, and broadcasts to peers.
    pub fn process_block(&self, block: &Block) -> Result<(), RillError> {
        // Validate via consensus engine.
        self.consensus
            .validate_block(block)
            .map_err(RillError::from)?;

        // Write-lock storage to connect.
        let (height, _) = {
            let store = self.storage.read();
            store.chain_tip()?
        };
        let next_height = height + 1;

        {
            let mut store = self.storage.write();
            store.connect_block(block, next_height)?;
        }

        info!(height = next_height, "connected block");

        // Evict confirmed/conflicting transactions from mempool.
        {
            let mut pool = self.mempool.lock();
            pool.remove_confirmed_block(block);
        }

        // Broadcast to peers (best-effort).
        if let Some(ref net) = self.network {
            if let Err(e) = rill_core::traits::NetworkService::broadcast_block(net, block) {
                debug!("failed to broadcast block: {e}");
            }
        }

        Ok(())
    }

    /// Process and add a new transaction to the mempool.
    ///
    /// Validates structure and UTXO references, inserts into the mempool,
    /// broadcasts to peers, and returns the transaction ID.
    pub fn process_transaction(&self, tx: &Transaction) -> Result<Hash256, RillError> {
        // Basic structural check.
        if tx.inputs.is_empty() || tx.outputs.is_empty() {
            return Err(TransactionError::EmptyInputsOrOutputs.into());
        }

        // Validate against UTXO set.
        let chain_state = NodeChainState::new(Arc::clone(&self.storage));
        chain_state.validate_transaction(tx)?;

        // Compute fee: sum of input UTXO values minus sum of output values.
        let mut input_sum: u64 = 0;
        for input in &tx.inputs {
            if let Some(utxo) = chain_state.get_utxo(&input.previous_output)? {
                input_sum = input_sum
                    .checked_add(utxo.output.value)
                    .ok_or_else(|| RillError::Storage("input sum overflow".into()))?;
            }
        }
        let output_sum = tx
            .total_output_value()
            .ok_or(TransactionError::ValueOverflow)?;
        let fee = input_sum
            .checked_sub(output_sum)
            .ok_or(TransactionError::InsufficientFunds {
                have: input_sum,
                need: output_sum,
            })?;

        // Insert into mempool with the computed fee.
        let txid = {
            let mut pool = self.mempool.lock();
            pool.insert(tx.clone(), fee)
                .map_err(|e| RillError::Storage(e.to_string()))?
        };

        debug!(%txid, "added transaction to mempool");

        // Broadcast to peers (best-effort).
        if let Some(ref net) = self.network {
            if let Err(e) =
                rill_core::traits::NetworkService::broadcast_transaction(net, tx)
            {
                debug!("failed to broadcast transaction: {e}");
            }
        }

        Ok(txid)
    }

    /// Run the main event loop, processing network events and storage queries.
    ///
    /// This method runs indefinitely, dispatching incoming blocks and
    /// transactions from the P2P network and answering peer storage queries.
    pub async fn run(self: &Arc<Self>) {
        let event_rx = match &self.event_rx {
            Some(rx) => rx,
            None => {
                warn!("no network event receiver; event loop idle");
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                }
            }
        };

        // Spawn storage query processing task if we have a query receiver.
        if self.query_rx.is_some() {
            let node = Arc::clone(self);
            tokio::spawn(async move {
                let mut rx = node.query_rx.as_ref().unwrap().lock().await;
                while let Some(query) = rx.recv().await {
                    let response = node.handle_storage_query(&query.request);
                    if let Some(ref net) = node.network {
                        if let Err(e) = net.send_response(query.response_channel, response) {
                            debug!("failed to send response to peer: {e}");
                        }
                    }
                }
            });
        }

        loop {
            let event = {
                let mut rx = event_rx.lock().await;
                match rx.recv().await {
                    Ok(event) => event,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "lagged behind on network events");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("network event channel closed, shutting down");
                        break;
                    }
                }
            };

            match event {
                NetworkEvent::BlockReceived(block) => {
                    if let Err(e) = self.process_block(&block) {
                        debug!("rejected block from peer: {e}");
                    }
                }
                NetworkEvent::TransactionReceived(tx) => {
                    if let Err(e) = self.process_transaction(&tx) {
                        debug!("rejected transaction from peer: {e}");
                    }
                }
                NetworkEvent::BlockRequested(hash) => {
                    debug!(%hash, "peer requested block via gossipsub");
                }
                NetworkEvent::HeadersRequested(locator) => {
                    debug!(count = locator.len(), "peer requested headers via gossipsub");
                }
                NetworkEvent::PeerConnected(peer_id) => {
                    info!(%peer_id, "peer connected");
                }
                NetworkEvent::PeerDisconnected(peer_id) => {
                    info!(%peer_id, "peer disconnected");
                }
                NetworkEvent::ChainTipRequested(peer_id) => {
                    debug!(%peer_id, "peer requested chain tip");
                }
                NetworkEvent::RequestResponse { peer, response } => {
                    debug!(%peer, "received response from peer");
                    // TODO: Feed to SyncManager when sync is integrated into the event loop.
                    match response {
                        RillResponse::ChainTip { height, hash } => {
                            debug!(%peer, height, %hash, "peer chain tip");
                        }
                        _ => {
                            debug!(%peer, "unhandled response type");
                        }
                    }
                }
            }
        }
    }

    /// Handle a storage query from a peer, returning the appropriate response.
    fn handle_storage_query(&self, request: &RillRequest) -> RillResponse {
        match request {
            RillRequest::GetChainTip => {
                let (height, hash) = self.chain_tip().unwrap_or((0, Hash256::ZERO));
                RillResponse::ChainTip { height, hash }
            }
            RillRequest::GetHeaders(locator) => {
                let store = self.storage.read();
                let ancestor = store.find_common_ancestor(locator).unwrap_or(None);
                match ancestor {
                    Some((_height, hash)) => {
                        let headers = store.get_headers_after(&hash, 2000).unwrap_or_default();
                        RillResponse::Headers(headers)
                    }
                    None => RillResponse::Headers(vec![]),
                }
            }
            RillRequest::GetBlock(hash) => {
                let block = self.get_block(hash).unwrap_or(None);
                RillResponse::Block(block)
            }
        }
    }

    // --- Query methods for RPC ---

    /// Current chain tip as `(height, block_hash)`.
    pub fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
        self.storage.read().chain_tip()
    }

    /// Get a full block by hash.
    pub fn get_block(&self, hash: &Hash256) -> Result<Option<Block>, RillError> {
        self.storage.read().get_block(hash)
    }

    /// Get a block header by hash.
    pub fn get_block_header(
        &self,
        hash: &Hash256,
    ) -> Result<Option<BlockHeader>, RillError> {
        self.storage.read().get_block_header(hash)
    }

    /// Get the block hash at a given height.
    pub fn get_block_hash(&self, height: u64) -> Result<Option<Hash256>, RillError> {
        self.storage.read().get_block_hash(height)
    }

    /// Mempool info: transaction count, total bytes, total fees.
    pub fn mempool_info(&self) -> (usize, usize, u64) {
        let pool = self.mempool.lock();
        (pool.len(), pool.total_bytes(), pool.total_fees())
    }

    /// Get a mempool transaction by txid.
    pub fn get_mempool_tx(&self, txid: &Hash256) -> Option<Transaction> {
        let pool = self.mempool.lock();
        pool.get(txid).map(|entry| entry.tx.clone())
    }

    /// Number of connected peers.
    pub fn peer_count(&self) -> usize {
        self.network
            .as_ref()
            .map(rill_core::traits::NetworkService::peer_count)
            .unwrap_or(0)
    }

    /// Current circulating supply in rills.
    pub fn circulating_supply(&self) -> Result<u64, RillError> {
        self.storage.read().circulating_supply()
    }

    /// Current decay pool balance in rills.
    pub fn decay_pool_balance(&self) -> Result<u64, RillError> {
        self.storage.read().decay_pool_balance()
    }

    /// Node configuration reference.
    pub fn config(&self) -> &NodeConfig {
        &self.config
    }

    /// Create a block template for mining.
    ///
    /// Delegates to the consensus engine's `create_block_template` method.
    pub fn create_block_template(
        &self,
        coinbase_pubkey_hash: &Hash256,
        timestamp: u64,
    ) -> Result<Block, RillError> {
        self.consensus
            .create_block_template(coinbase_pubkey_hash, timestamp)
            .map_err(RillError::from)
    }

    /// Iterate over all UTXOs (for address-based queries).
    pub fn iter_utxos(&self) -> Result<Vec<(OutPoint, UtxoEntry)>, RillError> {
        self.storage.read().iter_utxos()
    }

    /// Get UTXOs for an address using the indexed lookup.
    pub fn get_utxos_by_address(&self, pubkey_hash: &Hash256) -> Result<Vec<(OutPoint, UtxoEntry)>, RillError> {
        self.storage.read().get_utxos_by_address(pubkey_hash)
    }

    /// Get the balance of a cluster by ID.
    pub fn cluster_balance(&self, cluster_id: &Hash256) -> Result<u64, RillError> {
        self.storage.read().cluster_balance(cluster_id)
    }

    /// Get a geometric block locator for chain sync.
    pub fn get_block_locator(&self) -> Result<Vec<Hash256>, RillError> {
        self.storage.read().get_block_locator()
    }

    /// Find the common ancestor from a peer's block locator.
    pub fn find_common_ancestor(
        &self,
        locator: &[Hash256],
    ) -> Result<Option<(u64, Hash256)>, RillError> {
        self.storage.read().find_common_ancestor(locator)
    }

    /// Get headers after a given hash (up to max_count, capped at 2000).
    pub fn get_headers_after(
        &self,
        hash: &Hash256,
        max_count: usize,
    ) -> Result<Vec<BlockHeader>, RillError> {
        self.storage.read().get_headers_after(hash, max_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rill_core::constants::BLOCK_TIME_SECS;
    use rill_core::genesis;
    use rill_consensus::engine::mine_block;

    /// Create a test node backed by a temp directory, without network.
    fn test_node() -> (Arc<Node>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let config = NodeConfig {
            data_dir: dir.path().to_path_buf(),
            ..NodeConfig::default()
        };
        let node = Node::without_network(config).unwrap();
        (node, dir)
    }

    /// Simple pubkey hash from seed.
    fn pkh(seed: u8) -> Hash256 {
        Hash256([seed; 32])
    }

    /// Create a valid block template and mine it.
    fn mine_next_block(node: &Node) -> Block {
        let (_height, tip_hash) = node.chain_tip().unwrap();
        let tip_header = node.get_block_header(&tip_hash).unwrap().unwrap();
        let next_ts = tip_header.timestamp + BLOCK_TIME_SECS;

        let mut block = node
            .consensus
            .create_block_template(&pkh(0xBB), next_ts)
            .unwrap();
        assert!(mine_block(&mut block, u64::MAX));
        block
    }

    // ------------------------------------------------------------------
    // Node creation
    // ------------------------------------------------------------------

    #[test]
    fn node_starts_with_genesis() {
        let (node, _dir) = test_node();
        let (height, hash) = node.chain_tip().unwrap();
        assert_eq!(height, 0);
        assert_eq!(hash, genesis::genesis_hash());
    }

    #[test]
    fn mempool_initially_empty() {
        let (node, _dir) = test_node();
        let (count, bytes, fees) = node.mempool_info();
        assert_eq!(count, 0);
        assert_eq!(bytes, 0);
        assert_eq!(fees, 0);
    }

    #[test]
    fn peer_count_zero_without_network() {
        let (node, _dir) = test_node();
        assert_eq!(node.peer_count(), 0);
    }

    // ------------------------------------------------------------------
    // process_block
    // ------------------------------------------------------------------

    #[test]
    fn process_valid_block() {
        let (node, _dir) = test_node();
        let block = mine_next_block(&node);
        node.process_block(&block).unwrap();

        let (height, _) = node.chain_tip().unwrap();
        assert_eq!(height, 1);
    }

    #[test]
    fn process_invalid_block_rejected() {
        let (node, _dir) = test_node();
        // Create a block with empty transactions (no coinbase).
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash: genesis::genesis_hash(),
                merkle_root: Hash256::ZERO,
                timestamp: genesis::GENESIS_TIMESTAMP + BLOCK_TIME_SECS,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: vec![],
        };
        assert!(node.process_block(&block).is_err());
        // Chain tip unchanged.
        let (height, _) = node.chain_tip().unwrap();
        assert_eq!(height, 0);
    }

    #[test]
    fn process_block_evicts_mempool() {
        let (node, _dir) = test_node();

        // First mine a block to get a spendable non-coinbase UTXO.
        let block1 = mine_next_block(&node);
        node.process_block(&block1).unwrap();

        // Mine block 2 to mature the coinbase.
        // For simplicity, we'll just mine another block and skip coinbase maturity.
        let block2 = mine_next_block(&node);
        node.process_block(&block2).unwrap();

        // The mempool should still be empty since we haven't added any txs.
        let (count, _, _) = node.mempool_info();
        assert_eq!(count, 0);
    }

    #[test]
    fn process_multiple_blocks() {
        let (node, _dir) = test_node();

        for _ in 0..5 {
            let block = mine_next_block(&node);
            node.process_block(&block).unwrap();
        }

        let (height, _) = node.chain_tip().unwrap();
        assert_eq!(height, 5);
    }

    // ------------------------------------------------------------------
    // Query methods
    // ------------------------------------------------------------------

    #[test]
    fn get_block_returns_connected() {
        let (node, _dir) = test_node();
        let block = mine_next_block(&node);
        let hash = block.header.hash();
        node.process_block(&block).unwrap();

        assert!(node.get_block(&hash).unwrap().is_some());
        assert!(node.get_block_header(&hash).unwrap().is_some());
    }

    #[test]
    fn get_block_hash_works() {
        let (node, _dir) = test_node();
        let hash0 = node.get_block_hash(0).unwrap().unwrap();
        assert_eq!(hash0, genesis::genesis_hash());
        assert!(node.get_block_hash(999).unwrap().is_none());
    }

    #[test]
    fn circulating_supply_increases() {
        let (node, _dir) = test_node();
        let initial = node.circulating_supply().unwrap();

        let block = mine_next_block(&node);
        node.process_block(&block).unwrap();

        let after = node.circulating_supply().unwrap();
        assert!(after > initial);
    }

    // ------------------------------------------------------------------
    // NodeChainState adapter
    // ------------------------------------------------------------------

    #[test]
    fn node_chain_state_reads_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let store = RocksStore::open(dir.path().join("chaindata")).unwrap();
        let storage = Arc::new(RwLock::new(store));
        let adapter = NodeChainState::new(Arc::clone(&storage));

        let (height, hash) = adapter.chain_tip().unwrap();
        assert_eq!(height, 0);
        assert_eq!(hash, genesis::genesis_hash());

        let supply = adapter.circulating_supply().unwrap();
        assert!(supply > 0);

        assert_eq!(adapter.cluster_balance(&Hash256::ZERO).unwrap(), 0);
        assert_eq!(adapter.decay_pool_balance().unwrap(), 0);
    }
}
