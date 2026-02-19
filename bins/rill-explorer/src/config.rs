use anyhow::Result;

pub struct Config {
    pub rpc_endpoint: String,
    pub bind_addr: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            rpc_endpoint: std::env::var("EXPLORER_RPC_ENDPOINT")
                .unwrap_or_else(|_| "http://127.0.0.1:18332".into()),
            bind_addr: std::env::var("EXPLORER_BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8081".into()),
        })
    }
}
