//! Network configuration for the Rill P2P layer.

use rill_core::constants::DEFAULT_P2P_PORT;
use std::time::Duration;

/// Configuration for the P2P network node.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// IP address to listen on.
    pub listen_addr: String,
    /// TCP port to listen on.
    pub listen_port: u16,
    /// Bootstrap peer multiaddresses to connect on startup.
    pub bootstrap_peers: Vec<String>,
    /// Enable mDNS peer discovery (useful for local/testnet).
    pub enable_mdns: bool,
    /// Gossipsub heartbeat interval.
    pub gossipsub_heartbeat: Duration,
    /// Maximum number of connected peers.
    pub max_peers: usize,
    /// Timeout for outbound dial attempts.
    pub dial_timeout: Duration,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port: DEFAULT_P2P_PORT,
            bootstrap_peers: Vec::new(),
            enable_mdns: true,
            gossipsub_heartbeat: Duration::from_secs(1),
            max_peers: 50,
            dial_timeout: Duration::from_secs(10),
        }
    }
}

impl NetworkConfig {
    /// Configuration preset for testnet: mDNS enabled, no bootstrap peers.
    pub fn testnet() -> Self {
        Self {
            enable_mdns: true,
            ..Self::default()
        }
    }

    /// Configuration preset for mainnet: mDNS disabled, well-known bootstrap peers.
    pub fn mainnet() -> Self {
        Self {
            enable_mdns: false,
            ..Self::default()
        }
    }

    /// Build the libp2p multiaddr string for the configured listen address and port.
    pub fn listen_multiaddr(&self) -> String {
        format!("/ip4/{}/tcp/{}", self.listen_addr, self.listen_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_default_port() {
        let cfg = NetworkConfig::default();
        assert_eq!(cfg.listen_port, DEFAULT_P2P_PORT);
    }

    #[test]
    fn default_config_has_mdns_enabled() {
        let cfg = NetworkConfig::default();
        assert!(cfg.enable_mdns);
    }

    #[test]
    fn default_config_has_no_bootstrap_peers() {
        let cfg = NetworkConfig::default();
        assert!(cfg.bootstrap_peers.is_empty());
    }

    #[test]
    fn listen_multiaddr_format() {
        let cfg = NetworkConfig::default();
        let addr = cfg.listen_multiaddr();
        assert_eq!(addr, format!("/ip4/0.0.0.0/tcp/{DEFAULT_P2P_PORT}"));
    }

    #[test]
    fn listen_multiaddr_custom() {
        let cfg = NetworkConfig {
            listen_addr: "127.0.0.1".to_string(),
            listen_port: 9999,
            ..NetworkConfig::default()
        };
        assert_eq!(cfg.listen_multiaddr(), "/ip4/127.0.0.1/tcp/9999");
    }

    #[test]
    fn testnet_has_mdns() {
        let cfg = NetworkConfig::testnet();
        assert!(cfg.enable_mdns);
    }

    #[test]
    fn mainnet_disables_mdns() {
        let cfg = NetworkConfig::mainnet();
        assert!(!cfg.enable_mdns);
    }

    #[test]
    fn config_is_clone_and_debug() {
        let cfg = NetworkConfig::default();
        let cfg2 = cfg.clone();
        assert_eq!(format!("{:?}", cfg), format!("{:?}", cfg2));
    }
}
