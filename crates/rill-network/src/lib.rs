//! # rill-network — P2P layer using libp2p.
//!
//! Provides Gossipsub-based block and transaction propagation, Kademlia DHT
//! peer routing, Noise encryption over TCP/Yamux, and optional mDNS for
//! local peer discovery.
//!
//! The main entry point is [`NetworkNode::start`], which spawns a background
//! swarm task and returns a handle implementing [`rill_core::traits::NetworkService`].

pub mod behaviour;
pub mod config;
pub mod protocol;
pub mod service;

pub use config::NetworkConfig;
pub use protocol::{NetworkMessage, BLOCKS_TOPIC, TXS_TOPIC};
pub use service::{NetworkEvent, NetworkNode};
