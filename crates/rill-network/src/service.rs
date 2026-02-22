//! Network node implementing the [`NetworkService`] trait.
//!
//! Uses a command-channel architecture: the [`NetworkNode`] sends commands
//! over an mpsc channel to a background swarm task running on tokio.
//! This bridges the synchronous [`NetworkService`] trait with async libp2p.

use crate::behaviour::{self, RillBehaviour, PROTOCOL_VERSION};
use crate::config::NetworkConfig;
use crate::protocol::{NetworkMessage, RillCodec, RillRequest, RillResponse, BLOCKS_TOPIC, REQ_RESP_PROTOCOL, TXS_TOPIC};
use libp2p::gossipsub::{self, IdentTopic};
use libp2p::identity::Keypair;
use libp2p::kad;
use libp2p::multiaddr::Protocol;
use libp2p::request_response;
use libp2p::swarm::SwarmEvent;
use libp2p::{identify, mdns, Multiaddr, PeerId, StreamProtocol, SwarmBuilder};
use rill_core::error::NetworkError;
use rill_core::traits::NetworkService;
use rill_core::types::{Block, Hash256, Transaction};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

/// A storage query forwarded from a peer's request-response request.
///
/// The node processes this query against its storage and sends the response
/// back via [`Command::SendResponse`].
pub struct StorageQuery {
    /// The request from the peer.
    pub request: RillRequest,
    /// The peer that sent the request.
    pub peer: PeerId,
    /// The libp2p response channel to send the response back.
    pub response_channel: request_response::ResponseChannel<RillResponse>,
}

/// Commands sent from [`NetworkNode`] to the background swarm task.
#[derive(Debug)]
enum Command {
    /// Publish a message to a gossipsub topic.
    Publish { topic: String, data: Vec<u8> },
    /// Dial a remote peer address (used by connect_peer).
    Dial(Multiaddr),
    /// Send a request-response request to a specific peer.
    SendRequest { peer: PeerId, request: RillRequest },
    /// Send a response back to a peer via their response channel.
    SendResponse {
        channel: request_response::ResponseChannel<RillResponse>,
        response: RillResponse,
    },
    /// Shut down the swarm event loop.
    Shutdown,
}

/// Events emitted by the network layer for consumption by higher layers.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A new block was received from a peer.
    BlockReceived(Block),
    /// A new transaction was received from a peer.
    TransactionReceived(Transaction),
    /// A peer requested a block by hash.
    BlockRequested(Hash256),
    /// A peer requested headers from locator hashes.
    HeadersRequested(Vec<Hash256>),
    /// A new peer connected.
    PeerConnected(PeerId),
    /// A peer disconnected.
    PeerDisconnected(PeerId),
    /// A peer requested our chain tip.
    ChainTipRequested(PeerId),
    /// A response was received to one of our requests.
    RequestResponse {
        /// The peer that sent the response.
        peer: PeerId,
        /// The response payload.
        response: RillResponse,
    },
}

/// Shared atomic state between the [`NetworkNode`] handle and the swarm task.
struct SharedState {
    /// Number of currently connected peers (approximate).
    peer_count: AtomicUsize,
    /// Whether the swarm event loop is still running.
    running: AtomicBool,
}

/// P2P network node providing the [`NetworkService`] interface.
///
/// Created via [`NetworkNode::start`], which spawns a background tokio task
/// running the libp2p swarm event loop. Methods on this struct send commands
/// to that task over an unbounded mpsc channel.
pub struct NetworkNode {
    command_tx: mpsc::UnboundedSender<Command>,
    state: Arc<SharedState>,
    local_peer_id: PeerId,
}

impl std::fmt::Debug for NetworkNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkNode")
            .field("peer_id", &self.local_peer_id)
            .field("peer_count", &self.state.peer_count.load(Ordering::Relaxed))
            .field("running", &self.state.running.load(Ordering::Relaxed))
            .finish()
    }
}

