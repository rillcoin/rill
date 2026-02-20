//! Transaction building and submission for the faucet.
//!
//! Adapted from `bins/rill-cli/src/main.rs` `wallet_send()`.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::params::ArrayParams;
use tokio::sync::Mutex;

use rill_core::constants::MIN_TX_FEE;
use rill_core::error::RillError;
use rill_core::traits::ChainState;
use rill_core::types::{
    Block, BlockHeader, Hash256, OutPoint, Transaction, TxInput, TxOutput, UtxoEntry,
};
use rill_wallet::{CoinSelector, Wallet};

// ---------------------------------------------------------------------------
// RPC-backed chain state
// ---------------------------------------------------------------------------

/// Minimal `ChainState` implementation backed by data fetched via RPC.
///
/// `CoinSelector::select` only calls `circulating_supply` and
/// `cluster_balance`; all other methods return stub values.
struct RpcChainState {
    supply: u64,
    clusters: HashMap<Hash256, u64>,
}

impl ChainState for RpcChainState {
    fn circulating_supply(&self) -> Result<u64, RillError> {
        Ok(self.supply)
    }

    fn cluster_balance(&self, cluster_id: &Hash256) -> Result<u64, RillError> {
        Ok(*self.clusters.get(cluster_id).unwrap_or(&0))
    }

    fn get_utxo(&self, _outpoint: &OutPoint) -> Result<Option<UtxoEntry>, RillError> {
        Ok(None)
    }

    fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
        Ok((0, Hash256::ZERO))
    }

    fn get_block_header(&self, _hash: &Hash256) -> Result<Option<BlockHeader>, RillError> {
        Ok(None)
    }

    fn get_block(&self, _hash: &Hash256) -> Result<Option<Block>, RillError> {
        Ok(None)
    }

    fn get_block_hash(&self, _height: u64) -> Result<Option<Hash256>, RillError> {
        Ok(None)
    }

    fn decay_pool_balance(&self) -> Result<u64, RillError> {
        Ok(0)
    }

    fn validate_transaction(
        &self,
        _tx: &Transaction,
    ) -> Result<(), rill_core::error::TransactionError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// RPC client helper
// ---------------------------------------------------------------------------

pub fn rpc_client(endpoint: &str) -> Result<jsonrpsee::http_client::HttpClient> {
    jsonrpsee::http_client::HttpClientBuilder::default()
        .build(endpoint)
        .context("Failed to build RPC client")
}

// ---------------------------------------------------------------------------
// send_rill
// ---------------------------------------------------------------------------

