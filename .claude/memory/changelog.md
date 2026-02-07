# Changelog
## 2026-02-07
- Implemented rill-wallet crate: HD wallet with decay-aware coin selection
  - 6 modules: error, keys, coin_selection, encryption, builder, wallet
  - BLAKE3 KDF key derivation from master seed (not BIP-32, Ed25519 incompatible)
  - Decay-aware coin selection: spend highest-decay UTXOs first
  - AES-256-GCM encrypted wallet file persistence (BLAKE3 password KDF Phase 1)
  - TransactionBuilder with builder pattern, signing, multi-recipient support
  - Wallet: create/restore, UTXO scanning, balance with decay, send, save/load
  - Fixed u64 overflow in concentration calc using u128 intermediate
  - 76 unit tests, all passing
  - Workspace total: 699 tests (443 core + 71 decay + 26 consensus + 30 network + 53 node + 76 wallet)
  - All 6 library crates now complete
- Implemented rill-node-lib crate: full node composition
  - 5 modules: config, storage, node, rpc, lib
  - RocksDB-backed ChainStore with 6 column families, atomic WriteBatch
  - NodeChainState adapter (RwLock → ChainState trait bridge)
  - Node event loop composing storage/mempool/consensus/network
  - JSON-RPC server (jsonrpsee 0.24) with 9 RPC methods
  - 53 unit tests, all passing
  - Workspace total: 623 tests (443 core + 71 decay + 26 consensus + 30 network + 53 node)
- Implemented rill-network crate: P2P networking with libp2p 0.54
  - 4 modules: config, protocol, behaviour, service
  - 30 unit tests, all passing
  - Workspace total: 570 tests (443 core + 71 decay + 26 consensus + 30 network)

## 2026-02-06
- Project bootstrapped: workspace, all crates, agents, isolation, memory
