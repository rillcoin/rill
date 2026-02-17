//! JSON-RPC server for the Rill full node.
//!
//! Uses jsonrpsee 0.24 to expose a Bitcoin-like JSON-RPC interface for
//! querying blocks, transactions, mempool state, and network info.

use std::sync::Arc;

use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::{Server, ServerHandle};
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

use rill_core::constants::COIN;
use rill_core::error::RillError;
use rill_core::types::Hash256;

use crate::node::Node;

/// JSON representation of a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockJson {
    /// Block header hash as hex.
    pub hash: String,
    /// Block height.
    pub height: u64,
    /// Protocol version.
    pub version: u64,
    /// Previous block hash as hex.
    pub prev_hash: String,
    /// Merkle root as hex.
    pub merkle_root: String,
    /// Block timestamp (Unix seconds).
    pub timestamp: u64,
    /// Difficulty target.
    pub difficulty_target: u64,
    /// Nonce used for PoW.
    pub nonce: u64,
    /// Number of transactions in the block.
    pub tx_count: usize,
    /// Transaction IDs as hex strings.
    pub tx: Vec<String>,
}

/// JSON representation of a block header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderJson {
    /// Block header hash as hex.
    pub hash: String,
    /// Protocol version.
    pub version: u64,
    /// Previous block hash as hex.
    pub prev_hash: String,
    /// Merkle root as hex.
    pub merkle_root: String,
    /// Block timestamp (Unix seconds).
    pub timestamp: u64,
    /// Difficulty target.
    pub difficulty_target: u64,
    /// Nonce used for PoW.
    pub nonce: u64,
}

/// JSON representation of mempool info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolInfoJson {
    /// Number of transactions in the mempool.
    pub size: usize,
    /// Total bytes of all transactions.
    pub bytes: usize,
    /// Total fees in rills.
    pub total_fee: u64,
}

/// JSON representation of peer info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfoJson {
    /// Number of connected peers.
    pub connected: usize,
}

/// JSON representation of node info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfoJson {
    /// Current chain tip height.
    pub blocks: u64,
    /// Current chain tip hash.
    pub bestblockhash: String,
    /// Number of connected peers.
    pub connections: usize,
    /// Circulating supply in RILL (not rills).
    pub circulating_supply: f64,
    /// Decay pool balance in RILL (not rills).
    pub decay_pool: f64,
}

/// JSON representation of blockchain state (for `getblockchaininfo`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainInfoJson {
    /// Current chain tip height.
    pub height: u64,
    /// Best block hash as hex.
    pub best_block_hash: String,
    /// Circulating supply in rills (satoshi-equivalent).
    pub circulating_supply: u64,
    /// Decay pool balance in rills.
    pub decay_pool_balance: u64,
    /// Whether the node is in Initial Block Download mode.
    pub initial_block_download: bool,
    /// Number of UTXOs in the UTXO set.
    pub utxo_count: usize,
    /// Number of transactions in the mempool.
    pub mempool_size: usize,
    /// Number of connected peers.
    pub peer_count: usize,
}

/// JSON representation of sync status (for `getsyncstatus`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatusJson {
    /// Whether the node is in Initial Block Download mode.
    pub syncing: bool,
    /// Current chain tip height.
    pub current_height: u64,
    /// Number of connected peers.
    pub peer_count: usize,
    /// Current best block hash as hex.
    pub best_block_hash: String,
}

/// JSON representation of a block template for mining.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTemplateJson {
    /// Block version.
    pub version: u64,
    /// Previous block hash as hex.
    pub prev_hash: String,
    /// Merkle root as hex.
    pub merkle_root: String,
    /// Block timestamp (Unix seconds).
    pub timestamp: u64,
    /// Difficulty target.
    pub difficulty_target: u64,
    /// Initial nonce (always 0).
    pub nonce: u64,
    /// Hex-encoded bincode serialization of all transactions.
    pub transactions: String,
    /// Height of this block.
    pub height: u64,
}

