//! Transaction builder with decay-aware coin selection and signing.
//!
//! Provides a builder pattern for constructing transactions:
//! 1. Add recipients (address + amount)
//! 2. Build an unsigned transaction (performs coin selection)
//! 3. Sign all inputs with the wallet's keychain

use rill_core::address::Address;
use rill_core::crypto::sign_transaction_input;
use rill_core::traits::{ChainState, DecayCalculator};
use rill_core::types::{Hash256, OutPoint, Transaction, TxInput, TxOutput, UtxoEntry};

use crate::coin_selection::{CoinSelection, CoinSelector};
use crate::error::WalletError;
use crate::keys::KeyChain;

/// Default base fee in rills (fixed per transaction).
pub const DEFAULT_BASE_FEE: u64 = 1_000;

/// Default additional fee per input in rills.
pub const DEFAULT_FEE_PER_INPUT: u64 = 500;

/// A transaction recipient: address and amount.
#[derive(Debug, Clone)]
pub struct Recipient {
    /// Destination address.
    pub address: Address,
    /// Amount in rills.
    pub amount: u64,
}

/// An unsigned transaction ready for signing.
#[derive(Debug)]
pub struct UnsignedTransaction {
    /// The transaction with empty signatures.
    pub tx: Transaction,
    /// The coin selection result used to build this transaction.
    pub selection: CoinSelection,
    /// Pubkey hashes for each input (for signing key lookup).
    pub input_pubkey_hashes: Vec<Hash256>,
}

/// Builder for constructing and signing transactions.
///
/// # Example
/// ```ignore
/// let tx = TransactionBuilder::new()
///     .add_recipient(address, 5 * COIN)
///     .build(utxos, change_addr, &decay_calc, &chain_state, height)?;
/// let signed = TransactionBuilder::sign(tx, &keychain)?;
/// ```
pub struct TransactionBuilder {
    recipients: Vec<Recipient>,
    base_fee: u64,
    fee_per_input: u64,
    lock_time: u64,
}

impl TransactionBuilder {
    /// Create a new transaction builder with default fees.
    pub fn new() -> Self {
        Self {
            recipients: Vec::new(),
            base_fee: DEFAULT_BASE_FEE,
            fee_per_input: DEFAULT_FEE_PER_INPUT,
            lock_time: 0,
        }
    }

    /// Add a recipient to the transaction.
    pub fn add_recipient(&mut self, address: Address, amount: u64) -> &mut Self {
        self.recipients.push(Recipient { address, amount });
        self
    }

    /// Override the base fee (default: [`DEFAULT_BASE_FEE`]).
    pub fn set_base_fee(&mut self, fee: u64) -> &mut Self {
        self.base_fee = fee;
        self
    }

    /// Override the per-input fee (default: [`DEFAULT_FEE_PER_INPUT`]).
    pub fn set_fee_per_input(&mut self, fee: u64) -> &mut Self {
        self.fee_per_input = fee;
        self
    }

    /// Set the transaction lock time.
    pub fn set_lock_time(&mut self, lock_time: u64) -> &mut Self {
        self.lock_time = lock_time;
        self
    }

    /// Build an unsigned transaction by selecting coins and constructing outputs.
    ///
    /// # Arguments
    /// - `wallet_utxos` — available UTXOs owned by the wallet
    /// - `change_address` — address to receive change
    /// - `decay_calc` — decay calculator for effective value computation
    /// - `chain_state` — chain state for cluster balance and supply lookups
    /// - `height` — current block height
    pub fn build(
        &self,
        wallet_utxos: &[(OutPoint, UtxoEntry)],
        change_address: &Address,
        decay_calc: &dyn DecayCalculator,
        chain_state: &dyn ChainState,
        height: u64,
    ) -> Result<UnsignedTransaction, WalletError> {
        if self.recipients.is_empty() {
            return Err(WalletError::BuildError("no recipients".into()));
        }

        // Validate recipients
        let mut total_send: u64 = 0;
        for r in &self.recipients {
            if r.amount == 0 {
                return Err(WalletError::InvalidAmount("recipient amount is zero".into()));
            }
            total_send = total_send
                .checked_add(r.amount)
                .ok_or_else(|| WalletError::InvalidAmount("total amount overflow".into()))?;
        }

        // Coin selection
        let selection = CoinSelector::select(
            wallet_utxos,
            total_send,
            self.base_fee,
            self.fee_per_input,
            decay_calc,
            chain_state,
            height,
        )?;

        // Build inputs
        let mut inputs = Vec::with_capacity(selection.selected.len());
        let mut input_pubkey_hashes = Vec::with_capacity(selection.selected.len());
        for utxo in &selection.selected {
            inputs.push(TxInput {
                previous_output: utxo.outpoint.clone(),
                signature: vec![],
                public_key: vec![],
            });
            input_pubkey_hashes.push(utxo.entry.output.pubkey_hash);
        }

        // Build outputs: recipients + optional change
        let mut outputs = Vec::with_capacity(self.recipients.len() + 1);
        for r in &self.recipients {
            outputs.push(TxOutput {
                value: r.amount,
                pubkey_hash: r.address.pubkey_hash(),
            });
        }

        if selection.change > 0 {
            outputs.push(TxOutput {
                value: selection.change,
                pubkey_hash: change_address.pubkey_hash(),
            });
        }

        let tx = Transaction {
            version: 1,
            inputs,
            outputs,
            lock_time: self.lock_time,
        };

        Ok(UnsignedTransaction {
            tx,
            selection,
            input_pubkey_hashes,
        })
    }

