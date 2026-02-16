//! Composite libp2p [`NetworkBehaviour`] for the Rill P2P protocol.
//!
//! Combines Gossipsub (block/tx propagation), Kademlia (peer routing),
//! Identify (protocol handshake), and optional mDNS (local discovery).

use crate::protocol::MAX_MESSAGE_SIZE;
use libp2p::gossipsub;
use libp2p::identity::Keypair;
use libp2p::kad;
use libp2p::request_response;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{identify, mdns};
use sha2::{Digest, Sha256};
use std::time::Duration;

/// Rill protocol version string used in Identify.
pub const PROTOCOL_VERSION: &str = "/rill/1.0.0";

/// Kademlia protocol name for Rill DHT.
pub const KAD_PROTOCOL: &[u8] = b"/rill/kad/1.0.0";

/// Composite network behaviour combining all Rill sub-protocols.
#[derive(NetworkBehaviour)]
pub struct RillBehaviour {
    /// Gossipsub for block and transaction propagation.
    pub gossipsub: gossipsub::Behaviour,
    /// Kademlia DHT for peer routing.
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    /// Identify protocol for peer handshake and address exchange.
    pub identify: identify::Behaviour,
    /// Optional mDNS for local peer discovery (disabled on mainnet).
    pub mdns: libp2p::swarm::behaviour::toggle::Toggle<mdns::tokio::Behaviour>,
    /// Request-response for point-to-point block/header sync.
    pub request_response: request_response::Behaviour<crate::protocol::RillCodec>,
}

/// Build a gossipsub behaviour with content-addressed message IDs.
///
/// Uses SHA-256 hashing of message data for deduplication. Configured
/// with strict validation mode and the protocol's max transmit size.
pub fn build_gossipsub(heartbeat: Duration) -> Result<gossipsub::Behaviour, String> {
    let message_id_fn = |message: &gossipsub::Message| {
        let hash = Sha256::digest(&message.data);
        gossipsub::MessageId::from(hash.to_vec())
    };

    let config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(heartbeat)
        .validation_mode(gossipsub::ValidationMode::Strict)
        .max_transmit_size(MAX_MESSAGE_SIZE)
        .message_id_fn(message_id_fn)
        .build()
        .map_err(|e| format!("gossipsub config error: {e}"))?;

    gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(Keypair::generate_ed25519()),
        config,
    )
    .map_err(|e| format!("gossipsub behaviour error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_constant() {
        assert_eq!(PROTOCOL_VERSION, "/rill/1.0.0");
    }

    #[test]
    fn kad_protocol_constant() {
        assert_eq!(KAD_PROTOCOL, b"/rill/kad/1.0.0");
    }

    #[test]
    fn build_gossipsub_succeeds() {
        let gs = build_gossipsub(Duration::from_secs(1));
        assert!(gs.is_ok());
    }
}
