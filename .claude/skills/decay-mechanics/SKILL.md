---
name: decay-mechanics
description: >
  Reference material for RillCoin's concentration decay algorithm.
  Load when working on decay calculations, sigmoid functions, cluster
  indexing, or economic modeling. Contains the mathematical specification
  and implementation guidance.
---

# Decay Mechanics Reference

## Sigmoid Decay Function

The decay rate for a wallet is calculated using a sigmoid curve applied to its concentration ratio:

```
concentration_ratio = wallet_balance / total_supply
decay_rate = max_decay_rate * sigmoid(concentration_ratio, threshold, steepness)
```

### Fixed-Point Implementation

All calculations use `u64` with `10^8` precision factor (`PRECISION = 100_000_000`).

The sigmoid is implemented as a **precomputed lookup table** with 1024 entries covering the `[0, 1]` range of concentration ratios. Linear interpolation between table entries.

```rust
// Lookup table entry
struct SigmoidEntry {
    input: u64,    // concentration_ratio * PRECISION
    output: u64,   // sigmoid_value * PRECISION
}
```

### Parameters (Phase 1 defaults)

- `MAX_DECAY_RATE`: 5% per block period (`5_000_000` in fixed-point)
- `THRESHOLD`: 1% of supply (`1_000_000` in fixed-point)
- `STEEPNESS`: 10 (`1_000_000_000` in fixed-point)

## Cluster Index

Detects coordinated wallet groups attempting to evade decay by splitting holdings.

### Algorithm

1. Build transaction graph for trailing `CLUSTER_WINDOW` blocks
2. Identify connected components (wallets that transact with each other)
3. Score each cluster: `cluster_balance = sum(member_balances)`
4. Apply decay to `cluster_balance` instead of individual balances
5. Distribute proportionally: each member decays by `(member_balance / cluster_balance) * cluster_decay`

### Invariants

- Cluster merge must be commutative: `merge(A, B) == merge(B, A)`
- Cluster merge must be associative: `merge(merge(A, B), C) == merge(A, merge(B, C))`
- Singleton clusters (one wallet) must produce identical results to non-clustered decay

## Decay Pool

Decayed tokens flow to the `decay_pool`. Miners receive: `block_reward = base_reward + (decay_pool * REDISTRIBUTION_RATE)`.

**Critical invariant:** `total_effective_supply + decay_pool == total_mined_supply` must hold after every block.

See `references/sigmoid-table-generator.md` for the table generation algorithm.
