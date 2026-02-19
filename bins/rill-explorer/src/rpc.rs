use anyhow::{bail, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

pub struct RpcClient {
    client: Client,
    endpoint: String,
}

impl RpcClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("build reqwest client"),
            endpoint: endpoint.to_owned(),
        }
    }

    pub async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        let body = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });
        let resp: Value = self
            .client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if let Some(err) = resp.get("error") {
            if !err.is_null() {
                bail!("RPC error: {}", err);
            }
        }
        Ok(serde_json::from_value(resp["result"].clone())?)
    }

    // ── Convenience wrappers ──────────────────────────────────────────────────

    pub async fn get_block_count(&self) -> Result<u64> {
        self.call("getblockcount", json!([])).await
    }

    pub async fn get_block_hash(&self, height: u64) -> Result<String> {
        self.call("getblockhash", json!([height])).await
    }

    pub async fn get_block(&self, hash: &str) -> Result<Value> {
        self.call("getblock", json!([hash])).await
    }

    pub async fn get_transaction(&self, txid: &str) -> Result<Value> {
        self.call("gettransaction", json!([txid])).await
    }

    pub async fn get_blockchain_info(&self) -> Result<Value> {
        self.call("getblockchaininfo", json!([])).await
    }

    pub async fn get_mempool_info(&self) -> Result<Value> {
        self.call("getmempoolinfo", json!([])).await
    }

    pub async fn get_utxos_by_address(&self, address: &str) -> Result<Vec<Value>> {
        self.call("getutxosbyaddress", json!([address])).await
    }
}