/// JSON representation of a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionJson {
    /// Transaction ID as hex.
    pub txid: String,
    /// Transaction version.
    pub version: u64,
    /// Number of inputs.
    pub vin_count: usize,
    /// Number of outputs.
    pub vout_count: usize,
    /// Lock time.
    pub lock_time: u64,
}

/// JSON representation of a UTXO for address queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtxoJson {
    /// Transaction ID as hex.
    pub txid: String,
    /// Output index.
    pub index: u64,
    /// Value in rills.
    pub value: u64,
    /// Block height where this UTXO was created.
    pub block_height: u64,
    /// Whether this is a coinbase output.
    pub is_coinbase: bool,
    /// Cluster ID as hex.
    pub cluster_id: String,
    /// Pubkey hash as hex.
    pub pubkey_hash: String,
}

/// Parse a 64-character hex string into a Hash256.
pub fn parse_hash(hex_str: &str) -> Result<Hash256, ErrorObjectOwned> {
    if hex_str.len() != 64 {
        return Err(rpc_error(-1, "hash must be 64 hex characters"));
    }
    let bytes = hex::decode(hex_str)
        .map_err(|_| rpc_error(-1, "invalid hex in hash"))?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| rpc_error(-1, "hash must be 32 bytes"))?;
    Ok(Hash256(arr))
}

/// Create a JSON-RPC error.
fn rpc_error(code: i32, msg: &str) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(code, msg.to_string(), None::<()>)
}

/// The Rill JSON-RPC interface.
#[rpc(server)]
pub trait RillRpc {
    /// Returns the current block count (chain tip height).
    #[method(name = "getblockcount")]
    async fn get_block_count(&self) -> Result<u64, ErrorObjectOwned>;

    /// Returns the block hash at the given height.
    #[method(name = "getblockhash")]
    async fn get_block_hash(&self, height: u64) -> Result<String, ErrorObjectOwned>;

    /// Returns a block by its hash.
    #[method(name = "getblock")]
    async fn get_block(&self, hash: String) -> Result<BlockJson, ErrorObjectOwned>;

    /// Returns a block header by its hash.
    #[method(name = "getblockheader")]
    async fn get_block_header(
        &self,
        hash: String,
    ) -> Result<HeaderJson, ErrorObjectOwned>;

    /// Returns a transaction by its ID (searches mempool first, then blockchain).
    #[method(name = "gettransaction")]
    async fn get_transaction(
        &self,
        txid: String,
    ) -> Result<TransactionJson, ErrorObjectOwned>;

    /// Submits a raw transaction (hex-encoded bincode) to the network.
    #[method(name = "sendrawtransaction")]
    async fn send_raw_transaction(
        &self,
        hex_data: String,
    ) -> Result<String, ErrorObjectOwned>;

    /// Returns mempool info (size, bytes, fees).
    #[method(name = "getmempoolinfo")]
    async fn get_mempool_info(&self) -> Result<MempoolInfoJson, ErrorObjectOwned>;

    /// Returns peer info (connected count).
    #[method(name = "getpeerinfo")]
    async fn get_peer_info(&self) -> Result<PeerInfoJson, ErrorObjectOwned>;

    /// Returns general node info.
    #[method(name = "getinfo")]
    async fn get_info(&self) -> Result<NodeInfoJson, ErrorObjectOwned>;

    /// Returns a block template for mining.
    #[method(name = "getblocktemplate")]
    async fn get_block_template(
        &self,
        mining_address: String,
    ) -> Result<BlockTemplateJson, ErrorObjectOwned>;

    /// Submits a mined block (hex-encoded bincode serialization).
    #[method(name = "submitblock")]
    async fn submit_block(&self, hex_data: String) -> Result<String, ErrorObjectOwned>;

    /// Returns UTXOs owned by the given address.
    #[method(name = "getutxosbyaddress")]
    async fn get_utxos_by_address(&self, address: String) -> Result<Vec<UtxoJson>, ErrorObjectOwned>;

    /// Returns the balance of a cluster by ID.
    #[method(name = "getclusterbalance")]
    async fn get_cluster_balance(&self, cluster_id: String) -> Result<u64, ErrorObjectOwned>;

