//! Shared test helpers for E2E and integration tests.

use rill_core::merkle;
use rill_core::types::*;

/// Simple pubkey hash from a seed byte.
pub fn pkh(seed: u8) -> Hash256 {
    Hash256([seed; 32])
}

/// Create a coinbase transaction with a unique height marker.
///
/// Sets `lock_time: height` so that each coinbase produces a distinct txid
/// per block height, matching the production consensus engine behaviour.
pub fn make_coinbase(value: u64, pubkey_hash: Hash256, height: u64) -> Transaction {
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

/// Create a simple spending transaction (unsigned).
pub fn make_tx(
    inputs: Vec<OutPoint>,
    outputs: Vec<(u64, Hash256)>,
) -> Transaction {
    Transaction {
        version: 1,
        inputs: inputs
            .into_iter()
            .map(|op| TxInput {
                previous_output: op,
                signature: vec![0; 64],
                public_key: vec![0; 32],
            })
            .collect(),
        outputs: outputs
            .into_iter()
            .map(|(value, pubkey_hash)| TxOutput { value, pubkey_hash })
            .collect(),
        lock_time: 0,
    }
}

/// Create a block with correct merkle root.
pub fn make_block(prev_hash: Hash256, timestamp: u64, txs: Vec<Transaction>) -> Block {
    let txids: Vec<Hash256> = txs.iter().map(|tx| tx.txid().unwrap()).collect();
    let mr = merkle::merkle_root(&txids);
    Block {
        header: BlockHeader {
            version: 1,
            prev_hash,
            merkle_root: mr,
            timestamp,
            difficulty_target: u64::MAX,
            nonce: 0,
        },
        transactions: txs,
    }
}