    /// Sign all inputs of an unsigned transaction using the keychain.
    ///
    /// Looks up each input's signing key by pubkey hash. Returns an error
    /// if any required key is not found in the keychain.
    pub fn sign(
        unsigned: UnsignedTransaction,
        keychain: &KeyChain,
    ) -> Result<Transaction, WalletError> {
        let mut tx = unsigned.tx;

        for (i, pkh) in unsigned.input_pubkey_hashes.iter().enumerate() {
            let kp = keychain
                .keypair_for_pubkey_hash(pkh)
                .ok_or_else(|| WalletError::KeyNotFound(format!("pubkey hash {pkh}")))?;

            sign_transaction_input(&mut tx, i, kp)?;
        }

        Ok(tx)
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rill_core::address::Network;
    use rill_core::constants;
    use rill_core::crypto::verify_transaction_input;
    use rill_core::error::{DecayError, RillError, TransactionError};
    use rill_core::types::{Block, BlockHeader};
    use std::collections::HashMap;

    use crate::keys::{KeyChain, Seed};

    // --- Mocks (same as coin_selection tests) ---

    struct MockChainState {
        utxos: HashMap<OutPoint, UtxoEntry>,
        supply: u64,
        clusters: HashMap<Hash256, u64>,
    }

    impl MockChainState {
        fn new(supply: u64) -> Self {
            Self {
                utxos: HashMap::new(),
                supply,
                clusters: HashMap::new(),
            }
        }
    }

    impl ChainState for MockChainState {
        fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UtxoEntry>, RillError> {
            Ok(self.utxos.get(outpoint).cloned())
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

    fn setup_wallet_utxos(keychain: &mut KeyChain) -> Vec<(OutPoint, UtxoEntry)> {
        let mut utxos = Vec::new();
        for i in 0..3 {
            let pk = keychain.derive_keypair(i).public_key();
            let pkh = pk.pubkey_hash();
            let outpoint = OutPoint {
                txid: Hash256([i as u8 + 1; 32]),
                index: 0,
            };
            let entry = UtxoEntry {
                output: TxOutput {
                    value: 10 * constants::COIN,
                    pubkey_hash: pkh,
                },
                block_height: 50,
                is_coinbase: false,
                cluster_id: Hash256::ZERO,
            };
            utxos.push((outpoint, entry));
        }
        utxos
    }

    #[test]
    fn build_single_recipient() {
        let seed = Seed::from_bytes([1u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);
        let recipient_addr = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        let mut builder = TransactionBuilder::new();
        builder.add_recipient(recipient_addr, 5 * constants::COIN);
        let unsigned = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap();

        assert!(!unsigned.tx.inputs.is_empty());
        assert!(unsigned.tx.outputs.len() >= 1);
        // First output is the recipient
        assert_eq!(unsigned.tx.outputs[0].value, 5 * constants::COIN);
        assert_eq!(unsigned.tx.outputs[0].pubkey_hash, Hash256([0xAA; 32]));
    }

    #[test]
    fn build_multi_recipient() {
        let seed = Seed::from_bytes([2u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);
        let addr1 = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);
        let addr2 = Address::from_pubkey_hash(Hash256([0xBB; 32]), Network::Testnet);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        let mut builder = TransactionBuilder::new();
        builder.add_recipient(addr1, 3 * constants::COIN);
        builder.add_recipient(addr2, 2 * constants::COIN);
        let unsigned = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap();

        assert_eq!(unsigned.tx.outputs[0].value, 3 * constants::COIN);
        assert_eq!(unsigned.tx.outputs[1].value, 2 * constants::COIN);
    }

    #[test]
    fn build_with_change() {
        let seed = Seed::from_bytes([3u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);
        let recipient = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        let mut builder = TransactionBuilder::new();
        builder.add_recipient(recipient, 1 * constants::COIN);
        let unsigned = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap();

        // Should have a change output
        assert!(unsigned.tx.outputs.len() >= 2);
        let change_output = unsigned.tx.outputs.last().unwrap();
        assert_eq!(change_output.pubkey_hash, change_addr.pubkey_hash());
        assert!(change_output.value > 0);
    }

    #[test]
    fn build_no_recipients_fails() {
        let seed = Seed::from_bytes([4u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        let builder = TransactionBuilder::new();
        let err = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap_err();
        assert!(matches!(err, WalletError::BuildError(_)));
    }

    #[test]
    fn build_zero_amount_fails() {
        let seed = Seed::from_bytes([5u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);
        let recipient = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        let mut builder = TransactionBuilder::new();
        builder.add_recipient(recipient, 0);
        let err = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap_err();
        assert!(matches!(err, WalletError::InvalidAmount(_)));
    }

    #[test]
    fn build_custom_fee() {
        let seed = Seed::from_bytes([6u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);
        let recipient = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        let mut builder = TransactionBuilder::new();
        builder
            .add_recipient(recipient, 1 * constants::COIN)
            .set_base_fee(5000)
            .set_fee_per_input(2000);
        let unsigned = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap();

        assert_eq!(
            unsigned.selection.fee,
            5000 + 2000 * unsigned.selection.selected.len() as u64
        );
    }

    #[test]
    fn build_with_lock_time() {
        let seed = Seed::from_bytes([7u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);
        let recipient = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        let mut builder = TransactionBuilder::new();
        builder
            .add_recipient(recipient, 1 * constants::COIN)
            .set_lock_time(500);
        let unsigned = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap();
        assert_eq!(unsigned.tx.lock_time, 500);
    }

    #[test]
    fn sign_single_input() {
        let seed = Seed::from_bytes([8u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);
        let recipient = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        let mut builder = TransactionBuilder::new();
        builder.add_recipient(recipient, 5 * constants::COIN);
        let unsigned = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap();
        let pkh = unsigned.input_pubkey_hashes.clone();

        let signed = TransactionBuilder::sign(unsigned, &kc).unwrap();

        // Every input should have valid signature and pubkey
        for (i, hash) in pkh.iter().enumerate() {
            assert_eq!(signed.inputs[i].signature.len(), 64);
            assert_eq!(signed.inputs[i].public_key.len(), 32);
            assert!(verify_transaction_input(&signed, i, hash).is_ok());
        }
    }

    #[test]
    fn sign_multi_input() {
        let seed = Seed::from_bytes([9u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);
        let recipient = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        // Need enough to require multiple inputs
        let mut builder = TransactionBuilder::new();
        builder.add_recipient(recipient, 25 * constants::COIN);
        let unsigned = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap();
        assert!(unsigned.tx.inputs.len() >= 2);

        let pkh = unsigned.input_pubkey_hashes.clone();
        let signed = TransactionBuilder::sign(unsigned, &kc).unwrap();

        for (i, hash) in pkh.iter().enumerate() {
            assert!(verify_transaction_input(&signed, i, hash).is_ok());
        }
    }

    #[test]
    fn sign_missing_key_fails() {
        let seed = Seed::from_bytes([10u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);
        let recipient = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        let mut builder = TransactionBuilder::new();
        builder.add_recipient(recipient, 5 * constants::COIN);
        let mut unsigned = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap();

        // Replace a pubkey hash with an unknown one
        unsigned.input_pubkey_hashes[0] = Hash256([0xFF; 32]);

        // Create a fresh keychain without the unknown key
        let fresh_kc = KeyChain::new(Seed::from_bytes([99u8; 32]), Network::Testnet);
        let err = TransactionBuilder::sign(unsigned, &fresh_kc).unwrap_err();
        assert!(matches!(err, WalletError::KeyNotFound(_)));
    }

    #[test]
    fn builder_default() {
        let builder = TransactionBuilder::default();
        assert_eq!(builder.base_fee, DEFAULT_BASE_FEE);
        assert_eq!(builder.fee_per_input, DEFAULT_FEE_PER_INPUT);
        assert_eq!(builder.lock_time, 0);
    }

    #[test]
    fn build_insufficient_funds() {
        let seed = Seed::from_bytes([11u8; 32]);
        let mut kc = KeyChain::new(seed, Network::Testnet);
        let utxos = setup_wallet_utxos(&mut kc);
        let change_addr = kc.address_at(10);
        let recipient = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Testnet);

        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;

        let mut builder = TransactionBuilder::new();
        builder.add_recipient(recipient, 999 * constants::COIN);
        let err = builder.build(&utxos, &change_addr, &dc, &cs, 100).unwrap_err();
        assert!(matches!(err, WalletError::InsufficientFunds { .. }));
    }
}
