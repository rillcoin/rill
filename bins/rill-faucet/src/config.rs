//! Faucet configuration loaded from environment variables.

use std::path::PathBuf;

use anyhow::{Context, Result};
use rill_core::constants::COIN;

#[derive(Clone, Debug)]
pub struct Config {
    /// Path to the faucet wallet file.
    pub wallet_path: PathBuf,
    /// Wallet decryption password.
    pub wallet_password: String,
    /// RillCoin node JSON-RPC endpoint.
    pub rpc_endpoint: String,
    /// Address to bind the HTTP server.
    pub bind_addr: String,
    /// Amount to dispense per request, in rills (smallest unit).
    pub amount_rills: u64,
    /// Cooldown between claims per address/IP, in seconds.
    pub cooldown_secs: u64,
    /// Discord bot token (enables Discord integration).
    pub discord_bot_token: Option<String>,
    /// Discord application Ed25519 public key (hex-encoded, for signature verification).
    pub discord_public_key: Option<String>,
    /// Discord application ID (for slash command registration).
    pub discord_app_id: Option<String>,
}

impl Config {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let wallet_path = std::env::var("FAUCET_WALLET_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".rill")
                    .join("testnet-faucet.dat")
            });

        let wallet_password =
            std::env::var("FAUCET_WALLET_PASSWORD").context("FAUCET_WALLET_PASSWORD is required")?;

        let rpc_endpoint = std::env::var("FAUCET_RPC_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:28332".to_string());

        let bind_addr = std::env::var("FAUCET_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string());

        let amount_rill: u64 = std::env::var("FAUCET_AMOUNT_RILL")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .context("FAUCET_AMOUNT_RILL must be a positive integer")?;

        let amount_rills = amount_rill
            .checked_mul(COIN)
            .context("FAUCET_AMOUNT_RILL overflow")?;

        let cooldown_secs: u64 = std::env::var("FAUCET_COOLDOWN_SECS")
            .unwrap_or_else(|_| "86400".to_string())
            .parse()
            .context("FAUCET_COOLDOWN_SECS must be a positive integer")?;

        let discord_bot_token = std::env::var("DISCORD_BOT_TOKEN").ok();
        let discord_public_key = std::env::var("DISCORD_PUBLIC_KEY").ok();
        let discord_app_id = std::env::var("DISCORD_APPLICATION_ID").ok();

        Ok(Config {
            wallet_path,
            wallet_password,
            rpc_endpoint,
            bind_addr,
            amount_rills,
            cooldown_secs,
            discord_bot_token,
            discord_public_key,
            discord_app_id,
        })
    }

    /// Amount in whole RILL (for display).
    pub fn amount_rill(&self) -> u64 {
        self.amount_rills / COIN
    }
}
