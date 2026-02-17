# Changelog

## [Unreleased]

### 2026-02-17 - Phase 3+4: Production Readiness & Testnet Prep

**Phase 3 — Core Infrastructure:**
- RandomX PoW engine (feature-gated behind `randomx` flag), SHA-256 mock PoW default
- Header-first chain sync protocol with single-peer block download
- UTXO address index with O(k) lookups via RocksDB secondary index
- Cluster balance tracking and RPC query support (`getclusterbalance`)
- Expanded storage layer (533 LOC) with column families for address index
- Argon2id wallet encryption replacing raw AES-256-GCM
- CLI RPC integration for balance, send, and address commands
- Network request-response protocol for block/header fetching

**Phase 4 — Testnet Readiness (5 items):**
- Multi-peer parallel sync: PeerState tracking, timeout detection (30s), ban after 3 failures, round-robin block assignment across peers
- BIP-39 mnemonic backup: 24-word seed phrases for wallet create/restore, hex as fallback
- Decay-aware coin selection: CLI `send` now uses `CoinSelector::select()` (highest-decay UTXOs spent first), fetches cluster balances via RPC
- MIN_TX_FEE enforcement: mempool rejects transactions with fee < 1000 rills
- 15 end-to-end integration tests: mine, spend, difficulty, decay, wallet lifecycle, security regression

**Stats:**
- 818 tests passing (up from 76), zero clippy warnings
- 28 files changed, 3,289 insertions, 5 new files
- Commit: `bfd53c2`, pushed to origin/main

### 2026-02-17 - Blocker Fixes: VULN-COINBASE-TXID & Fee Computation

**VULN-COINBASE-TXID (High — resolved):**
- Coinbase transactions now set `lock_time = height` instead of `lock_time = 0`
- Since `lock_time` is included in the witness-stripped txid, each block height produces a unique coinbase txid
- Updated all `make_coinbase_unique` helpers across chain_state, storage, security_audit, and storage_test
- E2E regression test now verifies 3 blocks to same address produce 3 distinct UTXOs

**Node::process_transaction fee=0 (Medium — resolved):**
- `process_transaction()` now computes actual fee (`input_sum - output_sum`) before mempool insertion
- Uses checked arithmetic (`checked_add`, `checked_sub`) per project conventions
- RPC `sendrawtransaction` now works correctly with MIN_TX_FEE enforcement

**Stats:**
- 818 tests passing, zero clippy warnings, zero active blockers
- 8 files changed, 63 insertions
- Commit: `89f7eca`, pushed to origin/main

### 2026-02-16 - Repository Setup & Documentation

**Infrastructure:**
- Verified SSH access to GitHub (rillcoin account)
- Configured GitHub CLI authentication for rillcoin organization
- Pushed local repository to GitHub at `rillcoin/rill` for the first time
- Set up remote tracking for main branch

**Documentation:**
- Created comprehensive README.md (220 lines)
  - Project overview and philosophy
  - Architecture diagram and crate dependency graph
  - Getting started guide for all three binaries
  - Technical specifications (consensus, decay, network, storage)
  - Development standards and contribution guidelines
  - Build, test, and deployment instructions

**Status:**
- All tests passing (76 tests, 0 failures)
- Build clean across workspace
- Repository publicly accessible at https://github.com/rillcoin/rill
