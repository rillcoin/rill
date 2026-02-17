//! Node configuration for the Rill full node.
//!
//! Provides [`NodeConfig`] with defaults for data directory, RPC binding,
//! and network settings. The configuration can be customized programmatically
//! or loaded from a config file in the future.

use std::path::PathBuf;

use rill_core::constants::{NetworkType, DEFAULT_RPC_PORT};
use rill_network::NetworkConfig;

/// Block pruning mode.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PruneMode {
    /// Keep all blocks (default).
    #[default]
    Full,
    /// Keep only the most recent N blocks' full data. Headers and undo data
    /// are always preserved.
    Pruned(u64),
}

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
    /// Block pruning mode.
    pub prune_mode: PruneMode,
    /// Which network this node is participating in.
    ///
    /// Controls magic bytes, default ports, and the data directory sub-path.
    pub network_type: NetworkType,
}

impl Default for NodeConfig {
    fn default() -> Self {
        let network_type = NetworkType::default();
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rill")
            .join(network_type.data_dir_suffix());

        Self {
            data_dir,
            rpc_bind: "127.0.0.1".to_string(),
            rpc_port: DEFAULT_RPC_PORT,
            network: NetworkConfig::default(),
            log_level: "info".to_string(),
            prune_mode: PruneMode::default(),
            network_type,
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
    fn default_data_dir_ends_with_network_suffix() {
        let cfg = NodeConfig::default();
        // Default is Mainnet, so the last component should be "mainnet".
        assert!(
            cfg.data_dir.ends_with("mainnet"),
            "data_dir should end with 'mainnet': {:?}",
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

    #[test]
    fn default_prune_mode_is_full() {
        let cfg = NodeConfig::default();
        assert_eq!(cfg.prune_mode, PruneMode::Full);
    }

    #[test]
    fn prune_mode_pruned_variant() {
        let mode = PruneMode::Pruned(1000);
        assert_eq!(mode, PruneMode::Pruned(1000));
        assert_ne!(mode, PruneMode::Full);
    }

    #[test]
    fn config_default_network_type() {
        let cfg = NodeConfig::default();
        assert_eq!(cfg.network_type, NetworkType::Mainnet);
    }

    #[test]
    fn config_data_dir_includes_network() {
        use rill_core::constants::NetworkType;

        let mainnet_cfg = NodeConfig::default();
        assert!(
            mainnet_cfg.data_dir.ends_with("mainnet"),
            "mainnet data_dir should end with 'mainnet': {:?}",
            mainnet_cfg.data_dir
        );

        // Build a testnet config manually to verify the suffix changes.
        let testnet_cfg = NodeConfig {
            data_dir: PathBuf::from("/tmp/rill").join(NetworkType::Testnet.data_dir_suffix()),
            network_type: NetworkType::Testnet,
            ..NodeConfig::default()
        };
        assert!(
            testnet_cfg.data_dir.ends_with("testnet"),
            "testnet data_dir should end with 'testnet': {:?}",
            testnet_cfg.data_dir
        );

        let regtest_cfg = NodeConfig {
            data_dir: PathBuf::from("/tmp/rill").join(NetworkType::Regtest.data_dir_suffix()),
            network_type: NetworkType::Regtest,
            ..NodeConfig::default()
        };
        assert!(
            regtest_cfg.data_dir.ends_with("regtest"),
            "regtest data_dir should end with 'regtest': {:?}",
            regtest_cfg.data_dir
        );
    }
}
