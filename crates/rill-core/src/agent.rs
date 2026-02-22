//! Proof of Conduct agent wallet types.
//!
//! Agent wallets participate in the conduct scoring system, which adjusts
//! their decay multiplier based on on-chain behavior. Phase 1 uses static
//! defaults; dynamic scoring comes in Phase 2.

use serde::{Deserialize, Serialize};

use crate::conduct::VelocityBaseline;
use crate::types::{Hash256, WalletType};

// ---------------------------------------------------------------------------
// Contract types
// ---------------------------------------------------------------------------

/// Status of an agent-to-agent contract.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
    bincode::Encode, bincode::Decode,
)]
pub enum ContractStatus {
    /// Contract is open and awaiting fulfilment.
    Open,
    /// Contract was fulfilled by both parties.
    Fulfilled,
    /// Contract expired without fulfilment.
    Expired,
    /// Contract was disputed by one or both parties.
    Disputed,
}

/// A lightweight agent-to-agent contract tracked at L1.
///
/// Created by a `ContractCreate` transaction and stored in `CF_AGENT_CONTRACTS`.
/// The contract ID is the txid of the creating transaction.
///
/// # Examples
///
/// ```
/// use rill_core::agent::{AgentContract, ContractStatus};
/// use rill_core::types::Hash256;
///
/// let contract = AgentContract {
///     contract_id: Hash256([0xAA; 32]),
///     initiator: Hash256([0x11; 32]),
///     counterparty: Hash256([0x22; 32]),
///     created_at_block: 100,
///     expires_at_block: 200,
///     value: 50_00000000,
///     status: ContractStatus::Open,
/// };
/// assert_eq!(contract.status, ContractStatus::Open);
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize,
    bincode::Encode, bincode::Decode,
)]
pub struct AgentContract {
    /// Unique contract identifier (hash of the creating transaction).
    pub contract_id: Hash256,
    /// Pubkey hash of the initiating agent.
    pub initiator: Hash256,
    /// Pubkey hash of the counterparty agent.
    pub counterparty: Hash256,
    /// Block height at which the contract was created.
    pub created_at_block: u64,
    /// Block height at which the contract expires if not fulfilled.
    pub expires_at_block: u64,
    /// Contract value in rills.
    pub value: u64,
    /// Current status.
    pub status: ContractStatus,
}

/// A peer review submitted by one agent for another after contract completion.
///
/// Reviews are submitted via `PeerReview` transactions and contribute to the
/// subject agent's `peer_review` conduct signal.
///
/// # Examples
///
/// ```
/// use rill_core::agent::PeerReview;
/// use rill_core::types::Hash256;
///
/// let review = PeerReview {
///     reviewer: Hash256([0x11; 32]),
///     subject: Hash256([0x22; 32]),
///     score: 8,
///     block_height: 500,
///     contract_id: Hash256([0xAA; 32]),
/// };
/// assert!(review.score >= 1 && review.score <= 10);
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize,
    bincode::Encode, bincode::Decode,
)]
pub struct PeerReview {
    /// Pubkey hash of the reviewer.
    pub reviewer: Hash256,
    /// Pubkey hash of the reviewed agent.
    pub subject: Hash256,
    /// Score from 1 to 10.
    pub score: u8,
    /// Block height at which the review was submitted.
    pub block_height: u64,
    /// Contract this review relates to.
    pub contract_id: Hash256,
}

