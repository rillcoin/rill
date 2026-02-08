# Changelog

## 2026-02-08 - Phase 1 Complete: All Binaries Implemented

### bins/rill-miner
- **Complete mining binary** (310 lines)
- CLI arguments: RPC endpoint, mining address, threads, log level
- Async mining loop with jsonrpsee HTTP client
- Block template fetching via `getblocktemplate` RPC
- Mock PoW mining using `mine_block()` from rill-consensus
- Block submission via `submitblock` RPC
- Multi-threaded mining support
- Hashrate statistics tracking and periodic logging
- Graceful shutdown via Ctrl+C (SIGINT)
- Automatic template refresh on chain tip changes

### bins/rill-cli
- **Complete wallet CLI** (336 lines)
- Subcommands: `wallet create`, `wallet restore`, `address`, `balance`, `send`
- Secure password input with `rpassword` (no echo)
- HD wallet creation with 32-byte seed display
- Wallet restoration from hex-encoded seed
- Address derivation and display (Bech32m format)
- Balance and send commands (Phase 1 placeholders, ready for RPC integration)
- AES-256-GCM encrypted wallet files
- Default wallet location: `~/.rill/wallet.dat`
- Network support: mainnet and testnet

### crates/rill-node (RPC enhancements)
- Added `getblocktemplate` RPC method
  - Accepts mining address parameter
  - Creates block template via consensus engine
  - Returns JSON with header fields, transactions, height
- Added `submitblock` RPC method
  - Accepts hex-encoded bincode Block
  - Validates and processes block
  - Returns block hash on success
- New `create_block_template()` public method on Node

### Code Quality Fixes
- Fixed all clippy warnings across workspace
- Fixed jsonrpsee API usage (ClientT trait, ArrayParams)
- Fixed redundant closures in merkle.rs
- Fixed unnecessary cast in difficulty.rs
- Fixed for-kv-map warning in wallet.rs
- Fixed out-of-bounds indexing in consensus engine
- Fixed op-ref warning in network protocol
- All 699 tests passing
- Zero clippy warnings
- Clean build across entire workspace

### Project Status
**All 9 components complete:**
- ✓ rill-core (443 tests)
- ✓ rill-decay (71 tests)
- ✓ rill-consensus (26 tests)
- ✓ rill-network (30 tests)
- ✓ rill-node-lib (53 tests)
- ✓ rill-wallet (76 tests)
- ✓ bins/rill-node (0 tests - binary)
- ✓ bins/rill-miner (0 tests - binary)
- ✓ bins/rill-cli (0 tests - binary)

**Total: 699 tests, all passing**

### Next Steps
- Test private testnet milestone:
  - Multi-node deployment
  - Mining with rill-miner
  - Wallet operations with rill-cli
  - Block propagation verification
  - Transaction confirmation
  - Decay mechanics validation

---

## 2026-02-07

### bins/rill-node
- Complete full node binary with CLI (184 lines)
- RPC server, P2P networking, event loop
- Graceful shutdown handling

### crates/rill-wallet
- HD wallet with decay-aware coin selection (6 modules, 76 tests)
- BLAKE3 KDF, AES-256-GCM encryption
- TransactionBuilder with multi-recipient support

### crates/rill-node-lib
- Full node composition (5 modules, 53 tests)
- RocksDB storage, JSON-RPC server (9 methods)
- Node event loop

### crates/rill-network
- P2P networking with libp2p 0.54 (4 modules, 30 tests)
- Gossipsub, Kademlia DHT, Noise encryption

---

## 2026-02-06
- Project bootstrapped
- Workspace structure created
- All crate stubs and agent configuration
