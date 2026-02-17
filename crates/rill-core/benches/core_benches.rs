//! Criterion benchmarks for rill-core critical operations.
//!
//! Covers: Merkle tree construction, SHA-256 block hashing,
//! Ed25519 sign/verify, and transaction serialization.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use rill_core::crypto::KeyPair;
use rill_core::merkle::merkle_root;
use rill_core::types::{
    BlockHeader, Hash256, OutPoint, Transaction, TxInput, TxOutput,
};

/// Generate `n` deterministic 32-byte hashes for Merkle benchmarks.
fn make_txids(n: usize) -> Vec<Hash256> {
    (0..n)
        .map(|i| {
            let bytes = blake3::hash(&(i as u64).to_le_bytes());
            Hash256(*bytes.as_bytes())
        })
        .collect()
}

fn sample_block_header() -> BlockHeader {
    BlockHeader {
        version: 1,
        prev_hash: Hash256([0xAA; 32]),
        merkle_root: Hash256([0xBB; 32]),
        timestamp: 1_700_000_000,
        difficulty_target: u64::MAX,
        nonce: 42,
    }
}

fn sample_transaction() -> Transaction {
    Transaction {
        version: 1,
        inputs: vec![TxInput {
            previous_output: OutPoint {
                txid: Hash256([0x11; 32]),
                index: 0,
            },
            signature: vec![0u8; 64],
            public_key: vec![0u8; 32],
        }],
        outputs: vec![
            TxOutput {
                value: 50 * 100_000_000,
                pubkey_hash: Hash256([0xCC; 32]),
            },
            TxOutput {
                value: 25 * 100_000_000,
                pubkey_hash: Hash256([0xDD; 32]),
            },
        ],
        lock_time: 0,
    }
}

fn bench_merkle_root(c: &mut Criterion) {
    let txids_10 = make_txids(10);
    let txids_1000 = make_txids(1000);

    c.bench_function("merkle_root_10_txids", |b| {
        b.iter(|| merkle_root(black_box(&txids_10)))
    });

    c.bench_function("merkle_root_1000_txids", |b| {
        b.iter(|| merkle_root(black_box(&txids_1000)))
    });
}

fn bench_sha256_block_hash(c: &mut Criterion) {
    let header = sample_block_header();

    c.bench_function("sha256_block_hash", |b| {
        b.iter(|| black_box(&header).hash())
    });
}

fn bench_ed25519(c: &mut Criterion) {
    let keypair = KeyPair::from_secret_bytes([42u8; 32]);
    let message = blake3::hash(b"bench message");
    let msg_bytes = message.as_bytes();
    let signature = keypair.sign(msg_bytes);
    let pubkey = keypair.public_key();

    c.bench_function("ed25519_sign", |b| {
        b.iter(|| keypair.sign(black_box(msg_bytes)))
    });

    c.bench_function("ed25519_verify", |b| {
        b.iter(|| pubkey.verify(black_box(msg_bytes), black_box(&signature)))
    });
}

fn bench_transaction_serde(c: &mut Criterion) {
    let tx = sample_transaction();
    let encoded =
        bincode::encode_to_vec(&tx, bincode::config::standard()).expect("encode failed");

    c.bench_function("transaction_serialization", |b| {
        b.iter(|| bincode::encode_to_vec(black_box(&tx), bincode::config::standard()))
    });

    c.bench_function("transaction_deserialization", |b| {
        b.iter(|| {
            let (decoded, _): (Transaction, usize) =
                bincode::decode_from_slice(black_box(&encoded), bincode::config::standard())
                    .expect("decode failed");
            decoded
        })
    });
}

criterion_group!(
    benches,
    bench_merkle_root,
    bench_sha256_block_hash,
    bench_ed25519,
    bench_transaction_serde,
);
criterion_main!(benches);
