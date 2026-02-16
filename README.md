# Rill

> **Wealth should flow like water.**

A cryptocurrency with progressive concentration decay. Holdings above configurable thresholds gradually decay back into the mining pool, promoting circulation and discouraging long-term hoarding.

## Overview

Rill implements a novel economic model where concentrated wealth naturally flows back into circulation through a mathematically-governed decay mechanism. Large balances above threshold values experience sigmoid-function-based decay, with funds returning to miners as block rewards.

## Key Features

- **Concentration Decay**: Balances above thresholds decay to the mining pool using fixed-point sigmoid curves
- **Integer-Only Consensus**: All consensus math uses `u64` fixed-point arithmetic (10^8 precision) - no floating point
- **Ed25519 Signatures**: Fast, secure transaction signing with 32-byte public keys
- **BLAKE3 Merkle Trees**: High-performance transaction tree hashing
- **libp2p Networking**: Production-grade P2P with Gossipsub, Kademlia DHT, and MDNS discovery
- **RocksDB Storage**: Persistent blockchain storage with column families for blocks, transactions, and UTXO sets

## Architecture

```
rill-core       → Foundation types (Transaction, Block, Address, UTXO)
  ↓
rill-decay      → Concentration decay algorithm with fixed-point math
  ↓
rill-consensus  → Block validation, chain selection, mining rewards
  ↓
rill-network    → P2P networking layer (libp2p)
  ↓
rill-wallet     → HD wallet with decay-aware coin selection
  ↓
rill-node       → Full node with RPC server and mempool
```

## Project Structure

```
rill/
├── crates/
│   ├── rill-core/       # Core types and traits
│   ├── rill-decay/      # Decay algorithm implementation
│   ├── rill-consensus/  # Consensus rules and validation
│   ├── rill-network/    # P2P networking (libp2p)
│   ├── rill-wallet/     # HD wallet and transaction signing
│   ├── rill-node/       # Full node library
│   └── rill-tests/      # Integration and property tests
├── bins/
│   ├── rill-node/       # Full node binary
│   ├── rill-cli/        # Command-line wallet
│   └── rill-miner/      # Mining daemon
└── docs/                # Additional documentation
```

## Getting Started

### Prerequisites

- Rust 1.85+ (2024 edition)
- Cargo

### Building

```bash
# Clone the repository
git clone https://github.com/rillcoin/rill.git
cd rill

# Build all crates and binaries
cargo build --release

# Run tests
cargo test --workspace

# Run clippy
cargo clippy --workspace -- -D warnings
```

### Running a Node

```bash
# Start a full node
cargo run --release --bin rill-node

# Or use the compiled binary
./target/release/rill-node
```

### Using the CLI Wallet

```bash
# Create a new wallet
cargo run --release --bin rill-cli -- wallet create

# Check balance
cargo run --release --bin rill-cli -- wallet balance

# Send transaction
cargo run --release --bin rill-cli -- wallet send <address> <amount>
```

### Mining

```bash
# Start mining
cargo run --release --bin rill-miner
```

## Development

### Code Standards

- **Rust Edition**: 2024
- **MSRV**: 1.85
- **Formatting**: `cargo fmt` (default settings)
- **Linting**: `cargo clippy -- -D warnings` (zero warnings policy)
- **Testing**: All public APIs require doc comments and proptest coverage

### Testing

```bash
# Unit tests
cargo test --workspace

# Integration tests
cargo test -p rill-tests

# Property tests
cargo test --release proptest

# Benchmarks
cargo bench
```

### Git Workflow

- Branch naming: `<agent>/<description>` (e.g., `core/implement-transaction-type`)
- Commit messages: `<crate>: <description>` (e.g., `rill-core: implement Transaction struct`)
- Always run `cargo test --workspace` before committing
- Pre-commit hooks enforce code quality and project isolation

## Technical Details

### Consensus

- **Block Time**: ~10 minutes target
- **Proof of Work**: SHA-256 (mock implementation in Phase 1)
- **UTXO Model**: Bitcoin-style unspent transaction outputs
- **Transaction Format**: Bincode-serialized with Ed25519 signatures

### Decay Mechanics

The decay algorithm operates on individual UTXOs based on their age and value:

1. UTXOs below threshold: no decay
2. UTXOs above threshold: sigmoid decay curve applied
3. Decay rate increases with concentration (value above threshold)
4. Decayed value returns to mining pool as future block rewards

All decay calculations use integer-only fixed-point math with `u64` for consensus-critical determinism.

### Network Protocol

- **Transport**: TCP with libp2p
- **Encryption**: Noise protocol (XX handshake)
- **Multiplexing**: Yamux
- **Gossip**: Gossipsub for blocks and transactions
- **Discovery**: Kademlia DHT + mDNS for local peers
- **Wire Format**: Bincode

### Storage

RocksDB column families:
- `blocks`: Block headers and metadata
- `transactions`: Full transaction data
- `utxo`: Unspent transaction output set
- `mempool`: Pending transactions
- `chainstate`: Chain tip and metadata

## Documentation

- **ADRs**: `.claude/skills/architecture/` - Architectural Decision Records
- **Decay Mechanics**: `.claude/skills/decay-mechanics/` - Mathematical specification
- **Agent System**: `docs/AGENT-RUNNER.md` - Multi-agent development workflow

## Contributing

This project uses specialized subagents for development. See `.claude/agents/` for the agent architecture.

Before submitting changes:
1. Run `cargo test --workspace`
2. Run `cargo clippy --workspace -- -D warnings`
3. Run `cargo fmt --check`
4. Ensure all public APIs have documentation

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Philosophy

> "Wealth should flow like water."

RillCoin explores what happens when concentrated wealth naturally circulates rather than accumulating indefinitely. The decay mechanism is transparent, predictable, and governed by mathematics rather than discretion.

Large holders can avoid decay by:
- Spending or donating funds
- Distributing holdings across multiple addresses
- Participating in the economy rather than purely accumulating

The goal is a cryptocurrency that remains liquid and accessible, where the economic incentives favor circulation over concentration.

---

**Status**: Phase 1 implementation complete. Full node, wallet, and miner binaries functional. Testnet deployment pending.
