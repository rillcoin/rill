//! Rill full node binary.
//!
//! Starts a full node with RocksDB storage, JSON-RPC server, and P2P networking.
//! Processes blocks and transactions, validates the chain, and serves RPC queries.

use std::path::PathBuf;
use std::process;

use clap::Parser;
use rill_core::constants::NetworkType;
use rill_network::NetworkConfig;
use rill_node_lib::{start_rpc_server, Node, NodeConfig};
use tracing::{error, info};

/// Rill full node — "Wealth should flow like water."
#[derive(Parser, Debug)]
#[command(
    name = "rill-node",
    version,
    about = "Rill full node with RocksDB storage and JSON-RPC server"
)]
struct Args {
    /// Data directory for blockchain storage and config
    #[arg(long, default_value = None)]
    data_dir: Option<PathBuf>,

    /// RPC server bind address
    #[arg(long, default_value = "127.0.0.1")]
    rpc_bind: String,

    /// RPC server port
    #[arg(long, default_value_t = rill_core::constants::DEFAULT_RPC_PORT)]
    rpc_port: u16,

    /// P2P listen address
    #[arg(long, default_value = "0.0.0.0")]
    p2p_listen_addr: String,

    /// P2P listen port
    #[arg(long, default_value_t = rill_core::constants::DEFAULT_P2P_PORT)]
    p2p_listen_port: u16,

    /// Bootstrap peers (comma-separated)
    #[arg(long, value_delimiter = ',')]
    bootstrap_peers: Vec<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Log output format ("text" or "json")
    #[arg(long, default_value = "text")]
    log_format: String,

    /// Disable P2P networking (single-node mode)
    #[arg(long)]
    no_network: bool,

    /// Connect to the public test network (testnet) instead of mainnet.
    ///
    /// Uses separate magic bytes, ports, and data directory.
    #[arg(long, conflicts_with = "regtest")]
    testnet: bool,

    /// Run in local regression-test mode (regtest).
    ///
    /// Minimal proof-of-work difficulty; intended for development and testing.
    #[arg(long, conflicts_with = "testnet")]
    regtest: bool,
}

impl Args {
    /// Convert CLI args into a NodeConfig.
    fn into_config(self) -> (NodeConfig, String) {
        // Determine network type from CLI flags.
        let network_type = if self.regtest {
            NetworkType::Regtest
        } else if self.testnet {
            NetworkType::Testnet
        } else {
            NetworkType::Mainnet
        };

        let default_data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rill")
            .join(network_type.data_dir_suffix());

        let data_dir = self.data_dir.unwrap_or(default_data_dir);

        let mut bootstrap_peers = self.bootstrap_peers;
        let mut enable_mdns = true;

        // Disable network if requested.
        if self.no_network {
            bootstrap_peers.clear();
            enable_mdns = false;
        }

        let default_network = NetworkConfig::default();
        let network = NetworkConfig {
            listen_addr: self.p2p_listen_addr,
            listen_port: self.p2p_listen_port,
            bootstrap_peers,
            enable_mdns,
            node_key_path: Some(data_dir.join("node.key")),
            ..default_network
        };

        let config = NodeConfig {
            network_type,
            data_dir,
            rpc_bind: self.rpc_bind,
            rpc_port: self.rpc_port,
            log_level: self.log_level,
            network,
            ..NodeConfig::default()
        };

        (config, self.log_format)
    }
}

#[tokio::main]
async fn main() {
    // Parse CLI arguments.
    let args = Args::parse();
    let (config, log_format) = args.into_config();

    // Initialize logging.
    init_logging(&config.log_level, &log_format);

    info!("Rill Full Node v{}", env!("CARGO_PKG_VERSION"));
    info!("Wealth should flow like water.");
    info!("network: {:?}", config.network_type);
    info!("data_dir: {:?}", config.data_dir);
    info!("rpc_addr: {}", config.rpc_addr());
    info!("p2p_listen: {}", config.network.listen_multiaddr());
    info!("bootstrap_peers: {:?}", config.network.bootstrap_peers);
    info!("enable_mdns: {}", config.network.enable_mdns);

    // Create data directory if it doesn't exist.
    if let Err(e) = std::fs::create_dir_all(&config.data_dir) {
        error!("failed to create data_dir: {}", e);
        process::exit(1);
    }

    // Start the node.
    let node = match Node::new(config.clone()).await {
        Ok(n) => n,
        Err(e) => {
            error!("failed to start node: {}", e);
            process::exit(1);
        }
    };

    info!("Node initialized");

    // Check chain tip at startup.
    if let Ok((height, hash)) = node.chain_tip() {
        info!(
            "chain_tip: height={} hash={}",
            height,
            hex::encode(hash.as_bytes())
        );
    }

    // Start RPC server.
    let rpc_handle = match start_rpc_server(&config.rpc_addr(), node.clone()).await {
        Ok(handle) => {
            info!("RPC server listening on {}", config.rpc_addr());
            handle
        }
        Err(e) => {
            error!("failed to start RPC server: {}", e);
            process::exit(1);
        }
    };

    info!("Rill node running (Ctrl+C to stop)");

    // Set up Ctrl+C handler.
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        info!("received Ctrl+C, shutting down...");
    };

    // Run the node event loop and wait for shutdown.
    tokio::select! {
        _ = node.run() => {
            info!("node event loop exited");
        }
        _ = shutdown_signal => {
            info!("shutdown signal received");
        }
    }

    // Stop RPC server.
    rpc_handle.stop().ok();
    info!("RPC server stopped");
    info!("Rill node shutdown complete");
}

/// Initialize tracing subscriber with the given log level and output format.
///
/// Pass `format = "json"` for structured JSON output (suitable for log
/// aggregation pipelines). Any other value defaults to human-readable text.
fn init_logging(level_str: &str, format: &str) {
    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level_str));

    if format == "json" {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_target(true).with_level(true))
            .init();
    }
}
