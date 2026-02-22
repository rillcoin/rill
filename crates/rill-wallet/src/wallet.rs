//! Wallet composition: key management, UTXO tracking, transaction creation.
//!
//! The [`Wallet`] struct ties together key derivation, coin selection,
//! transaction building, and encrypted file persistence. It maintains an
//! in-memory set of owned UTXOs discovered by scanning the chain.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rill_core::address::{Address, Network};
use rill_core::constants::COIN;
use rill_core::traits::{ChainState, DecayCalculator};
use rill_core::types::{Hash256, OutPoint, Transaction, UtxoEntry};

use crate::builder::TransactionBuilder;
use crate::encryption;
use crate::error::WalletError;
use crate::keys::{KeyChain, KeyChainData, Seed};

/// Magic bytes identifying a Rill wallet file.
pub const WALLET_MAGIC: &[u8; 4] = b"RIWL";

/// Current wallet file format version.
pub const WALLET_VERSION: u32 = 1;

/// Balance summary with decay-adjusted values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletBalance {
    /// Total nominal (pre-decay) value in rills.
    pub nominal: u64,
    /// Total effective (post-decay) value in rills.
    pub effective: u64,
    /// Total decay amount in rills.
    pub decay: u64,
    /// Number of UTXOs.
    pub utxo_count: usize,
}

impl WalletBalance {
    /// Nominal balance in RILL (display helper, not for consensus).
    pub fn nominal_rill(&self) -> f64 {
        self.nominal as f64 / COIN as f64
    }

    /// Effective balance in RILL (display helper, not for consensus).
    pub fn effective_rill(&self) -> f64 {
        self.effective as f64 / COIN as f64
    }

    /// Decay amount in RILL (display helper, not for consensus).
    pub fn decay_rill(&self) -> f64 {
        self.decay as f64 / COIN as f64
    }
}

/// Wallet file header serialized as JSON.
#[derive(serde::Serialize, serde::Deserialize)]
struct WalletFileHeader {
    magic: String,
    version: u32,
}

/// HD wallet with decay-aware transaction support.
///
/// Manages a deterministic keychain, tracks owned UTXOs, and provides
/// high-level operations for sending and receiving funds.
pub struct Wallet {
    keychain: KeyChain,
    /// Owned UTXOs: outpoint -> entry.
    utxos: HashMap<OutPoint, UtxoEntry>,
    /// Set of pubkey hashes owned by this wallet (for UTXO scanning).
    owned_pubkey_hashes: HashSet<Hash256>,
}

impl Wallet {
    /// Create a new wallet with a random seed.
    pub fn create(network: Network) -> Self {
        let seed = Seed::generate();
        Self::from_seed(seed, network)
    }

    /// Create a wallet from an existing seed (deterministic recovery).
    pub fn from_seed(seed: Seed, network: Network) -> Self {
        let keychain = KeyChain::new(seed, network);
        Self {
            keychain,
            utxos: HashMap::new(),
            owned_pubkey_hashes: HashSet::new(),
        }
    }

    /// Derive the next receive address and register its pubkey hash.
    pub fn next_address(&mut self) -> Address {
        let addr = self.keychain.next_address();
        self.owned_pubkey_hashes.insert(addr.pubkey_hash());
        addr
    }

    /// The network this wallet is configured for.
    pub fn network(&self) -> Network {
        self.keychain.network()
    }

    /// Number of derived addresses.
    pub fn address_count(&self) -> u32 {
        self.keychain.next_index()
    }

    /// Scan a UTXO set and update the wallet's owned UTXOs.
    ///
    /// Phase 1: takes an explicit list. Production: bloom filter / indexer.
    pub fn scan_utxos(&mut self, utxo_set: &[(OutPoint, UtxoEntry)]) {
        self.utxos.clear();
        for (outpoint, entry) in utxo_set {
            if self.owned_pubkey_hashes.contains(&entry.output.pubkey_hash) {
                self.utxos.insert(outpoint.clone(), entry.clone());
            }
        }
    }

