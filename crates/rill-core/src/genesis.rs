//! Genesis block definition for the Rill network.
//!
//! The genesis block is the first block in the chain (height 0). It contains
//! a single coinbase transaction with the dev fund premine (5% of total supply).
//! The regular mining reward schedule starts at height 1.
//!
//! All values are hardcoded and deterministic — every node computes the
//! identical genesis block.

use std::sync::LazyLock;

use crate::constants::{BPS_PRECISION, DEV_FUND_BPS, MAX_SUPPLY};
use crate::merkle;
use crate::types::{Block, BlockHeader, Hash256, OutPoint, Transaction, TxInput, TxOutput};

/// Genesis block timestamp: January 1, 2026 00:00:00 UTC.
pub const GENESIS_TIMESTAMP: u64 = 1_767_225_600;

/// Message embedded in the genesis coinbase (like Bitcoin's "The Times" headline).
pub const GENESIS_MESSAGE: &[u8] = b"Wealth should flow like water. Rill genesis 2026.";

/// Dev fund premine: 5% of MAX_SUPPLY (1,050,000 RILL = 105,000,000,000,000 rills).
pub const DEV_FUND_PREMINE: u64 = MAX_SUPPLY / BPS_PRECISION * DEV_FUND_BPS;

/// Cached genesis data, computed once on first access.
struct GenesisData {
    block: Block,
    hash: Hash256,
    coinbase_txid: Hash256,
}

static GENESIS: LazyLock<GenesisData> = LazyLock::new(build_genesis);

/// Build the genesis block and cache derived values.
fn build_genesis() -> GenesisData {
    let coinbase = build_genesis_coinbase();
    // Hardcoded coinbase — serialization cannot fail.
    let coinbase_txid = coinbase
        .txid()
        .expect("genesis coinbase is hardcoded valid data");
    let mr = merkle::merkle_root(&[coinbase_txid]);

    let block = Block {
        header: BlockHeader {
            version: 1,
            prev_hash: Hash256::ZERO,
            merkle_root: mr,
            timestamp: GENESIS_TIMESTAMP,
            difficulty_target: u64::MAX,
            nonce: 0,
        },
        transactions: vec![coinbase],
    };
    let hash = block.header.hash();

    GenesisData {
        block,
        hash,
        coinbase_txid,
    }
}

/// Build the genesis coinbase transaction.
///
/// Contains the genesis message in the coinbase input and a single output
/// paying the dev fund premine.
fn build_genesis_coinbase() -> Transaction {
    Transaction {
        version: 1,
        inputs: vec![TxInput {
            previous_output: OutPoint::null(),
            signature: GENESIS_MESSAGE.to_vec(),
            public_key: vec![],
        }],
        outputs: vec![TxOutput {
            value: DEV_FUND_PREMINE,
            pubkey_hash: dev_fund_pubkey_hash(),
        }],
        lock_time: 0,
    }
}

/// The dev fund pubkey hash.
///
/// Derived deterministically as `BLAKE3(b"rill genesis dev fund")` for
/// transparency. In production, this would be replaced with a real
/// multisig key hash.
pub fn dev_fund_pubkey_hash() -> Hash256 {
    Hash256(blake3::hash(b"rill genesis dev fund").into())
}

/// The genesis block (height 0).
pub fn genesis_block() -> &'static Block {
    &GENESIS.block
}

/// The genesis block header hash.
pub fn genesis_hash() -> Hash256 {
    GENESIS.hash
}

/// The transaction ID of the genesis coinbase.
pub fn genesis_coinbase_txid() -> Hash256 {
    GENESIS.coinbase_txid
}

