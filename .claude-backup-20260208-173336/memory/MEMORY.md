# Rill — Project Memory
_Last updated: 2026-02-08_

## What Is Rill
RillCoin: progressive concentration decay cryptocurrency. Holdings above thresholds decay to mining pool. "Wealth should flow like water."

## Current State
- **Phase:** Phase 1 — All 6 library crates + rill-node binary complete
- **Tests:** 699 total, all passing, zero warnings
- **Phase 1 target:** Core types + decay algorithm (Weeks 1-4)
- **Milestone 1:** Private testnet (8-12 weeks)

## Crate Status
| Crate | Status | Tests |
|-------|--------|-------|
| rill-core | Complete | 443 |
| rill-decay | Complete | 71 |
| rill-consensus | Complete | 26 |
| rill-network | Complete | 30 |
| rill-node-lib | Complete | 53 |
| rill-wallet | Complete | 76 |
| bins/rill-node | **Complete** | 0 |
| bins/rill-cli | Stub | 0 |
| bins/rill-miner | Stub | 0 |

## Architecture
6 library crates + 3 binaries. Rust Cargo workspace.
- rill-core → rill-decay → rill-consensus → rill-network → rill-wallet → rill-node

## What's Next
1. **bins/rill-miner** — Mining loop: template → mine → submit
2. **bins/rill-cli** — Wallet CLI: create, address, send via RPC
3. Private testnet milestone: multi-node, mine, transact
