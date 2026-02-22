//! Proof of Conduct agent wallet types.
//!
//! Agent wallets participate in the conduct scoring system, which adjusts
//! their decay multiplier based on on-chain behavior. Phase 1 uses static
//! defaults; dynamic scoring comes in Phase 2.

use serde::{Deserialize, Serialize};

use crate::types::{Hash256, WalletType};

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
