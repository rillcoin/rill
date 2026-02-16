//! Trait interfaces for the Rill protocol.
//!
//! These traits define the contracts between crates:
//! - [`ChainState`] — read-only blockchain state (rill-node implements)
//! - [`DecayCalculator`] — decay math engine (rill-decay implements)
//! - [`BlockProducer`] — block creation and validation (rill-consensus implements)
//! - [`NetworkService`] — P2P networking (rill-network implements)

use crate::error::{BlockError, DecayError, NetworkError, RillError, TransactionError};
use crate::types::{Block, BlockHeader, Hash256, OutPoint, Transaction, UtxoEntry};

/// Read-only view of the blockchain state.
///
/// Provides access to the UTXO set, block headers, chain tip, and
/// aggregate state needed for validation and decay computation.
/// Implemented by the full node (rill-node) backed by RocksDB.
pub trait ChainState: Send + Sync {
    /// Look up a UTXO by outpoint. Returns `None` if spent or unknown.
    fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UtxoEntry>, RillError>;

    /// Check whether a UTXO exists and is unspent.
    ///
    /// Default implementation delegates to [`get_utxo`](Self::get_utxo).
    fn contains_utxo(&self, outpoint: &OutPoint) -> Result<bool, RillError> {
        Ok(self.get_utxo(outpoint)?.is_some())
    }

    /// Current chain tip as `(height, block_hash)`.
    fn chain_tip(&self) -> Result<(u64, Hash256), RillError>;

    /// Get a block header by its hash. Returns `None` if not found.
    fn get_block_header(&self, hash: &Hash256) -> Result<Option<BlockHeader>, RillError>;

    /// Get a full block by its hash. Returns `None` if not found.
    fn get_block(&self, hash: &Hash256) -> Result<Option<Block>, RillError>;

    /// Get the block hash at a given height. Returns `None` if height exceeds tip.
    fn get_block_hash(&self, height: u64) -> Result<Option<Hash256>, RillError>;

    /// Total circulating supply in rills (excludes decay pool and unmined coins).
    fn circulating_supply(&self) -> Result<u64, RillError>;

    /// Total balance of a decay-tracking cluster in rills.
    fn cluster_balance(&self, cluster_id: &Hash256) -> Result<u64, RillError>;

    /// Current balance of the decay pool in rills.
    fn decay_pool_balance(&self) -> Result<u64, RillError>;

    /// Validate a transaction against the current UTXO set and consensus rules.
    fn validate_transaction(&self, tx: &Transaction) -> Result<(), TransactionError>;

    /// Iterate over all UTXOs. Used for balance queries and UTXO scanning.
    /// Default implementation returns empty vec (override for production).
    fn iter_utxos(&self) -> Result<Vec<(OutPoint, UtxoEntry)>, RillError> {
        Ok(Vec::new())
    }
}

/// Pure computation of decay rates and effective values.
///
/// All decay math uses integer arithmetic with fixed-point precision.
/// The sigmoid-based decay curve maps cluster concentration to a per-block
/// decay rate. Implemented by the decay engine (rill-decay).
pub trait DecayCalculator: Send + Sync {
    /// Per-block decay rate for a given concentration, in parts-per-billion.
    ///
    /// `concentration_ppb` is `cluster_balance * CONCENTRATION_PRECISION / circulating_supply`.
    /// Returns 0 below the threshold, ramping up via sigmoid to `DECAY_R_MAX_PPB`.
    fn decay_rate_ppb(&self, concentration_ppb: u64) -> Result<u64, DecayError>;

    /// Total decay amount for a nominal value held at a given concentration for `blocks_held` blocks.
    ///
    /// Returns the absolute amount (in rills) that has decayed away.
    fn compute_decay(
        &self,
        nominal_value: u64,
        concentration_ppb: u64,
        blocks_held: u64,
    ) -> Result<u64, DecayError>;

    /// Effective (post-decay) value after holding at a given concentration for `blocks_held` blocks.
    ///
    /// Default implementation: `nominal_value - compute_decay(...)`.
    fn effective_value(
        &self,
        nominal_value: u64,
        concentration_ppb: u64,
        blocks_held: u64,
    ) -> Result<u64, DecayError> {
        let decay = self.compute_decay(nominal_value, concentration_ppb, blocks_held)?;
        nominal_value
            .checked_sub(decay)
            .ok_or(DecayError::ArithmeticOverflow)
    }