/// Dispense `amount_rills` to `recipient_str` from the faucet wallet.
///
/// Locks the wallet mutex for the full duration (RPC + signing) to prevent
/// double-spending the same UTXOs across concurrent requests.
///
/// Returns the transaction ID (hex string) on success.
pub async fn send_rill(
    wallet: Arc<Mutex<Wallet>>,
    wallet_path: &Path,
    password: &[u8],
    recipient_str: &str,
    amount_rills: u64,
    rpc_endpoint: &str,
) -> Result<String> {
    if amount_rills == 0 {
        bail!("Amount must be greater than zero");
    }

    let recipient = recipient_str
        .parse::<rill_core::address::Address>()
        .context("Invalid recipient address")?;

    let client = rpc_client(rpc_endpoint)?;

    // Lock wallet for the entire send operation to prevent UTXO reuse.
    let mut wallet = wallet.lock().await;

    // ------------------------------------------------------------------
    // Fetch UTXOs for every wallet address
    // ------------------------------------------------------------------
    let mut all_utxos: Vec<(OutPoint, UtxoEntry)> = Vec::new();
    let address_count = wallet.address_count();

    for i in 0..address_count {
        let addr_str = wallet.keychain_mut().address_at(i).encode();

        let mut params = ArrayParams::new();
        params.insert(addr_str).unwrap();

        let utxo_jsons: Vec<serde_json::Value> = client
            .request("getutxosbyaddress", params)
            .await
            .context("RPC getutxosbyaddress failed")?;

        for utxo_json in utxo_jsons {
            if let Some((outpoint, entry)) = parse_utxo_json(&utxo_json) {
                all_utxos.push((outpoint, entry));
            }
        }
    }

    wallet.scan_utxos(&all_utxos);

    if wallet.utxo_count() == 0 {
        bail!("Faucet wallet has no UTXOs — please fund it first");
    }

    // ------------------------------------------------------------------
    // Fetch chain info for decay-aware coin selection
    // ------------------------------------------------------------------
    let info: serde_json::Value = client
        .request("getinfo", ArrayParams::new())
        .await
        .context("RPC getinfo failed")?;

    let height = info["blocks"].as_u64().unwrap_or(0);
    let circulating_rill = info["circulating_supply"].as_f64().unwrap_or(0.0);
    let circulating_supply = (circulating_rill * rill_core::constants::COIN as f64) as u64;

    let utxo_list: Vec<(OutPoint, UtxoEntry)> = wallet.owned_utxos().into_iter().collect();

    // ------------------------------------------------------------------
    // Fetch cluster balances for all referenced clusters
    // ------------------------------------------------------------------
    let unique_clusters: HashSet<Hash256> =
        utxo_list.iter().map(|(_, e)| e.cluster_id).collect();

    let mut cluster_balances: HashMap<Hash256, u64> = HashMap::new();
    for cluster_id in &unique_clusters {
        let cluster_hex = hex::encode(cluster_id.as_bytes());
        let mut params = ArrayParams::new();
        params.insert(cluster_hex).unwrap();
        let balance: u64 = client
            .request("getclusterbalance", params)
            .await
            .unwrap_or(0);
        cluster_balances.insert(*cluster_id, balance);
    }

    let rpc_state = RpcChainState {
        supply: circulating_supply,
        clusters: cluster_balances,
    };

    // ------------------------------------------------------------------
    // Decay-aware coin selection
    // ------------------------------------------------------------------
    let decay_engine = rill_decay::engine::DecayEngine::new();
    let selection = CoinSelector::select(
        &utxo_list,
        amount_rills,
        MIN_TX_FEE,
        500,
        &decay_engine,
        &rpc_state,
        height,
    )
    .map_err(|e| anyhow::anyhow!("Coin selection failed: {e}"))?;

    let change = selection.change;
    let change_addr = wallet.next_address();

    // ------------------------------------------------------------------
    // Build transaction
    // ------------------------------------------------------------------
    let mut inputs: Vec<TxInput> = Vec::new();
    let mut input_pubkey_hashes: Vec<Hash256> = Vec::new();

    for wallet_utxo in &selection.selected {
        inputs.push(TxInput {
            previous_output: wallet_utxo.outpoint.clone(),
            signature: vec![],
            public_key: vec![],
        });
        input_pubkey_hashes.push(wallet_utxo.entry.output.pubkey_hash);
    }

    let mut outputs = vec![TxOutput {
        value: amount_rills,
        pubkey_hash: recipient.pubkey_hash(),
    }];
    if change > 0 {
        outputs.push(TxOutput {
            value: change,
            pubkey_hash: change_addr.pubkey_hash(),
        });
    }

    let mut tx = Transaction {
        version: 1,
        inputs,
        outputs,
        lock_time: 0,
    };

    // Sign each input
    for (i, pkh) in input_pubkey_hashes.iter().enumerate() {
        let kp = wallet
            .keychain()
            .keypair_for_pubkey_hash(pkh)
            .ok_or_else(|| anyhow::anyhow!("Signing key not found for input {i}"))?;
        rill_core::crypto::sign_transaction_input(&mut tx, i, kp)
            .context("Failed to sign transaction input")?;
    }

    // ------------------------------------------------------------------
    // Broadcast
    // ------------------------------------------------------------------
    let tx_bytes = bincode::encode_to_vec(&tx, bincode::config::standard())
        .context("Failed to serialize transaction")?;
    let tx_hex = hex::encode(&tx_bytes);

    let mut params = ArrayParams::new();
    params.insert(tx_hex).unwrap();
    let txid: String = client
        .request("sendrawtransaction", params)
        .await
        .context("RPC sendrawtransaction failed")?;

    // Persist updated wallet state (next_address counter may have advanced)
    wallet
        .save_to_file(wallet_path, password)
        .context("Failed to save wallet after send")?;

    Ok(txid)
}

