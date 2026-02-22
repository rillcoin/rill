# Proof of Conduct — Technical Review

> **Reviewer:** Claude (core/decay/consensus agents)
> **Date:** 2026-02-22
> **Input:** `PROOF_OF_CONDUCT_SPEC.md` v0.1
> **Verdict:** Architecturally sound. Requires significant rework of data types before implementation. No blockers to proceeding with Phase 1.

---

## Executive Summary

The PoC spec is a strong design that leverages RillCoin's unique L1 decay primitive. The core idea — making decay rate dynamic per wallet based on on-chain behaviour — is architecturally clean and fits naturally into the existing epoch processing pipeline.

However, the spec was written from a product/design perspective and contains **12 violations** of our Rust conventions and consensus rules that must be resolved before any code is written. Most are straightforward fixed-point conversions. Two require design decisions.

---

## 1. Convention Violations

### V-01: `f32` in `AgentWallet.conduct_multiplier` (CRITICAL)

**Spec says:** `pub conduct_multiplier: f32`
**Rule:** ADR-004 — No floats anywhere in consensus-critical code.

**Fix:** Use fixed-point `u64` with BPS (basis points) precision:
```rust
/// Conduct multiplier in basis points. 10_000 = 1.0×, 15_000 = 1.5×
pub conduct_multiplier_bps: u64,
```

Multiplier table becomes:
| Score Range | Multiplier | BPS Value |
|-------------|-----------|-----------|
| 900–1000 | 0.5× | 5_000 |
| 750–899 | 0.75× | 7_500 |
| 600–749 | 1.0× | 10_000 |
| 500–599 | 1.5× | 15_000 |
| 350–499 | 2.0× | 20_000 |
| 200–349 | 2.5× | 25_000 |
| 0–199 | 3.0× | 30_000 |
| Undertow | 10.0× | 100_000 |

**Decay integration:**
```rust
effective_decay_rate = base_decay_rate
    .checked_mul(conduct_multiplier_bps)?
    .checked_div(BPS_PRECISION)?;
```

### V-02: `f64` in `ConductProfile.effective_decay_rate` (CRITICAL)

**Spec says:** `pub effective_decay_rate: f64`
**Fix:** Use `u64` in PPB (parts-per-billion), matching existing `DECAY_R_MAX_PPB` precision.

### V-03: `f64` in `VelocityBaseline.mean` and `stddev` (CRITICAL)

**Spec says:** `pub mean: f64, pub stddev: f64`
**Fix:** Use `u128` for mean (same unit as epoch volumes — rills). For stddev, store variance as `u128` and compare `(volume - mean)^2 > 9 * variance` instead of `volume > mean + 3σ`. This avoids square roots entirely.

```rust
pub struct VelocityBaseline {
    pub epoch_volumes: VecDeque<u128>,  // last 90 epochs
    pub mean: u128,                     // in rills
    pub variance: u128,                 // in rills²
}
```

**Undertow check becomes:**
```rust
let delta = volume.abs_diff(mean);
// delta > 3σ  ⟺  delta² > 9 × variance
let delta_sq = (delta as u128).checked_mul(delta as u128)?;
let threshold = variance.checked_mul(9)?;
let triggered = delta_sq > threshold;
```

### V-04: Smoothing formula uses float coefficients (MODERATE)

**Spec says:** `new_score = (old_score × 0.85) + (raw_score × 0.15)`
**Fix:** Use integer arithmetic with a denominator:
```rust
// 85/100 old + 15/100 raw = (85 * old + 15 * raw) / 100
let new_score = old_score.checked_mul(85)?
    .checked_add(raw_score.checked_mul(15)?)?
    .checked_div(100)?;
```

Vouched variant: `(80 * old + 20 * raw) / 100`

### V-05: Unbounded `Vec<Address>` in `AgentWallet.vouchers` (MODERATE)