/// Persistent state for a registered agent wallet.
///
/// Stored in the `CF_AGENT_WALLETS` column family, keyed by `pubkey_hash`.
/// Phase 1: `conduct_score` stays at [`CONDUCT_SCORE_DEFAULT`](crate::constants::CONDUCT_SCORE_DEFAULT)
/// and `conduct_multiplier_bps` stays at [`CONDUCT_MULTIPLIER_DEFAULT_BPS`](crate::constants::CONDUCT_MULTIPLIER_DEFAULT_BPS).
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, bincode::Encode, bincode::Decode,
)]
pub struct AgentWalletState {
    /// BLAKE3 hash of the agent's Ed25519 public key.
    pub pubkey_hash: Hash256,
    /// Block height at which the agent was registered.
    pub registered_at_block: u64,
    /// Staked balance locked during registration.
    pub stake_balance: u64,
    /// Block height until which the stake is locked.
    pub stake_locked_until: u64,
    /// Current conduct score (0–1000).
    pub conduct_score: u16,
    /// Current decay multiplier in basis points (10,000 = 1.0×).
    pub conduct_multiplier_bps: u64,
    /// Whether the Undertow penalty is active.
    pub undertow_active: bool,
    /// Block height at which the Undertow penalty expires.
    pub undertow_expires_at: u64,
    /// Rolling velocity statistics for Undertow detection and scoring.
    pub velocity_baseline: VelocityBaseline,
    /// Pubkey hashes of agents this wallet vouches for (max `MAX_VOUCH_TARGETS`).
    pub vouch_targets: Vec<Hash256>,
    /// Pubkey hashes of agents vouching for this wallet (max `MAX_VOUCHERS`).
    pub vouchers: Vec<Hash256>,
    /// Number of contracts initiated or participated in (last 90 epochs).
    pub contracts_total: u64,
    /// Number of contracts fulfilled (last 90 epochs).
    pub contracts_fulfilled: u64,
    /// Number of contracts disputed (last 90 epochs).
    pub contracts_disputed: u64,
    /// Sum of peer review scores received (last 90 epochs).
    pub peer_review_sum: u64,
    /// Count of peer reviews received (last 90 epochs).
    pub peer_review_count: u64,
}

/// RPC response type summarizing an agent's conduct profile.
///
/// Combines on-chain agent state with derived values like effective decay
/// rate and wallet age for client display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConductProfile {
    /// BLAKE3 hash of the agent's Ed25519 public key.
    pub pubkey_hash: Hash256,
    /// Whether this is a Standard or Agent wallet.
    pub wallet_type: WalletType,
    /// Current conduct score (0–1000).
    pub conduct_score: u16,
    /// Current decay multiplier in basis points (10,000 = 1.0×).
    pub conduct_multiplier_bps: u64,
    /// Effective decay rate in parts-per-billion at current concentration.
    pub effective_decay_rate_ppb: u64,
    /// Whether the Undertow penalty is active.
    pub undertow_active: bool,
    /// Block height at which the agent was registered.
    pub registered_at_block: u64,
    /// Number of blocks since registration.
    pub wallet_age_blocks: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{CONDUCT_MULTIPLIER_DEFAULT_BPS, CONDUCT_SCORE_DEFAULT};

    #[test]
    fn agent_wallet_state_roundtrip() {
        let state = AgentWalletState {
            pubkey_hash: Hash256([0xAA; 32]),
            registered_at_block: 100,
            stake_balance: 50_00000000,
            stake_locked_until: 1540,
            conduct_score: CONDUCT_SCORE_DEFAULT,
            conduct_multiplier_bps: CONDUCT_MULTIPLIER_DEFAULT_BPS,
            undertow_active: false,
            undertow_expires_at: 0,
            velocity_baseline: VelocityBaseline::new(),
            vouch_targets: Vec::new(),
            vouchers: Vec::new(),
            contracts_total: 0,
            contracts_fulfilled: 0,
            contracts_disputed: 0,
            peer_review_sum: 0,
            peer_review_count: 0,
        };
        let encoded = bincode::encode_to_vec(&state, bincode::config::standard()).unwrap();
        let (decoded, _): (AgentWalletState, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
        assert_eq!(state, decoded);
    }

    #[test]
    fn conduct_profile_serde_roundtrip() {
        let profile = ConductProfile {
            pubkey_hash: Hash256([0xBB; 32]),
            wallet_type: WalletType::Agent,
            conduct_score: 500,
            conduct_multiplier_bps: 15_000,
            effective_decay_rate_ppb: 750_000_000,
            undertow_active: false,
            registered_at_block: 42,
            wallet_age_blocks: 1000,
        };
        let json = serde_json::to_string(&profile).unwrap();
        let decoded: ConductProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, decoded);
    }

    #[test]
    fn wallet_type_default_is_standard() {
        assert_eq!(WalletType::default(), WalletType::Standard);
    }
}