    /// Amount released from the decay pool to miners for the next block.
    ///
    /// `pool_balance` is the current decay pool balance in rills.
    /// Typically `pool_balance * DECAY_POOL_RELEASE_BPS / BPS_PRECISION`.
    fn decay_pool_release(&self, pool_balance: u64) -> Result<u64, DecayError>;
}

/// Block creation, validation, and reward computation.
///
/// Used by the miner to create block templates and by the node
/// to validate incoming blocks. Implemented by the consensus engine (rill-consensus).
pub trait BlockProducer: Send + Sync {
    /// Create a block template with selected mempool transactions and coinbase.
    ///
    /// The coinbase output pays to `coinbase_pubkey_hash` with the appropriate reward.
    /// `timestamp` is the proposed block timestamp (Unix seconds).
    fn create_block_template(
        &self,
        coinbase_pubkey_hash: &Hash256,
        timestamp: u64,
    ) -> Result<Block, BlockError>;

    /// Validate a complete block: header PoW, merkle root, all transactions, and reward.
    fn validate_block(&self, block: &Block) -> Result<(), BlockError>;

    /// Compute the base mining reward for a given block height (before decay pool bonus).
    ///
    /// Follows the halving schedule: `INITIAL_REWARD >> (height / HALVING_INTERVAL)`.
    fn block_reward(&self, height: u64) -> u64;

    /// Compute the difficulty target for a given block height.
    fn difficulty_target(&self, height: u64) -> Result<u64, BlockError>;

    /// Validate proof-of-work: block header hash must be numerically below the difficulty target.
    fn validate_pow(&self, header: &BlockHeader) -> Result<(), BlockError>;
}

/// P2P network operations.
///
/// Abstracts block and transaction propagation over libp2p.
/// Implementations handle the actual transport, peer management,
/// and Gossipsub protocol. Implemented by rill-network.
pub trait NetworkService: Send + Sync {
    /// Broadcast a validated block to all connected peers.
    fn broadcast_block(&self, block: &Block) -> Result<(), NetworkError>;

    /// Broadcast a validated transaction to all connected peers.
    fn broadcast_transaction(&self, tx: &Transaction) -> Result<(), NetworkError>;

    /// Number of currently connected peers.
    fn peer_count(&self) -> usize;

    /// Whether the node has at least one connected peer.
    ///
    /// Default implementation: `peer_count() > 0`.
    fn is_connected(&self) -> bool {
        self.peer_count() > 0
    }

    /// Request a specific block from peers by hash.
    fn request_block(&self, hash: &Hash256) -> Result<(), NetworkError>;

    /// Request block headers starting from the given locator hashes.
    ///
    /// Locator hashes are ordered newest-first, allowing peers to find
    /// the common ancestor and send headers from there.
    fn request_headers(&self, locator: &[Hash256]) -> Result<(), NetworkError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants;
    use crate::types::TxOutput;
    use std::collections::HashMap;

    // ------------------------------------------------------------------
    // Mock: ChainState
    // ------------------------------------------------------------------

    struct MockChainState {
        utxos: HashMap<OutPoint, UtxoEntry>,
        tip_height: u64,
        tip_hash: Hash256,
        supply: u64,
        clusters: HashMap<Hash256, u64>,
        pool: u64,
    }

    impl MockChainState {
        fn new() -> Self {
            Self {
                utxos: HashMap::new(),
                tip_height: 0,
                tip_hash: Hash256::ZERO,
                supply: 0,
                clusters: HashMap::new(),
                pool: 0,
            }
        }

        fn insert_utxo(&mut self, outpoint: OutPoint, entry: UtxoEntry) {
            self.utxos.insert(outpoint, entry);
        }
    }

    impl ChainState for MockChainState {
        fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UtxoEntry>, RillError> {
            Ok(self.utxos.get(outpoint).cloned())
        }

        fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
            Ok((self.tip_height, self.tip_hash))
        }

        fn get_block_header(&self, _hash: &Hash256) -> Result<Option<BlockHeader>, RillError> {
            Ok(None)
        }

        fn get_block(&self, _hash: &Hash256) -> Result<Option<Block>, RillError> {
            Ok(None)
        }

