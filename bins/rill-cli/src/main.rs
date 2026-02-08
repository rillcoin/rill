//! rill-cli — Command-line wallet interface for RillCoin.
//!
//! Provides wallet management, balance queries, and transaction creation
//! with secure password handling and encrypted wallet storage.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use rill_core::address::Network;
use rill_core::constants::COIN;
use rill_wallet::{Seed, Wallet};

/// RillCoin command-line wallet interface.
#[derive(Parser)]
#[command(name = "rill-cli")]
#[command(version, about = "Wealth should flow like water.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Wallet management subcommands.
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
    },
    /// Show the current receive address.
    Address(AddressArgs),
    /// Query wallet balance from the network.
    Balance(BalanceArgs),
    /// Send a transaction.
    Send(SendArgs),
}

#[derive(Subcommand)]
enum WalletAction {
    /// Create a new HD wallet.
    Create(WalletCreateArgs),
    /// Restore a wallet from seed phrase.
    Restore(WalletRestoreArgs),
}

#[derive(Args)]
struct WalletCreateArgs {
    /// Path to wallet file (default: ~/.rill/wallet.dat).
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Network (mainnet or testnet).
    #[arg(short, long, default_value = "testnet")]
    network: String,
}

#[derive(Args)]
struct WalletRestoreArgs {
    /// Path to wallet file (default: ~/.rill/wallet.dat).
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Hex-encoded seed (32 bytes). If not provided, will prompt securely.
    #[arg(short, long)]
    seed: Option<String>,

    /// Network (mainnet or testnet).
    #[arg(short, long, default_value = "testnet")]
    network: String,
}

#[derive(Args)]
struct AddressArgs {
    /// Path to wallet file (default: ~/.rill/wallet.dat).
    #[arg(short, long)]
    wallet: Option<PathBuf>,
}

#[derive(Args)]
struct BalanceArgs {
    /// Path to wallet file (default: ~/.rill/wallet.dat).
    #[arg(short, long)]
    wallet: Option<PathBuf>,

    /// RPC endpoint URL.
    #[arg(short, long, default_value = "http://127.0.0.1:18332")]
    rpc_endpoint: String,
}

#[derive(Args)]
struct SendArgs {
    /// Path to wallet file (default: ~/.rill/wallet.dat).
    #[arg(short, long)]
    wallet: Option<PathBuf>,

    /// Recipient address.
    #[arg(short, long)]
    to: String,

    /// Amount to send in RILL (e.g., 10.5).
    #[arg(short, long)]
    amount: f64,

    /// Transaction fee in rills (default: 1000).
    #[arg(short, long, default_value = "1000")]
    fee: u64,

    /// RPC endpoint URL.
    #[arg(short, long, default_value = "http://127.0.0.1:18332")]
    rpc_endpoint: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Wallet { action } => match action {
            WalletAction::Create(args) => wallet_create(args).await,
            WalletAction::Restore(args) => wallet_restore(args).await,
        },
        Commands::Address(args) => wallet_address(args).await,
        Commands::Balance(args) => wallet_balance(args).await,
        Commands::Send(args) => wallet_send(args).await,
    }
}

/// Create a new wallet with a random seed.
async fn wallet_create(args: WalletCreateArgs) -> Result<()> {
    let wallet_path = resolve_wallet_path(args.file)?;
    let network = parse_network(&args.network)?;

    if wallet_path.exists() {
        bail!("Wallet file already exists: {}", wallet_path.display());
    }

    let password = prompt_password("Enter wallet password")?;
    let password_confirm = prompt_password("Confirm password")?;

    if password != password_confirm {
        bail!("Passwords do not match");
    }

    // Generate seed and display it before creating wallet
    let seed = Seed::generate();
    let seed_hex = hex::encode(seed.as_bytes());

    println!("\n=== WALLET CREATED ===");
    println!("Network: {}", network_name(network));
    println!("\nSEED PHRASE (BACKUP THIS SECURELY):");
    println!("{}", seed_hex);
    println!("\nWARNING: This seed phrase will NOT be shown again.");
    println!("Store it in a secure location. Anyone with this seed can access your funds.");

    let mut wallet = Wallet::from_seed(seed, network);
    let _ = wallet.next_address(); // Derive first address

    // Create wallet directory if needed
    if let Some(parent) = wallet_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    wallet
        .save_to_file(&wallet_path, password.as_bytes())
        .context("Failed to save wallet")?;

    println!("\nWallet saved to: {}", wallet_path.display());
    Ok(())
}