/// Check whether a block is the genesis block by comparing header hashes.
pub fn is_genesis(block: &Block) -> bool {
    block.header.hash() == GENESIS.hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{COIN, MAX_SUPPLY};

    // --- Constants ---

    #[test]
    fn dev_fund_premine_is_five_percent() {
        assert_eq!(DEV_FUND_PREMINE, 1_050_000 * COIN);
        // 5% of 21M
        assert_eq!(DEV_FUND_PREMINE * BPS_PRECISION / DEV_FUND_BPS, MAX_SUPPLY);
    }

    #[test]
    fn genesis_timestamp_is_jan_1_2026() {
        // 56 years * 365 days + 14 leap days = 20454 days * 86400 sec/day
        assert_eq!(GENESIS_TIMESTAMP, 20454 * 86400);
    }

    #[test]
    fn genesis_message_not_empty() {
        assert!(!GENESIS_MESSAGE.is_empty());
        assert!(GENESIS_MESSAGE.starts_with(b"Wealth"));
    }

    // --- Block structure ---

    #[test]
    fn genesis_block_deterministic() {
        let a = genesis_block();
        let b = genesis_block();
        assert_eq!(a, b);
    }

    #[test]
    fn genesis_block_has_one_transaction() {
        assert_eq!(genesis_block().transactions.len(), 1);
    }

    #[test]
    fn genesis_coinbase_is_coinbase() {
        let block = genesis_block();
        let coinbase = block.coinbase().unwrap();
        assert!(coinbase.is_coinbase());
    }

    #[test]
    fn genesis_coinbase_has_message() {
        let block = genesis_block();
        let coinbase = &block.transactions[0];
        assert_eq!(coinbase.inputs[0].signature, GENESIS_MESSAGE);
    }

    #[test]
    fn genesis_coinbase_pays_dev_fund() {
        let block = genesis_block();
        let coinbase = &block.transactions[0];
        assert_eq!(coinbase.outputs.len(), 1);
        assert_eq!(coinbase.outputs[0].value, DEV_FUND_PREMINE);
        assert_eq!(coinbase.outputs[0].pubkey_hash, dev_fund_pubkey_hash());
    }

    #[test]
    fn genesis_coinbase_total_value() {
        let block = genesis_block();
        let total = block.transactions[0].total_output_value().unwrap();
        assert_eq!(total, DEV_FUND_PREMINE);
    }

    // --- Header ---

    #[test]
    fn genesis_header_prev_hash_zero() {
        assert!(genesis_block().header.prev_hash.is_zero());
    }

    #[test]
    fn genesis_header_version_one() {
        assert_eq!(genesis_block().header.version, 1);
    }

    #[test]
    fn genesis_header_timestamp() {
        assert_eq!(genesis_block().header.timestamp, GENESIS_TIMESTAMP);
    }

    #[test]
    fn genesis_header_max_difficulty() {
        assert_eq!(genesis_block().header.difficulty_target, u64::MAX);
    }

    // --- Merkle root ---

    #[test]
    fn genesis_merkle_root_correct() {
        let block = genesis_block();
        let txid = block.transactions[0].txid().unwrap();
        let expected = merkle::merkle_root(&[txid]);
        assert_eq!(block.header.merkle_root, expected);
    }

    #[test]
    fn genesis_merkle_root_nonzero() {
        assert!(!genesis_block().header.merkle_root.is_zero());
    }

    // --- Hash ---

    #[test]
    fn genesis_hash_deterministic() {
        assert_eq!(genesis_hash(), genesis_hash());
    }

    #[test]
    fn genesis_hash_nonzero() {
        assert!(!genesis_hash().is_zero());
    }

    #[test]
    fn genesis_hash_matches_header() {
        assert_eq!(genesis_hash(), genesis_block().header.hash());
    }

    // --- Txid ---

    #[test]
    fn genesis_coinbase_txid_deterministic() {
        assert_eq!(genesis_coinbase_txid(), genesis_coinbase_txid());
    }

    #[test]
    fn genesis_coinbase_txid_matches_computation() {
        let txid = genesis_block().transactions[0].txid().unwrap();
        assert_eq!(genesis_coinbase_txid(), txid);
    }

    // --- is_genesis ---

    #[test]
    fn is_genesis_true_for_genesis() {
        assert!(is_genesis(genesis_block()));
    }

    #[test]
    fn is_genesis_false_for_other_block() {
        let other = Block {
            header: BlockHeader {
                version: 1,
                prev_hash: genesis_hash(),
                merkle_root: Hash256::ZERO,
                timestamp: GENESIS_TIMESTAMP + 60,
                difficulty_target: u64::MAX,
                nonce: 0,
            },
            transactions: vec![],
        };
        assert!(!is_genesis(&other));
    }

    #[test]
    fn is_genesis_false_for_modified_genesis() {
        let mut modified = genesis_block().clone();
        modified.header.nonce = 999;
        assert!(!is_genesis(&modified));
    }

    // --- Dev fund ---

    #[test]
    fn dev_fund_pubkey_hash_deterministic() {
        assert_eq!(dev_fund_pubkey_hash(), dev_fund_pubkey_hash());
    }

    #[test]
    fn dev_fund_pubkey_hash_nonzero() {
        assert!(!dev_fund_pubkey_hash().is_zero());
    }
}
