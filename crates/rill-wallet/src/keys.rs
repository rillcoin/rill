//! Seed management and deterministic key derivation.
//!
//! Uses BLAKE3 keyed derivation to produce Ed25519 keypairs from a 32-byte
//! master seed. This is simpler than BIP-32 (which is incompatible with
//! Ed25519) while providing the same deterministic, recoverable properties.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

use rill_core::address::{Address, Network};
use rill_core::crypto::KeyPair;
use rill_core::types::Hash256;

use crate::error::WalletError;

/// BLAKE3 KDF context for child key derivation.
const KDF_CONTEXT: &str = "rill-wallet-key-derivation-v1";

/// A 32-byte master seed for deterministic key derivation.
///
/// Secret material is zeroized on drop to prevent leaking key material
/// in freed memory.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Seed {
    bytes: [u8; 32],
}

impl Seed {
    /// Generate a random seed from the OS cryptographic RNG.
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self { bytes }
    }

    /// Create a seed from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Get the raw seed bytes. Handle with care.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl Clone for Seed {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes,
        }
    }
}

impl fmt::Debug for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Seed")
            .field("bytes", &"[REDACTED]")
            .finish()
    }
}

/// Deterministic key derivation chain from a master seed.
///
/// Derives child Ed25519 keypairs using BLAKE3's keyed derivation function.
/// Each child index produces a unique, deterministic keypair that can be
/// recovered from the seed alone.
pub struct KeyChain {
    seed: Seed,
    network: Network,
    next_index: u32,
    /// Cache of derived keypairs by index.
    keypairs: HashMap<u32, KeyPair>,
    /// Reverse lookup: pubkey_hash -> derivation index.
    pubkey_hash_to_index: HashMap<Hash256, u32>,
}

impl KeyChain {
    /// Create a new keychain from a seed and network.
    pub fn new(seed: Seed, network: Network) -> Self {
        Self {
            seed,
            network,
            next_index: 0,
            keypairs: HashMap::new(),
            pubkey_hash_to_index: HashMap::new(),
        }
    }

    /// Derive the keypair for a specific child index.
    pub fn derive_keypair(&mut self, index: u32) -> &KeyPair {
        if !self.keypairs.contains_key(&index) {
            let kp = derive_child_keypair(&self.seed, index);
            let pkh = kp.public_key().pubkey_hash();
            self.pubkey_hash_to_index.insert(pkh, index);
            self.keypairs.insert(index, kp);
        }
        &self.keypairs[&index]
    }

    /// Derive the next keypair, advancing the internal index.
    pub fn next_keypair(&mut self) -> &KeyPair {
        let index = self.next_index;
        self.next_index = self.next_index.saturating_add(1);
        self.derive_keypair(index)
    }

    /// Get the address for a specific derivation index.
    pub fn address_at(&mut self, index: u32) -> Address {
        let kp = self.derive_keypair(index);
        Address::from_public_key(&kp.public_key(), self.network)
    }

    /// Derive the next address, advancing the internal index.
    pub fn next_address(&mut self) -> Address {
        let index = self.next_index;
        self.next_index = self.next_index.saturating_add(1);
        self.derive_keypair(index);
        let kp = &self.keypairs[&index];
        Address::from_public_key(&kp.public_key(), self.network)
    }

    /// Look up the keypair that owns a given pubkey hash.
    ///
    /// Performs a linear scan of derived keys. Returns `None` if no
    /// derived key matches the hash.
    pub fn keypair_for_pubkey_hash(&self, hash: &Hash256) -> Option<&KeyPair> {
        self.pubkey_hash_to_index
            .get(hash)
            .and_then(|idx| self.keypairs.get(idx))
    }

    /// Restore the keychain state by deriving all keys up to index `n`.
    ///
    /// Used when loading a wallet from file to rebuild the keypair cache
    /// and pubkey hash lookup table.
    pub fn restore_to_index(&mut self, n: u32) {
        for i in 0..n {
            self.derive_keypair(i);
        }
        self.next_index = n;
    }

    /// The network this keychain is configured for.
    pub fn network(&self) -> Network {
        self.network
    }

    /// The next derivation index that will be used.
    pub fn next_index(&self) -> u32 {
        self.next_index
    }