/// Fetch the total UTXO balance (in rills) for a list of addresses.
///
/// Returns `(total_rills, utxo_count)`.
pub async fn fetch_balance(client: &jsonrpsee::http_client::HttpClient, addresses: &[String]) -> u64 {
    let mut total: u64 = 0;
    for addr in addresses {
        let mut params = ArrayParams::new();
        params.insert(addr.clone()).unwrap();
        let utxos: Vec<serde_json::Value> = client
            .request("getutxosbyaddress", params)
            .await
            .unwrap_or_default();
        for utxo in utxos {
            total = total.saturating_add(utxo["value"].as_u64().unwrap_or(0));
        }
    }
    total
}

/// Fetch balance and UTXO count for a single address.
///
/// Returns `(total_rills, utxo_count)`.
pub async fn fetch_balance_for_address(
    client: &jsonrpsee::http_client::HttpClient,
    address: &str,
) -> Result<(u64, usize)> {
    let mut params = ArrayParams::new();
    params.insert(address.to_string()).unwrap();
    let utxos: Vec<serde_json::Value> = client
        .request("getutxosbyaddress", params)
        .await
        .context("RPC getutxosbyaddress failed")?;
    let mut total: u64 = 0;
    for utxo in &utxos {
        total = total.saturating_add(utxo["value"].as_u64().unwrap_or(0));
    }
    Ok((total, utxos.len()))
}

// ---------------------------------------------------------------------------
// send_from_mnemonic
// ---------------------------------------------------------------------------