/// Load an Ed25519 keypair from a file, or generate and save a new one.
///
/// The file stores the raw 32-byte Ed25519 secret key (seed).  On load,
/// the keypair is reconstructed deterministically from that seed, so the
/// peer ID remains stable across node restarts.
///
/// If the file does not exist, a fresh keypair is generated, the 32-byte
/// secret is written to the file, and the keypair is returned.  The file
/// is created with mode `0o600` on Unix so that only the owning user can
/// read it.
fn load_or_generate_keypair(path: &std::path::Path) -> Result<Keypair, String> {
    use std::io::{Read, Write};

    if path.exists() {
        let mut file = std::fs::File::open(path)
            .map_err(|e| format!("failed to open node key file '{}': {e}", path.display()))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| format!("failed to read node key file '{}': {e}", path.display()))?;
        // ed25519_from_bytes expects the 32-byte secret (seed).
        let keypair = Keypair::ed25519_from_bytes(bytes)
            .map_err(|e| format!("invalid node key in '{}': {e}", path.display()))?;
        info!(path = %path.display(), "loaded existing node identity key");
        Ok(keypair)
    } else {
        let keypair = Keypair::generate_ed25519();

        // Ensure the parent directory exists.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create directory '{}': {e}",
                    parent.display()
                )
            })?;
        }

        // Extract the 32-byte secret key seed.
        // SecretKey implements AsRef<[u8]>, giving us the raw seed bytes.
        let ed_keypair = keypair
            .clone()
            .try_into_ed25519()
            .map_err(|e| format!("keypair is not Ed25519: {e}"))?;
        let secret_bytes: Vec<u8> = ed_keypair.secret().as_ref().to_vec(); // 32 bytes

        let mut file = std::fs::File::create(path).map_err(|e| {
            format!("failed to create node key file '{}': {e}", path.display())
        })?;
        file.write_all(&secret_bytes).map_err(|e| {
            format!("failed to write node key file '{}': {e}", path.display())
        })?;

        // Restrict permissions to owner-read/write only (Unix).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("failed to set permissions on '{}': {e}", path.display()))?;
        }

        info!(path = %path.display(), "generated new node identity key");
        Ok(keypair)
    }
}

impl NetworkNode {
    /// Start the network node, returning a handle, event receiver, and query receiver.
    ///
    /// Spawns a background tokio task that runs the libp2p swarm event loop.
    /// The returned [`broadcast::Receiver`] receives [`NetworkEvent`]s from peers.
    /// The returned [`mpsc::UnboundedReceiver<StorageQuery>`] receives requests
    /// from peers that need to be answered from storage.
    pub async fn start(
        config: NetworkConfig,
    ) -> Result<(Self, broadcast::Receiver<NetworkEvent>, mpsc::UnboundedReceiver<StorageQuery>), String> {
        let keypair = match &config.node_key_path {
            Some(path) => load_or_generate_keypair(path)?,
            None => Keypair::generate_ed25519(),
        };
        let local_peer_id = PeerId::from(keypair.public());
        info!(%local_peer_id, "starting network node");

        // Build gossipsub
        let gossipsub = behaviour::build_gossipsub(config.gossipsub_heartbeat)?;

        // Build Kademlia
        let kad_config = kad::Config::new(
            StreamProtocol::try_from_owned(
                String::from_utf8_lossy(behaviour::KAD_PROTOCOL).into_owned(),
            )
            .map_err(|e| format!("invalid kad protocol: {e}"))?,
        );
        let store = kad::store::MemoryStore::new(local_peer_id);
        let kademlia = kad::Behaviour::with_config(local_peer_id, store, kad_config);

        // Build Identify
        let identify = identify::Behaviour::new(identify::Config::new(
            PROTOCOL_VERSION.to_string(),
            keypair.public(),
        ));

        // Build optional mDNS
        let mdns = if config.enable_mdns {
            Some(
                mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)
                    .map_err(|e| format!("mDNS error: {e}"))?,
            )
        } else {
            None
        };