    /// Access the seed (for wallet file serialization).
    pub(crate) fn seed(&self) -> &Seed {
        &self.seed
    }

    /// Get all known pubkey hashes.
    pub fn known_pubkey_hashes(&self) -> impl Iterator<Item = &Hash256> {
        self.pubkey_hash_to_index.keys()
    }
}

impl fmt::Debug for KeyChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyChain")
            .field("network", &self.network)
            .field("next_index", &self.next_index)
            .field("cached_keys", &self.keypairs.len())
            .finish()
    }
}

/// Serializable form of keychain state for wallet file persistence.
#[derive(Serialize, Deserialize, Clone)]
pub struct KeyChainData {
    /// Master seed bytes.
    pub seed: [u8; 32],
    /// Network identifier.
    pub network: Network,
    /// Next derivation index.
    pub next_index: u32,
}

impl KeyChainData {
    /// Create keychain data from a keychain.
    pub fn from_keychain(keychain: &KeyChain) -> Self {
        Self {
            seed: *keychain.seed().as_bytes(),
            network: keychain.network(),
            next_index: keychain.next_index(),
        }
    }

    /// Restore a keychain from serialized data.
    pub fn to_keychain(&self) -> KeyChain {
        let seed = Seed::from_bytes(self.seed);
        let mut keychain = KeyChain::new(seed, self.network);
        keychain.restore_to_index(self.next_index);
        keychain
    }
}

/// Derive a child keypair from a seed and index using BLAKE3 KDF.
fn derive_child_keypair(seed: &Seed, index: u32) -> KeyPair {
    let mut ikm = Vec::with_capacity(36);
    ikm.extend_from_slice(seed.as_bytes());
    ikm.extend_from_slice(&index.to_le_bytes());
    let derived = blake3::derive_key(KDF_CONTEXT, &ikm);
    KeyPair::from_secret_bytes(derived)
}

