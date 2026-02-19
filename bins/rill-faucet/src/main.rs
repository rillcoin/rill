//! rill-faucet — Testnet RILL faucet with web UI and Discord integration.
//!
//! Serves a web UI at `/` and REST API at `/api/faucet`, dispensing
//! a configurable amount of testnet RILL per address every 24 hours.
//! Optionally handles Discord slash commands via HTTP Interactions.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::sync::Mutex;
use tracing::{info, warn};

mod config;
mod discord;
mod rate_limit;
mod routes;
mod send;

use config::Config;
use rate_limit::RateLimiter;
use rill_wallet::Wallet;

/// Shared application state passed to every Axum handler.
#[derive(Clone)]
pub struct AppState {
    /// Faucet wallet, locked during sends to prevent UTXO reuse.
    pub wallet: Arc<Mutex<Wallet>>,
    /// Path to the wallet file (needed for saving after each send).
    pub wallet_path: PathBuf,
    /// Wallet decryption password (kept in memory for saves).
    pub wallet_password: Vec<u8>,
    /// Faucet configuration.
    pub config: Arc<Config>,
    /// Per-address and per-IP rate limiter.
    pub rate_limiter: Arc<Mutex<RateLimiter>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env().context("Failed to load faucet configuration")?;

    info!(
        wallet = %config.wallet_path.display(),
        rpc = %config.rpc_endpoint,
        bind = %config.bind_addr,
        amount_rill = config.amount_rill(),
        cooldown_secs = config.cooldown_secs,
        "Starting rill-faucet"
    );

    // Load the faucet wallet.
    let wallet = Wallet::load_from_file(&config.wallet_path, config.wallet_password.as_bytes())
        .with_context(|| {
            format!(
                "Failed to load faucet wallet at {}. \
                 Create one with: rill-cli --testnet wallet create --file {}",
                config.wallet_path.display(),
                config.wallet_path.display()
            )
        })?;

    info!("Faucet wallet loaded ({} address(es))", wallet.address_count());

    let rate_limiter = RateLimiter::new(Duration::from_secs(config.cooldown_secs));

    let state = AppState {
        wallet: Arc::new(Mutex::new(wallet)),
        wallet_path: config.wallet_path.clone(),
        wallet_password: config.wallet_password.as_bytes().to_vec(),
        rate_limiter: Arc::new(Mutex::new(rate_limiter)),
        config: Arc::new(config.clone()),
    };

    // Register Discord slash commands if credentials are present.
    if let (Some(token), Some(app_id)) = (&config.discord_bot_token, &config.discord_app_id) {
        match discord::register_commands(token, app_id).await {
            Ok(()) => info!("Discord slash commands registered"),
            Err(e) => warn!("Failed to register Discord commands: {e}"),
        }
    }

    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("Failed to bind to {}", config.bind_addr))?;

    info!("Listening on http://{}", config.bind_addr);

    axum::serve(listener, app)
        .await
        .context("HTTP server error")?;

    Ok(())
}
