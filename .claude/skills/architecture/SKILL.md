---
name: architecture
description: >
  Architectural Decision Records for RillCoin. Load when making design
  decisions, resolving trade-offs, or reviewing why a particular technology
  was chosen. Contains all ADRs and the crate dependency graph.
---

# RillCoin Architecture Decision Records

## ADR-001: Rust 2024 edition, stable, MSRV 1.85
Rust for memory safety without GC. Stable toolchain for reproducible builds.

## ADR-002: Ed25519 over secp256k1
Faster verification (~3x), no signature malleability, simpler implementation. Trade-off: less Bitcoin ecosystem tooling compatibility.

## ADR-003: BLAKE3 Merkle trees, SHA-256 block headers
BLAKE3 for speed in Merkle trees (internal). SHA-256 for block headers to maintain RandomX compatibility for Phase 2 mining.

## ADR-004: Integer-only consensus math
No floats anywhere in consensus-critical code. Fixed-point `u64` with `10^8` precision. Prevents platform-dependent rounding differences.

## ADR-005: libp2p (Gossipsub + Kademlia + Noise)
Gossipsub for block/tx propagation. Kademlia for peer discovery. Noise for encrypted transport. Mature Rust implementation.

## ADR-006: RocksDB with column families
Column families: `blocks`, `headers`, `utxos`, `peers`, `meta`. Provides atomic writes across families and efficient prefix scans.

## ADR-007: bincode for wire protocol
Compact binary serialization. Deterministic encoding for consensus. ~10x faster than JSON, ~3x smaller.

## ADR-008: Mock PoW Phase 1, RandomX Phase 2
Phase 1 uses simple hash-prefix PoW for fast iteration. Phase 2 switches to RandomX for ASIC resistance.

## ADR-009: 5% dev fund in genesis
Genesis block allocates 5% of max supply to development fund. Vests linearly over 4 years.

## ADR-010: Address format
`rill1<base58check(sha256(ripemd160(pubkey)))>`
Human-readable prefix for identification. Double-hash for collision resistance. Base58Check for error detection.

## ADR-011: Proof of Conduct (PoC) — Dynamic Decay for AI Agents

**Status:** Accepted (2026-02-22)
**Spec:** `PROOF_OF_CONDUCT_SPEC.md` | **Review:** `PROOF_OF_CONDUCT_REVIEW.md`

### Context
AI agent wallets need on-chain reputation that has economic teeth. Existing solutions (ERC-8004, Coinbase Agentic Wallets) decouple reputation from economics. RillCoin's L1 decay primitive uniquely enables tying conduct directly to wealth erosion.

### Decision
Adopt Proof of Conduct as a native L1 extension. The concentration decay rate becomes dynamic per agent wallet, controlled by a Conduct Score (0–1000) that derives a Conduct Multiplier applied to the base decay rate.

### Constraints (from technical review)
1. **Integer-only:** All conduct math uses fixed-point. Multiplier stored as BPS (`u64`), not `f32`. Variance comparison replaces stddev (no sqrt). Smoothing uses `(85 * old + 15 * raw) / 100`.
2. **Conduct period ≠ halving epoch:** Introduce "conduct period" (~1,440 blocks / 1 day) distinct from halving epoch (210,000 blocks). Conduct scores update per conduct period.
3. **Typed transactions:** New `TxType` enum on `Transaction` for agent operations (RegisterAgent, AgentContract*, PeerReview, Vouch/Unvouch, UndertowDispute). No backwards-compat concern pre-mainnet.
4. **Effective rate cap:** `effective_decay_rate` capped at `DECAY_PRECISION` to prevent overflow under Undertow (10× multiplier).
5. **State storage:** New RocksDB column family `agent_wallets` per ADR-006.
6. **Bounded collections:** Max 10 vouchers per wallet, max 5 vouch targets. Enforced at block validation.

### Phasing
- **Phase 1:** WalletType enum, AgentWallet struct, conduct_multiplier_bps integration in decay engine, RegisterAgent tx, RPC query. (Next)
- **Phase 2:** Conduct score engine, signal collectors, AgentContract type, conduct period processing.
- **Phase 3:** Undertow circuit breaker with variance-based trigger.
- **Phase 4:** Guild/vouching system.
- **Phase 5:** Block explorer integration.

### Trade-offs
- **Complexity:** 4+ new transaction types, new state, epoch processing extension. Mitigated by phased rollout.
- **Consensus surface area:** Conduct score is consensus-critical (all nodes must agree). Mitigated by integer-only math + on-chain-only inputs.
- **Agent wallet overhead:** Additional per-wallet state. Mitigated by bounded fields and minority wallet type.

## Crate Dependency Graph

```
rill-core (foundation types, traits, errors, constants)
  ↓
rill-decay (concentration decay algorithm, cluster index)
  ↓
rill-consensus (block validation, chain selection, UTXO management)
  ↓
rill-network (P2P, libp2p, message relay)
  ↓
rill-wallet (keys, addresses, transaction construction)
  ↓
rill-node (full node binary, RPC, storage, main loop)
```
