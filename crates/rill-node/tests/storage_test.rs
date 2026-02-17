//! Standalone test for storage module to avoid network dependency issues.

use rill_core::chain_state::ChainStore;
use rill_core::constants::COIN;
use rill_core::genesis;
use rill_core::merkle;
use rill_core::types::{Block, BlockHeader, Hash256, OutPoint, Transaction, TxInput, TxOutput};
use rill_node_lib::storage::RocksStore;

fn pkh(seed: u8) -> Hash256 {
    Hash256([seed; 32])
}

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

#[allow(dead_code)]
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

#[test]
fn cluster_balance_tracked_on_connect() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = RocksStore::open(dir.path().join("chaindata")).unwrap();
    let genesis_hash = genesis::genesis_hash();

    let cb1 = make_coinbase_unique(50 * COIN, pkh(0xBB), 1);
    let cb1_txid = cb1.txid().unwrap();
    let block1 = make_block(genesis_hash, 1_000_060, vec![cb1]);
    store.connect_block(&block1, 1).unwrap();

    let cluster_id = cb1_txid;
    let balance = store.cluster_balance(&cluster_id).unwrap();
    assert_eq!(balance, 50 * COIN);
}

#[test]
fn genesis_cluster_balance() {
    let dir = tempfile::tempdir().unwrap();
    let store = RocksStore::open(dir.path().join("chaindata")).unwrap();
    let genesis_coinbase_txid = genesis::genesis_coinbase_txid();

    let cluster_id = genesis_coinbase_txid;
    let balance = store.cluster_balance(&cluster_id).unwrap();
    assert_eq!(balance, genesis::DEV_FUND_PREMINE);
}
