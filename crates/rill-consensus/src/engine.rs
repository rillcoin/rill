//! Consensus engine implementing the [`BlockProducer`] trait.
//!
//! Wires together rill-core's validation, difficulty, and reward modules with
//! chain state and decay calculator to provide a complete block production and
//! validation pipeline.
//!
//! Phase 1: Mock PoW using SHA-256 header hashing (built into rill-core).
//! Phase 2 will add RandomX FFI behind the same trait interface.

use std::fmt;
use std::sync::Arc;

use rill_core::block_validation::{self, BlockContext};
use rill_core::constants::MAX_COINBASE_DATA;
use rill_core::error::BlockError;
use rill_core::traits::{BlockProducer, ChainState, DecayCalculator};
use rill_core::types::{
    Block, BlockHeader, Hash256, OutPoint, Transaction, TxInput, TxOutput,
};
use rill_core::{difficulty, merkle, reward};

/// The production consensus engine.
///
/// Implements [`BlockProducer`] by combining chain state queries, decay pool
/// release, difficulty adjustment, and PoW validation.
///
/// Requires a non-empty chain (genesis block must already be connected).
///
/// When compiled with the `randomx` feature, block validation and mining use
/// ASIC-resistant RandomX hashing instead of SHA-256 double-hash.
pub struct ConsensusEngine {
    chain_state: Arc<dyn ChainState>,
    decay: Arc<dyn DecayCalculator>,
    clock: Box<dyn Fn() -> u64 + Send + Sync>,
    /// Override the initial difficulty target for heights 0 and 1.
    /// If `None`, uses `TESTNET_INITIAL_TARGET` from rill-core constants.
    initial_target_override: Option<u64>,
    #[cfg(feature = "randomx")]
    randomx_validator: crate::randomx::RandomXValidator,
}

impl fmt::Debug for ConsensusEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConsensusEngine").finish_non_exhaustive()
    }
}