    /// Returns detailed blockchain state information.
    #[method(name = "getblockchaininfo")]
    async fn get_blockchain_info(&self) -> Result<BlockchainInfoJson, ErrorObjectOwned>;

    /// Returns the current sync status of the node.
    #[method(name = "getsyncstatus")]
    async fn get_sync_status(&self) -> Result<SyncStatusJson, ErrorObjectOwned>;
}

/// Implementation of the Rill JSON-RPC server.
pub struct RpcServerImpl {
    node: Arc<Node>,
}

impl RpcServerImpl {
    /// Create a new RPC server implementation wrapping the given node.
    pub fn new(node: Arc<Node>) -> Self {
        Self { node }
    }
}

#[async_trait]
impl RillRpcServer for RpcServerImpl {
    async fn get_block_count(&self) -> Result<u64, ErrorObjectOwned> {
        let (height, _) = self
            .node
            .chain_tip()
            .map_err(|e| rpc_error(-1, &e.to_string()))?;
        Ok(height)
    }

    async fn get_block_hash(&self, height: u64) -> Result<String, ErrorObjectOwned> {
        let hash = self
            .node
            .get_block_hash(height)
            .map_err(|e| rpc_error(-1, &e.to_string()))?
            .ok_or_else(|| rpc_error(-8, "block height out of range"))?;
        Ok(hex::encode(hash.as_bytes()))
    }

    async fn get_block(&self, hash: String) -> Result<BlockJson, ErrorObjectOwned> {
        let hash256 = parse_hash(&hash)?;
        let block = self
            .node
            .get_block(&hash256)
            .map_err(|e| rpc_error(-1, &e.to_string()))?
            .ok_or_else(|| rpc_error(-5, "block not found"))?;

        // Determine height from the height index (search by hash).
        let (tip_height, _) = self
            .node
            .chain_tip()
            .map_err(|e| rpc_error(-1, &e.to_string()))?;
        let mut height = 0u64;
        for h in 0..=tip_height {
            if let Ok(Some(h_hash)) = self.node.get_block_hash(h) {
                if h_hash == hash256 {
                    height = h;
                    break;
                }
            }
        }

        let tx_ids: Vec<String> = block
            .transactions
            .iter()
            .filter_map(|tx| tx.txid().ok())
            .map(|txid| hex::encode(txid.as_bytes()))
            .collect();

        Ok(BlockJson {
            hash,
            height,
            version: block.header.version,
            prev_hash: hex::encode(block.header.prev_hash.as_bytes()),
            merkle_root: hex::encode(block.header.merkle_root.as_bytes()),
            timestamp: block.header.timestamp,
            difficulty_target: block.header.difficulty_target,
            nonce: block.header.nonce,
            tx_count: block.transactions.len(),
            tx: tx_ids,
        })
    }

    async fn get_block_header(
        &self,
        hash: String,
    ) -> Result<HeaderJson, ErrorObjectOwned> {
        let hash256 = parse_hash(&hash)?;
        let header = self
            .node
            .get_block_header(&hash256)
            .map_err(|e| rpc_error(-1, &e.to_string()))?
            .ok_or_else(|| rpc_error(-5, "block header not found"))?;

        Ok(HeaderJson {
            hash,
            version: header.version,
            prev_hash: hex::encode(header.prev_hash.as_bytes()),
            merkle_root: hex::encode(header.merkle_root.as_bytes()),
            timestamp: header.timestamp,
            difficulty_target: header.difficulty_target,
            nonce: header.nonce,
        })
    }

    async fn get_transaction(
        &self,
        txid: String,
    ) -> Result<TransactionJson, ErrorObjectOwned> {
        let hash = parse_hash(&txid)?;

        // Search mempool first.
        if let Some(tx) = self.node.get_mempool_tx(&hash) {
            return Ok(TransactionJson {
                txid,
                version: tx.version,
                vin_count: tx.inputs.len(),
                vout_count: tx.outputs.len(),
                lock_time: tx.lock_time,
            });
        }

        Err(rpc_error(-5, "transaction not found"))
    }

