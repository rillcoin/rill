//! rill-cli — Command-line wallet interface for RillCoin.
//!
//! Provides wallet management, balance queries, and transaction creation
//! with secure password handling and encrypted wallet storage.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use rill_core::address::Network;
use rill_core::constants::{COIN, CONCENTRATION_PRECISION};
use rill_core::traits::DecayCalculator;
use rill_wallet::{CoinSelector, Seed, Wallet};

/// RPC-backed chain state adapter for decay-aware coin selection.
///
/// Provides `circulating_supply` and `cluster_balance` from data fetched
/// via RPC. All other `ChainState` methods return harmless stub values since
/// `CoinSelector::select()` only calls those two.
struct RpcChainState {
    supply: u64,
    clusters: std::collections::HashMap<rill_core::types::Hash256, u64>,
}

impl rill_core::traits::ChainState for RpcChainState {
    fn circulating_supply(&self) -> Result<u64, rill_core::error::RillError> {
        Ok(self.supply)
    }

    fn cluster_balance(
        &self,
        cluster_id: &rill_core::types::Hash256,
    ) -> Result<u64, rill_core::error::RillError> {
        Ok(*self.clusters.get(cluster_id).unwrap_or(&0))
    }

    fn get_utxo(
        &self,
        _outpoint: &rill_core::types::OutPoint,
    ) -> Result<Option<rill_core::types::UtxoEntry>, rill_core::error::RillError> {
        Ok(None)
    }

    fn chain_tip(&self) -> Result<(u64, rill_core::types::Hash256), rill_core::error::RillError> {
        Ok((0, rill_core::types::Hash256::ZERO))
    }

    fn get_block_header(
        &self,
        _hash: &rill_core::types::Hash256,
    ) -> Result<Option<rill_core::types::BlockHeader>, rill_core::error::RillError> {
        Ok(None)
    }

    fn get_block(
        &self,
        _hash: &rill_core::types::Hash256,
    ) -> Result<Option<rill_core::types::Block>, rill_core::error::RillError> {
        Ok(None)
    }

    fn get_block_hash(
        &self,
        _height: u64,
    ) -> Result<Option<rill_core::types::Hash256>, rill_core::error::RillError> {
        Ok(None)
    }

    fn decay_pool_balance(&self) -> Result<u64, rill_core::error::RillError> {
        Ok(0)
    }

