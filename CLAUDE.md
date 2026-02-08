# Rill — RillCoin Cryptocurrency

Progressive concentration decay cryptocurrency. Holdings above thresholds decay to the mining pool. "Wealth should flow like water."

## Project

- **Language:** Rust 2024 edition, stable toolchain, MSRV 1.85
- **Structure:** Cargo workspace — 6 library crates + 3 binaries
- **Crate graph:** rill-core → rill-decay → rill-consensus → rill-network → rill-wallet → rill-node

## Conventions

- Integer-only consensus math. No floats. Fixed-point u64 with 10^8 precision.
- All public APIs get doc comments and proptest coverage.
- Error types in `rill-core/src/error.rs`, constants in `constants.rs`.
- Run `cargo clippy --workspace -- -D warnings` before committing.
- Run `cargo test --workspace` to verify.

## Key Decisions

See `.claude/skills/architecture/` for the full ADR log. Summary: Ed25519 signatures, BLAKE3 Merkle trees, SHA-256 block headers, libp2p networking, RocksDB storage, bincode wire protocol, mock PoW for Phase 1.

## Agent Architecture

This project uses specialized subagents in `.claude/agents/`. Claude auto-delegates based on task type. Critical agents (decay, consensus, test) run on Opus; implementation agents run on Sonnet. See agent descriptions for delegation guidance.

## Rules

Modular rules in `.claude/rules/`. Isolation rules prevent cross-project contamination with Subtone and Renewly.
