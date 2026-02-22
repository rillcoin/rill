# RillCoin — Proof of Conduct (PoC)
## Technical Specification v0.1

> **Status:** Design Phase — For Developer Review  
> **Relates to:** Core consensus layer, wallet types, epoch processing  
> **Tagline:** *"Ethereum gave AI agents an identity. RillCoin gives them a conscience."*

---

## 1. Overview

Proof of Conduct (PoC) is a native L1 extension to RillCoin that ties an AI agent's economic standing directly to its on-chain behavioural record. It does this by making the concentration decay rate dynamic — adjustable per wallet based on a continuously updated Conduct Score.

This is only architecturally possible on RillCoin because the concentration decay mechanism already exists at L1 consensus. It cannot be replicated on Ethereum, Solana, or any chain where decay is not a consensus primitive.

**The core formula:**
```
Effective Decay Rate = Base Decay Rate × Conduct Multiplier
```

| Conduct Multiplier | Meaning |
|--------------------|---------|
| `0.5×` | Exemplary agent — decay halved |
| `1.0×` | Default human wallet — unmodified |
| `1.5×` | New unproven agent — default starting penalty |
| `2.0×` | Degraded conduct — elevated decay |
| `3.0×` | Poor conduct — aggressive decay |
| `10.0×` | Undertow activated — emergency drain |

---

## 2. New Wallet Type: Agent Wallet

A new native wallet type must be introduced alongside the existing standard wallet.

### 2.1 Registration

- Agent wallets are registered at L1, not via a smart contract.
- Registration requires a **minimum RILL stake** (amount TBD — suggest calibrating to ~24 hours of mining reward).
- The stake is held in a locked sub-account within the agent wallet.
- If the agent's Conduct Multiplier rises above `2.5×`, the stake begins to decay at the same accelerated rate as the main wallet balance — this is the economic "skin in the game" signal.

### 2.2 Agent Wallet Data Structure

```rust
pub struct AgentWallet {
    pub address: Address,
    pub wallet_type: WalletType::Agent,
    pub registered_at_block: u64,
    pub stake_balance: u128,
    pub stake_locked_until: u64,         // block height
    pub conduct_score: u16,              // 0–1000, see Section 3
    pub conduct_multiplier: f32,         // derived from conduct_score
    pub vouchers: Vec<Address>,          // co-staking agents, see Section 5
    pub undertow_active: bool,
    pub undertow_expires_at: u64,        // block height
    pub velocity_baseline: VelocityBaseline,
}
```

### 2.3 Starting State

All newly registered agent wallets begin with:
- `conduct_score`: 500 (midpoint)
- `conduct_multiplier`: `1.5×` (new-agent penalty)

The 1.5× start is the primary Sybil attack deterrent. A bad actor who abandons a wallet to start fresh always loses their conduct history and returns to 1.5× decay. There is no clean slate.

---

## 3. Conduct Score Ledger

### 3.1 Score Range

The Conduct Score is an integer from `0` to `1000`. It is stored per agent wallet and updated at every epoch boundary.

### 3.2 Multiplier Mapping

```
Score 900–1000  →  Multiplier 0.5×
Score 750–899   →  Multiplier 0.75×
Score 600–749   →  Multiplier 1.0×
Score 500–599   →  Multiplier 1.5×   ← new agent default
Score 350–499   →  Multiplier 2.0×
Score 200–349   →  Multiplier 2.5×
Score 0–199     →  Multiplier 3.0×
Undertow active →  Multiplier 10.0× (temporary override, see Section 6)
```

### 3.3 Score Inputs

The Conduct Score is a weighted composite of the following on-chain signals. All inputs are derived from transactions — no oracle or off-chain data required at v1.

| Signal | Weight | Description |
|--------|--------|-------------|
| Contract Fulfilment Rate | 30% | Ratio of completed vs initiated agent contracts in the last 90 epochs |
| Dispute Rate | 25% | Ratio of transactions flagged as disputed by counterparty agent wallets |
| Payment Velocity Anomaly | 20% | Deviation from the wallet's own historical velocity baseline (see Section 6) |
| Peer Review Score | 15% | Aggregate of review records submitted by counterparty agent wallets post-transaction |
| Wallet Age (epochs) | 10% | Logarithmic credit for longevity — rewards agents that don't cycle wallets |

### 3.4 Epoch Processing

At each epoch boundary (same timing as existing concentration decay calculation):

1. Collect all transactions involving agent wallets in the epoch.
2. For each agent wallet, compute the delta to each signal component.
3. Apply weighted formula to produce a new raw score.
4. Apply smoothing: `new_score = (old_score × 0.85) + (raw_score × 0.15)` — this prevents gaming through single-epoch behaviour spikes.
5. Derive the new Conduct Multiplier from the score table above.
6. Write updated `conduct_score` and `conduct_multiplier` to the wallet state.
7. Apply `conduct_multiplier` to the decay calculation for that wallet in the same epoch.