        fn get_block_hash(&self, _height: u64) -> Result<Option<Hash256>, RillError> {
            Ok(None)
        }

        fn circulating_supply(&self) -> Result<u64, RillError> {
            Ok(self.supply)
        }

        fn cluster_balance(&self, cluster_id: &Hash256) -> Result<u64, RillError> {
            Ok(*self.clusters.get(cluster_id).unwrap_or(&0))
        }

        fn decay_pool_balance(&self) -> Result<u64, RillError> {
            Ok(self.pool)
        }

        fn validate_transaction(&self, tx: &Transaction) -> Result<(), TransactionError> {
            if tx.inputs.is_empty() || tx.outputs.is_empty() {
                return Err(TransactionError::EmptyInputsOrOutputs);
            }
            for input in &tx.inputs {
                if !input.previous_output.is_null()
                    && !self.utxos.contains_key(&input.previous_output)
                {
                    return Err(TransactionError::UnknownUtxo(
                        input.previous_output.to_string(),
                    ));
                }
            }
            Ok(())
        }
    }

    // ------------------------------------------------------------------
    // Mock: DecayCalculator
    // ------------------------------------------------------------------

    struct MockDecayCalculator;

    impl DecayCalculator for MockDecayCalculator {
        fn decay_rate_ppb(&self, concentration_ppb: u64) -> Result<u64, DecayError> {
            if concentration_ppb > constants::DECAY_C_THRESHOLD_PPB {
                Ok(10_000_000) // 1% in parts-per-billion
            } else {
                Ok(0)
            }
        }

        fn compute_decay(
            &self,
            nominal_value: u64,
            concentration_ppb: u64,
            blocks_held: u64,
        ) -> Result<u64, DecayError> {
            let rate = self.decay_rate_ppb(concentration_ppb)?;
            let per_block = nominal_value
                .checked_mul(rate)
                .and_then(|v| v.checked_div(constants::DECAY_PRECISION))
                .ok_or(DecayError::ArithmeticOverflow)?;
            per_block
                .checked_mul(blocks_held)
                .ok_or(DecayError::ArithmeticOverflow)
        }

        fn decay_pool_release(&self, pool_balance: u64) -> Result<u64, DecayError> {
            Ok(pool_balance * constants::DECAY_POOL_RELEASE_BPS / constants::BPS_PRECISION)
        }
    }

    // ------------------------------------------------------------------
    // Mock: BlockProducer
    // ------------------------------------------------------------------

    struct MockBlockProducer;

    impl BlockProducer for MockBlockProducer {
        fn create_block_template(
            &self,
            coinbase_pubkey_hash: &Hash256,
            timestamp: u64,
        ) -> Result<Block, BlockError> {
            let coinbase = Transaction {
                version: 1,
                inputs: vec![crate::types::TxInput {
                    previous_output: OutPoint::null(),
                    signature: vec![],
                    public_key: vec![],
                }],
                outputs: vec![TxOutput {
                    value: self.block_reward(0),
                    pubkey_hash: *coinbase_pubkey_hash,
                }],
                lock_time: 0,
            };
            Ok(Block {
                header: BlockHeader {
                    version: 1,
                    prev_hash: Hash256::ZERO,
                    merkle_root: Hash256::ZERO,
                    timestamp,
                    difficulty_target: u64::MAX,
                    nonce: 0,
                },
                transactions: vec![coinbase],
            })
        }

        fn validate_block(&self, block: &Block) -> Result<(), BlockError> {
            if block.transactions.is_empty() {
                return Err(BlockError::NoCoinbase);
            }
            self.validate_pow(&block.header)?;
            Ok(())
        }

        fn block_reward(&self, height: u64) -> u64 {
            let halvings = height / constants::HALVING_INTERVAL;
            if halvings >= 64 {
                return 0;
            }
            constants::INITIAL_REWARD >> halvings
        }

        fn difficulty_target(&self, _height: u64) -> Result<u64, BlockError> {
            Ok(u64::MAX)
        }

        fn validate_pow(&self, _header: &BlockHeader) -> Result<(), BlockError> {
            // Mock: accept everything
            Ok(())
        }
    }

    // ------------------------------------------------------------------
    // Mock: NetworkService
    // ------------------------------------------------------------------

    struct MockNetworkService {
        peers: usize,
    }

    impl MockNetworkService {
        fn new(peers: usize) -> Self {
            Self { peers }
        }
    }

    impl NetworkService for MockNetworkService {
        fn broadcast_block(&self, _block: &Block) -> Result<(), NetworkError> {
            if self.peers == 0 {
                return Err(NetworkError::PeerDisconnected("no peers".into()));
            }
            Ok(())
        }

        fn broadcast_transaction(&self, _tx: &Transaction) -> Result<(), NetworkError> {
            if self.peers == 0 {
                return Err(NetworkError::PeerDisconnected("no peers".into()));
            }
            Ok(())
        }

        fn peer_count(&self) -> usize {
            self.peers
        }

        fn request_block(&self, _hash: &Hash256) -> Result<(), NetworkError> {
            if self.peers == 0 {
                return Err(NetworkError::PeerDisconnected("no peers".into()));
            }
            Ok(())
        }

        fn request_headers(&self, _locator: &[Hash256]) -> Result<(), NetworkError> {
            if self.peers == 0 {
                return Err(NetworkError::PeerDisconnected("no peers".into()));
            }
            Ok(())
        }
    }

    // ------------------------------------------------------------------
    // Object safety: verify each trait is dyn-compatible
    // ------------------------------------------------------------------

    fn _assert_chain_state_object_safe(cs: &dyn ChainState) {
        let _ = cs.chain_tip();
    }

    fn _assert_decay_calculator_object_safe(dc: &dyn DecayCalculator) {
        let _ = dc.decay_rate_ppb(0);
    }

    fn _assert_block_producer_object_safe(bp: &dyn BlockProducer) {
        let _ = bp.block_reward(0);
    }

    fn _assert_network_service_object_safe(ns: &dyn NetworkService) {
        let _ = ns.peer_count();
    }

    // ------------------------------------------------------------------
    // ChainState tests
    // ------------------------------------------------------------------

    #[test]
    fn chain_state_get_utxo_found() {
        let mut cs = MockChainState::new();
        let op = OutPoint { txid: Hash256([1; 32]), index: 0 };
        let entry = UtxoEntry {
            output: TxOutput { value: 100, pubkey_hash: Hash256::ZERO },
            block_height: 0,
            is_coinbase: false,
            cluster_id: Hash256::ZERO,
        };
        cs.insert_utxo(op.clone(), entry.clone());

        let result = cs.get_utxo(&op).unwrap();
        assert_eq!(result, Some(entry));
    }

    #[test]
    fn chain_state_get_utxo_missing() {
        let cs = MockChainState::new();
        let op = OutPoint { txid: Hash256([1; 32]), index: 0 };
        assert_eq!(cs.get_utxo(&op).unwrap(), None);
    }

    #[test]
    fn chain_state_contains_utxo_default() {
        let mut cs = MockChainState::new();
        let op = OutPoint { txid: Hash256([1; 32]), index: 0 };
        assert!(!cs.contains_utxo(&op).unwrap());

        cs.insert_utxo(
            op.clone(),
            UtxoEntry {
                output: TxOutput { value: 1, pubkey_hash: Hash256::ZERO },
                block_height: 0,
                is_coinbase: false,
                cluster_id: Hash256::ZERO,
            },
        );
        assert!(cs.contains_utxo(&op).unwrap());
    }

    #[test]
    fn chain_state_tip() {
        let mut cs = MockChainState::new();
        cs.tip_height = 42;
        cs.tip_hash = Hash256([0xAA; 32]);

        let (h, hash) = cs.chain_tip().unwrap();
        assert_eq!(h, 42);
        assert_eq!(hash, Hash256([0xAA; 32]));
    }

    #[test]
    fn chain_state_supply_and_pool() {
        let mut cs = MockChainState::new();
        cs.supply = 1_000_000 * constants::COIN;
        cs.pool = 50_000 * constants::COIN;

        assert_eq!(cs.circulating_supply().unwrap(), 1_000_000 * constants::COIN);
        assert_eq!(cs.decay_pool_balance().unwrap(), 50_000 * constants::COIN);
    }

    #[test]
    fn chain_state_cluster_balance() {
        let mut cs = MockChainState::new();
        let cid = Hash256([0xBB; 32]);
        cs.clusters.insert(cid, 500 * constants::COIN);

        assert_eq!(cs.cluster_balance(&cid).unwrap(), 500 * constants::COIN);
        assert_eq!(cs.cluster_balance(&Hash256::ZERO).unwrap(), 0);
    }

    #[test]
    fn chain_state_validate_tx_unknown_utxo() {
        let cs = MockChainState::new();
        let tx = Transaction {
            version: 1,
            inputs: vec![crate::types::TxInput {
                previous_output: OutPoint { txid: Hash256([0xFF; 32]), index: 0 },
                signature: vec![0; 64],
                public_key: vec![0; 32],
            }],
            outputs: vec![TxOutput { value: 100, pubkey_hash: Hash256::ZERO }],
            lock_time: 0,
        };
        let err = cs.validate_transaction(&tx).unwrap_err();
        assert!(matches!(err, TransactionError::UnknownUtxo(_)));
    }

    #[test]
    fn chain_state_validate_tx_empty() {
        let cs = MockChainState::new();
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![],
            lock_time: 0,
        };
        let err = cs.validate_transaction(&tx).unwrap_err();
        assert_eq!(err, TransactionError::EmptyInputsOrOutputs);
    }

    #[test]
    fn chain_state_as_dyn() {
        let cs = MockChainState::new();
        let dyn_cs: &dyn ChainState = &cs;
        assert_eq!(dyn_cs.chain_tip().unwrap(), (0, Hash256::ZERO));
    }

    // ------------------------------------------------------------------
    // DecayCalculator tests
    // ------------------------------------------------------------------

    #[test]
    fn decay_zero_below_threshold() {
        let dc = MockDecayCalculator;
        assert_eq!(dc.decay_rate_ppb(0).unwrap(), 0);
        assert_eq!(
            dc.decay_rate_ppb(constants::DECAY_C_THRESHOLD_PPB).unwrap(),
            0
        );
    }

    #[test]
    fn decay_nonzero_above_threshold() {
        let dc = MockDecayCalculator;
        let rate = dc
            .decay_rate_ppb(constants::DECAY_C_THRESHOLD_PPB + 1)
            .unwrap();
        assert!(rate > 0);
    }

    #[test]
    fn compute_decay_zero_blocks() {
        let dc = MockDecayCalculator;
        let decay = dc
            .compute_decay(1_000 * constants::COIN, constants::DECAY_C_THRESHOLD_PPB + 1, 0)
            .unwrap();
        assert_eq!(decay, 0);
    }

    #[test]
    fn compute_decay_positive() {
        let dc = MockDecayCalculator;
        let value = 1_000 * constants::COIN;
        let conc = constants::DECAY_C_THRESHOLD_PPB + 1;
        let decay = dc.compute_decay(value, conc, 100).unwrap();
        assert!(decay > 0);
        assert!(decay < value);
    }

    #[test]
    fn effective_value_default_impl() {
        let dc = MockDecayCalculator;
        let value = 1_000 * constants::COIN;
        let conc = constants::DECAY_C_THRESHOLD_PPB + 1;
        let blocks = 100;

        let decay = dc.compute_decay(value, conc, blocks).unwrap();
        let effective = dc.effective_value(value, conc, blocks).unwrap();
        assert_eq!(effective, value - decay);
    }

    #[test]
    fn effective_value_no_decay_below_threshold() {
        let dc = MockDecayCalculator;
        let value = 500 * constants::COIN;
        let effective = dc.effective_value(value, 0, 1000).unwrap();
        assert_eq!(effective, value);
    }

    #[test]
    fn decay_pool_release_calculation() {
        let dc = MockDecayCalculator;
        let pool = 10_000 * constants::COIN;
        let release = dc.decay_pool_release(pool).unwrap();
        // 1% of pool
        assert_eq!(release, pool * constants::DECAY_POOL_RELEASE_BPS / constants::BPS_PRECISION);
    }

    #[test]
    fn decay_calculator_as_dyn() {
        let dc = MockDecayCalculator;
        let dyn_dc: &dyn DecayCalculator = &dc;
        assert_eq!(dyn_dc.decay_rate_ppb(0).unwrap(), 0);
    }

    // ------------------------------------------------------------------
    // BlockProducer tests
    // ------------------------------------------------------------------

    #[test]
    fn block_reward_halving() {
        let bp = MockBlockProducer;
        assert_eq!(bp.block_reward(0), constants::INITIAL_REWARD);
        assert_eq!(bp.block_reward(constants::HALVING_INTERVAL - 1), constants::INITIAL_REWARD);
        assert_eq!(bp.block_reward(constants::HALVING_INTERVAL), constants::INITIAL_REWARD / 2);
        assert_eq!(
            bp.block_reward(2 * constants::HALVING_INTERVAL),
            constants::INITIAL_REWARD / 4
        );
    }

    #[test]
    fn block_reward_eventually_zero() {
        let bp = MockBlockProducer;
        assert_eq!(bp.block_reward(64 * constants::HALVING_INTERVAL), 0);
    }

    #[test]
    fn create_block_template_has_coinbase() {
        let bp = MockBlockProducer;
        let pkh = Hash256([0xAA; 32]);
        let block = bp.create_block_template(&pkh, 1_700_000_000).unwrap();
        assert!(!block.transactions.is_empty());
        assert!(block.transactions[0].is_coinbase());
        assert_eq!(block.transactions[0].outputs[0].pubkey_hash, pkh);
    }

    #[test]
    fn validate_block_rejects_empty() {
        let bp = MockBlockProducer;
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash: Hash256::ZERO,
                merkle_root: Hash256::ZERO,
                timestamp: 0,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: vec![],
        };
        let err = bp.validate_block(&block).unwrap_err();
        assert_eq!(err, BlockError::NoCoinbase);
    }

    #[test]
    fn validate_block_accepts_valid() {
        let bp = MockBlockProducer;
        let block = bp
            .create_block_template(&Hash256::ZERO, 1_700_000_000)
            .unwrap();
        assert!(bp.validate_block(&block).is_ok());
    }

    #[test]
    fn difficulty_target_returns_value() {
        let bp = MockBlockProducer;
        let target = bp.difficulty_target(0).unwrap();
        assert_eq!(target, u64::MAX);
    }

    #[test]
    fn block_producer_as_dyn() {
        let bp = MockBlockProducer;
        let dyn_bp: &dyn BlockProducer = &bp;
        assert_eq!(dyn_bp.block_reward(0), constants::INITIAL_REWARD);
    }

    // ------------------------------------------------------------------
    // NetworkService tests
    // ------------------------------------------------------------------

    #[test]
    fn network_peer_count() {
        let ns = MockNetworkService::new(5);
        assert_eq!(ns.peer_count(), 5);
    }

    #[test]
    fn network_is_connected_default() {
        assert!(MockNetworkService::new(1).is_connected());
        assert!(!MockNetworkService::new(0).is_connected());
    }

    #[test]
    fn network_broadcast_block_succeeds() {
        let ns = MockNetworkService::new(3);
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash: Hash256::ZERO,
                merkle_root: Hash256::ZERO,
                timestamp: 0,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: vec![],
        };
        assert!(ns.broadcast_block(&block).is_ok());
    }

    #[test]
    fn network_broadcast_fails_no_peers() {
        let ns = MockNetworkService::new(0);
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash: Hash256::ZERO,
                merkle_root: Hash256::ZERO,
                timestamp: 0,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: vec![],
        };
        assert!(ns.broadcast_block(&block).is_err());
    }

    #[test]
    fn network_broadcast_tx_succeeds() {
        let ns = MockNetworkService::new(2);
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![],
            lock_time: 0,
        };
        assert!(ns.broadcast_transaction(&tx).is_ok());
    }

    #[test]
    fn network_request_block_succeeds() {
        let ns = MockNetworkService::new(1);
        assert!(ns.request_block(&Hash256([1; 32])).is_ok());
    }

    #[test]
    fn network_request_headers_succeeds() {
        let ns = MockNetworkService::new(1);
        let locator = vec![Hash256([1; 32]), Hash256([2; 32])];
        assert!(ns.request_headers(&locator).is_ok());
    }

    #[test]
    fn network_request_block_fails_no_peers() {
        let ns = MockNetworkService::new(0);
        assert!(ns.request_block(&Hash256([1; 32])).is_err());
    }

    #[test]
    fn network_service_as_dyn() {
        let ns = MockNetworkService::new(3);
        let dyn_ns: &dyn NetworkService = &ns;
        assert_eq!(dyn_ns.peer_count(), 3);
        assert!(dyn_ns.is_connected());
    }
}