    /// Compute the current wallet balance with decay adjustments.
    pub fn balance(
        &self,
        decay_calc: &dyn DecayCalculator,
        chain_state: &dyn ChainState,
        height: u64,
    ) -> Result<WalletBalance, WalletError> {
        let supply = chain_state
            .circulating_supply()
            .map_err(|e| WalletError::BuildError(e.to_string()))?;

        let mut nominal: u64 = 0;
        let mut effective: u64 = 0;
        let mut decay: u64 = 0;

        for entry in self.utxos.values() {
            let val = entry.output.value;
            let blocks_held = height.saturating_sub(entry.block_height);

            let cluster_bal = chain_state
                .cluster_balance(&entry.cluster_id)
                .map_err(|e| WalletError::BuildError(e.to_string()))?;
            let concentration = if supply > 0 {
                ((cluster_bal as u128)
                    * (rill_core::constants::CONCENTRATION_PRECISION as u128)
                    / (supply as u128)) as u64
            } else {
                0
            };

            let eff = decay_calc
                .effective_value(val, concentration, blocks_held)
                .map_err(WalletError::Decay)?;
            let dec = val.saturating_sub(eff);

            nominal = nominal.saturating_add(val);
            effective = effective.saturating_add(eff);
            decay = decay.saturating_add(dec);
        }

        Ok(WalletBalance {
            nominal,
            effective,
            decay,
            utxo_count: self.utxos.len(),
        })
    }

    /// Number of owned UTXOs.
    pub fn utxo_count(&self) -> usize {
        self.utxos.len()
    }

    /// Access the keychain (for signing operations).
    pub fn keychain(&self) -> &KeyChain {
        &self.keychain
    }

    /// Access the keychain mutably (for address derivation).
    pub fn keychain_mut(&mut self) -> &mut KeyChain {
        &mut self.keychain
    }

    /// Get all owned UTXOs as a vector.
    pub fn owned_utxos(&self) -> Vec<(OutPoint, UtxoEntry)> {
        self.utxos.iter().map(|(op, entry)| (op.clone(), entry.clone())).collect()
    }

    /// Build and sign a transaction sending to the given recipients.
    ///
    /// Returns the signed transaction ready for broadcast.
    pub fn send(
        &mut self,
        recipients: &[(Address, u64)],
        decay_calc: &dyn DecayCalculator,
        chain_state: &dyn ChainState,
        height: u64,
    ) -> Result<Transaction, WalletError> {
        if recipients.is_empty() {
            return Err(WalletError::BuildError("no recipients".into()));
        }

        let utxo_list: Vec<(OutPoint, UtxoEntry)> = self.utxos.clone().into_iter().collect();
        let change_addr = self.next_address();

        let mut builder = TransactionBuilder::new();
        for (addr, amount) in recipients {
            builder.add_recipient(addr.clone(), *amount);
        }

        let unsigned = builder.build(&utxo_list, &change_addr, decay_calc, chain_state, height)?;
        TransactionBuilder::sign(unsigned, &self.keychain)
    }

    /// Register this wallet as an agent wallet.
    ///
    /// Constructs and signs a `TxType::AgentRegister` transaction that sends
    /// at least [`AGENT_REGISTRATION_STAKE`](rill_core::constants::AGENT_REGISTRATION_STAKE)
    /// to the wallet's own address as the stake output.
    pub fn register_as_agent(
        &mut self,
        decay_calc: &dyn DecayCalculator,
        chain_state: &dyn ChainState,
        height: u64,
    ) -> Result<Transaction, WalletError> {
        use rill_core::constants::AGENT_REGISTRATION_STAKE;
        use rill_core::types::TxType;

        let stake_addr = self.next_address();
        let stake_amount = AGENT_REGISTRATION_STAKE;

        // Build the transaction using the normal send flow (coin selection + change).
        let utxo_list: Vec<(OutPoint, UtxoEntry)> = self.utxos.clone().into_iter().collect();
        let change_addr = self.next_address();

        let mut builder = TransactionBuilder::new();
        builder.add_recipient(stake_addr, stake_amount);

        let mut unsigned = builder.build(&utxo_list, &change_addr, decay_calc, chain_state, height)?;

        // Override tx_type to AgentRegister.
        unsigned.tx.tx_type = TxType::AgentRegister;

        TransactionBuilder::sign(unsigned, &self.keychain)
    }