        // Build request-response
        let req_resp_config = request_response::Config::default()
            .with_request_timeout(Duration::from_secs(30));
        let req_resp = request_response::Behaviour::with_codec(
            RillCodec,
            [(StreamProtocol::new(REQ_RESP_PROTOCOL), request_response::ProtocolSupport::Full)],
            req_resp_config,
        );

        let behaviour = RillBehaviour {
            gossipsub,
            kademlia,
            identify,
            mdns: mdns.into(),
            request_response: req_resp,
        };

        let mut swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| format!("TCP transport error: {e}"))?
            .with_behaviour(|_| Ok(behaviour))
            .map_err(|e| format!("behaviour error: {e}"))?
            .build();

        // Subscribe to gossipsub topics
        let blocks_topic = IdentTopic::new(BLOCKS_TOPIC);
        let txs_topic = IdentTopic::new(TXS_TOPIC);
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&blocks_topic)
            .map_err(|e| format!("subscribe blocks: {e}"))?;
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&txs_topic)
            .map_err(|e| format!("subscribe txs: {e}"))?;

        // Listen on configured address
        let listen_addr: Multiaddr = config
            .listen_multiaddr()
            .parse()
            .map_err(|e| format!("invalid listen addr: {e}"))?;
        swarm
            .listen_on(listen_addr)
            .map_err(|e| format!("listen error: {e}"))?;

        // Bootstrap Kademlia with configured peers
        for peer_addr in &config.bootstrap_peers {
            if let Ok(addr) = peer_addr.parse::<Multiaddr>() {
                if let Some(Protocol::P2p(peer_id)) = addr.iter().last() {
                    swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr.clone());
                }
                let _ = swarm.dial(addr);
            }
        }

        if !config.bootstrap_peers.is_empty() {
            let _ = swarm.behaviour_mut().kademlia.bootstrap();
        }

        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = broadcast::channel(256);
        let (query_tx, query_rx) = mpsc::unbounded_channel();

        let state = Arc::new(SharedState {
            peer_count: AtomicUsize::new(0),
            running: AtomicBool::new(true),
        });

        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            swarm_event_loop(swarm, command_rx, event_tx, query_tx, state_clone).await;
        });

        let node = NetworkNode {
            command_tx,
            state,
            local_peer_id,
        };

        Ok((node, event_rx, query_rx))
    }

    /// The local peer ID assigned to this node.
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    /// Whether the background swarm event loop is still running.
    pub fn is_running(&self) -> bool {
        self.state.running.load(Ordering::Relaxed)
    }

    /// Whether this node has any connected peers.
    pub fn is_connected(&self) -> bool {
        self.state.peer_count.load(Ordering::Relaxed) > 0
    }

    /// Request the swarm to shut down gracefully.
    pub fn shutdown(&self) {
        let _ = self.command_tx.send(Command::Shutdown);
    }

    /// Dial a remote peer by multiaddr.
    pub fn connect_peer(&self, addr: Multiaddr) -> Result<(), NetworkError> {
        self.command_tx
            .send(Command::Dial(addr))
            .map_err(|_| NetworkError::PeerDisconnected("swarm task stopped".into()))
    }

    /// Send a request-response request to a specific peer.
    pub fn send_request(&self, peer: PeerId, request: RillRequest) -> Result<(), NetworkError> {
        self.command_tx
            .send(Command::SendRequest { peer, request })
            .map_err(|_| NetworkError::PeerDisconnected("swarm task stopped".into()))
    }

    /// Send a response back to a peer via their response channel.
    pub fn send_response(
        &self,
        channel: request_response::ResponseChannel<RillResponse>,
        response: RillResponse,
    ) -> Result<(), NetworkError> {
        self.command_tx
            .send(Command::SendResponse { channel, response })
            .map_err(|_| NetworkError::PeerDisconnected("swarm task stopped".into()))
    }

    /// Send a publish command to the swarm task.
    fn publish(&self, topic: &str, data: Vec<u8>) -> Result<(), NetworkError> {
        self.command_tx
            .send(Command::Publish {
                topic: topic.to_string(),
                data,
            })
            .map_err(|_| NetworkError::PeerDisconnected("swarm task stopped".into()))
    }
}