impl ConsensusEngine {
    /// Create a new ConsensusEngine with the system clock.
    ///
    /// When the `randomx` feature is enabled, this also initializes a
    /// [`RandomXValidator`](crate::randomx::RandomXValidator) seeded with the
    /// genesis block hash.
    pub fn new(
        chain_state: Arc<dyn ChainState>,
        decay: Arc<dyn DecayCalculator>,
    ) -> Self {
        Self {
            #[cfg(feature = "randomx")]
            randomx_validator: {
                let genesis_hash = rill_core::genesis::genesis_hash();
                crate::randomx::RandomXValidator::new(0, &genesis_hash)
                    .expect("RandomX validator init failed")
            },
            chain_state,
            decay,
            clock: Box::new(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }),
            initial_target_override: None,
        }
    }

    /// Create a new ConsensusEngine with a custom clock for testing.
    ///
    /// When the `randomx` feature is enabled, this also initializes a
    /// [`RandomXValidator`](crate::randomx::RandomXValidator) seeded with the
    /// genesis block hash.
    pub fn with_clock(
        chain_state: Arc<dyn ChainState>,
        decay: Arc<dyn DecayCalculator>,
        clock: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> Self {
        Self {
            #[cfg(feature = "randomx")]
            randomx_validator: {
                let genesis_hash = rill_core::genesis::genesis_hash();
                crate::randomx::RandomXValidator::new(0, &genesis_hash)
                    .expect("RandomX validator init failed")
            },
            chain_state,
            decay,
            clock: Box::new(clock),
            initial_target_override: None,
        }
    }

    /// Override the initial difficulty target used for heights 0 and 1.
    ///
    /// This is intended for testing, where `u64::MAX` allows any hash to
    /// pass PoW so that tests can focus on other validation logic without
    /// needing to mine real nonces.
    ///
    /// Available when the crate is compiled under test (`#[cfg(test)]`) or
    /// when the `testing` feature is enabled, so downstream test suites can
    /// use this builder without enabling it in production builds.
    #[cfg(any(test, feature = "testing"))]
    pub fn with_initial_target(mut self, target: u64) -> Self {
        self.initial_target_override = Some(target);
        self
    }

    /// Compute the total block reward: base mining reward + decay pool release.
    fn total_reward(&self, height: u64) -> Result<u64, BlockError> {
        let base = reward::block_reward(height);
        let pool_balance = self
            .chain_state
            .decay_pool_balance()
            .map_err(|_| BlockError::InvalidReward {
                got: 0,
                expected: 0,
            })?;
        let pool_release = self
            .decay
            .decay_pool_release(pool_balance)
            .unwrap_or(0);
        Ok(base.saturating_add(pool_release))
    }

    /// Create a block template that includes pending mempool transactions.
    ///
    /// This is the primary block-building entry point. It constructs a coinbase
    /// transaction, then validates pending mempool transactions (filtering out
    /// any that spend immature coinbase outputs or missing UTXOs) and computes
    /// the merkle root over all included transactions.
    ///
    /// Transactions that fail UTXO lookup or coinbase maturity checks are
    /// silently skipped rather than causing the template to fail. This is safe
    /// because the miner should not be penalized for stale mempool entries.
    ///
    /// # Attack vectors
    ///
    /// - An adversary could flood the mempool with transactions spending
    ///   immature coinbase outputs. We filter these out here so that blocks
    ///   produced from templates never contain invalid transactions.
    /// - The caller is responsible for size budgeting; the block validator's
    ///   `validate_block_structure` enforces MAX_BLOCK_SIZE as a safety net.
    /// - Double-spend across included transactions is prevented by tracking
    ///   spent outpoints within the template.
    pub fn create_block_template_with_txs(
        &self,
        coinbase_pubkey_hash: &Hash256,
        timestamp: u64,
        pending_txs: &[Transaction],
    ) -> Result<Block, BlockError> {
        let (tip_height, tip_hash) = self
            .chain_state
            .chain_tip()
            .map_err(|_| BlockError::InvalidPrevHash)?;

        let height = tip_height + 1;
        let total_reward = self.total_reward(height)?;
        let difficulty_target = self.difficulty_target(height)?;

        // Ensure timestamp is strictly after the parent's to pass validation.
        let parent_header = self
            .chain_state
            .get_block_header(&tip_hash)
            .map_err(|_| BlockError::InvalidPrevHash)?
            .ok_or(BlockError::InvalidPrevHash)?;
        let timestamp = timestamp.max(parent_header.timestamp + 1);

        // Encode height in coinbase for uniqueness (truncated to MAX_COINBASE_DATA).
        let height_bytes = height.to_le_bytes();
        let len = height_bytes.len().min(MAX_COINBASE_DATA);

        // Select valid mempool transactions, filtering out those that:
        // 1. Spend UTXOs that do not exist (stale mempool entries)
        // 2. Spend immature coinbase outputs
        // 3. Would cause a double-spend within this block
        //
        // Size budgeting is the caller's responsibility: the node layer uses
        // `Mempool::select_transactions(max_block_bytes)` to pre-select
        // transactions that fit within MAX_BLOCK_SIZE. The block validator's
        // `validate_block_structure` check enforces the limit as a safety net.
        let mut included_txs: Vec<Transaction> = Vec::new();
        let mut spent_outpoints = std::collections::HashSet::new();
        let mut total_fees: u64 = 0;

        for tx in pending_txs {
            // Attack vector: adversary submits coinbase-like transaction to mempool.
            // Skip any transaction that claims to be a coinbase.
            if tx.is_coinbase() {
                continue;
            }

            // Validate all inputs: UTXO existence, coinbase maturity, no intra-block
            // double-spend.
            let mut tx_valid = true;
            let mut tx_input_value: u64 = 0;
            let mut tx_spent = Vec::new();

            for input in &tx.inputs {
                // Check for double-spend within the block being built.
                if spent_outpoints.contains(&input.previous_output) {
                    tx_valid = false;
                    break;
                }

                // Look up the UTXO from chain state.
                let utxo = match self.chain_state.get_utxo(&input.previous_output) {
                    Ok(Some(u)) => u,
                    _ => {
                        tx_valid = false;
                        break;
                    }
                };

                // Enforce coinbase maturity: immature coinbase outputs cannot be spent.
                if !utxo.is_mature(height) {
                    tx_valid = false;
                    break;
                }

                // Accumulate input value using checked arithmetic to prevent overflow.
                tx_input_value = match tx_input_value.checked_add(utxo.output.value) {
                    Some(v) => v,
                    None => {
                        tx_valid = false;
                        break;
                    }
                };

                tx_spent.push(input.previous_output.clone());
            }

            if !tx_valid {
                continue;
            }

            // Verify outputs do not exceed inputs (fee must be non-negative).
            let tx_output_value = match tx.total_output_value() {
                Some(v) if v <= tx_input_value => v,
                _ => continue,
            };

            let fee = tx_input_value - tx_output_value;

            // Commit: mark outpoints as spent and include the transaction.
            for op in tx_spent {
                spent_outpoints.insert(op);
            }

            total_fees = total_fees.saturating_add(fee);
            included_txs.push(tx.clone());
        }

        // Rebuild coinbase with total_reward + collected fees (checked arithmetic).
        let coinbase_value = total_reward
            .checked_add(total_fees)
            .ok_or(BlockError::InvalidReward {
                got: u64::MAX,
                expected: total_reward,
            })?;

        let coinbase = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: height_bytes[..len].to_vec(),
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: coinbase_value,
                pubkey_hash: *coinbase_pubkey_hash,
            }],
            lock_time: height,
        };

        // Assemble all transactions: coinbase first, then selected mempool txs.
        let mut all_txs = Vec::with_capacity(1 + included_txs.len());
        all_txs.push(coinbase);
        all_txs.extend(included_txs);

        // Compute merkle root over all transaction IDs.
        let txids: Vec<Hash256> = all_txs
            .iter()
            .enumerate()
            .map(|(i, tx)| {
                tx.txid().map_err(|e| BlockError::TransactionError {
                    index: i,
                    source: e,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let merkle_root = merkle::merkle_root(&txids);

        Ok(Block {
            header: BlockHeader {
                version: 1,
                prev_hash: tip_hash,
                merkle_root,
                timestamp,
                difficulty_target,
                nonce: 0,
            },
            transactions: all_txs,
        })
    }

    /// Look up a block timestamp by height from the chain state.
    ///
    /// Returns 0 if the block is not found (safety fallback for difficulty calc).
    fn timestamp_at(&self, height: u64) -> u64 {
        self.chain_state
            .get_block_hash(height)
            .ok()
            .flatten()
            .and_then(|hash| self.chain_state.get_block_header(&hash).ok().flatten())
            .map(|h| h.timestamp)
            .unwrap_or(0)
    }
}

impl BlockProducer for ConsensusEngine {
    fn block_reward(&self, height: u64) -> u64 {
        reward::block_reward(height)
    }

    fn validate_pow(&self, header: &BlockHeader) -> Result<(), BlockError> {
        // Attack vector: An adversary submits a block with a valid SHA-256 hash
        // but invalid RandomX hash (or vice versa). The cfg gate ensures only
        // one PoW algorithm is active at compile time, preventing this.
        #[cfg(feature = "randomx")]
        {
            let hash = self
                .randomx_validator
                .hash(&header.header_bytes())
                .map_err(|_| BlockError::InvalidPoW)?;
            let hash_prefix =
                u64::from_le_bytes(hash.0[0..8].try_into().expect("hash is 32 bytes"));
            if hash_prefix <= header.difficulty_target {
                return Ok(());
            }
            return Err(BlockError::InvalidPoW);
        }

        #[cfg(not(feature = "randomx"))]
        {
            let hash = header.hash();
            let hash_prefix =
                u64::from_le_bytes(hash.0[0..8].try_into().expect("hash is 32 bytes"));
            if hash_prefix <= header.difficulty_target {
                Ok(())
            } else {
                Err(BlockError::InvalidPoW)
            }
        }
    }

    fn difficulty_target(&self, height: u64) -> Result<u64, BlockError> {
        // Use testnet initial target to prevent instant-mining of early blocks.
        // The difficulty adjustment algorithm will converge to the correct
        // target regardless of actual hashrate.
        let initial_target = self
            .initial_target_override
            .unwrap_or(rill_core::constants::TESTNET_INITIAL_TARGET);

        if height <= 1 {
            return Ok(initial_target);
        }

        // Get parent's difficulty target
        let parent_height = height - 1;
        let parent_hash = self
            .chain_state
            .get_block_hash(parent_height)
            .map_err(|_| BlockError::InvalidPrevHash)?
            .ok_or(BlockError::InvalidPrevHash)?;
        let parent_header = self
            .chain_state
            .get_block_header(&parent_hash)
            .map_err(|_| BlockError::InvalidPrevHash)?
            .ok_or(BlockError::InvalidPrevHash)?;

        let target = difficulty::target_for_height_with_initial(
            height,
            parent_header.difficulty_target,
            |h| self.timestamp_at(h),
            initial_target,
        );

        Ok(target)
    }

    fn create_block_template(
        &self,
        coinbase_pubkey_hash: &Hash256,
        timestamp: u64,
    ) -> Result<Block, BlockError> {
        // Delegate to the extended method with no pending transactions.
        // The node layer calls `create_block_template_with_txs` directly
        // when mempool transactions are available.
        self.create_block_template_with_txs(coinbase_pubkey_hash, timestamp, &[])
    }

    fn validate_block(&self, block: &Block) -> Result<(), BlockError> {
        let (tip_height, tip_hash) = self
            .chain_state
            .chain_tip()
            .map_err(|_| BlockError::InvalidPrevHash)?;

        let height = tip_height + 1;

        // Get parent header for timestamp
        let parent_header = self
            .chain_state
            .get_block_header(&tip_hash)
            .map_err(|_| BlockError::InvalidPrevHash)?
            .ok_or(BlockError::InvalidPrevHash)?;

        let expected_difficulty = self.difficulty_target(height)?;
        let total_reward = self.total_reward(height)?;
        let current_time = (self.clock)();

        let context = BlockContext {
            height,
            prev_hash: tip_hash,
            prev_timestamp: parent_header.timestamp,
            expected_difficulty,
            current_time,
            block_reward: total_reward,
        };

        let cs = &self.chain_state;
        block_validation::validate_block(block, &context, |outpoint| {
            cs.get_utxo(outpoint).ok().flatten()
        })?;

        Ok(())
    }
}

/// Attempt to mine a block by incrementing the nonce until PoW is satisfied.
///
/// Modifies `block.header.nonce` in place. Returns `true` if a valid nonce
/// was found within `[0, max_nonce]`, `false` otherwise.
///
/// Phase 1 uses SHA-256 double-hash PoW from [`block_validation::check_pow`].
/// For RandomX mining, use [`mine_block_randomx`] instead.
pub fn mine_block(block: &mut Block, max_nonce: u64) -> bool {
    for nonce in 0..=max_nonce {
        block.header.nonce = nonce;
        if block_validation::check_pow(block) {
            return true;
        }
    }
    false
}

/// Attempt to mine a block using RandomX proof-of-work.
///
/// Modifies `block.header.nonce` in place. Returns `Ok(true)` if a valid nonce
/// was found within `[0, max_nonce]`, `Ok(false)` if no valid nonce was found,
/// or `Err` if the RandomX hash computation fails.
///
/// The `miner` should be a [`RandomXMiner`](crate::randomx::RandomXMiner) or
/// [`RandomXValidator`](crate::randomx::RandomXValidator) instance whose key
/// has already been updated for the current block height.
#[cfg(feature = "randomx")]
pub fn mine_block_randomx(
    block: &mut Block,
    max_nonce: u64,
    miner: &crate::randomx::RandomXMiner,
) -> Result<bool, String> {
    for nonce in 0..=max_nonce {
        block.header.nonce = nonce;
        let hash = miner.hash(&block.header.header_bytes())?;
        let hash_prefix =
            u64::from_le_bytes(hash.0[0..8].try_into().expect("hash is 32 bytes"));
        if hash_prefix <= block.header.difficulty_target {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rill_core::constants::{
        BLOCK_TIME_SECS, COIN, DECAY_POOL_RELEASE_BPS, BPS_PRECISION, INITIAL_REWARD,
    };
    use rill_core::error::{DecayError, RillError, TransactionError};
    use rill_core::genesis;
    use std::collections::HashMap;

    // ======================================================================
    // Mock ChainState
    // ======================================================================

    struct MockChainState {
        headers: Vec<BlockHeader>,
        blocks: Vec<Block>,
        hashes: Vec<Hash256>,
        utxos: HashMap<OutPoint, rill_core::types::UtxoEntry>,
        supply: u64,
        pool: u64,
    }

    impl MockChainState {
        /// Create a chain with just the genesis block.
        fn with_genesis() -> Self {
            let genesis = genesis::genesis_block().clone();
            let hash = genesis.header.hash();
            Self {
                headers: vec![genesis.header.clone()],
                blocks: vec![genesis],
                hashes: vec![hash],
                utxos: HashMap::new(),
                supply: 1_000_000 * COIN,
                pool: 10_000 * COIN,
            }
        }

        /// Add a block at the next height with the given timestamp and difficulty.
        fn add_block(&mut self, timestamp: u64, difficulty: u64) {
            let prev_hash = *self.hashes.last().unwrap();
            let height = self.headers.len() as u64;
            let sig = height.to_le_bytes().to_vec();
            let coinbase = Transaction {
                version: 1,
                inputs: vec![TxInput {
                    previous_output: OutPoint::null(),
                    signature: sig,
                    public_key: vec![],
                }],
                outputs: vec![TxOutput {
                    value: reward::block_reward(height),
                    pubkey_hash: Hash256([0xAA; 32]),
                }],
                lock_time: height,
            };
            let txid = coinbase.txid().unwrap();
            let mr = merkle::merkle_root(&[txid]);
            let header = BlockHeader {
                version: 1,
                prev_hash,
                merkle_root: mr,
                timestamp,
                difficulty_target: difficulty,
                nonce: 0,
            };
            let hash = header.hash();
            let block = Block {
                header: header.clone(),
                transactions: vec![coinbase],
            };
            self.headers.push(header);
            self.blocks.push(block);
            self.hashes.push(hash);
        }

        fn tip_height(&self) -> u64 {
            self.headers.len() as u64 - 1
        }

        fn tip_hash(&self) -> Hash256 {
            *self.hashes.last().unwrap()
        }
    }

    impl ChainState for MockChainState {
        fn get_utxo(
            &self,
            outpoint: &OutPoint,
        ) -> Result<Option<rill_core::types::UtxoEntry>, RillError> {
            Ok(self.utxos.get(outpoint).cloned())
        }

        fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
            Ok((self.tip_height(), self.tip_hash()))
        }

        fn get_block_header(
            &self,
            hash: &Hash256,
        ) -> Result<Option<BlockHeader>, RillError> {
            Ok(self
                .hashes
                .iter()
                .position(|h| h == hash)
                .map(|i| self.headers[i].clone()))
        }

        fn get_block(&self, hash: &Hash256) -> Result<Option<Block>, RillError> {
            Ok(self
                .hashes
                .iter()
                .position(|h| h == hash)
                .map(|i| self.blocks[i].clone()))
        }

        fn get_block_hash(&self, height: u64) -> Result<Option<Hash256>, RillError> {
            Ok(self.hashes.get(height as usize).copied())
        }

        fn circulating_supply(&self) -> Result<u64, RillError> {
            Ok(self.supply)
        }

        fn cluster_balance(&self, _cluster_id: &Hash256) -> Result<u64, RillError> {
            Ok(0)
        }

        fn decay_pool_balance(&self) -> Result<u64, RillError> {
            Ok(self.pool)
        }

        fn validate_transaction(
            &self,
            tx: &Transaction,
        ) -> Result<(), TransactionError> {
            if tx.inputs.is_empty() || tx.outputs.is_empty() {
                return Err(TransactionError::EmptyInputsOrOutputs);
            }
            Ok(())
        }
    }

    // ======================================================================
    // Mock DecayCalculator
    // ======================================================================

    struct MockDecay;

    impl DecayCalculator for MockDecay {
        fn decay_rate_ppb(&self, _concentration_ppb: u64) -> Result<u64, DecayError> {
            Ok(0)
        }

        fn compute_decay(
            &self,
            _nominal_value: u64,
            _concentration_ppb: u64,
            _blocks_held: u64,
        ) -> Result<u64, DecayError> {
            Ok(0)
        }

        fn decay_pool_release(&self, pool_balance: u64) -> Result<u64, DecayError> {
            Ok(pool_balance * DECAY_POOL_RELEASE_BPS / BPS_PRECISION)
        }
    }

    // ======================================================================
    // Helpers
    // ======================================================================

    fn make_engine(cs: MockChainState) -> ConsensusEngine {
        let time = cs.headers.last().unwrap().timestamp + BLOCK_TIME_SECS;
        ConsensusEngine::with_clock(
            Arc::new(cs),
            Arc::new(MockDecay),
            move || time,
        )
        .with_initial_target(u64::MAX)
    }

    fn make_engine_at_time(cs: MockChainState, current_time: u64) -> ConsensusEngine {
        ConsensusEngine::with_clock(
            Arc::new(cs),
            Arc::new(MockDecay),
            move || current_time,
        )
        .with_initial_target(u64::MAX)
    }

    // ======================================================================
    // Construction
    // ======================================================================

    #[test]
    fn engine_new_succeeds() {
        let cs = MockChainState::with_genesis();
        let _engine = make_engine(cs);
    }

    #[test]
    fn engine_debug() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        let debug = format!("{engine:?}");
        assert!(debug.contains("ConsensusEngine"));
    }

    // ======================================================================
    // block_reward
    // ======================================================================

    #[test]
    fn block_reward_delegates_to_core() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        assert_eq!(engine.block_reward(0), INITIAL_REWARD);
        assert_eq!(engine.block_reward(210_000), INITIAL_REWARD / 2);
        assert_eq!(engine.block_reward(u64::MAX), 0);
    }

    // ======================================================================
    // validate_pow
    // ======================================================================

    #[test]
    fn validate_pow_accepts_easy() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        let header = BlockHeader {
            version: 1,
            prev_hash: Hash256::ZERO,
            merkle_root: Hash256::ZERO,
            timestamp: 1_000_000,
            difficulty_target: u64::MAX,
            nonce: 0,
        };
        assert!(engine.validate_pow(&header).is_ok());
    }

    #[test]
    fn validate_pow_rejects_hard() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        let header = BlockHeader {
            version: 1,
            prev_hash: Hash256::ZERO,
            merkle_root: Hash256::ZERO,
            timestamp: 1_000_000,
            difficulty_target: 0,
            nonce: 0,
        };
        assert_eq!(
            engine.validate_pow(&header).unwrap_err(),
            BlockError::InvalidPoW
        );
    }

    // ======================================================================
    // difficulty_target
    // ======================================================================

    #[test]
    fn difficulty_height_0_is_testnet_initial() {
        let cs = MockChainState::with_genesis();
        // Use a production-like engine (no initial target override)
        let time = cs.headers.last().unwrap().timestamp + BLOCK_TIME_SECS;
        let engine = ConsensusEngine::with_clock(
            Arc::new(cs),
            Arc::new(MockDecay),
            move || time,
        );
        assert_eq!(
            engine.difficulty_target(0).unwrap(),
            rill_core::constants::TESTNET_INITIAL_TARGET,
        );
    }

    #[test]
    fn difficulty_height_1_is_testnet_initial() {
        let cs = MockChainState::with_genesis();
        // Use a production-like engine (no initial target override)
        let time = cs.headers.last().unwrap().timestamp + BLOCK_TIME_SECS;
        let engine = ConsensusEngine::with_clock(
            Arc::new(cs),
            Arc::new(MockDecay),
            move || time,
        );
        assert_eq!(
            engine.difficulty_target(1).unwrap(),
            rill_core::constants::TESTNET_INITIAL_TARGET,
        );
    }

    #[test]
    fn difficulty_height_0_respects_override() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        // make_engine sets initial target to u64::MAX
        assert_eq!(engine.difficulty_target(0).unwrap(), u64::MAX);
    }

    #[test]
    fn difficulty_height_1_respects_override() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        // make_engine sets initial target to u64::MAX
        assert_eq!(engine.difficulty_target(1).unwrap(), u64::MAX);
    }

    #[test]
    fn difficulty_adjusts_for_slow_blocks() {
        let mut cs = MockChainState::with_genesis();
        let base_ts = genesis::GENESIS_TIMESTAMP;
        // Add blocks 2x slower than target (120s intervals)
        for i in 1..=3 {
            cs.add_block(base_ts + i * 120, u64::MAX);
        }
        let engine = make_engine(cs);
        // Height 4 should have higher (easier) target than MAX since blocks are slow
        // But since we start at MAX, it stays at MAX (can't go higher)
        let target = engine.difficulty_target(4).unwrap();
        assert_eq!(target, u64::MAX);
    }

    #[test]
    fn difficulty_adjusts_for_fast_blocks() {
        let mut cs = MockChainState::with_genesis();
        let base_ts = genesis::GENESIS_TIMESTAMP;
        let initial_target = u64::MAX / 2;
        // Add blocks 2x faster than target (30s intervals)
        for i in 1..=3 {
            cs.add_block(base_ts + i * 30, initial_target);
        }
        let engine = make_engine(cs);
        // Height 4: blocks are faster, target should decrease
        let target = engine.difficulty_target(4).unwrap();
        assert!(target < initial_target, "target should decrease for fast blocks");
    }

    // ======================================================================
    // create_block_template
    // ======================================================================

    #[test]
    fn template_creates_valid_block() {
        let cs = MockChainState::with_genesis();
        let tip_hash = cs.tip_hash();
        let engine = make_engine(cs);
        let pkh = Hash256([0xBB; 32]);
        let ts = genesis::GENESIS_TIMESTAMP + BLOCK_TIME_SECS;
        let block = engine.create_block_template(&pkh, ts).unwrap();

        assert_eq!(block.header.prev_hash, tip_hash);
        assert_eq!(block.header.timestamp, ts);
        assert_eq!(block.header.difficulty_target, u64::MAX);
        assert_eq!(block.transactions.len(), 1);
        assert!(block.transactions[0].is_coinbase());
        assert_eq!(block.transactions[0].outputs[0].pubkey_hash, pkh);
    }

    #[test]
    fn template_includes_decay_pool_release() {
        let cs = MockChainState::with_genesis();
        let pool = cs.pool;
        let engine = make_engine(cs);
        let pkh = Hash256([0xBB; 32]);
        let ts = genesis::GENESIS_TIMESTAMP + BLOCK_TIME_SECS;
        let block = engine.create_block_template(&pkh, ts).unwrap();

        let expected_pool_release = pool * DECAY_POOL_RELEASE_BPS / BPS_PRECISION;
        let expected_total = INITIAL_REWARD + expected_pool_release;
        assert_eq!(block.transactions[0].outputs[0].value, expected_total);
    }

    #[test]
    fn template_has_correct_merkle_root() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        let pkh = Hash256([0xBB; 32]);
        let ts = genesis::GENESIS_TIMESTAMP + BLOCK_TIME_SECS;
        let block = engine.create_block_template(&pkh, ts).unwrap();

        let txid = block.transactions[0].txid().unwrap();
        let expected_mr = merkle::merkle_root(&[txid]);
        assert_eq!(block.header.merkle_root, expected_mr);
    }

    #[test]
    fn template_at_different_heights() {
        let mut cs = MockChainState::with_genesis();
        let base_ts = genesis::GENESIS_TIMESTAMP;
        cs.add_block(base_ts + 60, u64::MAX);
        cs.add_block(base_ts + 120, u64::MAX);
        let engine = make_engine(cs);
        let pkh = Hash256([0xBB; 32]);
        let block = engine
            .create_block_template(&pkh, base_ts + 180)
            .unwrap();

        // Height 3 block
        assert!(block.transactions[0].is_coinbase());
        assert_eq!(block.transactions[0].outputs[0].pubkey_hash, pkh);
    }

    // ======================================================================
    // validate_block
    // ======================================================================

    #[test]
    fn validate_accepts_valid_template() {
        let cs = MockChainState::with_genesis();
        let tip_ts = cs.headers.last().unwrap().timestamp;
        let current_time = tip_ts + BLOCK_TIME_SECS;
        let engine = make_engine_at_time(cs, current_time);

        let pkh = Hash256([0xBB; 32]);
        let block = engine
            .create_block_template(&pkh, tip_ts + BLOCK_TIME_SECS)
            .unwrap();
        assert!(engine.validate_block(&block).is_ok());
    }

    #[test]
    fn validate_rejects_wrong_prev_hash() {
        let cs = MockChainState::with_genesis();
        let tip_ts = cs.headers.last().unwrap().timestamp;
        let current_time = tip_ts + BLOCK_TIME_SECS;
        let engine = make_engine_at_time(cs, current_time);

        let pkh = Hash256([0xBB; 32]);
        let mut block = engine
            .create_block_template(&pkh, tip_ts + BLOCK_TIME_SECS)
            .unwrap();
        block.header.prev_hash = Hash256([0xFF; 32]);
        // Recompute merkle root to keep PoW valid
        let txids: Vec<Hash256> = block
            .transactions
            .iter()
            .map(|tx| tx.txid().unwrap())
            .collect();
        block.header.merkle_root = merkle::merkle_root(&txids);

        assert_eq!(
            engine.validate_block(&block).unwrap_err(),
            BlockError::InvalidPrevHash
        );
    }

    #[test]
    fn validate_rejects_timestamp_before_parent() {
        let cs = MockChainState::with_genesis();
        let tip_ts = cs.headers.last().unwrap().timestamp;
        let current_time = tip_ts + BLOCK_TIME_SECS;
        let engine = make_engine_at_time(cs, current_time);

        let pkh = Hash256([0xBB; 32]);
        // Build a valid template, then force its timestamp to equal the parent's.
        let mut block = engine
            .create_block_template(&pkh, tip_ts + BLOCK_TIME_SECS)
            .unwrap();
        block.header.timestamp = tip_ts; // same as parent (not after)
        // Fix merkle root
        let txids: Vec<Hash256> = block
            .transactions
            .iter()
            .map(|tx| tx.txid().unwrap())
            .collect();
        block.header.merkle_root = merkle::merkle_root(&txids);

        assert_eq!(
            engine.validate_block(&block).unwrap_err(),
            BlockError::TimestampNotAfterParent
        );
    }

    #[test]
    fn validate_rejects_excess_reward() {
        let cs = MockChainState::with_genesis();
        let tip_ts = cs.headers.last().unwrap().timestamp;
        let current_time = tip_ts + BLOCK_TIME_SECS;
        let pool = cs.pool;
        let engine = make_engine_at_time(cs, current_time);

        let pool_release = pool * DECAY_POOL_RELEASE_BPS / BPS_PRECISION;
        let max_reward = INITIAL_REWARD + pool_release;

        // Create a block claiming more than allowed
        let prev_hash = genesis::genesis_hash();
        let coinbase = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: 1u64.to_le_bytes().to_vec(),
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: max_reward + 1,
                pubkey_hash: Hash256([0xBB; 32]),
            }],
            lock_time: 0,
        };
        let txid = coinbase.txid().unwrap();
        let mr = merkle::merkle_root(&[txid]);
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash,
                merkle_root: mr,
                timestamp: tip_ts + BLOCK_TIME_SECS,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: vec![coinbase],
        };

        assert!(matches!(
            engine.validate_block(&block).unwrap_err(),
            BlockError::InvalidReward { .. }
        ));
    }

    #[test]
    fn validate_accepts_partial_reward() {
        let cs = MockChainState::with_genesis();
        let tip_ts = cs.headers.last().unwrap().timestamp;
        let current_time = tip_ts + BLOCK_TIME_SECS;
        let engine = make_engine_at_time(cs, current_time);

        // Claim only 1 rill (well below max)
        let prev_hash = genesis::genesis_hash();
        let coinbase = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: 1u64.to_le_bytes().to_vec(),
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 1,
                pubkey_hash: Hash256([0xBB; 32]),
            }],
            lock_time: 0,
        };
        let txid = coinbase.txid().unwrap();
        let mr = merkle::merkle_root(&[txid]);
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash,
                merkle_root: mr,
                timestamp: tip_ts + BLOCK_TIME_SECS,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: vec![coinbase],
        };

        assert!(engine.validate_block(&block).is_ok());
    }

    // ======================================================================
    // mine_block
    // ======================================================================

    #[test]
    fn mine_with_easy_difficulty() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        let pkh = Hash256([0xBB; 32]);
        let ts = genesis::GENESIS_TIMESTAMP + BLOCK_TIME_SECS;
        let mut block = engine.create_block_template(&pkh, ts).unwrap();

        // u64::MAX difficulty: any nonce works
        assert!(mine_block(&mut block, 0));
        assert_eq!(block.header.nonce, 0);
    }

    #[test]
    fn mine_fails_with_impossible_difficulty() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        let pkh = Hash256([0xBB; 32]);
        let ts = genesis::GENESIS_TIMESTAMP + BLOCK_TIME_SECS;
        let mut block = engine.create_block_template(&pkh, ts).unwrap();

        // Set difficulty to 0: practically impossible
        block.header.difficulty_target = 0;
        assert!(!mine_block(&mut block, 1000));
    }

    #[test]
    fn mine_sets_correct_nonce() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        let pkh = Hash256([0xBB; 32]);
        let ts = genesis::GENESIS_TIMESTAMP + BLOCK_TIME_SECS;
        let mut block = engine.create_block_template(&pkh, ts).unwrap();

        assert!(mine_block(&mut block, u64::MAX));
        // After mining, block should pass PoW check
        assert!(block_validation::check_pow(&block));
    }

    // ======================================================================
    // Integration: template → mine → validate
    // ======================================================================

    #[test]
    fn full_cycle_template_mine_validate() {
        let cs = MockChainState::with_genesis();
        let tip_ts = cs.headers.last().unwrap().timestamp;
        let current_time = tip_ts + BLOCK_TIME_SECS;
        let engine = make_engine_at_time(cs, current_time);

        let pkh = Hash256([0xBB; 32]);
        let mut block = engine
            .create_block_template(&pkh, tip_ts + BLOCK_TIME_SECS)
            .unwrap();

        assert!(mine_block(&mut block, u64::MAX));
        assert!(engine.validate_block(&block).is_ok());
    }

    #[test]
    fn full_cycle_multi_block() {
        let mut cs = MockChainState::with_genesis();
        let base_ts = genesis::GENESIS_TIMESTAMP;

        // Add a few blocks
        for i in 1..=5 {
            cs.add_block(base_ts + i * BLOCK_TIME_SECS, u64::MAX);
        }

        let tip_ts = cs.headers.last().unwrap().timestamp;
        let current_time = tip_ts + BLOCK_TIME_SECS;
        let engine = make_engine_at_time(cs, current_time);

        let pkh = Hash256([0xCC; 32]);
        let mut block = engine
            .create_block_template(&pkh, tip_ts + BLOCK_TIME_SECS)
            .unwrap();

        assert!(mine_block(&mut block, u64::MAX));
        assert!(engine.validate_block(&block).is_ok());
    }

    // ======================================================================
    // total_reward
    // ======================================================================

    #[test]
    fn total_reward_includes_pool_release() {
        let cs = MockChainState::with_genesis();
        let pool = cs.pool;
        let engine = make_engine(cs);

        let total = engine.total_reward(1).unwrap();
        let expected_pool_release = pool * DECAY_POOL_RELEASE_BPS / BPS_PRECISION;
        assert_eq!(total, INITIAL_REWARD + expected_pool_release);
    }

    #[test]
    fn total_reward_zero_pool() {
        let mut cs = MockChainState::with_genesis();
        cs.pool = 0;
        let engine = make_engine(cs);

        let total = engine.total_reward(1).unwrap();
        assert_eq!(total, INITIAL_REWARD);
    }

    // ======================================================================
    // Object safety
    // ======================================================================

    #[test]
    fn engine_is_object_safe() {
        let cs = MockChainState::with_genesis();
        let engine = make_engine(cs);
        let dyn_bp: &dyn BlockProducer = &engine;
        assert_eq!(dyn_bp.block_reward(0), INITIAL_REWARD);
    }
}