    async fn send_raw_transaction(
        &self,
        hex_data: String,
    ) -> Result<String, ErrorObjectOwned> {
        let raw = hex::decode(&hex_data)
            .map_err(|_| rpc_error(-22, "invalid hex encoding"))?;

        let (tx, _): (rill_core::types::Transaction, _) =
            bincode::decode_from_slice(&raw, bincode::config::standard())
                .map_err(|e| rpc_error(-22, &format!("decode error: {e}")))?;

        let txid = self
            .node
            .process_transaction(&tx)
            .map_err(|e| rpc_error(-25, &e.to_string()))?;

        Ok(hex::encode(txid.as_bytes()))
    }

    async fn get_mempool_info(&self) -> Result<MempoolInfoJson, ErrorObjectOwned> {
        let (size, bytes, total_fee) = self.node.mempool_info();
        Ok(MempoolInfoJson {
            size,
            bytes,
            total_fee,
        })
    }

    async fn get_peer_info(&self) -> Result<PeerInfoJson, ErrorObjectOwned> {
        Ok(PeerInfoJson {
            connected: self.node.peer_count(),
        })
    }

    async fn get_info(&self) -> Result<NodeInfoJson, ErrorObjectOwned> {
        let (height, tip_hash) = self
            .node
            .chain_tip()
            .map_err(|e| rpc_error(-1, &e.to_string()))?;

        let supply = self
            .node
            .circulating_supply()
            .map_err(|e| rpc_error(-1, &e.to_string()))?;
        let pool = self
            .node
            .decay_pool_balance()
            .map_err(|e| rpc_error(-1, &e.to_string()))?;

        Ok(NodeInfoJson {
            blocks: height,
            bestblockhash: hex::encode(tip_hash.as_bytes()),
            connections: self.node.peer_count(),
            circulating_supply: supply as f64 / COIN as f64,
            decay_pool: pool as f64 / COIN as f64,
        })
    }

    async fn get_block_template(
        &self,
        mining_address: String,
    ) -> Result<BlockTemplateJson, ErrorObjectOwned> {
        // Parse the mining address to extract pubkey hash.
        let address = mining_address
            .parse::<rill_core::address::Address>()
            .map_err(|e| rpc_error(-5, &format!("invalid address: {e}")))?;
        let pubkey_hash = address.pubkey_hash();

        // Get current timestamp.
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Create block template.
        let (height, _) = self
            .node
            .chain_tip()
            .map_err(|e| rpc_error(-1, &e.to_string()))?;
        let block = self
            .node
            .create_block_template(&pubkey_hash, timestamp)
            .map_err(|e| rpc_error(-1, &e.to_string()))?;

        // Serialize transactions.
        let tx_bytes = bincode::encode_to_vec(&block.transactions, bincode::config::standard())
            .map_err(|e| rpc_error(-1, &format!("serialization error: {e}")))?;

        Ok(BlockTemplateJson {
            version: block.header.version,
            prev_hash: hex::encode(block.header.prev_hash.as_bytes()),
            merkle_root: hex::encode(block.header.merkle_root.as_bytes()),
            timestamp: block.header.timestamp,
            difficulty_target: block.header.difficulty_target,
            nonce: block.header.nonce,
            transactions: hex::encode(tx_bytes),
            height: height + 1,
        })
    }

    async fn submit_block(&self, hex_data: String) -> Result<String, ErrorObjectOwned> {
        // Decode the hex-encoded bincode block.
        let raw = hex::decode(&hex_data)
            .map_err(|_| rpc_error(-22, "invalid hex encoding"))?;

        let (block, _): (rill_core::types::Block, _) =
            bincode::decode_from_slice(&raw, bincode::config::standard())
                .map_err(|e| rpc_error(-22, &format!("decode error: {e}")))?;

        // Process the block.
        self.node
            .process_block(&block)
            .map_err(|e| rpc_error(-25, &e.to_string()))?;

        // Return the block hash.
        let hash = block.header.hash();
        Ok(hex::encode(hash.as_bytes()))
    }