impl NetworkService for NetworkNode {
    /// Broadcast a validated block to all connected peers.
    fn broadcast_block(&self, block: &Block) -> Result<(), NetworkError> {
        let msg = NetworkMessage::NewBlock(block.clone());
        let data = msg.encode()?;
        self.publish(BLOCKS_TOPIC, data)
    }

    /// Broadcast a validated transaction to all connected peers.
    fn broadcast_transaction(&self, tx: &Transaction) -> Result<(), NetworkError> {
        let msg = NetworkMessage::NewTransaction(tx.clone());
        let data = msg.encode()?;
        self.publish(TXS_TOPIC, data)
    }

    /// Number of currently connected peers.
    fn peer_count(&self) -> usize {
        self.state.peer_count.load(Ordering::Relaxed)
    }

    /// Request a specific block from peers by hash.
    fn request_block(&self, hash: &Hash256) -> Result<(), NetworkError> {
        let msg = NetworkMessage::GetBlock(*hash);
        let data = msg.encode()?;
        self.publish(BLOCKS_TOPIC, data)
    }

    /// Request block headers starting from the given locator hashes.
    fn request_headers(&self, locator: &[Hash256]) -> Result<(), NetworkError> {
        let msg = NetworkMessage::GetHeaders(locator.to_vec());
        let data = msg.encode()?;
        self.publish(TXS_TOPIC, data)
    }
}

