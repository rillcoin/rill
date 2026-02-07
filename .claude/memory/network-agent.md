# Network Agent — Session Notes
## Status: Implemented (Phase 1)
## Last Session: 2026-02-07

## What Was Done
- Implemented complete rill-network crate with 30 unit tests
- 4 modules: config, protocol, behaviour, service

## Architecture
- **Command-channel bridge**: Sync `NetworkService` trait → mpsc::UnboundedSender → async swarm task
- **SharedState**: AtomicUsize peer_count + AtomicBool running (lock-free, Send+Sync)
- **broadcast::channel** for NetworkEvents (multiple subscribers)

## Modules
### config.rs
- `NetworkConfig`: listen_addr, listen_port, bootstrap_peers, enable_mdns, gossipsub_heartbeat, max_peers, dial_timeout
- `Default`, `testnet()`, `mainnet()`, `listen_multiaddr()`
- 8 tests

### protocol.rs
- `NetworkMessage` enum: NewBlock, NewTransaction, GetBlock, GetHeaders
- MAGIC_BYTES prefix + bincode encode/decode
- Topic routing: blocks→BLOCKS_TOPIC, txs→TXS_TOPIC
- Constants: BLOCKS_TOPIC, TXS_TOPIC, MAX_MESSAGE_SIZE
- 13 tests

### behaviour.rs
- `RillBehaviour` via #[derive(NetworkBehaviour)]: gossipsub, kademlia, identify, Toggle<mdns>
- `build_gossipsub()`: SHA-256 content-based message IDs, ValidationMode::Strict
- Constants: PROTOCOL_VERSION, KAD_PROTOCOL
- 3 tests

### service.rs
- `NetworkNode`: command_tx, state (Arc<SharedState>), local_peer_id
- `NetworkNode::start(config)` → spawns swarm_event_loop tokio task
- `NetworkService` impl: broadcast_block/tx, peer_count, request_block/headers
- `swarm_event_loop`: tokio::select! over commands + swarm events
- `NetworkEvent` enum: BlockReceived, TransactionReceived, BlockRequested, HeadersRequested, PeerConnected, PeerDisconnected
- `connect_peer()` for dialing remote peers
- `shutdown()` for graceful stop
- 6 tests

## Dependencies Added
- `sha2.workspace = true` in Cargo.toml (for gossipsub message ID hashing)

## Key Design Decisions
1. Phase 1: GetBlock/GetHeaders use gossipsub (Phase 2 will add request-response)
2. Connection count ≈ peer count (Phase 1 simplification)
3. Toggle for mDNS: disabled on mainnet, enabled on testnet
4. Ed25519 keypair generated per node (libp2p identity, separate from wallet keys)

## What's Next
- Integration testing with actual two-node P2P connections
- Phase 2: request-response protocol for block/header requests
- Phase 2: proper PeerId-based peer tracking (vs connection count)
- Wire into rill-node for actual block/tx relay
