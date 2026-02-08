//! Ed25519 cryptographic operations for the Rill protocol.
//!
//! Provides key generation, transaction signing, and signature verification.
//! Uses ed25519-dalek for the underlying Ed25519 implementation and BLAKE3
//! for pubkey hashing and signing hashes.
//!
//! # Signing scheme
//!
//! Transaction inputs are signed using a **sighash** that commits to:
//! - Transaction version and lock_time
//! - All input outpoints (txid + index)
//! - All outputs (value + pubkey_hash)
//! - The index of the input being signed
//!
//! Signatures and public keys are excluded from the sighash to avoid
//! circularity and allow inputs to be signed independently in any order.

use ed25519_dalek::{Signer, Verifier};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::error::CryptoError;
use crate::types::{Hash256, Transaction};

/// Ed25519 keypair for signing transactions.
///
/// Wraps [`ed25519_dalek::SigningKey`]. The secret key is zeroized on drop
/// by the underlying library. Use [`KeyPair::generate`] for random keys or
/// [`KeyPair::from_secret_bytes`] for deterministic derivation from a seed.
pub struct KeyPair {
    signing_key: ed25519_dalek::SigningKey,
}

impl KeyPair {
    /// Generate a random keypair using the OS cryptographic RNG.
    pub fn generate() -> Self {
        let mut csprng = rand::rngs::OsRng;
        Self {
            signing_key: ed25519_dalek::SigningKey::generate(&mut csprng),
        }
    }

    /// Create a keypair from 32-byte secret key material.
    pub fn from_secret_bytes(bytes: [u8; 32]) -> Self {
        Self {
            signing_key: ed25519_dalek::SigningKey::from_bytes(&bytes),
        }
    }

    /// Derive the public key from this keypair.
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            verifying_key: self.signing_key.verifying_key(),
        }
    }

    /// Get the raw secret key bytes (32 bytes). Handle with care.
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Sign a message, returning the raw 64-byte Ed25519 signature.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing_key.sign(message).to_bytes()
    }
}

impl Clone for KeyPair {
    fn clone(&self) -> Self {
        Self::from_secret_bytes(self.secret_bytes())
    }
}

impl fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyPair")
            .field("public_key", &self.public_key())
            .finish_non_exhaustive()
    }
}

/// Ed25519 public key for verifying signatures and deriving addresses.
///
/// The pubkey hash (BLAKE3 of the raw 32-byte key) is used in [`TxOutput`](crate::types::TxOutput)
/// to identify the recipient.
#[derive(Clone)]
pub struct PublicKey {
    verifying_key: ed25519_dalek::VerifyingKey,
}

impl PublicKey {
    /// Create a public key from raw bytes (32 bytes).
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        let vk = ed25519_dalek::VerifyingKey::from_bytes(bytes)
            .map_err(|_| CryptoError::InvalidPublicKey)?;
        Ok(Self { verifying_key: vk })
    }

    /// Get the raw public key bytes (32 bytes).
    pub fn to_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Compute the BLAKE3 pubkey hash used in transaction outputs.
    pub fn pubkey_hash(&self) -> Hash256 {
        pubkey_hash(&self.to_bytes())
    }

    /// Verify an Ed25519 signature on a message.
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> Result<(), CryptoError> {
        let sig = ed25519_dalek::Signature::from_bytes(signature);
        self.verifying_key
            .verify(message, &sig)
            .map_err(|_| CryptoError::VerificationFailed)
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({})", hex::encode(self.to_bytes()))
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.to_bytes()))
    }
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for PublicKey {}

impl std::hash::Hash for PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_bytes().hash(state);
    }
}

impl Serialize for PublicKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = <[u8; 32]>::deserialize(deserializer)?;
        Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

/// Compute the BLAKE3 pubkey hash from raw public key bytes.
///
/// This is the hash stored in [`TxOutput::pubkey_hash`](crate::types::TxOutput::pubkey_hash)
/// to identify the owner of an output.
pub fn pubkey_hash(pubkey_bytes: &[u8; 32]) -> Hash256 {
    Hash256(blake3::hash(pubkey_bytes).into())
}