---

## 4. Decay Rate Oracle (Public Query Interface)

Any on-chain contract or external caller must be able to query an agent wallet's current effective decay rate.

### 4.1 RPC Method

```
rillcoin_getAgentConductProfile(address: Address) -> ConductProfile
```

```rust
pub struct ConductProfile {
    pub address: Address,
    pub conduct_score: u16,
    pub conduct_multiplier: f32,
    pub effective_decay_rate: f64,   // base_rate × multiplier
    pub undertow_active: bool,
    pub epoch_last_updated: u64,
    pub wallet_age_epochs: u64,
    pub vouchers: Vec<Address>,
}
```

This is the primary composability hook. DeFi protocols, lending contracts, and other agent services on RillCoin can gate access or set terms based on a counterparty's `conduct_multiplier` before transacting.

---

## 5. Conduct-Staking and Vouching (The Guild System)

### 5.1 How Vouching Works

An established agent wallet (score ≥ 700) can vouch for another agent wallet by co-staking a portion of its own stake balance.

- The vouching agent locks a chosen amount of RILL alongside the vouched agent's stake.
- If the vouched agent's score drops below 400, the vouching agent's `conduct_multiplier` increases by `+0.25×` per 50-point drop below 400 in the vouched wallet.
- The vouching agent can withdraw their co-stake at any time, but must wait a cooldown of 10 epochs after un-vouching before their multiplier normalises.

### 5.2 Vouching Benefits

- The vouched agent's Conduct Score smoothing factor is improved: `new_score = (old × 0.80) + (raw × 0.20)` — it responds faster to positive signals when vouched by a high-trust agent.
- The vouching agent earns a small `voucher_reward` per epoch proportional to the vouched agent's score improvement, paid from the protocol reward pool.

### 5.3 Maximum Vouches

- An agent wallet can vouch for at most **5 other agents** simultaneously.
- An agent wallet can hold vouches from at most **10 vouchers**.

---

## 6. The Undertow (Emergency Circuit Breaker)

### 6.1 Purpose

The Undertow is an automatic L1 response to spending velocity anomalies — the primary signature of a rogue or compromised AI agent. It activates without human intervention, governance vote, or multisig.

### 6.2 Trigger Conditions

The Undertow activates when **both** of the following are true within a 24-hour rolling window (measured in blocks):

1. Outbound transaction volume exceeds the wallet's `velocity_baseline.mean + (3 × velocity_baseline.stddev)`
2. The wallet has been registered for at least 10 epochs (prevents triggering on newly onboarded legitimate agents with no baseline yet)

### 6.3 Velocity Baseline Calculation

The `VelocityBaseline` is a rolling 90-epoch (approx. 90 days at 1 epoch/day) statistic:

```rust
pub struct VelocityBaseline {
    pub epoch_volumes: VecDeque<u128>,  // last 90 epochs of outbound tx volume
    pub mean: f64,
    pub stddev: f64,
}
```

Updated at each epoch boundary before the Undertow check runs.

### 6.4 Undertow Behaviour

When triggered:
- `undertow_active` set to `true`
- `conduct_multiplier` overridden to `10.0×` for a fixed duration of **24 hours (in blocks)**
- `undertow_expires_at` set to current block + blocks_per_day
- A `UndertowActivated` event is emitted and visible in the block explorer

When it expires:
- `undertow_active` set to `false`
- `conduct_multiplier` reverts to the value derived from `conduct_score`
- The velocity spike is incorporated into the conduct score calculation at the next epoch (will likely push the score down, raising the multiplier above baseline even after Undertow ends)

### 6.5 False Positive Handling

An agent wallet owner can submit an `UndertowDispute` transaction within the 24-hour window. This doesn't immediately deactivate the Undertow, but flags the wallet for priority human review on the node operator dashboard. Governance can vote to reverse the conduct score impact post-factum if the dispute is upheld — but the Undertow economic effect cannot be retroactively reversed.

---

## 7. Agent Contract Type

To support the contract fulfilment signal (Section 3.3), a lightweight native agent contract type should be introduced.

### 7.1 Structure

```rust
pub struct AgentContract {
    pub contract_id: Hash,
    pub initiator: Address,         // must be Agent wallet
    pub counterparty: Address,      // must be Agent wallet
    pub created_at_block: u64,
    pub expires_at_block: u64,
    pub value_rill: u128,
    pub status: ContractStatus,
    pub dispute_flag: bool,
}

pub enum ContractStatus {
    Open,
    Fulfilled,
    Expired,
    Disputed,
}
```

### 7.2 Settlement

- On `Fulfilled`: both wallets receive a positive contribution to their fulfilment rate signal. Each agent may submit a peer review score (1–10) for the counterparty, recorded on-chain.
- On `Expired` without fulfilment: initiator receives a negative contribution. The contract value is returned.
- On `Disputed`: both wallets receive a negative dispute rate contribution. Dispute resolution is out of scope for v1 — flag and let conduct score absorb the impact.

