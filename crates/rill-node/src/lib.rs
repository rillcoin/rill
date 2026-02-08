//! # rill-node — Full node: RocksDB, RPC, orchestration.
//!
//! Composes all Rill subsystems into a running full node:
//! - [`storage::RocksStore`] — persistent chain state backed by RocksDB
//! - [`node::Node`] — event loop wiring storage, mempool, consensus, and network
//! - [`rpc`] — JSON-RPC server for external access
//! - [`config::NodeConfig`] — node configuration

pub mod config;
pub mod node;
pub mod rpc;
pub mod storage;

pub use config::NodeConfig;
pub use node::Node;
pub use rpc::start_rpc_server;
pub use storage::RocksStore;