/// Restore a wallet from a seed phrase.
async fn wallet_restore(args: WalletRestoreArgs) -> Result<()> {
    let wallet_path = resolve_wallet_path(args.file)?;
    let network = parse_network(&args.network)?;

    if wallet_path.exists() {
        bail!("Wallet file already exists: {}", wallet_path.display());
    }

    let seed_hex = if let Some(s) = args.seed {
        s
    } else {
        prompt_password("Enter seed phrase (hex)")?
    };

    let seed_bytes = hex::decode(seed_hex.trim()).context("Invalid hex seed")?;
    if seed_bytes.len() != 32 {
        bail!("Seed must be exactly 32 bytes (64 hex characters)");
    }

    let mut seed_array = [0u8; 32];
    seed_array.copy_from_slice(&seed_bytes);
    let seed = Seed::from_bytes(seed_array);

    let password = prompt_password("Enter new wallet password")?;
    let password_confirm = prompt_password("Confirm password")?;

    if password != password_confirm {
        bail!("Passwords do not match");
    }

    let mut wallet = Wallet::from_seed(seed, network);
    let _ = wallet.next_address(); // Derive first address

    // Create wallet directory if needed
    if let Some(parent) = wallet_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    wallet
        .save_to_file(&wallet_path, password.as_bytes())
        .context("Failed to save wallet")?;

    println!("\n=== WALLET RESTORED ===");
    println!("Network: {}", network_name(network));
    println!("Wallet saved to: {}", wallet_path.display());
    Ok(())
}

/// Display the current receive address.
async fn wallet_address(args: AddressArgs) -> Result<()> {
    let wallet_path = resolve_wallet_path(args.wallet)?;
    let password = prompt_password("Wallet password")?;

    let mut wallet = Wallet::load_from_file(&wallet_path, password.as_bytes())
        .context("Failed to load wallet (check password)")?;

    let address = wallet.next_address();
    println!("{}", address.encode());
    Ok(())
}

/// Query and display the wallet balance.
///
/// Phase 1: This is a placeholder. In production, it would query the node
/// via RPC to get UTXO set, chain state, and compute decay-adjusted balance.
async fn wallet_balance(args: BalanceArgs) -> Result<()> {
    let wallet_path = resolve_wallet_path(args.wallet)?;
    let password = prompt_password("Wallet password")?;

    let wallet = Wallet::load_from_file(&wallet_path, password.as_bytes())
        .context("Failed to load wallet (check password)")?;

    println!("\n=== WALLET BALANCE ===");
    println!("Network: {}", network_name(wallet.network()));
    println!("Addresses: {}", wallet.address_count());
    println!("UTXOs: {}", wallet.utxo_count());
    println!("\nPhase 1: Balance query requires node RPC integration.");
    println!("RPC endpoint: {}", args.rpc_endpoint);
    println!("\nNominal:   0.00000000 RILL");
    println!("Effective: 0.00000000 RILL");
    println!("Decay:     0.00000000 RILL");

    Ok(())
}

/// Send a transaction.
///
/// Phase 1: This is a placeholder. In production, it would:
/// 1. Load wallet
/// 2. Query UTXO set via RPC
/// 3. Build and sign transaction
/// 4. Broadcast via RPC
async fn wallet_send(args: SendArgs) -> Result<()> {
    let wallet_path = resolve_wallet_path(args.wallet)?;
    let password = prompt_password("Wallet password")?;

    let wallet = Wallet::load_from_file(&wallet_path, password.as_bytes())
        .context("Failed to load wallet (check password)")?;

    // Parse recipient address
    let recipient = args
        .to
        .parse::<rill_core::address::Address>()
        .context("Invalid recipient address")?;

    // Convert RILL to rills
    let amount_rills = (args.amount * COIN as f64) as u64;

    if amount_rills == 0 {
        bail!("Amount must be greater than zero");
    }

    println!("\n=== SEND TRANSACTION ===");
    println!("Network: {}", network_name(wallet.network()));
    println!("To: {}", recipient.encode());
    println!("Amount: {:.8} RILL ({} rills)", args.amount, amount_rills);
    println!("Fee: {} rills", args.fee);
    println!("\nPhase 1: Transaction broadcast requires node RPC integration.");
    println!("RPC endpoint: {}", args.rpc_endpoint);
    println!("\nTransaction NOT sent (RPC not implemented).");

    Ok(())
}

/// Prompt for a password securely (no echo).
fn prompt_password(prompt: &str) -> Result<String> {
    rpassword::prompt_password(format!("{}: ", prompt)).context("Failed to read password")
}

/// Resolve wallet file path, using default if not provided.
fn resolve_wallet_path(path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = path {
        return Ok(p);
    }

    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".rill").join("wallet.dat"))
}

/// Parse network string to Network enum.
fn parse_network(s: &str) -> Result<Network> {
    match s.to_lowercase().as_str() {
        "mainnet" => Ok(Network::Mainnet),
        "testnet" => Ok(Network::Testnet),
        _ => bail!("Invalid network (must be 'mainnet' or 'testnet')"),
    }
}

/// Human-readable network name.
fn network_name(network: Network) -> &'static str {
    match network {
        Network::Mainnet => "Mainnet",
        Network::Testnet => "Testnet",
    }
}
