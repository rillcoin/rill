# Rill

> **Wealth should flow like water.**

![Build](https://img.shields.io/badge/build-passing-brightgreen)
![Tests](https://img.shields.io/badge/tests-920%2B-brightgreen)
![Rust](https://img.shields.io/badge/rust-1.85%2B-orange)
![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue)

A cryptocurrency with progressive concentration decay. Holdings above configurable thresholds gradually decay back into the mining pool, promoting circulation and discouraging long-term hoarding.

---

## Overview

Rill implements a novel economic model where concentrated wealth naturally flows back into circulation through a mathematically-governed decay mechanism. Large balances above threshold values experience sigmoid-function-based decay, with decayed funds returning to miners as supplemental block rewards.

The project is implemented in Rust across **6 library crates** and **3 binaries**, with over **920 tests** including unit, integration, property-based, and adversarial test coverage.

---

## Key Features

- **Concentration Decay**: Balances above thresholds decay to the mining pool using fixed-point sigmoid curves
- **Integer-Only Consensus**: All consensus math uses `u64` fixed-point arithmetic (10^8 precision) — no floating point
- **Ed25519 Signatures**: Fast, secure transaction signing with 32-byte public keys
- **BLAKE3 Merkle Trees**: High-performance transaction tree hashing
- **libp2p Networking**: Production-grade P2P with Gossipsub, Kademlia DHT, and mDNS discovery
- **RocksDB Storage**: Persistent blockchain storage with 8 column families
- **HD Wallet**: BIP-39 seed phrases, decay-aware coin selection, AES-256-GCM encrypted storage
- **JSON-RPC**: Bitcoin-style RPC interface for node queries and transaction submission

---

## Quick Start

### Prerequisites

- Rust 1.85+ (2024 edition)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Building

```bash
git clone https://github.com/rillcoin/rill.git
cd rill

# Build all binaries
cargo build --release

# Run all 920+ tests
cargo test --workspace

# Check for warnings
cargo clippy --workspace -- -D warnings
```

### Running a Local Testnet Node

```bash
# Start a regtest node (instant blocks, no real PoW)
./target/release/rill-node --regtest --data-dir /tmp/rill-regtest
```

### Create a Wallet

```bash
./target/release/rill-cli wallet create --network testnet
```

### Start Mining

```bash
./target/release/rill-miner \
    --rpc-endpoint http://127.0.0.1:38332 \
    --mining-address trill1<your-address>
```

### Run a 3-Node Testnet with Docker

```bash
docker-compose up --build
```

---

## Architecture

```
rill-core       Foundation types (Transaction, Block, Address, UTXO, traits)
  |
rill-decay      Concentration decay algorithm — sigmoid curves, fixed-point math
  |
rill-consensus  Block validation, PoW, difficulty adjustment, mining rewards
  |
rill-network    P2P networking (libp2p: Gossipsub, Kademlia, Noise, mDNS)
  |
rill-wallet     HD wallet, BIP-39 mnemonics, decay-aware coin selection
  |
rill-node       Full node: RocksDB storage, JSON-RPC server, mempool
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for a detailed description of every crate, the storage column families, network protocol, and design decisions.

---

## Project Structure

```
rill/
├── crates/
│   ├── rill-core/       # Core types, traits, crypto, constants
│   ├── rill-decay/      # Sigmoid decay algorithm (fixed-point, integer-only)
│   ├── rill-consensus/  # Block production, validation, PoW, difficulty
│   ├── rill-network/    # P2P networking (libp2p)
│   ├── rill-wallet/     # HD wallet, coin selection, encrypted persistence
│   ├── rill-node/       # Full node library: storage, RPC, mempool
│   └── rill-tests/      # Integration, property-based, and adversarial tests
├── bins/
│   ├── rill-node/       # Full node binary
│   ├── rill-cli/        # Command-line wallet and node query tool
│   └── rill-miner/      # Standalone mining daemon
├── docs/
│   ├── ARCHITECTURE.md  # Technical architecture overview
│   └── TESTNET.md       # Testnet and deployment guide
├── Dockerfile           # Multi-stage build (rust:1.85-bookworm → debian:bookworm-slim)
└── docker-compose.yml   # 3-node local testnet
```

---

## Network Types

| Mode     | Flag        | P2P Port | RPC Port | Description                        |
|----------|-------------|----------|----------|------------------------------------|
| mainnet  | (default)   | 18333    | 18332    | Production network                 |
| testnet  | `--testnet` | 28333    | 28332    | Public test network                |
| regtest  | `--regtest` | 38333    | 38332    | Local dev network, instant blocks  |

---

## CLI Reference

```bash
# Node
rill-node --testnet --data-dir /tmp/rill
rill-node --regtest --data-dir /tmp/rill --no-network

# Miner
rill-miner --rpc-endpoint http://127.0.0.1:18332 --mining-address rill1...

# Wallet
rill-cli wallet create --network testnet
rill-cli wallet restore --network testnet
rill-cli address
rill-cli balance --rpc-endpoint http://127.0.0.1:28332
rill-cli send --to trill1... --amount 10.5 --rpc-endpoint http://127.0.0.1:28332

# Node queries
rill-cli getblockchaininfo --rpc-endpoint http://127.0.0.1:28332
rill-cli getsyncstatus
rill-cli getpeerinfo
rill-cli validateaddress rill1...
```

---

## RPC Methods

Available JSON-RPC methods:

| Method               | Description                                        |
|----------------------|----------------------------------------------------|
| `getblockcount`      | Current chain height                               |
| `getblockhash`       | Block hash at a given height                       |
| `getblock`           | Full block data                                    |
| `getblockheader`     | Block header fields                                |
| `gettransaction`     | Transaction by ID                                  |
| `sendrawtransaction` | Submit a raw transaction                           |
| `getmempoolinfo`     | Mempool size and fee info                          |
| `getblockchaininfo`  | Height, supply, decay pool, IBD status, UTXO count |
| `getsyncstatus`      | Sync state, height, peer count                     |
| `getpeerinfo`        | Connected peer count                               |
| `getinfo`            | General node info                                  |
| `getblocktemplate`   | Block template for mining                          |
| `submitblock`        | Submit a mined block                               |
| `getutxosbyaddress`  | UTXOs for an address                               |
| `getclusterbalance`  | Decay cluster total balance                        |

See [docs/TESTNET.md](docs/TESTNET.md) for full RPC documentation with request/response examples.

---

## Development

### Code Standards

- **Rust Edition**: 2024
- **MSRV**: 1.85
- **Formatting**: `cargo fmt` (default settings)
- **Linting**: `cargo clippy -- -D warnings` (zero warnings policy)
- **Arithmetic**: All consensus math uses `checked_add`, `checked_mul`, etc.
- **Errors**: `thiserror` for library crates, `anyhow` for binaries
- **Logging**: `tracing` with structured fields

### Testing

```bash
# All tests
cargo test --workspace

# Integration tests only
cargo test -p rill-tests

# Property-based tests (faster in release mode)
cargo test --release proptest

# Benchmarks
cargo bench
```

### Git Workflow

- Branch naming: `<agent>/<description>` (e.g., `core/implement-transaction-type`)
- Commit messages: `<crate>: <description>` (e.g., `rill-core: implement Transaction struct`)
- Always run `cargo test --workspace` before committing
- Pre-commit hooks enforce code quality and project isolation

---

## Decay Mechanics

The decay algorithm operates on individual UTXOs grouped into clusters:

1. UTXOs in clusters below 0.1% of circulating supply: **no decay**
2. UTXOs in clusters above threshold: sigmoid decay rate applied per block
3. Decay rate increases with concentration, approaching a maximum of ~15% per year
4. Decayed value accumulates in a pool and flows back to miners as supplemental rewards

All decay calculations use integer-only fixed-point arithmetic with `u64` for consensus determinism.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#concentration-decay-mechanism) for the full mathematical specification.

---

## Technical Details

### Consensus

- **Block Time**: 60 seconds target
- **Proof of Work**: SHA-256 double-hash (Phase 1); RandomX planned for Phase 2
- **Difficulty Adjustment**: LWMA over 60-block window, clamped to [prev/3, prev*3]
- **Supply Cap**: 21,000,000 RILL (mining) + 1,050,000 RILL (dev fund premine, 4-year vesting)
- **Initial Reward**: 50 RILL per block, halving every 210,000 blocks
- **UTXO Model**: Bitcoin-style unspent transaction outputs
- **Coinbase Maturity**: 100 blocks

### Network Protocol

- **Transport**: TCP with libp2p Noise encryption (XX handshake) and Yamux multiplexing
- **Block/TX Gossip**: Gossipsub topics `/rill/blocks/1` and `/rill/txs/1`
- **Peer Discovery**: Kademlia DHT + mDNS
- **Wire Format**: bincode

### Storage (RocksDB Column Families)

| Column Family   | Contents                                    |
|-----------------|---------------------------------------------|
| `blocks`        | Full block data                             |
| `headers`       | Block headers                               |
| `utxos`         | Unspent transaction output set              |
| `height_index`  | Height-to-hash mapping                      |
| `undo`          | Undo data for chain reorganizations         |
| `metadata`      | Chain tip, UTXO count, and misc metadata    |
| `clusters`      | Aggregate cluster balances for decay        |
| `address_index` | Address-to-UTXO index                       |

---

## Documentation

- **[docs/TESTNET.md](docs/TESTNET.md)**: Building, running, mining, CLI usage, Docker, RPC reference
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)**: Crate descriptions, design decisions, storage layout, decay mechanism
- **[docs/AGENT-RUNNER.md](docs/AGENT-RUNNER.md)**: Multi-agent development workflow
- **ADRs**: `.claude/skills/architecture/` — Architectural Decision Records
- **Decay Spec**: `.claude/skills/decay-mechanics/` — Mathematical specification

---

## Contributing

Before submitting changes:

1. Run `cargo test --workspace`
2. Run `cargo clippy --workspace -- -D warnings`
3. Run `cargo fmt --check`
4. Ensure all public APIs have doc comments

This project uses specialized subagents for development. See `.claude/agents/` for the agent architecture.

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

---

## Philosophy

> "Wealth should flow like water."

RillCoin explores what happens when concentrated wealth naturally circulates rather than accumulating indefinitely. The decay mechanism is transparent, predictable, and governed by mathematics rather than discretion.

Large holders can avoid decay by spending, distributing holdings across multiple addresses, or participating in the economy rather than purely accumulating. The goal is a cryptocurrency that remains liquid and accessible, where the economic incentives favor circulation over concentration.

---

**Status**: Phase 4 implementation complete. Full node, wallet, and miner binaries functional. 920+ tests. Testnet deployment ready.
