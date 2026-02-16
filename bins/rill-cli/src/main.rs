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
async fn wallet_balance(args: BalanceArgs) -> Result<()> {
    let wallet_path = resolve_wallet_path(args.wallet)?;
    let password = prompt_password("Wallet password")?;

    let mut wallet = Wallet::load_from_file(&wallet_path, password.as_bytes())
        .context("Failed to load wallet (check password)")?;

    // Connect to RPC and fetch UTXOs for all wallet addresses
    let client = jsonrpsee::http_client::HttpClientBuilder::default()
        .build(&args.rpc_endpoint)
        .context("Failed to connect to RPC")?;

    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::core::params::ArrayParams;

    // Collect UTXOs for all wallet addresses
    let mut all_utxos: Vec<(rill_core::types::OutPoint, rill_core::types::UtxoEntry)> = Vec::new();

    // Derive addresses up to the wallet's current index to scan
    let address_count = wallet.address_count();
    for i in 0..address_count {
        let addr = wallet.keychain_mut().address_at(i);
        let addr_str = addr.encode();

        let mut params = ArrayParams::new();
        params.insert(addr_str.clone()).unwrap();

        let utxo_jsons: Vec<serde_json::Value> = client
            .request("getutxosbyaddress", params)
            .await
            .with_context(|| format!("RPC getutxosbyaddress failed for {addr_str}"))?;

        for utxo_json in utxo_jsons {
            let txid_hex = utxo_json["txid"].as_str().unwrap_or_default();
            let txid_bytes = hex::decode(txid_hex).unwrap_or_default();
            let index = utxo_json["index"].as_u64().unwrap_or(0);
            let value = utxo_json["value"].as_u64().unwrap_or(0);
            let block_height = utxo_json["block_height"].as_u64().unwrap_or(0);
            let is_coinbase = utxo_json["is_coinbase"].as_bool().unwrap_or(false);
            let cluster_hex = utxo_json["cluster_id"].as_str().unwrap_or_default();
            let cluster_bytes = hex::decode(cluster_hex).unwrap_or_default();
            let pkh_hex = utxo_json["pubkey_hash"].as_str().unwrap_or_default();
            let pkh_bytes = hex::decode(pkh_hex).unwrap_or_default();

            if txid_bytes.len() == 32 && cluster_bytes.len() == 32 && pkh_bytes.len() == 32 {
                let outpoint = rill_core::types::OutPoint {
                    txid: rill_core::types::Hash256(txid_bytes.try_into().unwrap()),
                    index,
                };
                let entry = rill_core::types::UtxoEntry {
                    output: rill_core::types::TxOutput {
                        value,
                        pubkey_hash: rill_core::types::Hash256(pkh_bytes.try_into().unwrap()),
                    },
                    block_height,
                    is_coinbase,
                    cluster_id: rill_core::types::Hash256(cluster_bytes.try_into().unwrap()),
                };
                all_utxos.push((outpoint, entry));
            }
        }
    }

    // Scan UTXOs into wallet
    wallet.scan_utxos(&all_utxos);

    // Get chain info for height
    let info: serde_json::Value = client
        .request("getinfo", jsonrpsee::core::params::ArrayParams::new())
        .await
        .context("RPC getinfo failed")?;
    let height = info["blocks"].as_u64().unwrap_or(0);

    // Compute nominal balance
    let nominal: u64 = all_utxos.iter().map(|(_, e)| e.output.value).sum();

    println!("\n=== WALLET BALANCE ===");
    println!("Network: {}", network_name(wallet.network()));
    println!("Addresses: {}", wallet.address_count());
    println!("UTXOs: {}", wallet.utxo_count());
    println!();
    println!("Nominal:   {:.8} RILL", nominal as f64 / COIN as f64);
    println!("(Decay-adjusted balance requires full chain state - showing nominal only in CLI)");
    println!();
    println!("Current height: {}", height);

    Ok(())
}

