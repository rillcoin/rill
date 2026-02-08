//! Core protocol types: transactions, blocks, UTXOs.
//!
//! All monetary values are in rills (1 RILL = 10^8 rills).
//! All numeric fields use u64 per protocol convention.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

use crate::error::TransactionError;

/// A 32-byte hash value.
///
/// Used for transaction IDs (BLAKE3), block header hashes (SHA-256),
/// and merkle roots (BLAKE3).
#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default,
    bincode::Encode, bincode::Decode,
)]
pub struct Hash256(pub [u8; 32]);

impl Hash256 {
    /// The zero hash (32 zero bytes). Used for coinbase previous outpoints.
    pub const ZERO: Self = Self([0u8; 32]);

    /// Create a Hash256 from a byte array.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return the underlying bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Check if this is the zero hash.
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 32]
    }
}

impl fmt::Display for Hash256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl From<[u8; 32]> for Hash256 {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Hash256 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Reference to a specific output of a previous transaction.
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash,
    bincode::Encode, bincode::Decode,
)]
pub struct OutPoint {
    /// Transaction ID containing the referenced output.
    pub txid: Hash256,
    /// Index of the output within the transaction.
    pub index: u64,
}

impl OutPoint {
    /// The null outpoint, used for coinbase transaction inputs.
    pub fn null() -> Self {
        Self {
            txid: Hash256::ZERO,
            index: u64::MAX,
        }
    }

    /// Check if this is the null outpoint (coinbase marker).
    pub fn is_null(&self) -> bool {
        self.txid.is_zero() && self.index == u64::MAX
    }
}

impl fmt::Display for OutPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.txid, self.index)
    }
}

/// A transaction input, spending a previous output.
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
    bincode::Encode, bincode::Decode,
)]
pub struct TxInput {
    /// The outpoint being spent. Null outpoint for coinbase.
    pub previous_output: OutPoint,
    /// Ed25519 signature (64 bytes). Empty for coinbase inputs.
    pub signature: Vec<u8>,
    /// Ed25519 public key (32 bytes). Empty for coinbase inputs.
    pub public_key: Vec<u8>,
}

/// A transaction output, creating a new UTXO.
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
    bincode::Encode, bincode::Decode,
)]
pub struct TxOutput {
    /// Value in rills (1 RILL = 10^8 rills).
    pub value: u64,
    /// BLAKE3 hash of the recipient's Ed25519 public key.
    pub pubkey_hash: Hash256,
}

/// A transaction transferring value between addresses.
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
    bincode::Encode, bincode::Decode,
)]
pub struct Transaction {
    /// Protocol version.
    pub version: u64,
    /// Inputs consuming previous outputs.
    pub inputs: Vec<TxInput>,
    /// New outputs created by this transaction.
    pub outputs: Vec<TxOutput>,
    /// Block height or timestamp before which this tx is invalid.
    pub lock_time: u64,
}

impl Transaction {
    /// Compute the transaction ID (BLAKE3 hash of the canonical encoding).
    ///
    /// Uses bincode with standard config for deterministic serialization.
    /// Returns an error if serialization fails.
    pub fn txid(&self) -> Result<Hash256, TransactionError> {
        let encoded = bincode::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| TransactionError::Serialization(e.to_string()))?;
        Ok(Hash256(blake3::hash(&encoded).into()))
    }

    /// Check if this is a coinbase transaction (single input with null outpoint).
    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1 && self.inputs[0].previous_output.is_null()
    }

    /// Sum of all output values. Returns None on overflow.
    pub fn total_output_value(&self) -> Option<u64> {
        self.outputs
            .iter()
            .try_fold(0u64, |acc, out| acc.checked_add(out.value))
    }
}

/// Block header containing the proof-of-work puzzle.
///
/// Hash is computed as double SHA-256 over a fixed byte layout for RandomX compatibility.
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
    bincode::Encode, bincode::Decode,
)]
pub struct BlockHeader {
    /// Protocol version.
    pub version: u64,
    /// Hash of the previous block header.
    pub prev_hash: Hash256,
    /// BLAKE3 merkle root of the block's transactions.
    pub merkle_root: Hash256,
    /// Unix timestamp in seconds.
    pub timestamp: u64,
    /// Compact difficulty target.
    pub difficulty_target: u64,
    /// Proof-of-work nonce.
    pub nonce: u64,
}

impl BlockHeader {
    /// Header size in bytes when serialized for hashing (4 u64 fields + 2 * 32-byte hashes).
    const HASH_SIZE: usize = 4 * 8 + 2 * 32;

    /// Compute the block header hash (double SHA-256).
    ///
    /// Uses an explicit fixed byte layout: version || prev_hash || merkle_root ||
    /// timestamp || difficulty_target || nonce, all little-endian.
    pub fn hash(&self) -> Hash256 {
        let mut data = Vec::with_capacity(Self::HASH_SIZE);
        data.extend_from_slice(&self.version.to_le_bytes());
        data.extend_from_slice(self.prev_hash.as_bytes());
        data.extend_from_slice(self.merkle_root.as_bytes());
        data.extend_from_slice(&self.timestamp.to_le_bytes());
        data.extend_from_slice(&self.difficulty_target.to_le_bytes());
        data.extend_from_slice(&self.nonce.to_le_bytes());
        let first = Sha256::digest(&data);
        Hash256(Sha256::digest(first).into())
    }
}