/// Compute the signing hash (sighash) for a transaction input.
///
/// Commits to all inputs (outpoints only), all outputs, version, lock_time,
/// and the index of the input being signed. Signatures and public keys are
/// excluded to allow independent signing of each input.
pub fn signing_hash(tx: &Transaction, input_index: usize) -> Result<Hash256, CryptoError> {
    if input_index >= tx.inputs.len() {
        return Err(CryptoError::InputIndexOutOfBounds {
            index: input_index,
            len: tx.inputs.len(),
        });
    }

    let mut data = Vec::new();

    // Version
    data.extend_from_slice(&tx.version.to_le_bytes());

    // All input outpoints (no signatures/pubkeys)
    data.extend_from_slice(&(tx.inputs.len() as u64).to_le_bytes());
    for input in &tx.inputs {
        data.extend_from_slice(input.previous_output.txid.as_bytes());
        data.extend_from_slice(&input.previous_output.index.to_le_bytes());
    }

    // All outputs
    data.extend_from_slice(&(tx.outputs.len() as u64).to_le_bytes());
    for output in &tx.outputs {
        data.extend_from_slice(&output.value.to_le_bytes());
        data.extend_from_slice(output.pubkey_hash.as_bytes());
    }

    // Lock time
    data.extend_from_slice(&tx.lock_time.to_le_bytes());

    // Input index being signed
    data.extend_from_slice(&(input_index as u64).to_le_bytes());

    Ok(Hash256(blake3::hash(&data).into()))
}

/// Sign a transaction input in place.
///
/// Computes the signing hash for the given input, signs it with the keypair,
/// and writes the signature and public key bytes into the input.
/// Inputs can be signed in any order since the sighash excludes signatures.
pub fn sign_transaction_input(
    tx: &mut Transaction,
    input_index: usize,
    keypair: &KeyPair,
) -> Result<(), CryptoError> {
    let sighash = signing_hash(tx, input_index)?;
    let signature = keypair.sign(sighash.as_bytes());
    let pubkey_bytes = keypair.public_key().to_bytes();

    tx.inputs[input_index].signature = signature.to_vec();
    tx.inputs[input_index].public_key = pubkey_bytes.to_vec();
    Ok(())
}