/// Send a transaction.
async fn wallet_send(args: SendArgs) -> Result<()> {
    let wallet_path = resolve_wallet_path(args.wallet)?;
    let password = prompt_password("Wallet password")?;

    let mut wallet = Wallet::load_from_file(&wallet_path, password.as_bytes())
        .context("Failed to load wallet (check password)")?;

    let recipient = args.to
        .parse::<rill_core::address::Address>()
        .context("Invalid recipient address")?;

    let amount_rills = (args.amount * COIN as f64) as u64;
    if amount_rills == 0 {
        bail!("Amount must be greater than zero");
    }

    // Connect to RPC
    let client = jsonrpsee::http_client::HttpClientBuilder::default()
        .build(&args.rpc_endpoint)
        .context("Failed to connect to RPC")?;

    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::core::params::ArrayParams;

    // Fetch UTXOs for wallet addresses (same as balance)
    let mut all_utxos: Vec<(rill_core::types::OutPoint, rill_core::types::UtxoEntry)> = Vec::new();
    let address_count = wallet.address_count();
    for i in 0..address_count {
        let addr = wallet.keychain_mut().address_at(i);
        let addr_str = addr.encode();

        let mut params = ArrayParams::new();
        params.insert(addr_str).unwrap();

        let utxo_jsons: Vec<serde_json::Value> = client
            .request("getutxosbyaddress", params)
            .await
            .context("RPC getutxosbyaddress failed")?;

        for utxo_json in utxo_jsons {
            // Parse UTXO JSON (same as balance)
            let txid_hex = utxo_json["txid"].as_str().unwrap_or_default();
            let txid_bytes = hex::decode(txid_hex).unwrap_or_default();
            let index = utxo_json["index"].as_u64().unwrap_or(0);
            let value = utxo_json["value"].as_u64().unwrap_or(0);
            let block_height = utxo_json["block_height"].as_u64().unwrap_or(0);
            let is_coinbase = utxo_json["is_coinbase"].as_bool().unwrap_or(false);
            let cluster_hex = utxo_json["cluster_id"].as_str().unwrap_or_default();
            let cluster_bytes = hex::decode(cluster_hex).unwrap_or_default();
            let pkh_hex = utxo_json["pubkey_hash"].as_str().unwrap_or_default();
            let pkh_bytes = hex::decode(pkh_hex).unwrap_or_default();

            if txid_bytes.len() == 32 && cluster_bytes.len() == 32 && pkh_bytes.len() == 32 {
                let outpoint = rill_core::types::OutPoint {
                    txid: rill_core::types::Hash256(txid_bytes.try_into().unwrap()),
                    index,
                };
                let entry = rill_core::types::UtxoEntry {
                    output: rill_core::types::TxOutput {
                        value,
                        pubkey_hash: rill_core::types::Hash256(pkh_bytes.try_into().unwrap()),
                    },
                    block_height,
                    is_coinbase,
                    cluster_id: rill_core::types::Hash256(cluster_bytes.try_into().unwrap()),
                };
                all_utxos.push((outpoint, entry));
            }
        }
    }

    // Scan UTXOs into wallet
    wallet.scan_utxos(&all_utxos);

    if wallet.utxo_count() == 0 {
        bail!("No UTXOs found for wallet addresses");
    }

    // Get chain info for height
    let info: serde_json::Value = client
        .request("getinfo", ArrayParams::new())
        .await
        .context("RPC getinfo failed")?;
    let _height = info["blocks"].as_u64().unwrap_or(0);

    // Build transaction using simple greedy coin selection
    let change_addr = wallet.next_address();
    let utxo_list: Vec<(rill_core::types::OutPoint, rill_core::types::UtxoEntry)> =
        wallet.owned_utxos().into_iter().collect();

    // Simple greedy selection (no decay adjustment for CLI Phase 2)
    let total_needed = amount_rills + args.fee;
    let mut selected = Vec::new();
    let mut total_value = 0u64;
    for (op, entry) in &utxo_list {
        selected.push((op.clone(), entry.clone()));
        total_value += entry.output.value;
        if total_value >= total_needed {
            break;
        }
    }
    if total_value < total_needed {
        bail!("Insufficient funds: have {} rills, need {} rills",
            total_value, total_needed);
    }

    let change = total_value - total_needed;

    // Build transaction
    let mut inputs = Vec::new();
    let mut input_pubkey_hashes = Vec::new();
    for (op, entry) in &selected {
        inputs.push(rill_core::types::TxInput {
            previous_output: op.clone(),
            signature: vec![],
            public_key: vec![],
        });
        input_pubkey_hashes.push(entry.output.pubkey_hash);
    }

    let mut outputs = vec![rill_core::types::TxOutput {
        value: amount_rills,
        pubkey_hash: recipient.pubkey_hash(),
    }];
    if change > 0 {
        outputs.push(rill_core::types::TxOutput {
            value: change,
            pubkey_hash: change_addr.pubkey_hash(),
        });
    }

    let mut tx = rill_core::types::Transaction {
        version: 1,
        inputs,
        outputs,
        lock_time: 0,
    };

    // Sign each input
    for (i, pkh) in input_pubkey_hashes.iter().enumerate() {
        let kp = wallet.keychain().keypair_for_pubkey_hash(pkh)
            .ok_or_else(|| anyhow::anyhow!("signing key not found for input {i}"))?;
        rill_core::crypto::sign_transaction_input(&mut tx, i, kp)
            .context("Failed to sign transaction input")?;
    }

    // Serialize and submit via RPC
    let tx_bytes = bincode::encode_to_vec(&tx, bincode::config::standard())
        .context("Failed to serialize transaction")?;
    let tx_hex = hex::encode(&tx_bytes);

    let mut params = ArrayParams::new();
    params.insert(tx_hex).unwrap();
    let txid: String = client
        .request("sendrawtransaction", params)
        .await
        .context("RPC sendrawtransaction failed")?;

    println!("\n=== TRANSACTION SENT ===");
    println!("TxID: {txid}");
    println!("To: {}", recipient.encode());
    println!("Amount: {:.8} RILL ({} rills)", args.amount, amount_rills);
    println!("Fee: {} rills", args.fee);
    if change > 0 {
        println!("Change: {:.8} RILL ({} rills)", change as f64 / COIN as f64, change);
    }

    // Save wallet with updated state
    wallet.save_to_file(&wallet_path, password.as_bytes())
        .context("Failed to save wallet")?;

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