/// A complete block: header plus transactions.
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
    bincode::Encode, bincode::Decode,
)]
pub struct Block {
    /// Block header with proof-of-work.
    pub header: BlockHeader,
    /// Ordered list of transactions. First transaction must be coinbase.
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Get the coinbase transaction, if the block is non-empty.
    pub fn coinbase(&self) -> Option<&Transaction> {
        self.transactions.first()
    }
}

/// An entry in the unspent transaction output set.
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
    bincode::Encode, bincode::Decode,
)]
pub struct UtxoEntry {
    /// The unspent output.
    pub output: TxOutput,
    /// Height of the block containing this UTXO.
    pub block_height: u64,
    /// Whether this output is from a coinbase transaction.
    pub is_coinbase: bool,
    /// Cluster ID for decay calculation tracking.
    pub cluster_id: Hash256,
}

impl UtxoEntry {
    /// Check if this UTXO has matured and can be spent.
    ///
    /// Coinbase outputs require [`COINBASE_MATURITY`](crate::constants::COINBASE_MATURITY)
    /// confirmations. Non-coinbase outputs are always mature.
    pub fn is_mature(&self, current_height: u64) -> bool {
        if !self.is_coinbase {
            return true;
        }
        current_height.saturating_sub(self.block_height) >= crate::constants::COINBASE_MATURITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::COIN;

    fn sample_pubkey_hash() -> Hash256 {
        Hash256([0xAA; 32])
    }

    fn sample_tx() -> Transaction {
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
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: sample_pubkey_hash(),
            }],
            lock_time: 0,
        }
    }

    fn sample_coinbase() -> Transaction {
        Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: sample_pubkey_hash(),
            }],
            lock_time: 0,
        }
    }

    fn sample_header() -> BlockHeader {
        BlockHeader {
            version: 1,
            prev_hash: Hash256::ZERO,
            merkle_root: Hash256::ZERO,
            timestamp: 1_700_000_000,
            difficulty_target: u64::MAX,
            nonce: 0,
        }
    }

    // --- Hash256 ---

    #[test]
    fn hash256_zero_is_zero() {
        let h = Hash256::ZERO;
        assert!(h.is_zero());
        assert_eq!(h, Hash256::default());
    }

    #[test]
    fn hash256_nonzero_is_not_zero() {
        assert!(!Hash256([1; 32]).is_zero());
    }

    #[test]
    fn hash256_display_hex() {
        let h = Hash256([0xAB; 32]);
        let s = format!("{h}");
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(&s[0..2], "ab");
    }

    #[test]
    fn hash256_from_bytes() {
        let bytes = [42u8; 32];
        let h = Hash256::from_bytes(bytes);
        assert_eq!(h.as_bytes(), &bytes);
        assert_eq!(Hash256::from(bytes), h);
    }

    // --- OutPoint ---

    #[test]
    fn outpoint_null_detection() {
        assert!(OutPoint::null().is_null());
    }

    #[test]
    fn outpoint_non_null() {
        let op = OutPoint { txid: Hash256([1; 32]), index: 0 };
        assert!(!op.is_null());
    }

    #[test]
    fn outpoint_display() {
        let op = OutPoint { txid: Hash256([0xFF; 32]), index: 3 };
        let s = format!("{op}");
        assert!(s.ends_with(":3"));
    }

    // --- Transaction ---

    #[test]
    fn coinbase_detection() {
        assert!(sample_coinbase().is_coinbase());
        assert!(!sample_tx().is_coinbase());
    }

    #[test]
    fn multi_input_not_coinbase() {
        let tx = Transaction {
            version: 1,
            inputs: vec![
                TxInput {
                    previous_output: OutPoint::null(),
                    signature: vec![],
                    public_key: vec![],
                },
                TxInput {
                    previous_output: OutPoint::null(),
                    signature: vec![],
                    public_key: vec![],
                },
            ],
            outputs: vec![],
            lock_time: 0,
        };
        assert!(!tx.is_coinbase());
    }

    #[test]
    fn total_output_value_sums_correctly() {
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![
                TxOutput { value: 100, pubkey_hash: Hash256::ZERO },
                TxOutput { value: 200, pubkey_hash: Hash256::ZERO },
                TxOutput { value: 300, pubkey_hash: Hash256::ZERO },
            ],
            lock_time: 0,
        };
        assert_eq!(tx.total_output_value(), Some(600));
    }

    #[test]
    fn total_output_value_overflow_returns_none() {
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![
                TxOutput { value: u64::MAX, pubkey_hash: Hash256::ZERO },
                TxOutput { value: 1, pubkey_hash: Hash256::ZERO },
            ],
            lock_time: 0,
        };
        assert_eq!(tx.total_output_value(), None);
    }

    #[test]
    fn total_output_value_empty() {
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![],
            lock_time: 0,
        };
        assert_eq!(tx.total_output_value(), Some(0));
    }

    #[test]
    fn txid_deterministic() {
        let tx = sample_tx();
        assert_eq!(tx.txid().unwrap(), tx.txid().unwrap());
    }

    #[test]
    fn txid_changes_with_data() {
        let tx1 = sample_tx();
        let mut tx2 = sample_tx();
        tx2.lock_time = 1;
        assert_ne!(tx1.txid().unwrap(), tx2.txid().unwrap());
    }

    #[test]
    fn txid_is_nonzero() {
        assert!(!sample_tx().txid().unwrap().is_zero());
    }

    // --- BlockHeader ---

    #[test]
    fn block_header_hash_deterministic() {
        let h = sample_header();
        assert_eq!(h.hash(), h.hash());
    }

    #[test]
    fn block_header_hash_changes_with_nonce() {
        let h1 = sample_header();
        let mut h2 = h1.clone();
        h2.nonce = 1;
        assert_ne!(h1.hash(), h2.hash());
    }

    #[test]
    fn block_header_hash_is_nonzero() {
        assert!(!sample_header().hash().is_zero());
    }

    #[test]
    fn block_header_hash_fixed_size_input() {
        // Verify the hash input is always exactly HASH_SIZE bytes
        let h = sample_header();
        let mut data = Vec::new();
        data.extend_from_slice(&h.version.to_le_bytes());
        data.extend_from_slice(h.prev_hash.as_bytes());
        data.extend_from_slice(h.merkle_root.as_bytes());
        data.extend_from_slice(&h.timestamp.to_le_bytes());
        data.extend_from_slice(&h.difficulty_target.to_le_bytes());
        data.extend_from_slice(&h.nonce.to_le_bytes());
        assert_eq!(data.len(), BlockHeader::HASH_SIZE);
    }

    // --- Block ---

    #[test]
    fn block_coinbase_accessor() {
        let block = Block {
            header: sample_header(),
            transactions: vec![sample_coinbase()],
        };
        assert!(block.coinbase().unwrap().is_coinbase());
    }

    #[test]
    fn block_empty_has_no_coinbase() {
        let block = Block {
            header: sample_header(),
            transactions: vec![],
        };
        assert!(block.coinbase().is_none());
    }

    // --- UtxoEntry ---

    #[test]
    fn utxo_coinbase_not_mature_early() {
        let entry = UtxoEntry {
            output: TxOutput { value: 50 * COIN, pubkey_hash: Hash256::ZERO },
            block_height: 100,
            is_coinbase: true,
            cluster_id: Hash256::ZERO,
        };
        assert!(!entry.is_mature(150));
    }

    #[test]
    fn utxo_coinbase_mature_at_threshold() {
        let entry = UtxoEntry {
            output: TxOutput { value: 50 * COIN, pubkey_hash: Hash256::ZERO },
            block_height: 100,
            is_coinbase: true,
            cluster_id: Hash256::ZERO,
        };
        assert!(entry.is_mature(200));
    }

    #[test]
    fn utxo_coinbase_mature_past_threshold() {
        let entry = UtxoEntry {
            output: TxOutput { value: 50 * COIN, pubkey_hash: Hash256::ZERO },
            block_height: 100,
            is_coinbase: true,
            cluster_id: Hash256::ZERO,
        };
        assert!(entry.is_mature(300));
    }

    #[test]
    fn utxo_non_coinbase_always_mature() {
        let entry = UtxoEntry {
            output: TxOutput { value: 100, pubkey_hash: Hash256::ZERO },
            block_height: 100,
            is_coinbase: false,
            cluster_id: Hash256::ZERO,
        };
        assert!(entry.is_mature(100));
        assert!(entry.is_mature(0));
    }

    // --- Bincode round-trips ---

    #[test]
    fn bincode_round_trip_transaction() {
        let tx = sample_tx();
        let encoded = bincode::encode_to_vec(&tx, bincode::config::standard()).unwrap();
        let (decoded, _): (Transaction, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn bincode_round_trip_block_header() {
        let header = sample_header();
        let encoded = bincode::encode_to_vec(&header, bincode::config::standard()).unwrap();
        let (decoded, _): (BlockHeader, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn bincode_round_trip_block() {
        let block = Block {
            header: sample_header(),
            transactions: vec![sample_coinbase(), sample_tx()],
        };
        let encoded = bincode::encode_to_vec(&block, bincode::config::standard()).unwrap();
        let (decoded, _): (Block, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
        assert_eq!(block, decoded);
    }

    #[test]
    fn bincode_round_trip_utxo_entry() {
        let entry = UtxoEntry {
            output: TxOutput { value: 50 * COIN, pubkey_hash: Hash256([0xCC; 32]) },
            block_height: 12345,
            is_coinbase: true,
            cluster_id: Hash256([0xDD; 32]),
        };
        let encoded = bincode::encode_to_vec(&entry, bincode::config::standard()).unwrap();
        let (decoded, _): (UtxoEntry, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
        assert_eq!(entry, decoded);
    }
}