/// Background task running the libp2p swarm event loop.
///
/// Receives commands from [`NetworkNode`] and emits [`NetworkEvent`]s
/// to subscribers via the broadcast channel. Forwards request-response
/// requests to the node via the query channel for processing.
async fn swarm_event_loop(
    mut swarm: libp2p::Swarm<RillBehaviour>,
    mut command_rx: mpsc::UnboundedReceiver<Command>,
    event_tx: broadcast::Sender<NetworkEvent>,
    query_tx: mpsc::UnboundedSender<StorageQuery>,
    state: Arc<SharedState>,
) {
    loop {
        tokio::select! {
            cmd = command_rx.recv() => {
                match cmd {
                    Some(Command::Publish { topic, data }) => {
                        let topic = IdentTopic::new(topic);
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                            debug!("gossipsub publish error: {e}");
                        }
                    }
                    Some(Command::Dial(addr)) => {
                        if let Err(e) = swarm.dial(addr) {
                            debug!("dial error: {e}");
                        }
                    }
                    Some(Command::SendRequest { peer, request }) => {
                        let _ = swarm.behaviour_mut().request_response.send_request(&peer, request);
                    }
                    Some(Command::SendResponse { channel, response }) => {
                        let _ = swarm.behaviour_mut().request_response.send_response(channel, response);
                    }
                    Some(Command::Shutdown) | None => {
                        info!("shutting down swarm event loop");
                        state.running.store(false, Ordering::Relaxed);
                        break;
                    }
                }
            }
            event = swarm.next() => {
                // swarm.next() returns Option<SwarmEvent>, but with libp2p
                // it only returns None if the swarm is shut down.
                let Some(event) = event else {
                    state.running.store(false, Ordering::Relaxed);
                    break;
                };

                match event {
                    SwarmEvent::Behaviour(behaviour::RillBehaviourEvent::Gossipsub(
                        gossipsub::Event::Message { message, .. },
                    )) => {
                        if let Some(net_msg) = NetworkMessage::decode(&message.data) {
                            let event = match net_msg {
                                NetworkMessage::NewBlock(block) => {
                                    NetworkEvent::BlockReceived(block)
                                }
                                NetworkMessage::NewTransaction(tx) => {
                                    NetworkEvent::TransactionReceived(tx)
                                }
                                NetworkMessage::GetBlock(hash) => {
                                    NetworkEvent::BlockRequested(hash)
                                }
                                NetworkMessage::GetHeaders(locator) => {
                                    NetworkEvent::HeadersRequested(locator)
                                }
                            };
                            let _ = event_tx.send(event);
                        } else {
                            debug!("failed to decode gossipsub message");
                        }
                    }

                    SwarmEvent::Behaviour(behaviour::RillBehaviourEvent::RequestResponse(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    request_response::Message::Request { request, channel, .. } => {
                                        debug!(%peer, "received request-response request");

                                        // Emit ChainTipRequested event for sync manager.
                                        if matches!(request, RillRequest::GetChainTip) {
                                            let _ = event_tx.send(NetworkEvent::ChainTipRequested(peer));
                                        }

                                        // Forward the request to the node for processing.
                                        let query = StorageQuery {
                                            request,
                                            peer,
                                            response_channel: channel,
                                        };
                                        if let Err(e) = query_tx.send(query) {
                                            debug!("failed to send storage query: {e}");
                                        }
                                    }
                                    request_response::Message::Response { response, .. } => {
                                        debug!(%peer, "received request-response response");

                                        // Emit as NetworkEvent for the sync manager to process.
                                        let _ = event_tx.send(NetworkEvent::RequestResponse {
                                            peer,
                                            response,
                                        });
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure { peer, error, .. } => {
                                warn!(%peer, %error, "outbound request failed");
                            }
                            request_response::Event::InboundFailure { peer, error, .. } => {
                                warn!(%peer, %error, "inbound request failed");
                            }
                            request_response::Event::ResponseSent { .. } => {}
                        }
                    }

                    SwarmEvent::Behaviour(behaviour::RillBehaviourEvent::Mdns(
                        mdns::Event::Discovered(peers),
                    )) => {
                        for (peer_id, addr) in peers {
                            debug!(%peer_id, %addr, "mDNS discovered peer");
                            swarm
                                .behaviour_mut()
                                .kademlia
                                .add_address(&peer_id, addr);
                        }
                    }

                    SwarmEvent::Behaviour(behaviour::RillBehaviourEvent::Mdns(
                        mdns::Event::Expired(peers),
                    )) => {
                        for (peer_id, addr) in peers {
                            debug!(%peer_id, %addr, "mDNS peer expired");
                        }
                    }

                    SwarmEvent::Behaviour(behaviour::RillBehaviourEvent::Identify(
                        identify::Event::Received { peer_id, info, .. },
                    )) => {
                        debug!(%peer_id, "identify received");
                        for addr in info.listen_addrs {
                            swarm
                                .behaviour_mut()
                                .kademlia
                                .add_address(&peer_id, addr);
                        }
                    }

                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        let count = state.peer_count.fetch_add(1, Ordering::Relaxed) + 1;
                        info!(%peer_id, count, "peer connected");
                        let _ = event_tx.send(NetworkEvent::PeerConnected(peer_id));
                    }

                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        let prev = state.peer_count.load(Ordering::Relaxed);
                        if prev > 0 {
                            state.peer_count.fetch_sub(1, Ordering::Relaxed);
                        }
                        let count = state.peer_count.load(Ordering::Relaxed);
                        info!(%peer_id, count, "peer disconnected");
                        let _ = event_tx.send(NetworkEvent::PeerDisconnected(peer_id));
                    }

                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!(%address, "listening on");
                    }

                    SwarmEvent::ListenerError { error, .. } => {
                        error!(%error, "listener error");
                    }

                    _ => {}
                }
            }
        }
    }
}