/// Verify a transaction input's signature against an expected pubkey hash.
///
/// Checks that:
/// 1. The input contains a valid 64-byte signature and 32-byte public key
/// 2. The public key's BLAKE3 hash matches `expected_pubkey_hash` (the UTXO owner)
/// 3. The Ed25519 signature verifies against the sighash
pub fn verify_transaction_input(
    tx: &Transaction,
    input_index: usize,
    expected_pubkey_hash: &Hash256,
) -> Result<(), CryptoError> {
    if input_index >= tx.inputs.len() {
        return Err(CryptoError::InputIndexOutOfBounds {
            index: input_index,
            len: tx.inputs.len(),
        });
    }

    let input = &tx.inputs[input_index];

    // Parse public key (must be exactly 32 bytes)
    let pk_bytes: [u8; 32] = input
        .public_key
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::InvalidPublicKey)?;
    let pk = PublicKey::from_bytes(&pk_bytes)?;

    // Verify pubkey hash matches the UTXO owner
    if pk.pubkey_hash() != *expected_pubkey_hash {
        return Err(CryptoError::PubkeyHashMismatch);
    }

    // Parse signature (must be exactly 64 bytes)
    let sig_bytes: [u8; 64] = input
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::InvalidSignature)?;

    // Compute sighash and verify signature
    let sighash = signing_hash(tx, input_index)?;
    pk.verify(sighash.as_bytes(), &sig_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::COIN;
    use crate::types::{OutPoint, TxInput, TxOutput};

    // --- KeyPair ---

    #[test]
    fn keypair_generate_unique() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        assert_ne!(kp1.public_key(), kp2.public_key());
    }

    #[test]
    fn keypair_from_secret_deterministic() {
        let seed = [42u8; 32];
        let kp1 = KeyPair::from_secret_bytes(seed);
        let kp2 = KeyPair::from_secret_bytes(seed);
        assert_eq!(kp1.public_key(), kp2.public_key());
        assert_eq!(kp1.secret_bytes(), kp2.secret_bytes());
    }

    #[test]
    fn keypair_different_seeds_different_keys() {
        let kp1 = KeyPair::from_secret_bytes([1u8; 32]);
        let kp2 = KeyPair::from_secret_bytes([2u8; 32]);
        assert_ne!(kp1.public_key(), kp2.public_key());
    }

    #[test]
    fn keypair_clone() {
        let kp = KeyPair::generate();
        let kp2 = kp.clone();
        assert_eq!(kp.public_key(), kp2.public_key());
        assert_eq!(kp.secret_bytes(), kp2.secret_bytes());
    }

    #[test]
    fn keypair_debug_hides_secret() {
        let kp = KeyPair::generate();
        let debug = format!("{kp:?}");
        assert!(debug.contains("KeyPair"));
        assert!(debug.contains("public_key"));
        // Secret bytes should NOT appear in debug output
        let secret_hex = hex::encode(kp.secret_bytes());
        assert!(!debug.contains(&secret_hex));
    }

    // --- PublicKey ---

    #[test]
    fn pubkey_from_bytes_roundtrip() {
        let kp = KeyPair::generate();
        let pk = kp.public_key();
        let bytes = pk.to_bytes();
        let pk2 = PublicKey::from_bytes(&bytes).unwrap();
        assert_eq!(pk, pk2);
    }

    #[test]
    fn pubkey_from_invalid_bytes_fails() {
        // About half of all 32-byte values fail Ed25519 point decompression.
        // Try small y values until we find one that's invalid.
        let mut found_invalid = false;
        for i in 0u8..=20 {
            let mut bytes = [0u8; 32];
            bytes[0] = i;
            if PublicKey::from_bytes(&bytes).is_err() {
                assert_eq!(
                    PublicKey::from_bytes(&bytes).unwrap_err(),
                    CryptoError::InvalidPublicKey
                );
                found_invalid = true;
                break;
            }
        }
        assert!(
            found_invalid,
            "expected at least one y value in 0..=20 to fail Ed25519 decompression"
        );
    }

    #[test]
    fn pubkey_hash_deterministic() {
        let kp = KeyPair::from_secret_bytes([7u8; 32]);
        let pk = kp.public_key();
        assert_eq!(pk.pubkey_hash(), pk.pubkey_hash());
    }

    #[test]
    fn pubkey_hash_differs_for_different_keys() {
        let pk1 = KeyPair::from_secret_bytes([1u8; 32]).public_key();
        let pk2 = KeyPair::from_secret_bytes([2u8; 32]).public_key();
        assert_ne!(pk1.pubkey_hash(), pk2.pubkey_hash());
    }

    #[test]
    fn pubkey_hash_matches_standalone_fn() {
        let kp = KeyPair::generate();
        let pk = kp.public_key();
        assert_eq!(pk.pubkey_hash(), pubkey_hash(&pk.to_bytes()));
    }

    #[test]
    fn pubkey_display() {
        let kp = KeyPair::generate();
        let pk = kp.public_key();
        let display = format!("{pk}");
        assert_eq!(display.len(), 64); // 32 bytes = 64 hex chars
        assert!(display.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn pubkey_serde_json_roundtrip() {
        let kp = KeyPair::generate();
        let pk = kp.public_key();
        let json = serde_json::to_string(&pk).unwrap();
        let pk2: PublicKey = serde_json::from_str(&json).unwrap();
        assert_eq!(pk, pk2);
    }

    // --- Sign / Verify messages ---

    #[test]
    fn sign_verify_message() {
        let kp = KeyPair::generate();
        let msg = b"hello rill";
        let sig = kp.sign(msg);
        assert!(kp.public_key().verify(msg, &sig).is_ok());
    }

    #[test]
    fn verify_wrong_key_fails() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let msg = b"hello rill";
        let sig = kp1.sign(msg);
        let err = kp2.public_key().verify(msg, &sig).unwrap_err();
        assert_eq!(err, CryptoError::VerificationFailed);
    }

    #[test]
    fn verify_wrong_message_fails() {
        let kp = KeyPair::generate();
        let sig = kp.sign(b"original");
        let err = kp.public_key().verify(b"tampered", &sig).unwrap_err();
        assert_eq!(err, CryptoError::VerificationFailed);
    }

    // --- Signing hash ---

    fn unsigned_tx(kp: &KeyPair) -> Transaction {
        Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint {
                    txid: Hash256([0x11; 32]),
                    index: 0,
                },
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: kp.public_key().pubkey_hash(),
            }],
            lock_time: 0,
        }
    }

    #[test]
    fn signing_hash_deterministic() {
        let kp = KeyPair::generate();
        let tx = unsigned_tx(&kp);
        let h1 = signing_hash(&tx, 0).unwrap();
        let h2 = signing_hash(&tx, 0).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn signing_hash_changes_with_output() {
        let kp = KeyPair::generate();
        let tx1 = unsigned_tx(&kp);
        let mut tx2 = tx1.clone();
        tx2.outputs[0].value = 49 * COIN;

        assert_ne!(
            signing_hash(&tx1, 0).unwrap(),
            signing_hash(&tx2, 0).unwrap()
        );
    }

    #[test]
    fn signing_hash_changes_with_index() {
        let kp = KeyPair::generate();
        let mut tx = unsigned_tx(&kp);
        tx.inputs.push(TxInput {
            previous_output: OutPoint {
                txid: Hash256([0x22; 32]),
                index: 1,
            },
            signature: vec![],
            public_key: vec![],
        });

        assert_ne!(
            signing_hash(&tx, 0).unwrap(),
            signing_hash(&tx, 1).unwrap()
        );
    }

    #[test]
    fn signing_hash_changes_with_version() {
        let kp = KeyPair::generate();
        let tx1 = unsigned_tx(&kp);
        let mut tx2 = tx1.clone();
        tx2.version = 2;

        assert_ne!(
            signing_hash(&tx1, 0).unwrap(),
            signing_hash(&tx2, 0).unwrap()
        );
    }

    #[test]
    fn signing_hash_changes_with_locktime() {
        let kp = KeyPair::generate();
        let tx1 = unsigned_tx(&kp);
        let mut tx2 = tx1.clone();
        tx2.lock_time = 100;

        assert_ne!(
            signing_hash(&tx1, 0).unwrap(),
            signing_hash(&tx2, 0).unwrap()
        );
    }

    #[test]
    fn signing_hash_out_of_bounds() {
        let kp = KeyPair::generate();
        let tx = unsigned_tx(&kp);
        let err = signing_hash(&tx, 1).unwrap_err();
        assert_eq!(
            err,
            CryptoError::InputIndexOutOfBounds { index: 1, len: 1 }
        );
    }

    #[test]
    fn signing_hash_excludes_signatures() {
        let kp = KeyPair::generate();
        let tx1 = unsigned_tx(&kp);
        let mut tx2 = tx1.clone();
        tx2.inputs[0].signature = vec![0xAA; 64];
        tx2.inputs[0].public_key = vec![0xBB; 32];

        // Sighash should be identical regardless of signature/pubkey content
        assert_eq!(
            signing_hash(&tx1, 0).unwrap(),
            signing_hash(&tx2, 0).unwrap()
        );
    }

    // --- Transaction signing / verification ---

    #[test]
    fn sign_verify_transaction_input_roundtrip() {
        let kp = KeyPair::generate();
        let mut tx = unsigned_tx(&kp);
        let expected_hash = kp.public_key().pubkey_hash();

        sign_transaction_input(&mut tx, 0, &kp).unwrap();

        // Input should now have signature and pubkey populated
        assert_eq!(tx.inputs[0].signature.len(), 64);
        assert_eq!(tx.inputs[0].public_key.len(), 32);

        assert!(verify_transaction_input(&tx, 0, &expected_hash).is_ok());
    }

    #[test]
    fn verify_tx_wrong_pubkey_hash() {
        let kp = KeyPair::generate();
        let mut tx = unsigned_tx(&kp);
        sign_transaction_input(&mut tx, 0, &kp).unwrap();

        let wrong_hash = Hash256([0xFF; 32]);
        let err = verify_transaction_input(&tx, 0, &wrong_hash).unwrap_err();
        assert_eq!(err, CryptoError::PubkeyHashMismatch);
    }

    #[test]
    fn verify_tx_tampered_output_fails() {
        let kp = KeyPair::generate();
        let mut tx = unsigned_tx(&kp);
        let expected_hash = kp.public_key().pubkey_hash();
        sign_transaction_input(&mut tx, 0, &kp).unwrap();

        // Tamper with output value after signing
        tx.outputs[0].value = 999;

        let err = verify_transaction_input(&tx, 0, &expected_hash).unwrap_err();
        assert_eq!(err, CryptoError::VerificationFailed);
    }

    #[test]
    fn verify_tx_tampered_input_outpoint_fails() {
        let kp = KeyPair::generate();
        let mut tx = unsigned_tx(&kp);
        let expected_hash = kp.public_key().pubkey_hash();
        sign_transaction_input(&mut tx, 0, &kp).unwrap();

        // Tamper with input outpoint after signing
        tx.inputs[0].previous_output.index = 99;

        let err = verify_transaction_input(&tx, 0, &expected_hash).unwrap_err();
        assert_eq!(err, CryptoError::VerificationFailed);
    }

    #[test]
    fn verify_tx_wrong_signer_fails() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let mut tx = unsigned_tx(&kp1);

        // Sign with kp2 but the UTXO expects kp1's pubkey hash
        sign_transaction_input(&mut tx, 0, &kp2).unwrap();

        let expected_hash = kp1.public_key().pubkey_hash();
        let err = verify_transaction_input(&tx, 0, &expected_hash).unwrap_err();
        assert_eq!(err, CryptoError::PubkeyHashMismatch);
    }

    #[test]
    fn verify_tx_bad_signature_length() {
        let kp = KeyPair::generate();
        let mut tx = unsigned_tx(&kp);
        let expected_hash = kp.public_key().pubkey_hash();

        tx.inputs[0].signature = vec![0; 63]; // too short
        tx.inputs[0].public_key = kp.public_key().to_bytes().to_vec();

        let err = verify_transaction_input(&tx, 0, &expected_hash).unwrap_err();
        assert_eq!(err, CryptoError::InvalidSignature);
    }

    #[test]
    fn verify_tx_bad_pubkey_length() {
        let kp = KeyPair::generate();
        let mut tx = unsigned_tx(&kp);
        let expected_hash = kp.public_key().pubkey_hash();

        tx.inputs[0].signature = vec![0; 64];
        tx.inputs[0].public_key = vec![0; 31]; // too short

        let err = verify_transaction_input(&tx, 0, &expected_hash).unwrap_err();
        assert_eq!(err, CryptoError::InvalidPublicKey);
    }

    #[test]
    fn verify_tx_input_out_of_bounds() {
        let kp = KeyPair::generate();
        let tx = unsigned_tx(&kp);
        let expected_hash = kp.public_key().pubkey_hash();
        let err = verify_transaction_input(&tx, 5, &expected_hash).unwrap_err();
        assert_eq!(
            err,
            CryptoError::InputIndexOutOfBounds { index: 5, len: 1 }
        );
    }

    #[test]
    fn sign_multiple_inputs() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();

        let mut tx = Transaction {
            version: 1,
            inputs: vec![
                TxInput {
                    previous_output: OutPoint {
                        txid: Hash256([0x11; 32]),
                        index: 0,
                    },
                    signature: vec![],
                    public_key: vec![],
                },
                TxInput {
                    previous_output: OutPoint {
                        txid: Hash256([0x22; 32]),
                        index: 1,
                    },
                    signature: vec![],
                    public_key: vec![],
                },
            ],
            outputs: vec![TxOutput {
                value: 100 * COIN,
                pubkey_hash: kp1.public_key().pubkey_hash(),
            }],
            lock_time: 0,
        };

        // Sign each input with its respective key (any order is fine)
        sign_transaction_input(&mut tx, 1, &kp2).unwrap();
        sign_transaction_input(&mut tx, 0, &kp1).unwrap();

        // Both should verify
        assert!(verify_transaction_input(&tx, 0, &kp1.public_key().pubkey_hash()).is_ok());
        assert!(verify_transaction_input(&tx, 1, &kp2.public_key().pubkey_hash()).is_ok());
    }

    #[test]
    fn sign_input_out_of_bounds() {
        let kp = KeyPair::generate();
        let mut tx = unsigned_tx(&kp);
        let err = sign_transaction_input(&mut tx, 5, &kp).unwrap_err();
        assert_eq!(
            err,
            CryptoError::InputIndexOutOfBounds { index: 5, len: 1 }
        );
    }
}
