//! Criterion benchmarks for rill-node storage operations.
//!
//! Covers: connect_block and UTXO lookup via RocksDB-backed storage.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

use rill_core::chain_state::ChainStore;
use rill_core::types::{
    Block, BlockHeader, Hash256, OutPoint, Transaction, TxInput, TxOutput,
};
use rill_core::{genesis, merkle, reward};

use rill_node_lib::storage::RocksStore;

/// Build a valid block at the given height on top of the store's current tip.
fn build_block(store: &RocksStore, height: u64) -> Block {
    let (_, prev_hash) = store.chain_tip().unwrap();
    let parent_header = store.get_block_header(&prev_hash).unwrap().unwrap();

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

    Block {
        header: BlockHeader {
            version: 1,
            prev_hash,
            merkle_root: mr,
            timestamp: parent_header.timestamp + 60,
            difficulty_target: u64::MAX,
            nonce: 0,
        },
        transactions: vec![coinbase],
    }
}

fn bench_connect_block(c: &mut Criterion) {
    // Each iteration opens a fresh store and connects one block above genesis.
    // We pre-build the block outside the timed section to measure only connect_block.
    c.bench_function("connect_block", |b| {
        b.iter_with_setup(
            || {
                let dir = TempDir::new().unwrap();
                let store = RocksStore::open(dir.path()).unwrap();
                let block = build_block(&store, 1);
                (dir, store, block)
            },
            |(_dir, mut store, block)| {
                store.connect_block(black_box(&block), 1).unwrap();
            },
        )
    });
}

fn bench_utxo_lookup(c: &mut Criterion) {
    // Set up a store with genesis + 10 blocks so there are UTXOs to look up.
    let dir = TempDir::new().unwrap();
    let mut store = RocksStore::open(dir.path()).unwrap();

    // Connect 10 blocks to populate UTXOs.
    for h in 1..=10 {
        let block = build_block(&store, h);
        store.connect_block(&block, h).unwrap();
    }

    // Get the genesis coinbase outpoint -- it exists as a UTXO.
    let genesis = genesis::genesis_block();
    let genesis_txid = genesis.transactions[0].txid().unwrap();
    let existing_outpoint = OutPoint {
        txid: genesis_txid,
        index: 0,
    };

    // A nonexistent outpoint for miss benchmarks.
    let missing_outpoint = OutPoint {
        txid: Hash256([0xFF; 32]),
        index: 999,
    };

    c.bench_function("utxo_lookup", |b| {
        b.iter(|| store.get_utxo(black_box(&existing_outpoint)))
    });

    c.bench_function("utxo_lookup_miss", |b| {
        b.iter(|| store.get_utxo(black_box(&missing_outpoint)))
    });
}

criterion_group!(benches, bench_connect_block, bench_utxo_lookup);
criterion_main!(benches);
