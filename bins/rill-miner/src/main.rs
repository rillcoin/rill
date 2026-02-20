//! Rill miner: standalone mining binary using RPC.
//!
//! Connects to a rill-node RPC server, fetches block templates, mines them
//! using mock PoW (SHA-256 double-hash), and submits found blocks.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::params::ArrayParams;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use rill_consensus::engine::mine_block;
use rill_core::types::Block;

/// CLI arguments for the miner.
#[derive(Debug, Parser)]
#[command(name = "rill-miner")]
#[command(about = "RillCoin standalone miner", long_about = None)]
struct Args {
    /// RPC server endpoint.
    #[arg(long, default_value = "http://127.0.0.1:18332")]
    rpc_endpoint: String,

    /// Mining address to receive block rewards (required).
    #[arg(long)]
    mining_address: String,

    /// Number of mining threads.
    #[arg(long, default_value = "1")]
    threads: usize,

    /// Log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    log_level: String,
}

/// JSON RPC response for getblocktemplate.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlockTemplateJson {
    version: u64,
    prev_hash: String,
    merkle_root: String,
    timestamp: u64,
    difficulty_target: u64,
    nonce: u64,
    transactions: String,
    height: u64,
}

/// Statistics tracker for mining.
struct MiningStats {
    blocks_found: AtomicU64,
    hashes_computed: AtomicU64,
    start_time: Instant,
}

impl MiningStats {
    fn new() -> Self {
        Self {
            blocks_found: AtomicU64::new(0),
            hashes_computed: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    fn increment_hashes(&self, count: u64) {
        self.hashes_computed.fetch_add(count, Ordering::Relaxed);
    }

    fn increment_blocks(&self) {
        self.blocks_found.fetch_add(1, Ordering::Relaxed);
    }

    fn hashrate(&self) -> f64 {
        let hashes = self.hashes_computed.load(Ordering::Relaxed) as f64;
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            hashes / elapsed
        } else {
            0.0
        }
    }

    fn blocks_found(&self) -> u64 {
        self.blocks_found.load(Ordering::Relaxed)
    }
}

/// Fetch a block template from the RPC server.
async fn fetch_template(
    client: &HttpClient,
    mining_address: &str,
) -> Result<BlockTemplateJson> {
    let mut params = ArrayParams::new();
    params.insert(mining_address).ok();
    let response: BlockTemplateJson = client
        .request("getblocktemplate", params)
        .await
        .context("failed to fetch block template")?;
    Ok(response)
}

/// Submit a mined block to the RPC server.
async fn submit_block(client: &HttpClient, block: &Block) -> Result<String> {
    // Serialize block to bincode then hex.
    let encoded = bincode::encode_to_vec(block, bincode::config::standard())
        .context("failed to serialize block")?;
    let hex_data = hex::encode(encoded);

    let mut params = ArrayParams::new();
    params.insert(hex_data).ok();
    let hash: String = client
        .request("submitblock", params)
        .await
        .context("failed to submit block")?;
    Ok(hash)
}

/// Reconstruct a Block from the template JSON.
fn template_to_block(template: &BlockTemplateJson) -> Result<Block> {
    let prev_hash_bytes =
        hex::decode(&template.prev_hash).context("invalid prev_hash hex")?;
    let prev_hash: [u8; 32] = prev_hash_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("prev_hash must be 32 bytes"))?;

    let merkle_root_bytes =
        hex::decode(&template.merkle_root).context("invalid merkle_root hex")?;
    let merkle_root: [u8; 32] = merkle_root_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("merkle_root must be 32 bytes"))?;

    let tx_bytes = hex::decode(&template.transactions).context("invalid transactions hex")?;
    let (transactions, _): (Vec<rill_core::types::Transaction>, _) =
        bincode::decode_from_slice(&tx_bytes, bincode::config::standard())
            .context("failed to decode transactions")?;

    Ok(Block {
        header: rill_core::types::BlockHeader {
            version: template.version,
            prev_hash: rill_core::types::Hash256(prev_hash),
            merkle_root: rill_core::types::Hash256(merkle_root),
            timestamp: template.timestamp,
            difficulty_target: template.difficulty_target,
            nonce: template.nonce,
        },
        transactions,
    })
}

