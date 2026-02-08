# node Agent — Session Notes
## Status: Implemented (Phase 1)
## Last Session: 2026-02-07
## Test Count: 53 (8 config + 22 storage + 11 node + 12 rpc)

## Implemented Modules
1. **config.rs** — `NodeConfig` with data_dir, rpc_bind, rpc_port, network, log_level. `db_path()`, `rpc_addr()`.
2. **storage.rs** — `RocksStore` implementing `ChainStore` with RocksDB column families (blocks, headers, utxos, height_index, undo, metadata). Atomic `WriteBatch`, genesis auto-init, supply tracking, Phase 1 cluster_balance=0.
3. **node.rs** — `NodeChainState` adapter (RwLock<RocksStore> → ChainState), `Node` struct composing storage/mempool/consensus/network. `process_block`, `process_transaction`, `run` event loop, query methods.
4. **rpc.rs** — jsonrpsee 0.24 JSON-RPC server with getblockcount, getblockhash, getblock, getblockheader, gettransaction, sendrawtransaction, getmempoolinfo, getpeerinfo, getinfo.
5. **lib.rs** — Module declarations and re-exports.

## Key Design Decisions
- `parking_lot::RwLock` on storage, `parking_lot::Mutex` on mempool (blocking IO, no async lock benefit)
- `NodeChainState` adapter takes read lock per call, allowing concurrent reads while block processing holds write lock
- Genesis auto-connected on first `RocksStore::open`
- Supply tracked atomically in metadata CF, incremented/decremented with connect/disconnect
- Network is optional (`Node::without_network` for testing)
- event_rx behind `tokio::sync::Mutex` (only used in async event loop)

## Dependencies Added
- `bincode`, `hex`, `parking_lot` in rill-node/Cargo.toml
- `macros` feature added to jsonrpsee workspace dependency

## What's Next
- Phase 2: cluster tracking in storage (currently returns 0)
- Block body search for gettransaction (currently mempool-only)
- Proper fee calculation in process_transaction (currently fee=0)
- Initial block download (IBD) protocol
- Chain reorganization handling