**Spec says:** `pub vouchers: Vec<Address>` — but Section 5.3 says max 10 vouchers.
**Fix:** Use a bounded type and enforce the limit at validation:
```rust
pub vouchers: Vec<Address>,  // invariant: vouchers.len() <= MAX_VOUCHERS
pub const MAX_VOUCHERS: usize = 10;
pub const MAX_VOUCH_TARGETS: usize = 5;
```
Block validation must reject transactions that would exceed these bounds.

### V-06: `u128` for `stake_balance` and `value_rill` (MINOR)

**Spec says:** `pub stake_balance: u128` and `pub value_rill: u128`
**Note:** Our existing codebase uses `u64` for all monetary values (max ~1.84 × 10^19 rills = ~184 billion RILL). Since MAX_TOTAL_SUPPLY is ~22M RILL = 2.2 × 10^15 rills, `u64` is sufficient. However, `VelocityBaseline.epoch_volumes` may need `u128` if a single agent processes very high volume in one epoch.

**Decision needed:** Standardize on `u64` for balances (consistent with existing code), `u128` only for intermediate arithmetic.

### V-07: `WalletType::Agent` syntax in struct literal (MINOR)

**Spec says:** `pub wallet_type: WalletType::Agent`
**Fix:** This is invalid Rust. Should be:
```rust
pub wallet_type: WalletType,

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletType {
    Standard,
    Agent,
}
```

---

## 2. Architectural Integration Points

### I-01: Where does `WalletType` live?

**Recommendation:** `rill-core/src/types.rs` — it's a foundation type that consensus, wallet, and node all need to reference. The `AgentWallet` extended fields belong in a new `rill-core/src/agent.rs` module.

### I-02: Conduct multiplier integration with existing decay

The existing decay pipeline in `rill-decay/src/engine.rs` computes:
```
effective_value = nominal × (1 - decay_rate)^blocks_held
```

The conduct multiplier modifies `decay_rate` before this calculation:
```
adjusted_rate = min(decay_rate × conduct_multiplier_bps / BPS_PRECISION, DECAY_R_MAX_PPB)
```

**Capping at R_MAX is essential.** A 10× multiplier on a high-concentration wallet could produce a rate > 100%, which is economically nonsensical. The spec doesn't mention this cap — it must be added.

### I-03: Epoch boundary processing order

The spec says conduct score and decay happen "at the same epoch boundary." The exact ordering matters:

1. Update `VelocityBaseline` from epoch transaction data
2. Check Undertow trigger conditions
3. Collect signal components (fulfilment, disputes, velocity, peer review, age)
4. Compute raw score → apply smoothing → derive multiplier
5. **Then** run existing decay calculation with the new multiplier

This must be deterministic across all nodes. The ordering should be specified as a consensus rule.

### I-04: New transaction types needed

The spec implies several new transaction types:
- `RegisterAgent` — creates agent wallet with stake
- `AgentContractOpen` / `AgentContractFulfil` / `AgentContractDispute`
- `PeerReview` — post-transaction review submission
- `Vouch` / `Unvouch` — co-staking operations
- `UndertowDispute` — challenge an Undertow activation

These need to be added to the `Transaction` type (likely as a new `tx_type` field or as special output scripts). **Design decision required** — see D-01 below.

### I-05: State storage

Agent wallet metadata must be persisted. Options:
- **A)** New RocksDB column family `agent_wallets` (preferred — clean separation)
- **B)** Extend UTXO entries with agent metadata (couples concerns)

Recommendation: Option A, with a new column family. Consistent with ADR-006.

---

## 3. Design Decisions Required

### D-01: Transaction type mechanism

How do we represent agent-specific transactions?

**Option A — Typed transactions:** Add a `TxType` enum to `Transaction`. Clean but breaks serialization compatibility.
**Option B — Special opcodes in output scripts:** Bitcoin-style. Backwards compatible but complex.
**Option C — Dedicated agent transaction struct:** Separate from financial transactions. Cleanest separation but more consensus code.

**Recommendation:** Option A. We're pre-mainnet — no backwards compatibility concern. A `TxType` enum is the simplest path.

### D-02: Conduct score as consensus-critical or soft state?

