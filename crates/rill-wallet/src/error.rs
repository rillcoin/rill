//! Wallet error types.

use rill_core::error::{CryptoError, DecayError, TransactionError};
use thiserror::Error;

/// Errors that can occur in wallet operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum WalletError {
    /// Insufficient funds to cover the transaction amount plus fees.
    #[error("insufficient funds: have {have}, need {need}")]
    InsufficientFunds {
        /// Available balance in rills.
        have: u64,
        /// Required amount in rills.
        need: u64,
    },

    /// No UTXOs available for spending.
    #[error("no UTXOs available")]
    NoUtxos,

    /// Invalid monetary amount.
    #[error("invalid amount: {0}")]
    InvalidAmount(String),

    /// Invalid address string.
    #[error("invalid address: {0}")]
    InvalidAddress(String),

    /// Key derivation failure.
    #[error("key derivation: {0}")]
    KeyDerivation(String),

    /// Encryption failure.
    #[error("encryption: {0}")]
    Encryption(String),

    /// Decryption failure.
    #[error("decryption: {0}")]
    Decryption(String),

    /// Wrong password for wallet file.
    #[error("invalid password")]
    InvalidPassword,

    /// Wallet file is corrupted or has invalid format.
    #[error("corrupted file: {0}")]
    CorruptedFile(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(String),

    /// Required key not found in keychain.
    #[error("key not found: {0}")]
    KeyNotFound(String),

    /// Transaction build error.
    #[error("build error: {0}")]
    BuildError(String),

    /// Cryptographic error from rill-core.
    #[error(transparent)]
    Crypto(#[from] CryptoError),

    /// Transaction validation error from rill-core.
    #[error(transparent)]
    Transaction(#[from] TransactionError),

    /// Decay computation error from rill-core.
    #[error(transparent)]
    Decay(#[from] DecayError),

    /// Serialization error.
    #[error("serialization: {0}")]
    Serialization(String),

    /// Invalid BIP-39 mnemonic phrase.
    #[error("invalid mnemonic: {0}")]
    InvalidMnemonic(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_insufficient_funds() {
        let e = WalletError::InsufficientFunds {
            have: 100,
            need: 200,
        };
        assert_eq!(e.to_string(), "insufficient funds: have 100, need 200");
    }

    #[test]
    fn display_no_utxos() {
        let e = WalletError::NoUtxos;
        assert_eq!(e.to_string(), "no UTXOs available");
    }

    #[test]
    fn display_invalid_password() {
        let e = WalletError::InvalidPassword;
        assert_eq!(e.to_string(), "invalid password");
    }

    #[test]
    fn clone_and_eq() {
        let e1 = WalletError::InvalidAmount("zero".into());
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }

    #[test]
    fn from_crypto_error() {
        let crypto = CryptoError::InvalidPublicKey;
        let wallet: WalletError = crypto.into();
        assert_eq!(wallet, WalletError::Crypto(CryptoError::InvalidPublicKey));
    }

    #[test]
    fn from_transaction_error() {
        let tx = TransactionError::EmptyInputsOrOutputs;
        let wallet: WalletError = tx.into();
        assert_eq!(
            wallet,
            WalletError::Transaction(TransactionError::EmptyInputsOrOutputs)
        );
    }

    #[test]
    fn from_decay_error() {
        let decay = DecayError::ArithmeticOverflow;
        let wallet: WalletError = decay.into();
        assert_eq!(
            wallet,
            WalletError::Decay(DecayError::ArithmeticOverflow)
        );
    }
}
