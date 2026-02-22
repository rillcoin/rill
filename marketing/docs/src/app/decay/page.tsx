import type { Metadata } from "next";
import Link from "next/link";
import { ArrowLeft, ArrowRight } from "lucide-react";
import CodeBlock from "@/components/CodeBlock";

export const metadata: Metadata = {
  title: "Decay Mechanics",
  description:
    "Deep technical reference for RillCoin concentration decay — sigmoid table, fixed-point arithmetic, cluster tracking, lineage decay, and the decay pool.",
};

const SIGMOID_TABLE = [
  [0.0, 0.5000000],
  [0.5, 0.6224593],
  [1.0, 0.7310586],
  [1.5, 0.8175744],
  [2.0, 0.8807970],
  [2.5, 0.9241418],
  [3.0, 0.9525741],
  [3.5, 0.9706878],
  [4.0, 0.9820137],
  [4.5, 0.9890130],
  [5.0, 0.9933071],
  [5.5, 0.9959298],
  [6.0, 0.9975274],
  [6.5, 0.9984965],
  [7.0, 0.9990889],
  [7.5, 0.9994472],
  [8.0, 0.9996646],
];

export default function DecayPage() {
  return (
    <div className="max-w-4xl mx-auto px-6 py-12 lg:py-16">
      <div className="mb-10">
        <div className="flex items-center gap-2 mb-4">
          <span
            className="text-xs font-semibold uppercase tracking-widest"
            style={{ color: "var(--text-dim)" }}
          >
            Protocol
          </span>
        </div>
        <h1
          className="font-serif mb-3"
          style={{ fontSize: "2.5rem", lineHeight: 1.15, color: "var(--text-primary)" }}
        >
          Decay Mechanics
        </h1>
        <p className="text-base" style={{ color: "var(--text-muted)" }}>
          Complete technical reference for concentration decay — the sigmoid
          function, fixed-point arithmetic, cluster tracking, lineage decay, and
          the decay pool.
        </p>
      </div>

      <div className="doc-prose">
        {/* Overview */}
        <div
          className="rounded-xl p-5 mb-8"
          style={{
            background: "var(--raised)",
            border: "1px solid var(--border-dim)",
          }}
        >
          <h4 style={{ marginTop: 0 }}>Design Principle</h4>
          <p style={{ marginBottom: 0 }}>
            Concentration decay is RillCoin&apos;s core economic primitive. When any
            cluster of UTXOs exceeds a threshold concentration relative to the
            circulating supply, those holdings decay at a rate proportional to
            their concentration. Decayed tokens flow to the mining reward pool,
            redistributing wealth to active network participants. All arithmetic
            is integer-only using checked u64 with no floating point.
          </p>
        </div>

        {/* Concentration metric */}
        <h2>Concentration Metric</h2>
        <p>
          Concentration is measured in <strong>parts-per-billion (PPB)</strong>.
          A PPB value of 1,000,000 (one million) represents a concentration of
          0.1% of the circulating supply — this is the decay threshold.
        </p>
        <CodeBlock language="rust" title="Concentration formula (rill-decay/src/lib.rs)">
          {`// concentration_ppb = cluster_balance × 1_000_000_000 / circulating_supply
//
// cluster_balance:    sum of all UTXOs with this cluster_id (in rills)
// circulating_supply: total unspent supply (in rills)
// Result:             parts per billion (u64)

pub fn concentration_ppb(cluster_balance: u64, circulating_supply: u64) -> u64 {
    cluster_balance
        .checked_mul(CONCENTRATION_PRECISION)  // × 1_000_000_000
        .unwrap_or(u64::MAX)
        .checked_div(circulating_supply)
        .unwrap_or(0)
}

pub const CONCENTRATION_PRECISION: u64 = 1_000_000_000;
pub const DECAY_C_THRESHOLD_PPB:   u64 = 1_000_000;    // 0.1% of supply`}
        </CodeBlock>
        <p>
          A cluster must exceed <code>DECAY_C_THRESHOLD_PPB</code> (1,000,000
          PPB = 0.1% of supply) before any decay is applied. Below this
          threshold, the decay rate is exactly zero.
        </p>

        {/* Cluster index */}
        <h2>Cluster Index</h2>
        <p>
          Every UTXO is tagged with a <code>cluster_id</code> — a 32-byte
          BLAKE3 hash stored in the <code>TxOutput.cluster_id</code> field. All
          UTXOs sharing a cluster_id are aggregated into a single{" "}
          <em>cluster balance</em> for decay calculation.
        </p>
        <p>
          The cluster index is maintained in RocksDB as a mapping:
        </p>
        <CodeBlock language="text">
          {`cluster_id  →  aggregate_balance (u64 rills)`}
        </CodeBlock>
        <p>
          When a UTXO is spent, its value is subtracted from the cluster
          balance. When a UTXO is created, its value is added. The cluster
          balance is always the sum of all <em>unspent</em> UTXOs with that
          cluster_id.
        </p>
        <p>
          The default cluster_id for wallet-generated outputs is:
        </p>
        <CodeBlock language="formula">
          {`default_cluster_id = BLAKE3(ed25519_root_public_key_bytes)`}
        </CodeBlock>

        {/* Sigmoid function */}
        <h2>Sigmoid Decay Function</h2>
        <p>
          The decay rate uses a fixed-point sigmoid lookup table. The sigmoid
          function maps concentration (normalized by{" "}
          <code>CONCENTRATION_PRECISION</code>) to a rate value. Linear
          interpolation is used between table entries.
        </p>
        <p>
          Constants:
        </p>
        <ul>
          <li>
            <code>SIGMOID_PRECISION = 1,000,000,000</code> — Fixed-point scale
            for sigmoid output
          </li>
          <li>
            <code>TABLE_STEP = 500,000,000</code> — Step size between entries
            (0.5 in float terms)
          </li>
        </ul>

        <div
          className="rounded-lg overflow-hidden mb-6"
          style={{ border: "1px solid var(--border-dim)" }}
        >
          <table style={{ marginBottom: 0 }}>
            <thead>
              <tr>
                <th>x (concentration)</th>
                <th>sigmoid(x)</th>
                <th>Fixed-point (× 10⁹)</th>
              </tr>
            </thead>
            <tbody>
              {SIGMOID_TABLE.map(([x, y]) => (
                <tr key={x}>
                  <td>{x.toFixed(1)}</td>
                  <td>{y.toFixed(7)}</td>
                  <td>{Math.round(y * 1_000_000_000).toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <h3>Linear Interpolation</h3>
        <p>
          For a concentration value <em>x</em> between table entries, the
          sigmoid output is linearly interpolated:
        </p>
        <CodeBlock language="rust" title="Sigmoid interpolation">
          {`pub fn sigmoid_fixed(x: u64) -> u64 {
    // x is in fixed-point with CONCENTRATION_PRECISION = 1_000_000_000
    // TABLE_STEP = 500_000_000 (represents 0.5 in float)

    let idx = (x / TABLE_STEP) as usize;

    if idx >= SIGMOID_TABLE.len() - 1 {
        return SIGMOID_TABLE[SIGMOID_TABLE.len() - 1];
    }

    let y0 = SIGMOID_TABLE[idx];
    let y1 = SIGMOID_TABLE[idx + 1];
    let frac = x % TABLE_STEP;  // fractional part within step

    // Linear interpolation: y0 + (y1 - y0) * frac / TABLE_STEP
    y0 + (y1.saturating_sub(y0))
            .checked_mul(frac)
            .unwrap_or(0)
            / TABLE_STEP
}`}
        </CodeBlock>

        {/* Decay rate calculation */}
        <h2>Decay Rate Calculation</h2>
        <p>
          The effective decay rate in PPB per block is derived from the sigmoid
          output:
        </p>
        <CodeBlock language="rust" title="Decay rate formula">
          {`// decay_rate = (sigmoid(concentration_x) - 0.5) × DECAY_R_MAX_PPB × 2
//
// Constants:
//   DECAY_R_MAX_PPB = 1_500_000_000  (150% per year at max, in PPB units)
//   SIGMOID_HALF    = 500_000_000    (0.5 in fixed-point)

pub fn decay_rate_ppb(concentration_ppb: u64) -> u64 {
    if concentration_ppb < DECAY_C_THRESHOLD_PPB {
        return 0;
    }

    // Normalize concentration for sigmoid input
    let x = concentration_ppb
        .checked_mul(CONCENTRATION_PRECISION)
        .unwrap_or(u64::MAX)
        / DECAY_C_THRESHOLD_PPB;

    let sig = sigmoid_fixed(x);

    // Shift by 0.5 and scale to max rate
    let shifted = sig.saturating_sub(SIGMOID_HALF);

    shifted
        .checked_mul(DECAY_R_MAX_PPB)
        .unwrap_or(u64::MAX)
        .checked_mul(2)
        .unwrap_or(u64::MAX)
        / SIGMOID_PRECISION
}

pub const DECAY_R_MAX_PPB: u64 = 1_500_000_000; // 150% per year at max concentration
pub const SIGMOID_HALF:    u64 =   500_000_000; // 0.5 in fixed-point
pub const SIGMOID_PRECISION: u64 = 1_000_000_000;`}
        </CodeBlock>

        <h3>Decay Rate Interpretation</h3>
        <div
          className="rounded-lg overflow-hidden mb-6"
          style={{ border: "1px solid var(--border-dim)" }}
        >
          <table style={{ marginBottom: 0 }}>
            <thead>
              <tr>
                <th>Concentration</th>
                <th>% of Supply</th>
                <th>Approx. Annual Decay</th>
              </tr>
            </thead>
            <tbody>
              {[
                ["Below threshold", "< 0.1%", "0% — no decay"],
                ["At threshold (1,000,000 PPB)", "0.1%", "~0% — minimal"],
                ["1% of supply (10,000,000 PPB)", "1%", "~15% / year"],
                ["5% of supply (50,000,000 PPB)", "5%", "~120% / year"],
                ["10%+ of supply", "10%+", "~150% / year (max)"],
              ].map(([conc, pct, rate]) => (
                <tr key={conc}>
                  <td>{conc}</td>
                  <td>{pct}</td>
                  <td
                    style={{
                      color:
                        rate === "0% — no decay"
                          ? "var(--cyan-400)"
                          : "var(--orange-400)",
                    }}
                  >
                    {rate}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Effective value */}
        <h2>Effective Value</h2>
        <p>
          The <strong>effective value</strong> of a UTXO is its nominal value
          reduced by accrued decay. Decay accrues linearly with the number of
          blocks the UTXO has been held.
        </p>
        <CodeBlock language="rust" title="Effective value calculation">
          {`// effective_value = nominal × (1 - decay_rate × blocks_held / DECAY_PRECISION)
//
// DECAY_PRECISION = 10_000_000_000 (10^10)
// All arithmetic uses checked u64 — no floating point.

pub fn effective_value(
    nominal: u64,
    decay_rate_ppb: u64,
    blocks_held: u64,
) -> u64 {
    let decay_factor = decay_rate_ppb
        .checked_mul(blocks_held)
        .unwrap_or(u64::MAX);

    let decay_amount = nominal
        .checked_mul(decay_factor)
        .unwrap_or(u64::MAX)
        .checked_div(DECAY_PRECISION)
        .unwrap_or(nominal);

    nominal.saturating_sub(decay_amount)
}

pub const DECAY_PRECISION: u64 = 10_000_000_000; // 10^10`}
        </CodeBlock>
        <p>
          When a UTXO is spent, the difference between nominal and effective
          value flows to the decay pool. Coin selection in the wallet prefers
          high-decay UTXOs to minimize ongoing decay losses.
        </p>

        {/* Lineage decay */}
        <h2>Lineage Decay</h2>
        <p>
          In addition to concentration-based decay, RillCoin tracks{" "}
          <strong>lineage</strong> — how long value has been concentrated in
          the same cluster lineage. Lineage decay adds a secondary pressure that
          grows with holding duration, independent of concentration.
        </p>
        <div
          className="rounded-lg overflow-hidden mb-6"
          style={{ border: "1px solid var(--border-dim)" }}
        >
          <table style={{ marginBottom: 0 }}>
            <thead>
              <tr>
                <th>Constant</th>
                <th>Value</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              {[
                [
                  "LINEAGE_HALF_LIFE",
                  "52,596 blocks",
                  "~100 days — lineage weight halves",
                ],
                [
                  "LINEAGE_FULL_RESET",
                  "525,960 blocks",
                  "~1 year — lineage resets to zero",
                ],
              ].map(([k, v, d]) => (
                <tr key={k}>
                  <td>
                    <code>{k}</code>
                  </td>
                  <td>{v}</td>
                  <td>{d}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p>
          Lineage is tracked per cluster. When a UTXO is spent and a new output
          is created in the same cluster, the lineage counter increments.
          Lineage resets to zero after <code>LINEAGE_FULL_RESET</code> blocks
          without activity, encouraging periodic economic participation.
        </p>

        {/* Decay pool */}
        <h2>Decay Pool</h2>
        <p>
          All decayed amounts flow into a global <strong>decay pool</strong>.
          The decay pool is a persistent balance maintained in RocksDB, credited
          atomically when UTXOs are spent.
        </p>
        <p>
          Each block, a fraction of the decay pool is released to the block
          miner:
        </p>
        <CodeBlock language="formula">
          {`decay_pool_release = decay_pool_balance × DECAY_POOL_RELEASE_BPS / 10_000

where DECAY_POOL_RELEASE_BPS = 100  (1% per block)`}
        </CodeBlock>
        <p>
          This 1% per block release creates a smooth, continuous flow of
          redistributed wealth to miners rather than a sudden dump. A pool of
          100,000 RILL would release 1,000 RILL to the next block&apos;s miner.
        </p>
        <p>
          The decay pool release is included in the coinbase transaction output
          value and subject to the same <code>COINBASE_MATURITY = 100</code>{" "}
          block maturity requirement.
        </p>

        {/* Example walkthrough */}
        <h2>Example: Full Decay Calculation</h2>
        <p>
          Suppose a cluster holds 1,000,000 RILL (100,000,000,000,000 rills)
          and the circulating supply is 10,000,000 RILL (1,000,000,000,000,000
          rills). The cluster holds 10% of supply.
        </p>
        <CodeBlock language="text" title="Step-by-step example">
          {`1. Concentration PPB:
   cluster_balance = 100_000_000_000_000 rills  (1,000,000 RILL)
   supply          = 1_000_000_000_000_000 rills (10,000,000 RILL)
   concentration   = 100_000_000_000_000 × 1_000_000_000 / 1_000_000_000_000_000
                   = 100_000_000 PPB  (10% of supply)

2. Decay threshold check:
   100_000_000 PPB > 1_000_000 PPB (threshold) → proceed

3. Sigmoid lookup:
   x = 100_000_000 × 1_000_000_000 / 1_000_000 = 100_000_000_000
   (well above table maximum → clamp to SIGMOID_TABLE.last())
   sigmoid(x) ≈ 999_664_600 (in fixed-point × 10^9)

4. Decay rate:
   shifted = 999_664_600 - 500_000_000 = 499_664_600
   rate    = 499_664_600 × 1_500_000_000 × 2 / 1_000_000_000
           ≈ 1_498_993_800 PPB/year

5. Effective value after 1 block (525,960 blocks/year):
   decay_rate_per_block = 1_498_993_800 / 525_960 ≈ 2_849 PPB
   decay_amount = nominal × 2_849 / 10_000_000_000
   For a 1,000,000 RILL UTXO held 1 block:
   decay = 100_000_000_000_000 × 2_849 / 10_000_000_000 ≈ 28_490 rills
         = 0.00028490 RILL per block`}
        </CodeBlock>
      </div>

      {/* Navigation */}
      <div
        className="flex items-center justify-between mt-12 pt-6"
        style={{ borderTop: "1px solid var(--border-subtle)" }}
      >
        <Link
          href="/protocol"
          className="flex items-center gap-2 text-sm transition-colors"
          style={{ color: "var(--text-muted)" }}
        >
          <ArrowLeft size={14} />
          Architecture
        </Link>
        <Link
          href="/conduct"
          className="flex items-center gap-2 text-sm font-medium transition-colors"
          style={{ color: "var(--cyan-400)" }}
        >
          Proof of Conduct
          <ArrowRight size={14} />
        </Link>
      </div>
    </div>
  );
}
