//! # rill-wallet — HD wallet with decay-aware coin selection.
//!
//! Provides deterministic key derivation from a master seed, decay-aware
//! coin selection that spends high-decay UTXOs first, transaction building
//! and signing, and encrypted wallet file persistence.
//!
//! # Modules
//!
//! - [`error`] — `WalletError` enum
//! - [`keys`] — Seed, KeyChain, BLAKE3-based key derivation
//! - [`coin_selection`] — Decay-aware UTXO selection
//! - [`encryption`] — AES-256-GCM wallet file encryption
//! - [`builder`] — Transaction builder with signing
//! - [`wallet`] — High-level wallet composition

pub mod builder;
pub mod coin_selection;
pub mod encryption;
pub mod error;
pub mod keys;
pub mod wallet;

// Re-exports for convenient access
pub use builder::{Recipient, TransactionBuilder, UnsignedTransaction};
pub use coin_selection::{CoinSelection, CoinSelector, WalletUtxo};
pub use encryption::{decrypt, encrypt};
pub use error::WalletError;
pub use keys::{KeyChain, KeyChainData, Seed};
pub use wallet::{Wallet, WalletBalance};
