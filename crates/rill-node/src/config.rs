//! Node configuration for the Rill full node.
//!
//! Provides [`NodeConfig`] with defaults for data directory, RPC binding,
//! and network settings. The configuration can be customized programmatically
//! or loaded from a config file in the future.

use std::path::PathBuf;

use rill_core::constants::DEFAULT_RPC_PORT;
use rill_network::NetworkConfig;

/// Configuration for a full node instance.
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Root directory for all persistent data.
    pub data_dir: PathBuf,
    /// IP address for the JSON-RPC server to bind to.
    pub rpc_bind: String,
    /// Port for the JSON-RPC server.
    pub rpc_port: u16,
    /// P2P network configuration.
    pub network: NetworkConfig,
    /// Log level filter string (e.g. "info", "debug", "rill_node=trace").
    pub log_level: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rill");

        Self {
            data_dir,
            rpc_bind: "127.0.0.1".to_string(),
            rpc_port: DEFAULT_RPC_PORT,
            network: NetworkConfig::default(),
            log_level: "info".to_string(),
        }
    }
}

impl NodeConfig {
    /// Path to the RocksDB chain data directory.
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("chaindata")
    }

    /// Socket address string for the RPC server.
    pub fn rpc_addr(&self) -> String {
        format!("{}:{}", self.rpc_bind, self.rpc_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rpc_port() {
        let cfg = NodeConfig::default();
        assert_eq!(cfg.rpc_port, DEFAULT_RPC_PORT);
    }

    #[test]
    fn default_rpc_bind_is_localhost() {
        let cfg = NodeConfig::default();
        assert_eq!(cfg.rpc_bind, "127.0.0.1");
    }

    #[test]
    fn default_log_level_is_info() {
        let cfg = NodeConfig::default();
        assert_eq!(cfg.log_level, "info");
    }

    #[test]
    fn default_data_dir_ends_with_rill() {
        let cfg = NodeConfig::default();
        assert!(
            cfg.data_dir.ends_with("rill"),
            "data_dir should end with 'rill': {:?}",
            cfg.data_dir
        );
    }

    #[test]
    fn rpc_addr_format() {
        let cfg = NodeConfig::default();
        let addr = cfg.rpc_addr();
        assert_eq!(addr, format!("127.0.0.1:{DEFAULT_RPC_PORT}"));
    }

    #[test]
    fn rpc_addr_custom() {
        let cfg = NodeConfig {
            rpc_bind: "0.0.0.0".to_string(),
            rpc_port: 9999,
            ..NodeConfig::default()
        };
        assert_eq!(cfg.rpc_addr(), "0.0.0.0:9999");
    }

    #[test]
    fn db_path_appends_chaindata() {
        let cfg = NodeConfig {
            data_dir: PathBuf::from("/tmp/rill-test"),
            ..NodeConfig::default()
        };
        assert_eq!(cfg.db_path(), PathBuf::from("/tmp/rill-test/chaindata"));
    }

    #[test]
    fn config_is_clone_and_debug() {
        let cfg = NodeConfig::default();
        let cfg2 = cfg.clone();
        let debug = format!("{cfg2:?}");
        assert!(debug.contains("NodeConfig"));
    }
}