/// Send `amount_rills` to `recipient_str` from an ephemeral wallet derived
/// from the given mnemonic phrase.
///
/// Unlike `send_rill`, this does not require a wallet file — the keychain is
/// constructed in-memory from the mnemonic and discarded after the send.
pub async fn send_from_mnemonic(
    mnemonic: &str,
    recipient_str: &str,
    amount_rills: u64,
    rpc_endpoint: &str,
) -> Result<(String, u64)> {
    use rill_wallet::mnemonic_to_seed;

    if amount_rills == 0 {
        bail!("Amount must be greater than zero");
    }

    let recipient = recipient_str
        .parse::<rill_core::address::Address>()
        .context("Invalid recipient address")?;

    let seed = mnemonic_to_seed(mnemonic)
        .map_err(|e| anyhow::anyhow!("Invalid mnemonic: {e}"))?;
    let mut keychain = rill_wallet::KeyChain::new(seed, rill_core::address::Network::Testnet);

    let client = rpc_client(rpc_endpoint)?;

    // Scan addresses 0..N until we find a gap of 2 addresses with no UTXOs.
    let mut all_utxos: Vec<(OutPoint, UtxoEntry)> = Vec::new();
    let mut gap = 0u32;
    let mut index = 0u32;

    while gap < 2 {
        let addr_str = keychain.address_at(index).encode();
        let mut params = ArrayParams::new();
        params.insert(addr_str).unwrap();

        let utxo_jsons: Vec<serde_json::Value> = client
            .request("getutxosbyaddress", params)
            .await
            .context("RPC getutxosbyaddress failed")?;

        if utxo_jsons.is_empty() {
            gap += 1;
        } else {
            gap = 0;
            for utxo_json in utxo_jsons {
                if let Some((outpoint, entry)) = parse_utxo_json(&utxo_json) {
                    all_utxos.push((outpoint, entry));
                }
            }
        }
        index += 1;
    }

    if all_utxos.is_empty() {
        bail!("Wallet has no UTXOs");
    }

    // Fetch chain info for decay-aware coin selection.
    let info: serde_json::Value = client
        .request("getinfo", ArrayParams::new())
        .await
        .context("RPC getinfo failed")?;

    let height = info["blocks"].as_u64().unwrap_or(0);
    let circulating_rill = info["circulating_supply"].as_f64().unwrap_or(0.0);
    let circulating_supply = (circulating_rill * rill_core::constants::COIN as f64) as u64;

    // Fetch cluster balances.
    let unique_clusters: HashSet<Hash256> =
        all_utxos.iter().map(|(_, e)| e.cluster_id).collect();

    let mut cluster_balances: HashMap<Hash256, u64> = HashMap::new();
    for cluster_id in &unique_clusters {
        let cluster_hex = hex::encode(cluster_id.as_bytes());
        let mut params = ArrayParams::new();
        params.insert(cluster_hex).unwrap();
        let balance: u64 = client
            .request("getclusterbalance", params)
            .await
            .unwrap_or(0);
        cluster_balances.insert(*cluster_id, balance);
    }

    let rpc_state = RpcChainState {
        supply: circulating_supply,
        clusters: cluster_balances,
    };

    // Coin selection.
    let decay_engine = rill_decay::engine::DecayEngine::new();
    let selection = CoinSelector::select(
        &all_utxos,
        amount_rills,
        MIN_TX_FEE,
        500,
        &decay_engine,
        &rpc_state,
        height,
    )
    .map_err(|e| anyhow::anyhow!("Coin selection failed: {e}"))?;

    let fee = selection.fee;
    let change = selection.change;
    let change_addr = keychain.address_at(index); // next unused address

    // Build transaction.
    let mut inputs: Vec<TxInput> = Vec::new();
    let mut input_pubkey_hashes: Vec<Hash256> = Vec::new();

    for wallet_utxo in &selection.selected {
        inputs.push(TxInput {
            previous_output: wallet_utxo.outpoint.clone(),
            signature: vec![],
            public_key: vec![],
        });
        input_pubkey_hashes.push(wallet_utxo.entry.output.pubkey_hash);
    }

    let mut outputs = vec![TxOutput {
        value: amount_rills,
        pubkey_hash: recipient.pubkey_hash(),
    }];
    if change > 0 {
        outputs.push(TxOutput {
            value: change,
            pubkey_hash: change_addr.pubkey_hash(),
        });
    }

    let mut tx = Transaction {
        version: 1,
        inputs,
        outputs,
        lock_time: 0,
    };

    // Sign each input.
    for (i, pkh) in input_pubkey_hashes.iter().enumerate() {
        let kp = keychain
            .keypair_for_pubkey_hash(pkh)
            .ok_or_else(|| anyhow::anyhow!("Signing key not found for input {i}"))?;
        rill_core::crypto::sign_transaction_input(&mut tx, i, kp)
            .context("Failed to sign transaction input")?;
    }

    // Broadcast.
    let tx_bytes = bincode::encode_to_vec(&tx, bincode::config::standard())
        .context("Failed to serialize transaction")?;
    let tx_hex = hex::encode(&tx_bytes);

    let mut params = ArrayParams::new();
    params.insert(tx_hex).unwrap();
    let txid: String = client
        .request("sendrawtransaction", params)
        .await
        .context("RPC sendrawtransaction failed")?;

    Ok((txid, fee))
}

/// Parse a UTXO JSON object from the RPC response into typed values.
fn parse_utxo_json(utxo_json: &serde_json::Value) -> Option<(OutPoint, UtxoEntry)> {
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
        let outpoint = OutPoint {
            txid: Hash256(txid_bytes.try_into().unwrap()),
            index,
        };
        let entry = UtxoEntry {
            output: TxOutput {
                value,
                pubkey_hash: Hash256(pkh_bytes.try_into().unwrap()),
            },
            block_height,
            is_coinbase,
            cluster_id: Hash256(cluster_bytes.try_into().unwrap()),
        };
        Some((outpoint, entry))
    } else {
        None
    }
}