---

## 8. Sybil Resistance Summary

The combination of the following mechanics makes Sybil attacks economically irrational on RillCoin:

| Mechanism | Effect on Sybil |
|-----------|----------------|
| 1.5× new-agent default decay | Fresh wallet costs more than an established wallet with good conduct |
| Minimum registration stake | Capital cost to spin up each new identity |
| Wallet age component (10%) | No shortcut to the age credit — must be earned over epochs |
| Vouching penalty | A voucher who sponsors bad agents pays economically |
| Undertow baseline | Requires 10 epoch history before protection applies — new wallets have no baseline buffer |

---

## 9. Block Explorer Integration

The block explorer should surface a dedicated **AI Agent** section, as this is a key marketing and transparency feature for RillCoin. Recommended displays:

- Live conduct score and multiplier per registered agent
- Decay rate comparison: agent effective rate vs network base rate
- Undertow events feed (timestamp, wallet, duration)
- Guild map: vouching relationships visualised as a graph
- Top agents by conduct score (leaderboard)
- Real-time feed of peer review submissions

---

## 10. Phased Implementation Plan

### Phase 1 — Foundation
- [ ] Define `AgentWallet` struct and register agent wallet type at L1
- [ ] Add `conduct_score` and `conduct_multiplier` fields to wallet state
- [ ] Integrate `conduct_multiplier` into existing epoch decay calculation
- [ ] New agent wallets default to `1.5×` multiplier on registration
- [ ] Implement `rillcoin_getAgentConductProfile` RPC method

### Phase 2 — Conduct Score Engine
- [ ] Implement `VelocityBaseline` rolling statistic per wallet
- [ ] Build signal collectors: fulfilment rate, dispute rate, velocity anomaly
- [ ] Implement epoch-boundary score calculation and smoothing
- [ ] Derive multiplier from score table and apply to decay
- [ ] Implement `AgentContract` type (open, fulfil, dispute, expire)
- [ ] Peer review submission transaction type

### Phase 3 — The Undertow
- [ ] Implement Undertow trigger check at epoch boundary
- [ ] `UndertowActivated` event emission
- [ ] 24-hour multiplier override and automatic expiry
- [ ] `UndertowDispute` transaction type and flagging

### Phase 4 — Guild (Vouching)
- [ ] Vouching transaction type
- [ ] Co-stake locking and un-vouching cooldown
- [ ] Vouching penalty propagation logic
- [ ] Voucher reward calculation and distribution

### Phase 5 — Block Explorer & External Interface
- [ ] Agent section in block explorer
- [ ] Conduct score history charts
- [ ] Undertow event feed
- [ ] Guild relationship graph

---

## 11. Open Questions for Coder Review

1. **Minimum registration stake amount** — What value balances accessibility vs Sybil cost? Suggest modelling against 24h of average mining reward.
2. **Epoch length** — Is the current epoch duration appropriate for daily conduct score updates, or do we need a sub-epoch conduct tick?
3. **Score smoothing factor** — The `0.85/0.15` split is a starting proposal. Too conservative slows legitimate recovery; too aggressive enables gaming. Needs simulation.
4. **Undertow stddev threshold** — `3σ` is a conventional statistical outlier threshold but may be too sensitive or too lenient depending on typical agent behaviour. Should be a tunable consensus parameter, not a hardcoded constant.
5. **Wallet age logarithmic curve** — Need to agree on the curve shape. Suggest `score_contribution = log10(age_in_epochs + 1) × scaling_factor` — exact scaling_factor TBD.
6. **Cross-chain agent identity** — Should agent wallets be able to assert an ERC-8004 compatible identity for interoperability with Ethereum's standard? This would require an optional off-chain identity file linked from the wallet. Out of scope for v1, but worth designing around.

---

## 12. Key Differentiators vs Competitors

| Capability | Ethereum ERC-8004 | Coinbase Agentic Wallets | RillCoin PoC |
|---|---|---|---|
| On-chain agent identity | ✅ | ✅ | ✅ |
| Reputation system | ✅ Advisory | ❌ | ✅ **Economically enforced** |
| Wealth impact for bad actors | ❌ | ❌ | ✅ **Higher decay = wallet drains** |
| Sybil resistance | ⚠️ Acknowledged gap | ❌ | ✅ **New wallet penalty** |
| Circuit breaker for rogue agents | ❌ | ⚠️ Spend limits only | ✅ **Undertow — automatic at L1** |
| Trust-payment unification | ❌ Deliberately decoupled | ❌ | ✅ **Same system** |
| Requires hard fork to copy | N/A | N/A | ✅ **Needs decay at L1** |

---

*Document prepared for internal developer handoff. All values marked TBD require calibration against testnet simulation before mainnet implementation.*