// Use libp2p::futures for swarm.next()
use libp2p::futures::StreamExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_node_debug_format() {
        // Verify Debug impl doesn't panic (can't create real node without tokio runtime)
        let state = Arc::new(SharedState {
            peer_count: AtomicUsize::new(0),
            running: AtomicBool::new(false),
        });
        let (tx, _rx) = mpsc::unbounded_channel();
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let node = NetworkNode {
            command_tx: tx,
            state,
            local_peer_id: peer_id,
        };
        let debug_str = format!("{node:?}");
        assert!(debug_str.contains("NetworkNode"));
        assert!(debug_str.contains("peer_count: 0"));
        assert!(debug_str.contains("running: false"));
    }

    #[test]
    fn shared_state_peer_count_starts_at_zero() {
        let state = SharedState {
            peer_count: AtomicUsize::new(0),
            running: AtomicBool::new(true),
        };
        assert_eq!(state.peer_count.load(Ordering::Relaxed), 0);
        assert!(state.running.load(Ordering::Relaxed));
    }

    #[test]
    fn channel_closed_returns_error() {
        let (tx, _rx) = mpsc::unbounded_channel::<Command>();
        let state = Arc::new(SharedState {
            peer_count: AtomicUsize::new(0),
            running: AtomicBool::new(false),
        });
        let keypair = Keypair::generate_ed25519();
        let node = NetworkNode {
            command_tx: tx,
            state,
            local_peer_id: PeerId::from(keypair.public()),
        };
        // Drop the receiver side by dropping _rx (already dropped above — wait, _rx exists)
        // We need a different approach: create a channel where receiver is already dropped
        drop(node);

        // Create a node with a dropped receiver
        let (tx2, rx2) = mpsc::unbounded_channel::<Command>();
        drop(rx2);
        let state2 = Arc::new(SharedState {
            peer_count: AtomicUsize::new(0),
            running: AtomicBool::new(false),
        });
        let keypair2 = Keypair::generate_ed25519();
        let node2 = NetworkNode {
            command_tx: tx2,
            state: state2,
            local_peer_id: PeerId::from(keypair2.public()),
        };
        let result = node2.publish("test", vec![0u8; 4]);
        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::PeerDisconnected(msg) => {
                assert!(msg.contains("swarm task stopped"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn peer_count_on_fresh_node() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = Arc::new(SharedState {
            peer_count: AtomicUsize::new(0),
            running: AtomicBool::new(true),
        });
        let keypair = Keypair::generate_ed25519();
        let node = NetworkNode {
            command_tx: tx,
            state,
            local_peer_id: PeerId::from(keypair.public()),
        };
        assert_eq!(node.peer_count(), 0);
        assert!(!node.is_connected());
    }

    #[test]
    fn shutdown_sends_command() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let state = Arc::new(SharedState {
            peer_count: AtomicUsize::new(0),
            running: AtomicBool::new(true),
        });
        let keypair = Keypair::generate_ed25519();
        let node = NetworkNode {
            command_tx: tx,
            state,
            local_peer_id: PeerId::from(keypair.public()),
        };
        node.shutdown();
        let cmd = rx.try_recv().unwrap();
        assert!(matches!(cmd, Command::Shutdown));
    }

    #[test]
    fn network_event_is_clone_and_debug() {
        let event = NetworkEvent::BlockRequested(Hash256::ZERO);
        let _cloned = event.clone();
        let debug = format!("{event:?}");
        assert!(debug.contains("BlockRequested"));
    }

    #[test]
    fn send_request_sends_command() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let state = Arc::new(SharedState {
            peer_count: AtomicUsize::new(0),
            running: AtomicBool::new(true),
        });
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let node = NetworkNode {
            command_tx: tx,
            state,
            local_peer_id: peer_id,
        };
        node.send_request(peer_id, RillRequest::GetChainTip).unwrap();
        let cmd = rx.try_recv().unwrap();
        assert!(matches!(cmd, Command::SendRequest { .. }));
    }
}