/// Main mining loop for a single thread.
async fn mining_worker(
    client: HttpClient,
    mining_address: String,
    stats: Arc<MiningStats>,
    running: Arc<AtomicBool>,
) {
    let mut last_template_height = 0u64;
    let mut current_block: Option<Block> = None;

    while running.load(Ordering::Relaxed) {
        // Fetch a new template periodically or if we don't have one.
        if current_block.is_none() {
            match fetch_template(&client, &mining_address).await {
                Ok(template) => {
                    if template.height != last_template_height {
                        info!(
                            "new template at height {}, difficulty_target={}",
                            template.height, template.difficulty_target
                        );
                        last_template_height = template.height;
                    }
                    match template_to_block(&template) {
                        Ok(block) => {
                            current_block = Some(block);
                        }
                        Err(e) => {
                            error!("failed to parse template: {e}");
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    }
                }
                Err(e) => {
                    error!("failed to fetch template: {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            }
        }

        // Mine the block (try up to 100k nonces per iteration).
        if let Some(ref mut block) = current_block {
            let start_nonce = block.header.nonce;
            let max_nonce = start_nonce.saturating_add(100_000);

            if mine_block(block, max_nonce) {
                // Found a valid block!
                let hash = block.header.hash();
                let height = last_template_height;
                info!(
                    "FOUND BLOCK! height={} hash={} nonce={}",
                    height, hash, block.header.nonce
                );

                // Submit the block.
                match submit_block(&client, block).await {
                    Ok(submitted_hash) => {
                        info!("block submitted successfully: {}", submitted_hash);
                        stats.increment_blocks();
                        // Throttle mining to prevent difficulty death spiral on
                        // low-peer-count testnets. Without this, the miner outpaces
                        // the LWMA difficulty window, causing runaway difficulty.
                        info!("waiting 30s before next block to stabilize difficulty...");
                        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                        current_block = None; // Fetch new template.
                    }
                    Err(e) => {
                        error!("failed to submit block: {e}");
                        current_block = None; // Fetch new template.
                    }
                }
            } else {
                // Didn't find a block in this iteration. Continue from where we left off.
                stats.increment_hashes(max_nonce - start_nonce + 1);
                block.header.nonce = max_nonce.saturating_add(1);

                // Periodically check for new template (every ~1M nonces).
                if block.header.nonce % 1_000_000 == 0 {
                    current_block = None; // Force re-fetch.
                }
            }
        }

        // Yield to avoid hogging the runtime.
        tokio::task::yield_now().await;
    }

    info!("mining worker shutting down");
}

/// Log mining statistics periodically.
async fn stats_logger(stats: Arc<MiningStats>, running: Arc<AtomicBool>) {
    while running.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let hashrate = stats.hashrate();
        let blocks = stats.blocks_found();
        info!(
            "hashrate: {:.2} H/s | blocks found: {}",
            hashrate, blocks
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&args.log_level)),
        )
        .init();

    info!("rill-miner v{}", env!("CARGO_PKG_VERSION"));
    info!("Wealth should flow like water.");
    info!("RPC endpoint: {}", args.rpc_endpoint);
    info!("Mining address: {}", args.mining_address);
    info!("Mining threads: {}", args.threads);

    // Create RPC client.
    let client = HttpClientBuilder::default()
        .build(&args.rpc_endpoint)
        .context("failed to create RPC client")?;

    // Verify connection.
    let _block_count: u64 = client
        .request("getblockcount", ArrayParams::new())
        .await
        .context("failed to connect to RPC server")?;
    info!("connected to RPC server");

    // Create shared stats and running flag.
    let stats = Arc::new(MiningStats::new());
    let running = Arc::new(AtomicBool::new(true));

    // Set up signal handler for graceful shutdown.
    let running_clone = Arc::clone(&running);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        warn!("received SIGINT, shutting down...");
        running_clone.store(false, Ordering::Relaxed);
    });

    // Spawn stats logger.
    let stats_clone = Arc::clone(&stats);
    let running_clone = Arc::clone(&running);
    tokio::spawn(stats_logger(stats_clone, running_clone));

    // Spawn mining workers.
    let mut handles = vec![];
    for i in 0..args.threads {
        let client_clone = client.clone();
        let address_clone = args.mining_address.clone();
        let stats_clone = Arc::clone(&stats);
        let running_clone = Arc::clone(&running);

        let handle = tokio::spawn(async move {
            info!("starting mining thread {}", i);
            mining_worker(client_clone, address_clone, stats_clone, running_clone).await;
        });
        handles.push(handle);
    }

    // Wait for all workers to finish.
    for handle in handles {
        handle.await.ok();
    }

    info!("miner shutdown complete");
    Ok(())
}
