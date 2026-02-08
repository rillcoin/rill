# decay Agent — Session Notes
## Status: Phase 1 Complete
## Last Session: 2026-02-06
## Tests: 71 passing (unit + proptest)

## Modules Implemented

### sigmoid.rs — Fixed-point sigmoid lookup table
- 17-entry precomputed table at x = 0.0, 0.5, 1.0, ..., 8.0
- Linear interpolation between entries
- `sigmoid_positive(x_scaled: u128) -> u64` — returns sigmoid * SIGMOID_PRECISION (1e9)
- Input scaled by CONCENTRATION_PRECISION (1e9): x_scaled=1e9 → sigmoid(1.0)
- Saturates at table max for x > 8.0
- Callers use symmetry sigmoid(-x) = PRECISION - sigmoid(x) for negative inputs

### engine.rs — DecayEngine implementing DecayCalculator
- `DecayEngine::new()` — zero-config, all params from rill-core constants
- `decay_rate_ppb(concentration_ppb)` — returns 0 below threshold, sigmoid-based above
  - rate = R_MAX * sigmoid(k * (C - C_threshold))
  - Jump from 0 to ~R_MAX*0.5 at threshold boundary (intended for whales only)
- `compute_decay(nominal, conc, blocks)` — compound decay via binary exponentiation
  - `fixed_pow(base, exp, precision)` — O(log n) fixed-point exponentiation, u128 intermediates
  - retention_total = ((PRECISION - rate) / PRECISION)^blocks_held
  - effective = nominal * retention_total / PRECISION, decay = nominal - effective
- `decay_pool_release(pool_balance)` — 1% release per block (DECAY_POOL_RELEASE_BPS / BPS_PRECISION)

### cluster.rs — UTXO lineage-based clustering
- `determine_output_cluster(input_cluster_ids, txid)` — coinbase→new, single→inherit, multi→BLAKE3 merge
  - Dedup + sort for determinism, order-independent
- `lineage_factor(blocks_held)` — piecewise linear: [0,HALF_LIFE]→[1.0,0.5], [HALF_LIFE,FULL_RESET]→[0.5,0.0]
- `lineage_adjusted_balance(nominal, blocks)` — nominal * lineage_factor / PRECISION

## Design Decisions
1. **Sigmoid table at 0.5 intervals** — 17 entries covers [0,8] with max interpolation error ~0.4%
2. **Clamp at threshold** — rate=0 below threshold, jump to ~R_MAX*0.5 at boundary
   - Acceptable because threshold=0.1% of supply is whale territory
   - Alternative (2*sigmoid-1 mapping) considered but more complex for marginal benefit
3. **Binary exponentiation (fixed_pow)** — O(log n) compound decay, u128 intermediates prevent overflow
4. **Piecewise linear lineage** — steep first half (100%→50% over HALF_LIFE), gentle second half (50%→0% over 9*HALF_LIFE)
5. **BLAKE3 cluster merge** — hash sorted deduped cluster IDs for deterministic merge
6. **No floats anywhere** — all const table values hardcoded as integer literals

## Key Constants (from rill-core)
- DECAY_R_MAX_PPB = 1,500,000,000 (15% per block max rate)
- DECAY_PRECISION = 10,000,000,000 (denominator for rate)
- DECAY_C_THRESHOLD_PPB = 1,000,000 (0.1% concentration threshold)
- CONCENTRATION_PRECISION = 1,000,000,000
- DECAY_K = 2000 (sigmoid steepness)
- LINEAGE_HALF_LIFE = 52,596 (~1 month)
- LINEAGE_FULL_RESET = 525,960 (~1 year)

## Dependencies Added
- blake3 (for cluster merge hashing)

## What's Next
1. rill-decay Phase 1 complete — sigmoid, engine, clustering all implemented
2. Next: rill-consensus (RandomX PoW, difficulty adjustment, block validation)
3. Future: decay pool state integration when rill-node connects everything
