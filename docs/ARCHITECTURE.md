# Rill Architecture

This document describes the technical architecture of the Rill cryptocurrency node, including crate responsibilities, key design decisions, storage layout, and the concentration decay mechanism.

---

## Table of Contents

- [Crate Dependency Graph](#crate-dependency-graph)
- [Crate Descriptions](#crate-descriptions)
- [Binary Descriptions](#binary-descriptions)
- [Key Design Decisions](#key-design-decisions)
- [Concentration Decay Mechanism](#concentration-decay-mechanism)
- [Storage Layout](#storage-layout)
- [Network Protocol](#network-protocol)
- [Test Architecture](#test-architecture)

---

## Crate Dependency Graph

```
rill-core
  |
  +-- rill-decay
        |
        +-- rill-consensus
              |
              +-- rill-network
                    |
                    +-- rill-wallet
                          |
                          +-- rill-node  (library crate)
```

Dependency flows strictly downward. No circular dependencies. `rill-core` has zero dependencies on other `rill-*` crates. Each layer builds on the types and traits defined below it.

`rill-tests` is a separate test-only crate that depends on all of the above for end-to-end integration and adversarial testing.

---

## Crate Descriptions

### rill-core

**Path:** `crates/rill-core/`

The foundational layer. Every other crate depends on it. Defines all shared types, traits, and constants.

Responsibilities:
- Core types: `Transaction`, `Block`, `BlockHeader`, `TxInput`, `TxOutput`, `OutPoint`, `UtxoEntry`, `Hash256`
- Address types with bech32 encoding (`rill1...` mainnet, `trill1...` testnet)
- Trait interfaces: `ChainState`, `DecayCalculator`, `BlockProducer`, `NetworkService`
- Ed25519 keypair generation, transaction signing, and signature verification
- BLAKE3 Merkle tree construction and root computation
- SHA-256 block header hashing (for PoW)
- Genesis block construction
- All protocol constants (supply caps, timing, decay parameters, port defaults)
- Error types via `thiserror`

No floating-point arithmetic. All monetary values are `u64` in rills (1 RILL = 10^8 rills).

### rill-decay

**Path:** `crates/rill-decay/`

Implements the novel progressive concentration decay algorithm. This is the most safety-critical crate.

Responsibilities:
- Sigmoid-based decay rate curve using fixed-point integer lookup tables
- Cluster-level concentration tracking (aggregates UTXO values by cluster ID)
- Per-block effective value computation: `effective = nominal * (1 - rate)^blocks_held`
- Binary exponentiation for compound decay, integer-only
- Decay pool accounting: records decayed value for distribution to miners
- Lineage tracking: measures how long a concentration cluster has existed

All calculations use checked integer arithmetic (`checked_add`, `checked_mul`). No floating-point is permitted on any consensus code path.

### rill-consensus

**Path:** `crates/rill-consensus/`

Implements block production, validation, and proof-of-work.

Responsibilities:
- Block producer: assembles transactions from the mempool, computes Merkle root, mines the coinbase transaction
- Block validator: checks PoW, verifies all transaction signatures, validates UTXO spends, enforces consensus rules
- Difficulty adjustment: LWMA (Linear Weighted Moving Average) over a 60-block window, 60-second target block time, clamped to [prev/3, prev*3]
- Mining rewards: base coinbase reward with halving every 210,000 blocks, plus decay pool distributions
- Fork choice: longest chain with most cumulative work
- Checkpoints: hard-coded (height, hash) pairs to prevent deep reorgs

Phase 1 uses SHA-256 double-hash PoW. Phase 2 will use RandomX via FFI behind the same `BlockProducer` trait interface.

### rill-network

**Path:** `crates/rill-network/`

P2P networking layer using libp2p.

Responsibilities:
- Transport: TCP with Noise protocol (XX handshake) for encryption, Yamux for stream multiplexing
- Gossipsub: block and transaction propagation on topics `/rill/blocks/1` and `/rill/txs/1`
- Kademlia DHT: peer routing and discovery
- mDNS: automatic local peer discovery (useful for testnet and development)
- Request-response: block sync protocol for fetching headers and full blocks from peers
- Rate limiting: per-peer limits for blocks, transactions, and header requests
- Wire format: bincode serialization for all messages

### rill-wallet

**Path:** `crates/rill-wallet/`

HD wallet with decay-aware coin selection.

Responsibilities:
- Deterministic key derivation from a 32-byte master seed using BLAKE3
- BIP-39 mnemonic encoding and decoding (24-word seed phrases)
- Ed25519 keypair management via a `KeyChain`
- Address derivation and address index management
- Decay-aware coin selection: UTXOs with highest decay exposure are spent first, minimizing loss
- Transaction building and input signing
- Encrypted wallet file persistence using AES-256-GCM

### rill-node (library crate)

**Path:** `crates/rill-node/`

Full node library that composes all other crates into a running node.

Responsibilities:
- `Node` struct: owns the chain store, mempool, consensus engine, and network handle
- Chain state implementation of `ChainState` trait, backed by RocksDB
- Mempool management: transaction acceptance, fee ordering, eviction
- Block processing: validates and commits blocks, applies UTXO set changes atomically
- Undo data: stores enough information to reverse a block commit during reorgs
- JSON-RPC server implementation using jsonrpsee
- Node configuration (`NodeConfig`)

### rill-tests

**Path:** `crates/rill-tests/`

Test-only crate for integration and adversarial testing.

Responsibilities:
- End-to-end tests that exercise the full node stack
- Adversarial tests: double-spend attempts, invalid signatures, malformed blocks, reorg scenarios
- Property-based tests with proptest
- Test helpers and fixtures shared across test files

---

## Binary Descriptions

### rill-node

**Path:** `bins/rill-node/`

The full node binary. Parses command-line arguments, initializes a `Node` from `NodeConfig`, starts the JSON-RPC server, and runs the P2P event loop until shutdown.

Key flags: `--testnet`, `--regtest`, `--data-dir`, `--rpc-port`, `--p2p-listen-port`, `--bootstrap-peers`, `--no-network`, `--log-format`.

See `docs/TESTNET.md` for full flag reference.

### rill-cli

**Path:** `bins/rill-cli/`

Command-line wallet and node query tool. Communicates with a running `rill-node` via JSON-RPC.

Subcommands:
- `wallet create` — generates a new HD wallet with a 24-word seed phrase
- `wallet restore` — restores a wallet from an existing seed phrase or hex seed
- `address` — prints the current receive address
- `balance` — queries UTXOs from the node and displays nominal and effective (post-decay) balance
- `send` — builds, signs, and broadcasts a transaction; uses decay-aware coin selection
- `getblockchaininfo` — queries comprehensive chain state
- `getsyncstatus` — queries node sync status
- `getpeerinfo` — queries connected peer count
- `validateaddress` — validates a Rill address client-side (no node required)

### rill-miner

**Path:** `bins/rill-miner/`

Standalone mining daemon. Connects to `rill-node` via RPC, fetches block templates (`getblocktemplate`), mines using SHA-256 double-hash PoW, and submits valid blocks (`submitblock`).

Supports multiple mining threads via `--threads`. Logs hashrate and block count every 30 seconds.

---

## Key Design Decisions

### Ed25519 Signatures

Transaction inputs are authorized by Ed25519 signatures over the transaction hash. Ed25519 was chosen for its small key and signature sizes (32-byte public keys, 64-byte signatures), fast verification, and strong security properties. The `ed25519-dalek` crate provides the implementation.

### BLAKE3 Merkle Trees

Transaction Merkle roots in block headers are computed using BLAKE3. BLAKE3 is significantly faster than SHA-256 for this use case and is resistant to length-extension attacks. The Merkle tree is a standard binary tree with sibling concatenation before hashing.

### SHA-256 Block Headers

Block header hashing for proof-of-work uses SHA-256 double-hash (SHA256d), consistent with Bitcoin. The block hash commits to: version, previous block hash, Merkle root, timestamp, difficulty target, and nonce.

### libp2p Networking

The P2P layer uses libp2p to avoid reimplementing transport security, stream multiplexing, and peer discovery. The specific protocol choices are:
- Noise XX handshake for authenticated encryption
- Yamux for stream multiplexing over a single TCP connection
- Gossipsub for efficient broadcast of blocks and transactions
- Kademlia DHT for peer routing in the global network
- mDNS for zero-configuration local peer discovery

### RocksDB Storage

Persistent chain state is stored in RocksDB. The column-family design (see [Storage Layout](#storage-layout)) allows independent compaction policies per column family and atomic batch writes across all families in a single `WriteBatch`.

### Bincode Wire Format

All P2P messages and RPC block/transaction payloads are serialized with bincode using a fixed (non-self-describing) encoding. This is compact and fast to encode/decode. Human-readable RPC responses use JSON.

### Integer-Only Consensus Math

All consensus-critical calculations, including decay, fees, balances, and rewards, use `u64` fixed-point arithmetic with 10^8 precision (rills). Floating-point is explicitly forbidden in all consensus paths to guarantee deterministic, platform-independent results.

---

## Concentration Decay Mechanism

Rill's defining feature is progressive concentration decay. The mechanism discourages long-term wealth hoarding by applying a per-block decay rate to UTXOs belonging to high-concentration clusters.

### Clusters

Each UTXO is associated with a cluster ID, which identifies a set of addresses treated as a single economic actor for decay purposes. The cluster balance is the total value of all UTXOs in the cluster.

### Concentration

Concentration is measured as parts-per-billion (PPB) of the circulating supply:

```
concentration_ppb = cluster_balance * 1_000_000_000 / circulating_supply
```

The decay threshold is 1,000,000 PPB (0.1% of circulating supply). Clusters below this threshold experience no decay.

### Sigmoid Decay Rate

Clusters above the threshold experience a per-block decay rate governed by a sigmoid curve:

```
rate = R_MAX * sigmoid(k * (concentration - threshold))
```

- `R_MAX` is 1,500,000,000 PPB (approximately 15% per year at the block rate)
- `k` controls the steepness of the sigmoid curve
- At the threshold, rate is approximately 50% of `R_MAX`
- At high concentrations, rate asymptotically approaches `R_MAX`

The sigmoid is approximated using a fixed-point integer lookup table for determinism.

### Effective Value

The effective value of a UTXO after `n` blocks is:

```
effective = nominal * (1 - rate)^n
```

Computed using fixed-point binary exponentiation. The difference between nominal and effective values represents decayed wealth that has flowed into the decay pool.

### Decay Pool

Decayed value accumulates in a decay pool. Miners receive a distribution from this pool in addition to the base coinbase reward, weighted by the block's difficulty contribution. This ensures decayed wealth flows back into circulation through the mining economy rather than disappearing.

---

## Storage Layout

RocksDB is opened with eight column families:

| Column Family   | Key                       | Value                          | Description                                  |
|-----------------|---------------------------|--------------------------------|----------------------------------------------|
| `blocks`        | block hash (32 bytes)     | bincode-serialized `Block`     | Full block data                              |
| `headers`       | block hash (32 bytes)     | bincode-serialized `BlockHeader` | Block headers only (lighter than full block) |
| `utxos`         | `OutPoint` (txid + index) | bincode-serialized `UtxoEntry` | Unspent transaction output set               |
| `height_index`  | height (8 bytes BE)       | block hash (32 bytes)          | Maps block height to block hash              |
| `undo`          | block hash (32 bytes)     | list of spent `UtxoEntry`s     | Undo data for block reorgs                   |
| `metadata`      | key bytes                 | value bytes                    | Chain tip, UTXO count, and other metadata    |
| `clusters`      | cluster ID (32 bytes)     | total balance (8 bytes)        | Aggregate cluster balances for decay         |
| `address_index` | address pubkey hash prefix | list of `OutPoint`s            | Address-to-UTXO index (prefix-compressed)    |

All writes to multiple column families in a single logical operation use a `WriteBatch` for atomicity. Block commits and rollbacks are fully atomic: either all column families are updated or none are.

### Pruning

Full block data in the `blocks` column family can be pruned for nodes that do not need to serve historical blocks. Headers and undo data are preserved. Pruning is controlled programmatically; there is no CLI flag for pruning in the current release.

---

## Network Protocol

### P2P Message Flow

On connection, peers exchange version information. Blocks are propagated via Gossipsub: when a node mines or receives a new valid block, it publishes the bincode-serialized block on the `/rill/blocks/1` topic. Transactions are propagated on `/rill/txs/1`.

Block sync (initial block download) uses a request-response protocol:
1. The syncing node sends a block locator (list of known block hashes from tip back to genesis)
2. Peers respond with headers for unknown blocks
3. The syncing node requests full blocks for each header

### Rate Limiting

Per-peer rate limits prevent resource exhaustion attacks:
- 10 blocks per minute via request-response
- 100 transactions per minute via gossipsub
- 5 header requests per minute

### Magic Bytes

Each network uses distinct 4-byte magic bytes prepended to P2P messages:
- Mainnet: `RILL` (0x52494C4C)
- Testnet: `TEST` (0x54455354)
- Regtest: `REGT` (0x52454754)

---

## Test Architecture

### Unit Tests

Unit tests live alongside the source in each crate using Rust's built-in `#[cfg(test)]` modules. Every public function has at least one unit test covering the happy path, and error paths where applicable.

Run all unit tests:

```bash
cargo test --workspace
```

### Integration Tests

End-to-end integration tests are in `crates/rill-tests/`. These tests construct full node instances, process blocks, and verify chain state through the public API without mocking internal components.

Run integration tests only:

```bash
cargo test -p rill-tests
```

### Property-Based Tests

Property-based tests use the `proptest` crate to generate randomized inputs and verify invariants hold across all inputs. Property tests exist for:
- Decay calculations (no overflow, monotonicity, zero decay below threshold)
- Transaction validation (all invalid inputs rejected)
- Merkle tree construction (root changes on any transaction change)
- Address encoding/decoding (round-trip correctness)

Run property tests (release mode is faster):

```bash
cargo test --release proptest
```

### Adversarial Tests

`rill-tests` includes adversarial scenarios:
- Double-spend attempts
- Transactions with invalid signatures
- Blocks with invalid PoW
- Blocks referencing non-existent UTXOs
- Chain reorganization handling

The project has over 920 tests across all crates.