/// Validate that a seed produces valid keypairs.
pub fn validate_seed(seed: &Seed) -> Result<(), WalletError> {
    // Try deriving a single keypair to verify the seed is usable
    let _kp = derive_child_keypair(seed, 0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_generate_unique() {
        let s1 = Seed::generate();
        let s2 = Seed::generate();
        assert_ne!(s1.as_bytes(), s2.as_bytes());
    }

    #[test]
    fn seed_from_bytes_roundtrip() {
        let bytes = [42u8; 32];
        let seed = Seed::from_bytes(bytes);
        assert_eq!(seed.as_bytes(), &bytes);
    }

    #[test]
    fn seed_debug_hides_bytes() {
        let seed = Seed::from_bytes([0xAB; 32]);
        let debug = format!("{seed:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("ab"));
    }

    #[test]
    fn seed_clone() {
        let seed = Seed::from_bytes([7u8; 32]);
        let cloned = seed.clone();
        assert_eq!(seed.as_bytes(), cloned.as_bytes());
    }

    #[test]
    fn derive_deterministic() {
        let seed = Seed::from_bytes([1u8; 32]);
        let kp1 = derive_child_keypair(&seed, 0);
        let kp2 = derive_child_keypair(&seed, 0);
        assert_eq!(kp1.public_key(), kp2.public_key());
    }

    #[test]
    fn derive_unique_per_index() {
        let seed = Seed::from_bytes([1u8; 32]);
        let kp0 = derive_child_keypair(&seed, 0);
        let kp1 = derive_child_keypair(&seed, 1);
        assert_ne!(kp0.public_key(), kp1.public_key());
    }

    #[test]
    fn derive_unique_per_seed() {
        let kp1 = derive_child_keypair(&Seed::from_bytes([1u8; 32]), 0);
        let kp2 = derive_child_keypair(&Seed::from_bytes([2u8; 32]), 0);
        assert_ne!(kp1.public_key(), kp2.public_key());
    }

    #[test]
    fn keychain_next_keypair_advances() {
        let seed = Seed::from_bytes([3u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        assert_eq!(kc.next_index(), 0);

        let pk0 = kc.next_keypair().public_key();
        assert_eq!(kc.next_index(), 1);

        let pk1 = kc.next_keypair().public_key();
        assert_eq!(kc.next_index(), 2);

        assert_ne!(pk0, pk1);
    }

    #[test]
    fn keychain_address_at() {
        let seed = Seed::from_bytes([4u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Mainnet);
        let addr = kc.address_at(5);
        assert_eq!(addr.network(), Network::Mainnet);

        // Deriving again gives the same address
        let addr2 = kc.address_at(5);
        assert_eq!(addr, addr2);
    }

    #[test]
    fn keychain_next_address() {
        let seed = Seed::from_bytes([5u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let addr0 = kc.next_address();
        let addr1 = kc.next_address();
        assert_ne!(addr0, addr1);
        assert_eq!(addr0.network(), Network::Testnet);
    }

    #[test]
    fn keychain_pubkey_hash_lookup() {
        let seed = Seed::from_bytes([6u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Mainnet);

        // Derive a few keys
        let pk0 = kc.next_keypair().public_key();
        let pk1 = kc.next_keypair().public_key();
        let hash0 = pk0.pubkey_hash();
        let hash1 = pk1.pubkey_hash();

        // Lookup by pubkey hash
        let found0 = kc.keypair_for_pubkey_hash(&hash0).unwrap();
        assert_eq!(found0.public_key(), pk0);
        let found1 = kc.keypair_for_pubkey_hash(&hash1).unwrap();
        assert_eq!(found1.public_key(), pk1);

        // Unknown hash returns None
        assert!(kc.keypair_for_pubkey_hash(&Hash256::ZERO).is_none());
    }

    #[test]
    fn keychain_restore_to_index() {
        let seed_bytes = [7u8; 32];

        // Derive keys 0..5 in the original keychain
        let mut kc1 = KeyChain::new(Seed::from_bytes(seed_bytes), Network::Mainnet);
        let mut pubkeys = Vec::new();
        for _ in 0..5 {
            pubkeys.push(kc1.next_keypair().public_key());
        }

        // Create a new keychain and restore to index 5
        let mut kc2 = KeyChain::new(Seed::from_bytes(seed_bytes), Network::Mainnet);
        kc2.restore_to_index(5);

        // All previously derived keys should match
        for (i, pk) in pubkeys.iter().enumerate() {
            let kp = kc2.keypair_for_pubkey_hash(&pk.pubkey_hash()).unwrap();
            assert_eq!(kp.public_key(), *pk, "mismatch at index {i}");
        }
        assert_eq!(kc2.next_index(), 5);
    }

    #[test]
    fn keychain_data_serde_roundtrip() {
        let seed = Seed::from_bytes([8u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        kc.next_keypair();
        kc.next_keypair();

        let data = KeyChainData::from_keychain(&kc);
        let json = serde_json::to_string(&data).unwrap();
        let restored_data: KeyChainData = serde_json::from_str(&json).unwrap();

        assert_eq!(restored_data.seed, *kc.seed().as_bytes());
        assert_eq!(restored_data.network, Network::Testnet);
        assert_eq!(restored_data.next_index, 2);
    }

    #[test]
    fn keychain_data_restore() {
        let seed = Seed::from_bytes([9u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Mainnet);
        let pk0 = kc.next_keypair().public_key();
        let pk1 = kc.next_keypair().public_key();

        let data = KeyChainData::from_keychain(&kc);
        let restored = data.to_keychain();

        // Restored keychain should have the same pubkeys
        assert_eq!(
            restored
                .keypair_for_pubkey_hash(&pk0.pubkey_hash())
                .unwrap()
                .public_key(),
            pk0
        );
        assert_eq!(
            restored
                .keypair_for_pubkey_hash(&pk1.pubkey_hash())
                .unwrap()
                .public_key(),
            pk1
        );
        assert_eq!(restored.next_index(), 2);
    }

    #[test]
    fn keychain_debug_format() {
        let seed = Seed::from_bytes([10u8; 32]);
        let kc = KeyChain::new(seed, Network::Mainnet);
        let debug = format!("{kc:?}");
        assert!(debug.contains("KeyChain"));
        assert!(debug.contains("Mainnet"));
    }

    #[test]
    fn validate_seed_works() {
        let seed = Seed::from_bytes([11u8; 32]);
        assert!(validate_seed(&seed).is_ok());
    }
}
