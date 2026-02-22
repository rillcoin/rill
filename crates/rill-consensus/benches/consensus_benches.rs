//! Criterion benchmarks for rill-consensus critical operations.
//!
//! Covers: block validation and difficulty adjustment.
//! Uses mock ChainState and DecayCalculator identical to the engine tests.

use std::collections::HashMap;
use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use rill_core::constants::{
    BLOCK_TIME_SECS, BPS_PRECISION, COIN, DECAY_POOL_RELEASE_BPS,
};
use rill_core::error::{DecayError, RillError, TransactionError};
use rill_core::traits::{BlockProducer, ChainState, DecayCalculator};
use rill_core::types::{
    Block, BlockHeader, Hash256, OutPoint, Transaction, TxInput, TxOutput, TxType, UtxoEntry,
};
use rill_core::{genesis, merkle, reward};

use rill_consensus::engine::{mine_block, ConsensusEngine};

// --- Mock ChainState ---

struct MockChainState {
    headers: Vec<BlockHeader>,
    blocks: Vec<Block>,
    hashes: Vec<Hash256>,
    utxos: HashMap<OutPoint, UtxoEntry>,
    supply: u64,
    pool: u64,
}

impl MockChainState {
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

    fn add_block(&mut self, timestamp: u64, difficulty: u64) {
        let prev_hash = *self.hashes.last().unwrap();
        let height = self.headers.len() as u64;
        let sig = height.to_le_bytes().to_vec();
        let coinbase = Transaction {
            version: 1,
            tx_type: TxType::default(),
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
    fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UtxoEntry>, RillError> {
        Ok(self.utxos.get(outpoint).cloned())
    }

    fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
        Ok((self.tip_height(), self.tip_hash()))
    }

    fn get_block_header(&self, hash: &Hash256) -> Result<Option<BlockHeader>, RillError> {
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

    fn validate_transaction(&self, tx: &Transaction) -> Result<(), TransactionError> {
        if tx.inputs.is_empty() || tx.outputs.is_empty() {
            return Err(TransactionError::EmptyInputsOrOutputs);
        }
        Ok(())
    }
}

// --- Mock DecayCalculator ---

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

fn make_engine_and_block() -> (ConsensusEngine, Block) {
    let cs = MockChainState::with_genesis();
    let tip_ts = cs.headers.last().unwrap().timestamp;
    let current_time = tip_ts + BLOCK_TIME_SECS;

    let engine = ConsensusEngine::with_clock(
        Arc::new(cs),
        Arc::new(MockDecay),
        move || current_time,
    );

    let pkh = Hash256([0xBB; 32]);
    let mut block = engine
        .create_block_template(&pkh, tip_ts + BLOCK_TIME_SECS)
        .unwrap();
    mine_block(&mut block, u64::MAX);

    (engine, block)
}

fn bench_block_validation(c: &mut Criterion) {
    let (engine, block) = make_engine_and_block();

    c.bench_function("block_validation", |b| {
        b.iter(|| engine.validate_block(black_box(&block)))
    });
}

fn bench_difficulty_adjustment(c: &mut Criterion) {
    // Build a chain with 65 blocks so the LWMA window is fully populated.
    let mut cs = MockChainState::with_genesis();
    let base_ts = genesis::GENESIS_TIMESTAMP;
    for i in 1..=65 {
        cs.add_block(base_ts + i * BLOCK_TIME_SECS, u64::MAX / 2);
    }

    let tip_ts = cs.headers.last().unwrap().timestamp;
    let current_time = tip_ts + BLOCK_TIME_SECS;
    let engine = ConsensusEngine::with_clock(
        Arc::new(cs),
        Arc::new(MockDecay),
        move || current_time,
    );

    let next_height = 66u64;

    c.bench_function("difficulty_adjustment", |b| {
        b.iter(|| engine.difficulty_target(black_box(next_height)))
    });
}

criterion_group!(benches, bench_block_validation, bench_difficulty_adjustment);
criterion_main!(benches);