The spec implies conduct score is consensus-critical (all nodes must agree on the exact score). This means:
- Score calculation must be **fully deterministic** (integer-only ✓)
- All signal inputs must be derivable from the block chain alone (the spec says yes for v1)
- Score must be committed to block state (e.g., in a state root)

**Recommendation:** Consensus-critical. The whole point is that decay — a consensus primitive — depends on it.

### D-03: Conduct multiplier cap

The spec doesn't define what happens when `base_rate × 10.0` exceeds the maximum meaningful rate.

**Recommendation:** Cap `effective_decay_rate` at `DECAY_PRECISION` (100% per block). In practice, Undertow at 10× on a high-concentration wallet means the wallet drains extremely fast — which is the intended behaviour — but it should never produce arithmetic overflow.

---

## 4. Open Questions Resolved

From Section 11 of the spec:

| # | Question | Resolution |
|---|----------|------------|
| 1 | Minimum registration stake | Defer to testnet simulation. Use a consensus parameter, not a constant. |
| 2 | Epoch length | Current epoch (210,000 blocks ≈ 146 days) is too long for daily conduct updates. **Need a sub-epoch "conduct tick"** — suggest every 1,440 blocks (≈1 day). |
| 3 | Score smoothing factor | 85/15 as integer `(85 * old + 15 * raw) / 100`. Tunable via consensus parameter. |
| 4 | Undertow σ threshold | Use variance comparison (no sqrt). 3σ → compare `delta² > 9 × variance`. Make the `9` a consensus parameter. |
| 5 | Wallet age curve | `log10(age + 1) × scaling_factor` uses floats. Replace with a lookup table: 10 entries mapping age ranges to score contributions. Integer-only, deterministic, tuneable. |
| 6 | Cross-chain identity | Out of scope for v1. Design wallet metadata to be extensible (reserved fields). |

### New issue: Conduct tick vs halving epoch

The spec conflates "epoch" (our halving interval, 210K blocks) with what should be a shorter "conduct epoch" or "conduct tick." These are different concepts:
- **Halving epoch:** 210,000 blocks. Controls reward schedule.
- **Conduct tick:** ~1,440 blocks (1 day). Controls conduct score updates and Undertow checks.

The spec should be updated to use distinct terminology. Suggest: **"conduct period"** for the daily tick.

---

## 5. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Integer overflow in multiplied decay rates | High | Cap at DECAY_PRECISION. Use checked arithmetic. u128 intermediates. |
| Conduct score disagreement between nodes | High | All inputs on-chain. Deterministic integer math. Include score in state commitment. |
| Sybil via vouching rings | Medium | Vouching penalty propagation limits this. Max 5 vouch targets. Simulate on testnet. |
| Undertow false positives | Medium | Variance-based detection. 10-epoch minimum history. Dispute mechanism exists. |
| State bloat from agent metadata | Low | Bounded voucher lists. Agent wallets are a minority of total wallets. |
| Complexity budget | Medium | 4 new transaction types + epoch processing + new state. Phased approach is correct. |

---

## 6. Recommendation

**Proceed with Phase 1** after resolving:

1. ✅ All float types → fixed-point (V-01 through V-04) — straightforward
2. ✅ Bounded vectors (V-05) — straightforward
3. ⚠️ Transaction type mechanism (D-01) — needs decision, recommend Option A
4. ⚠️ Conduct tick vs halving epoch (new issue) — needs spec update
5. ⚠️ Effective decay rate cap (D-03) — needs explicit constant

Phase 1 scope (from spec) is well-bounded:
- `WalletType` enum + `AgentWallet` struct in `rill-core`
- `conduct_multiplier_bps` integration in `rill-decay`
- `RegisterAgent` transaction type
- `rillcoin_getAgentConductProfile` RPC method
- Default 1.5× (15,000 BPS) for new agent wallets

This is achievable without touching the conduct score engine, Undertow, or vouching — those come in Phases 2–4.

---

*Review complete. Ready for ADR-011 and implementation planning.*