    async fn get_utxos_by_address(&self, address: String) -> Result<Vec<UtxoJson>, ErrorObjectOwned> {
        let addr = address
            .parse::<rill_core::address::Address>()
            .map_err(|e| rpc_error(-5, &format!("invalid address: {e}")))?;
        let pubkey_hash = addr.pubkey_hash();

        // Use indexed lookup (O(k) where k = address UTXOs)
        let utxos = self.node.get_utxos_by_address(&pubkey_hash)
            .map_err(|e| rpc_error(-1, &e.to_string()))?;

        let mut result = Vec::new();
        for (outpoint, entry) in utxos {
            result.push(UtxoJson {
                txid: hex::encode(outpoint.txid.as_bytes()),
                index: outpoint.index,
                value: entry.output.value,
                block_height: entry.block_height,
                is_coinbase: entry.is_coinbase,
                cluster_id: hex::encode(entry.cluster_id.as_bytes()),
                pubkey_hash: hex::encode(entry.output.pubkey_hash.as_bytes()),
            });
        }

        Ok(result)
    }

    async fn get_cluster_balance(&self, cluster_id: String) -> Result<u64, ErrorObjectOwned> {
        let hash = parse_hash(&cluster_id)?;
        let balance = self.node.cluster_balance(&hash)
            .map_err(|e| rpc_error(-1, &e.to_string()))?;
        Ok(balance)
    }

    async fn get_blockchain_info(&self) -> Result<BlockchainInfoJson, ErrorObjectOwned> {
        let (height, tip_hash) = self
            .node
            .chain_tip()
            .map_err(|e| rpc_error(-1, &e.to_string()))?;

        let circulating_supply = self
            .node
            .circulating_supply()
            .map_err(|e| rpc_error(-1, &e.to_string()))?;

        let decay_pool_balance = self
            .node
            .decay_pool_balance()
            .map_err(|e| rpc_error(-1, &e.to_string()))?;

        let (mempool_size, _, _) = self.node.mempool_info();

        Ok(BlockchainInfoJson {
            height,
            best_block_hash: hex::encode(tip_hash.as_bytes()),
            circulating_supply,
            decay_pool_balance,
            initial_block_download: self.node.is_ibd(),
            utxo_count: self.node.utxo_count(),
            mempool_size,
            peer_count: self.node.peer_count(),
        })
    }

    async fn get_sync_status(&self) -> Result<SyncStatusJson, ErrorObjectOwned> {
        let (height, tip_hash) = self
            .node
            .chain_tip()
            .map_err(|e| rpc_error(-1, &e.to_string()))?;

        Ok(SyncStatusJson {
            syncing: self.node.is_ibd(),
            current_height: height,
            peer_count: self.node.peer_count(),
            best_block_hash: hex::encode(tip_hash.as_bytes()),
        })
    }
}

