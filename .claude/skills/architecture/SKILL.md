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