    /// Save the wallet to an encrypted file.
    ///
    /// # File format
    /// ```text
    /// header_len (4 bytes LE) || header_json || encrypted_payload
    /// ```
    /// The header is unencrypted JSON containing magic bytes and version.
    /// The payload is AES-256-GCM encrypted keychain data.
    pub fn save_to_file(&self, path: &Path, password: &[u8]) -> Result<(), WalletError> {
        let header = WalletFileHeader {
            magic: String::from_utf8_lossy(WALLET_MAGIC).to_string(),
            version: WALLET_VERSION,
        };
        let header_json =
            serde_json::to_vec(&header).map_err(|e| WalletError::Serialization(e.to_string()))?;

        let kc_data = KeyChainData::from_keychain(&self.keychain);
        let payload_json =
            serde_json::to_vec(&kc_data).map_err(|e| WalletError::Serialization(e.to_string()))?;

        let encrypted = encryption::encrypt(&payload_json, password)?;

        let header_len = header_json.len() as u32;
        let mut file_data =
            Vec::with_capacity(4 + header_json.len() + encrypted.len());
        file_data.extend_from_slice(&header_len.to_le_bytes());
        file_data.extend_from_slice(&header_json);
        file_data.extend_from_slice(&encrypted);

        std::fs::write(path, &file_data).map_err(|e| WalletError::IoError(e.to_string()))
    }

    /// Load a wallet from an encrypted file.
    pub fn load_from_file(path: &Path, password: &[u8]) -> Result<Self, WalletError> {
        let file_data =
            std::fs::read(path).map_err(|e| WalletError::IoError(e.to_string()))?;

        if file_data.len() < 4 {
            return Err(WalletError::CorruptedFile("file too short".into()));
        }

        let header_len =
            u32::from_le_bytes(file_data[..4].try_into().unwrap()) as usize;
        if file_data.len() < 4 + header_len {
            return Err(WalletError::CorruptedFile("header truncated".into()));
        }

        let header_json = &file_data[4..4 + header_len];
        let header: WalletFileHeader = serde_json::from_slice(header_json)
            .map_err(|e| WalletError::CorruptedFile(format!("invalid header: {e}")))?;

        if header.magic != String::from_utf8_lossy(WALLET_MAGIC).as_ref() {
            return Err(WalletError::CorruptedFile("invalid magic bytes".into()));
        }
        if header.version != WALLET_VERSION {
            return Err(WalletError::CorruptedFile(format!(
                "unsupported version: {}",
                header.version
            )));
        }

        let encrypted = &file_data[4 + header_len..];
        let payload_json = encryption::decrypt(encrypted, password)?;

        let kc_data: KeyChainData = serde_json::from_slice(&payload_json)
            .map_err(|e| WalletError::CorruptedFile(format!("invalid payload: {e}")))?;

        let keychain = kc_data.to_keychain();

        // Rebuild owned pubkey hashes from restored keychain
        let mut owned = HashSet::new();
        for pkh in keychain.known_pubkey_hashes() {
            owned.insert(*pkh);
        }

        Ok(Self {
            keychain,
            utxos: HashMap::new(),
            owned_pubkey_hashes: owned,
        })
    }
}

impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wallet")
            .field("network", &self.keychain.network())
            .field("addresses", &self.keychain.next_index())
            .field("utxos", &self.utxos.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rill_core::constants;
    use rill_core::error::{DecayError, RillError, TransactionError};
    use rill_core::types::{Block, BlockHeader, TxOutput};
    use std::collections::HashMap as StdHashMap;

    // --- Mocks ---

    struct MockChainState {
        supply: u64,
        clusters: StdHashMap<Hash256, u64>,
    }

    impl MockChainState {
        fn new(supply: u64) -> Self {
            Self {
                supply,
                clusters: StdHashMap::new(),
            }
        }
    }

    impl ChainState for MockChainState {
        fn get_utxo(&self, _: &OutPoint) -> Result<Option<UtxoEntry>, RillError> {
            Ok(None)
        }
        fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
            Ok((100, Hash256::ZERO))
        }
        fn get_block_header(&self, _: &Hash256) -> Result<Option<BlockHeader>, RillError> {
            Ok(None)
        }
        fn get_block(&self, _: &Hash256) -> Result<Option<Block>, RillError> {
            Ok(None)
        }
        fn get_block_hash(&self, _: u64) -> Result<Option<Hash256>, RillError> {
            Ok(None)
        }
        fn circulating_supply(&self) -> Result<u64, RillError> {
            Ok(self.supply)
        }
        fn cluster_balance(&self, cluster_id: &Hash256) -> Result<u64, RillError> {
            Ok(*self.clusters.get(cluster_id).unwrap_or(&0))
        }
        fn decay_pool_balance(&self) -> Result<u64, RillError> {
            Ok(0)
        }
        fn validate_transaction(&self, _: &Transaction) -> Result<(), TransactionError> {
            Ok(())
        }
    }

    struct MockDecayCalculator;

    impl DecayCalculator for MockDecayCalculator {
        fn decay_rate_ppb(&self, concentration_ppb: u64) -> Result<u64, DecayError> {
            if concentration_ppb > constants::DECAY_C_THRESHOLD_PPB {
                Ok(10_000_000)
            } else {
                Ok(0)
            }
        }

        fn compute_decay(
            &self,
            nominal_value: u64,
            concentration_ppb: u64,
            blocks_held: u64,
        ) -> Result<u64, DecayError> {
            let rate = self.decay_rate_ppb(concentration_ppb)?;
            let per_block = nominal_value
                .checked_mul(rate)
                .and_then(|v| v.checked_div(constants::DECAY_PRECISION))
                .ok_or(DecayError::ArithmeticOverflow)?;
            per_block
                .checked_mul(blocks_held)
                .ok_or(DecayError::ArithmeticOverflow)
        }

        fn decay_pool_release(&self, pool_balance: u64) -> Result<u64, DecayError> {
            Ok(pool_balance * constants::DECAY_POOL_RELEASE_BPS / constants::BPS_PRECISION)
        }
    }

    fn make_utxo_for_wallet(pubkey_hash: Hash256, value: u64, height: u64) -> (OutPoint, UtxoEntry) {
        use rand::RngCore;
        let mut txid = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut txid);
        let outpoint = OutPoint {
            txid: Hash256(txid),
            index: 0,
        };
        let entry = UtxoEntry {
            output: TxOutput {
                value,
                pubkey_hash,
            },
            block_height: height,
            is_coinbase: false,
            cluster_id: Hash256::ZERO,
        };
        (outpoint, entry)
    }

    #[test]
    fn create_wallet() {
        let w = Wallet::create(Network::Testnet);
        assert_eq!(w.network(), Network::Testnet);
        assert_eq!(w.utxo_count(), 0);
        assert_eq!(w.address_count(), 0);
    }

    #[test]
    fn from_seed_deterministic() {
        let seed1 = Seed::from_bytes([1u8; 32]);
        let seed2 = Seed::from_bytes([1u8; 32]);
        let mut w1 = Wallet::from_seed(seed1, Network::Mainnet);
        let mut w2 = Wallet::from_seed(seed2, Network::Mainnet);

        let a1 = w1.next_address();
        let a2 = w2.next_address();
        assert_eq!(a1, a2);
    }

    #[test]
    fn next_address_unique() {
        let mut w = Wallet::create(Network::Testnet);
        let a0 = w.next_address();
        let a1 = w.next_address();
        assert_ne!(a0, a1);
        assert_eq!(w.address_count(), 2);
    }

    #[test]
    fn scan_utxos_finds_owned() {
        let mut w = Wallet::create(Network::Testnet);
        let addr = w.next_address();
        let pkh = addr.pubkey_hash();

        let utxo_set = vec![
            make_utxo_for_wallet(pkh, 10 * COIN, 50),
            make_utxo_for_wallet(Hash256([0xFF; 32]), 20 * COIN, 50), // not ours
        ];

        w.scan_utxos(&utxo_set);
        assert_eq!(w.utxo_count(), 1);
    }

    #[test]
    fn scan_utxos_multiple_addresses() {
        let mut w = Wallet::create(Network::Testnet);
        let addr0 = w.next_address();
        let addr1 = w.next_address();

        let utxo_set = vec![
            make_utxo_for_wallet(addr0.pubkey_hash(), 5 * COIN, 50),
            make_utxo_for_wallet(addr1.pubkey_hash(), 3 * COIN, 50),
            make_utxo_for_wallet(Hash256([0xFF; 32]), 100 * COIN, 50),
        ];

        w.scan_utxos(&utxo_set);
        assert_eq!(w.utxo_count(), 2);
    }

    #[test]
    fn balance_no_utxos() {
        let w = Wallet::create(Network::Testnet);
        let cs = MockChainState::new(1_000_000 * COIN);
        let dc = MockDecayCalculator;

        let bal = w.balance(&dc, &cs, 100).unwrap();
        assert_eq!(bal.nominal, 0);
        assert_eq!(bal.effective, 0);
        assert_eq!(bal.decay, 0);
        assert_eq!(bal.utxo_count, 0);
    }

    #[test]
    fn balance_with_utxos_no_decay() {
        let mut w = Wallet::create(Network::Testnet);
        let addr = w.next_address();

        let utxo_set = vec![
            make_utxo_for_wallet(addr.pubkey_hash(), 10 * COIN, 50),
        ];
        w.scan_utxos(&utxo_set);

        let cs = MockChainState::new(1_000_000 * COIN);
        let dc = MockDecayCalculator;

        let bal = w.balance(&dc, &cs, 100).unwrap();
        assert_eq!(bal.nominal, 10 * COIN);
        assert_eq!(bal.effective, 10 * COIN);
        assert_eq!(bal.decay, 0);
        assert_eq!(bal.utxo_count, 1);
    }

    #[test]
    fn balance_display_helpers() {
        let bal = WalletBalance {
            nominal: 5 * COIN,
            effective: 4 * COIN,
            decay: 1 * COIN,
            utxo_count: 2,
        };
        assert!((bal.nominal_rill() - 5.0).abs() < f64::EPSILON);
        assert!((bal.effective_rill() - 4.0).abs() < f64::EPSILON);
        assert!((bal.decay_rill() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn send_transaction() {
        let mut w = Wallet::from_seed(Seed::from_bytes([1u8; 32]), Network::Testnet);
        // Pre-derive addresses and add UTXOs
        let addr0 = w.next_address();
        let addr1 = w.next_address();
        let utxo_set = vec![
            make_utxo_for_wallet(addr0.pubkey_hash(), 10 * COIN, 50),
            make_utxo_for_wallet(addr1.pubkey_hash(), 10 * COIN, 50),
        ];
        w.scan_utxos(&utxo_set);

        let cs = MockChainState::new(1_000_000 * COIN);
        let dc = MockDecayCalculator;

        let recipient = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);
        let tx = w.send(&[(recipient, 5 * COIN)], &dc, &cs, 100).unwrap();

        assert!(!tx.inputs.is_empty());
        assert!(!tx.outputs.is_empty());
        // First output is the payment
        assert_eq!(tx.outputs[0].value, 5 * COIN);
    }

    #[test]
    fn send_no_recipients_fails() {
        let mut w = Wallet::create(Network::Testnet);
        let cs = MockChainState::new(1_000_000 * COIN);
        let dc = MockDecayCalculator;

        let err = w.send(&[], &dc, &cs, 100).unwrap_err();
        assert!(matches!(err, WalletError::BuildError(_)));
    }

    #[test]
    fn send_insufficient_funds_fails() {
        let mut w = Wallet::from_seed(Seed::from_bytes([2u8; 32]), Network::Testnet);
        let addr = w.next_address();
        let utxo_set = vec![
            make_utxo_for_wallet(addr.pubkey_hash(), 1 * COIN, 50),
        ];
        w.scan_utxos(&utxo_set);

        let cs = MockChainState::new(1_000_000 * COIN);
        let dc = MockDecayCalculator;

        let recipient = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);
        let err = w.send(&[(recipient, 999 * COIN)], &dc, &cs, 100).unwrap_err();
        assert!(matches!(err, WalletError::InsufficientFunds { .. }));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.wallet");
        let password = b"test-password";

        // Create wallet, derive some addresses
        let mut w = Wallet::from_seed(Seed::from_bytes([3u8; 32]), Network::Testnet);
        let addr0 = w.next_address();
        let addr1 = w.next_address();

        w.save_to_file(&path, password).unwrap();

        // Load it back
        let loaded = Wallet::load_from_file(&path, password).unwrap();
        assert_eq!(loaded.network(), Network::Testnet);
        assert_eq!(loaded.address_count(), 2);

        // Pubkey hashes should match
        assert!(loaded.owned_pubkey_hashes.contains(&addr0.pubkey_hash()));
        assert!(loaded.owned_pubkey_hashes.contains(&addr1.pubkey_hash()));
    }

    #[test]
    fn load_wrong_password_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.wallet");

        let w = Wallet::from_seed(Seed::from_bytes([4u8; 32]), Network::Testnet);
        w.save_to_file(&path, b"correct").unwrap();

        let err = Wallet::load_from_file(&path, b"wrong").unwrap_err();
        assert_eq!(err, WalletError::InvalidPassword);
    }

    #[test]
    fn load_corrupted_file_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.wallet");

        std::fs::write(&path, b"garbage").unwrap();

        let err = Wallet::load_from_file(&path, b"pass").unwrap_err();
        assert!(matches!(err, WalletError::CorruptedFile(_)));
    }

    #[test]
    fn load_truncated_header_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.wallet");

        // Write just 2 bytes (not enough for header length)
        std::fs::write(&path, &[0u8; 2]).unwrap();

        let err = Wallet::load_from_file(&path, b"pass").unwrap_err();
        assert!(matches!(err, WalletError::CorruptedFile(_)));
    }

    #[test]
    fn load_nonexistent_file_fails() {
        let path = Path::new("/tmp/nonexistent_rill_wallet_test_file");
        let err = Wallet::load_from_file(path, b"pass").unwrap_err();
        assert!(matches!(err, WalletError::IoError(_)));
    }

    #[test]
    fn wallet_debug_format() {
        let w = Wallet::create(Network::Mainnet);
        let debug = format!("{w:?}");
        assert!(debug.contains("Wallet"));
        assert!(debug.contains("Mainnet"));
    }

    #[test]
    fn save_load_preserves_next_address_determinism() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.wallet");
        let password = b"password";

        let mut w = Wallet::from_seed(Seed::from_bytes([5u8; 32]), Network::Testnet);
        w.next_address();
        w.next_address();
        w.save_to_file(&path, password).unwrap();

        let mut loaded = Wallet::load_from_file(&path, password).unwrap();
        // The next address from loaded wallet should be at index 2
        let addr_loaded = loaded.next_address();

        // Create a fresh wallet from same seed, derive 3 addresses
        let mut fresh = Wallet::from_seed(Seed::from_bytes([5u8; 32]), Network::Testnet);
        fresh.next_address();
        fresh.next_address();
        let addr_fresh = fresh.next_address();

        assert_eq!(addr_loaded, addr_fresh);
    }
}