/// Start the JSON-RPC server on the given address.
///
/// Returns a [`ServerHandle`] that can be used to stop the server.
pub async fn start_rpc_server(
    addr: &str,
    node: Arc<Node>,
) -> Result<ServerHandle, RillError> {
    let server = Server::builder()
        .build(addr)
        .await
        .map_err(|e| RillError::Storage(format!("RPC server error: {e}")))?;

    let rpc_impl = RpcServerImpl::new(node);
    let handle = server.start(rpc_impl.into_rpc());

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hash_valid() {
        let hex_str = "aa".repeat(32);
        let hash = parse_hash(&hex_str).unwrap();
        assert_eq!(hash, Hash256([0xAA; 32]));
    }

    #[test]
    fn parse_hash_zero() {
        let hex_str = "00".repeat(32);
        let hash = parse_hash(&hex_str).unwrap();
        assert_eq!(hash, Hash256::ZERO);
    }

    #[test]
    fn parse_hash_wrong_length() {
        let err = parse_hash("abcdef").unwrap_err();
        assert!(err.message().contains("64 hex characters"));
    }

    #[test]
    fn parse_hash_invalid_hex() {
        let hex_str = "zz".repeat(32);
        let err = parse_hash(&hex_str).unwrap_err();
        assert!(err.message().contains("invalid hex"));
    }

    #[test]
    fn block_json_serializes() {
        let block = BlockJson {
            hash: "aa".repeat(32),
            height: 42,
            version: 1,
            prev_hash: "00".repeat(32),
            merkle_root: "bb".repeat(32),
            timestamp: 1_000_000,
            difficulty_target: u64::MAX,
            nonce: 0,
            tx_count: 1,
            tx: vec!["cc".repeat(32)],
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"height\":42"));
        assert!(json.contains("\"tx_count\":1"));
    }

    #[test]
    fn mempool_info_json_serializes() {
        let info = MempoolInfoJson {
            size: 10,
            bytes: 5000,
            total_fee: 100_000,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"size\":10"));
    }

    #[test]
    fn node_info_json_serializes() {
        let info = NodeInfoJson {
            blocks: 100,
            bestblockhash: "ff".repeat(32),
            connections: 5,
            circulating_supply: 1050000.0,
            decay_pool: 0.0,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"blocks\":100"));
        assert!(json.contains("\"connections\":5"));
    }

    #[test]
    fn transaction_json_serializes() {
        let tx = TransactionJson {
            txid: "dd".repeat(32),
            version: 1,
            vin_count: 2,
            vout_count: 3,
            lock_time: 0,
        };
        let json = serde_json::to_string(&tx).unwrap();
        assert!(json.contains("\"vin_count\":2"));
    }

    #[test]
    fn header_json_serializes() {
        let header = HeaderJson {
            hash: "aa".repeat(32),
            version: 1,
            prev_hash: "00".repeat(32),
            merkle_root: "bb".repeat(32),
            timestamp: 1_000_000,
            difficulty_target: u64::MAX,
            nonce: 42,
        };
        let json = serde_json::to_string(&header).unwrap();
        assert!(json.contains("\"nonce\":42"));
    }

    #[test]
    fn peer_info_json_serializes() {
        let info = PeerInfoJson { connected: 3 };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"connected\":3"));
    }

    #[test]
    fn utxo_json_serializes() {
        let utxo = UtxoJson {
            txid: "aa".repeat(32),
            index: 0,
            value: 1_000_000_000,
            block_height: 42,
            is_coinbase: true,
            cluster_id: "bb".repeat(32),
            pubkey_hash: "cc".repeat(32),
        };
        let json = serde_json::to_string(&utxo).unwrap();
        assert!(json.contains("\"index\":0"));
        assert!(json.contains("\"value\":1000000000"));
        assert!(json.contains("\"block_height\":42"));
        assert!(json.contains("\"is_coinbase\":true"));
    }

    #[test]
    fn parse_hash_for_cluster_balance() {
        let cluster_id = "bb".repeat(32);
        let hash = parse_hash(&cluster_id).unwrap();
        assert_eq!(hash, Hash256([0xBB; 32]));
    }

    #[test]
    fn sync_status_json_serializes() {
        let status = SyncStatusJson {
            syncing: false,
            current_height: 100,
            peer_count: 3,
            best_block_hash: "aa".repeat(32),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"syncing\":false"));
        assert!(json.contains("\"current_height\":100"));
        assert!(json.contains("\"peer_count\":3"));
    }

    #[test]
    fn blockchain_info_json_serializes() {
        let info = BlockchainInfoJson {
            height: 500,
            best_block_hash: "aa".repeat(32),
            circulating_supply: 5_000_000_000_000,
            decay_pool_balance: 0,
            initial_block_download: true,
            utxo_count: 42,
            mempool_size: 5,
            peer_count: 3,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"height\":500"));
        assert!(json.contains("\"initial_block_download\":true"));
        assert!(json.contains("\"utxo_count\":42"));
        assert!(json.contains("\"mempool_size\":5"));
        assert!(json.contains("\"peer_count\":3"));
    }
}