    fn validate_transaction(
        &self,
        _tx: &rill_core::types::Transaction,
    ) -> Result<(), rill_core::error::TransactionError> {
        Ok(())
    }
}

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
    let mnemonic = rill_wallet::seed_to_mnemonic(&seed);

    println!("\n=== WALLET CREATED ===");
    println!("Network: {}", network_name(network));
    println!("\nSEED PHRASE (BACKUP THIS — 24 WORDS):");
    println!("  {}", mnemonic);
    println!("\nAdvanced: hex seed = {}", seed_hex);
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

    let seed_input = if let Some(s) = args.seed {
        s
    } else {
        prompt_password("Enter seed (24-word mnemonic or hex)")?
    };

    let seed = parse_seed_input(&seed_input)?;

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

    // Get chain info for height and circulating supply
    let info: serde_json::Value = client
        .request("getinfo", ArrayParams::new())
        .await
        .context("RPC getinfo failed")?;
    let height = info["blocks"].as_u64().unwrap_or(0);
    let circulating_supply_rill = info["circulating_supply"].as_f64().unwrap_or(0.0);
    let circulating_supply = (circulating_supply_rill * COIN as f64) as u64;

    // Compute nominal balance
    let nominal: u64 = all_utxos.iter().map(|(_, e)| e.output.value).sum();

    // Compute decay-adjusted balance
    let decay_engine = rill_decay::engine::DecayEngine::new();

    // Group UTXOs by cluster_id
    let mut clusters: std::collections::HashMap<rill_core::types::Hash256, Vec<&(rill_core::types::OutPoint, rill_core::types::UtxoEntry)>> = std::collections::HashMap::new();
    for utxo in &all_utxos {
        clusters.entry(utxo.1.cluster_id).or_default().push(utxo);
    }

    let mut total_effective = 0u64;
    let mut cluster_details: Vec<(String, u64, u64)> = Vec::new(); // (cluster_id_short, nominal, effective)

    for (cluster_id, utxos) in &clusters {
        let cluster_hex = hex::encode(cluster_id.as_bytes());

        // Fetch cluster balance from node
        let mut params = ArrayParams::new();
        params.insert(cluster_hex.clone()).unwrap();
        let cluster_balance: u64 = client
            .request("getclusterbalance", params)
            .await
            .unwrap_or(0); // Best-effort; if RPC fails, assume 0 concentration

        // Compute concentration
        let concentration_ppb = if circulating_supply > 0 {
            // cluster_balance * CONCENTRATION_PRECISION / circulating_supply
            (cluster_balance as u128 * CONCENTRATION_PRECISION as u128 / circulating_supply as u128) as u64
        } else {
            0
        };

        let mut cluster_nominal = 0u64;
        let mut cluster_effective = 0u64;

        for (_, entry) in utxos {
            let blocks_held = height.saturating_sub(entry.block_height);
            let effective = decay_engine
                .effective_value(entry.output.value, concentration_ppb, blocks_held)
                .unwrap_or(entry.output.value); // Fallback to nominal on error
            cluster_nominal += entry.output.value;
            cluster_effective += effective;
        }

        total_effective += cluster_effective;
        cluster_details.push((cluster_hex[..8].to_string(), cluster_nominal, cluster_effective));
    }

    let total_decay = nominal.saturating_sub(total_effective);

    println!("\n=== WALLET BALANCE ===");
    println!("Network: {}", network_name(wallet.network()));
    println!("Addresses: {}", wallet.address_count());
    println!("UTXOs: {}", wallet.utxo_count());
    println!();
    println!("Nominal:   {:.8} RILL", nominal as f64 / COIN as f64);
    println!("Effective: {:.8} RILL", total_effective as f64 / COIN as f64);
    if total_decay > 0 {
        println!("Decay:    -{:.8} RILL ({:.2}%)",
            total_decay as f64 / COIN as f64,
            if nominal > 0 { total_decay as f64 / nominal as f64 * 100.0 } else { 0.0 });
    }

    if cluster_details.len() > 1 {
        println!();
        println!("Per-cluster breakdown:");
        for (cluster_short, c_nom, c_eff) in &cluster_details {
            let c_decay = c_nom.saturating_sub(*c_eff);
            println!("  Cluster {}...: {:.8} RILL nominal, {:.8} RILL effective (decay: {:.8})",
                cluster_short,
                *c_nom as f64 / COIN as f64,
                *c_eff as f64 / COIN as f64,
                c_decay as f64 / COIN as f64);
        }
    }

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

    // Get chain info for height and circulating supply
    let info: serde_json::Value = client
        .request("getinfo", ArrayParams::new())
        .await
        .context("RPC getinfo failed")?;
    let height = info["blocks"].as_u64().unwrap_or(0);
    let circulating_supply_rill = info["circulating_supply"].as_f64().unwrap_or(0.0);
    let circulating_supply = (circulating_supply_rill * COIN as f64) as u64;

    // Collect all UTXOs for coin selection
    let utxo_list: Vec<(rill_core::types::OutPoint, rill_core::types::UtxoEntry)> =
        wallet.owned_utxos().into_iter().collect();

    // Collect unique cluster IDs and fetch each cluster's balance via RPC
    let unique_cluster_ids: std::collections::HashSet<rill_core::types::Hash256> =
        utxo_list.iter().map(|(_, entry)| entry.cluster_id).collect();

    let mut cluster_balances: std::collections::HashMap<rill_core::types::Hash256, u64> =
        std::collections::HashMap::new();
    for cluster_id in &unique_cluster_ids {
        let cluster_hex = hex::encode(cluster_id.as_bytes());
        let mut params = ArrayParams::new();
        params.insert(cluster_hex).unwrap();
        let balance: u64 = client
            .request("getclusterbalance", params)
            .await
            .unwrap_or(0);
        cluster_balances.insert(*cluster_id, balance);
    }

    // Build RPC-backed chain state for decay-aware coin selection
    let rpc_state = RpcChainState {
        supply: circulating_supply,
        clusters: cluster_balances,
    };

    // Decay-aware coin selection: spends highest-decay UTXOs first
    let decay_engine = rill_decay::engine::DecayEngine::new();
    let selection =
        CoinSelector::select(&utxo_list, amount_rills, args.fee, 500, &decay_engine, &rpc_state, height)
            .map_err(|e| anyhow::anyhow!("Coin selection failed: {e}"))?;

    let change = selection.change;
    let change_addr = wallet.next_address();

    // Build transaction using selected UTXOs
    let mut inputs = Vec::new();
    let mut input_pubkey_hashes = Vec::new();
    for wallet_utxo in &selection.selected {
        inputs.push(rill_core::types::TxInput {
            previous_output: wallet_utxo.outpoint.clone(),
            signature: vec![],
            public_key: vec![],
        });
        input_pubkey_hashes.push(wallet_utxo.entry.output.pubkey_hash);
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
    println!("Fee: {} rills", selection.fee);
    if change > 0 {
        println!("Change: {:.8} RILL ({} rills)", change as f64 / COIN as f64, change);
    }

    // Save wallet with updated state
    wallet.save_to_file(&wallet_path, password.as_bytes())
        .context("Failed to save wallet")?;

    Ok(())
}

/// Parse seed input as either a BIP-39 mnemonic (multi-word) or hex string.
fn parse_seed_input(input: &str) -> Result<Seed> {
    let trimmed = input.trim();
    let word_count = trimmed.split_whitespace().count();
    if word_count > 1 {
        // Treat as mnemonic
        rill_wallet::mnemonic_to_seed(trimmed)
            .map_err(|e| anyhow::anyhow!("Invalid mnemonic: {e}"))
    } else {
        // Treat as hex
        let seed_bytes = hex::decode(trimmed).context("Invalid hex seed")?;
        if seed_bytes.len() != 32 {
            anyhow::bail!("Seed must be exactly 32 bytes (64 hex characters)");
        }
        let mut seed_array = [0u8; 32];
        seed_array.copy_from_slice(&seed_bytes);
        Ok(Seed::from_bytes(seed_array))
    }
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
