//! Rill full node binary.
//!
//! Starts a full node with RocksDB storage, JSON-RPC server, and P2P networking.
//! Processes blocks and transactions, validates the chain, and serves RPC queries.

use std::path::PathBuf;
use std::process;

use clap::Parser;
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

    /// Disable P2P networking (single-node mode)
    #[arg(long)]
    no_network: bool,
}

impl Args {
    /// Convert CLI args into a NodeConfig.
    fn into_config(self) -> NodeConfig {
        let mut config = NodeConfig::default();

        if let Some(data_dir) = self.data_dir {
            config.data_dir = data_dir;
        }

        config.rpc_bind = self.rpc_bind;
        config.rpc_port = self.rpc_port;
        config.log_level = self.log_level;

        // Set P2P listen address and port.
        config.network.listen_addr = self.p2p_listen_addr;
        config.network.listen_port = self.p2p_listen_port;

        // Set bootstrap peers.
        config.network.bootstrap_peers = self.bootstrap_peers;

        // Disable network if requested.
        if self.no_network {
            config.network.bootstrap_peers.clear();
            config.network.enable_mdns = false;
        }

        config
    }
}

#[tokio::main]
async fn main() {
    // Parse CLI arguments.
    let args = Args::parse();
    let config = args.into_config();

    // Initialize logging.
    init_logging(&config.log_level);

    info!("🌊 Rill Full Node v{}", env!("CARGO_PKG_VERSION"));
    info!("Wealth should flow like water.");
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

    info!("✓ Node initialized");

    // Check chain tip at startup.
    if let Ok((height, hash)) = node.chain_tip() {
        info!("chain_tip: height={} hash={}", height, hex::encode(hash.as_bytes()));
    }

    // Start RPC server.
    let rpc_handle = match start_rpc_server(&config.rpc_addr(), node.clone()).await {
        Ok(handle) => {
            info!("✓ RPC server listening on {}", config.rpc_addr());
            handle
        }
        Err(e) => {
            error!("failed to start RPC server: {}", e);
            process::exit(1);
        }
    };

    info!("✓ Rill node running (Ctrl+C to stop)");

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
    info!("✓ RPC server stopped");
    info!("✓ Rill node shutdown complete");
}

/// Initialize tracing subscriber with the given log level.
fn init_logging(level_str: &str) {
    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Default to the specified level, but also allow more granular control.
        EnvFilter::new(level_str)
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_level(true))
        .init();
}
