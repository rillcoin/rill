import type { Metadata } from "next";
import Link from "next/link";
import { ArrowLeft, ArrowRight } from "lucide-react";
import CodeBlock from "@/components/CodeBlock";

export const metadata: Metadata = {
  title: "Mining",
  description:
    "RillCoin mining guide — SHA-256 PoW, difficulty adjustment, halving schedule, block templates, and decay pool rewards.",
};

// Halving schedule: block 0, 210000, 420000, ...
// Block time: 60s → blocks/year ≈ 525960
// ~4 years ≈ 210000 blocks → 2026-02 start
const HALVING_SCHEDULE = [
  { era: 1, height: 0, reward: "50", approxDate: "2025 (genesis)", cumulative: "10,500,000" },
  { era: 2, height: 210000, reward: "25", approxDate: "~2029", cumulative: "15,750,000" },
  { era: 3, height: 420000, reward: "12.5", approxDate: "~2033", cumulative: "18,375,000" },
  { era: 4, height: 630000, reward: "6.25", approxDate: "~2037", cumulative: "19,687,500" },
  { era: 5, height: 840000, reward: "3.125", approxDate: "~2041", cumulative: "20,343,750" },
  { era: 6, height: 1050000, reward: "1.5625", approxDate: "~2045", cumulative: "20,671,875" },
];

export default function MiningPage() {
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
          Mining
        </h1>
        <p className="text-base" style={{ color: "var(--text-muted)" }}>
          SHA-256 proof-of-work, difficulty adjustment, halving schedule, block
          templates, and decay pool supplemental rewards.
        </p>
      </div>

      <div className="doc-prose">
        {/* Overview */}
        <h2>Overview</h2>
        <p>
          RillCoin uses SHA-256 proof-of-work — the same algorithm as Bitcoin.
          Miners compute a valid block header hash below the current target and
          submit it via the <code>submitblock</code> RPC. Difficulty adjusts
          every <code>DIFFICULTY_WINDOW = 60</code> blocks to maintain the
          60-second target block time.
        </p>

        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-8">
          {[
            { label: "Algorithm", value: "SHA-256" },
            { label: "Block Time", value: "60 seconds" },
            { label: "Initial Reward", value: "50 RILL" },
            { label: "Difficulty Window", value: "60 blocks" },
          ].map(({ label, value }) => (
            <div
              key={label}
              className="rounded-lg p-4 text-center"
              style={{
                background: "var(--raised)",
                border: "1px solid var(--border-subtle)",
              }}
            >
              <span
                className="block text-xs uppercase tracking-wider mb-1"
                style={{ color: "var(--text-dim)" }}
              >
                {label}
              </span>
              <span
                className="block text-base font-semibold font-mono"
                style={{ color: "var(--text-primary)" }}
              >
                {value}
              </span>
            </div>
          ))}
        </div>

        {/* Block reward */}
        <h2>Block Reward</h2>
        <p>
          The block reward consists of three components:
        </p>
        <CodeBlock language="formula">
          {`total_reward = block_subsidy + transaction_fees + decay_pool_release

block_subsidy  = INITIAL_REWARD >> (height / HALVING_INTERVAL)
               = 50 RILL >> (height / 210_000)

decay_pool_release = decay_pool_balance × DECAY_POOL_RELEASE_BPS / 10_000
                   = decay_pool_balance × 1%`}
        </CodeBlock>
        <p>
          The decay pool release provides an additional incentive that grows as
          concentration decay accumulates. As more tokens decay into the pool,
          miners receive larger supplemental rewards each block.
        </p>

        {/* Halving schedule */}
        <h2>Halving Schedule</h2>
        <p>
          Block subsidies halve every 210,000 blocks (~4 years at 60-second
          block times). Total mineable supply asymptotically approaches
          21,000,000 RILL.
        </p>

        <div
          className="rounded-lg overflow-hidden mb-6"
          style={{ border: "1px solid var(--border-dim)" }}
        >
          <table style={{ marginBottom: 0 }}>
            <thead>
              <tr>
                <th>Era</th>
                <th>Start Height</th>
                <th>Block Reward</th>
                <th>Approx. Date</th>
                <th>Cumulative Mined</th>
              </tr>
            </thead>
            <tbody>
              {HALVING_SCHEDULE.map((row) => (
                <tr key={row.era}>
                  <td>{row.era}</td>
                  <td>{row.height.toLocaleString()}</td>
                  <td>
                    <span style={{ color: "var(--orange-400)" }}>
                      {row.reward} RILL
                    </span>
                  </td>
                  <td style={{ color: "var(--text-dim)" }}>{row.approxDate}</td>
                  <td>{row.cumulative} RILL</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Coinbase maturity */}
        <h2>Coinbase Maturity</h2>
        <p>
          Coinbase outputs (block rewards) cannot be spent until{" "}
          <code>COINBASE_MATURITY = 100</code> additional blocks have been
          mined on top of the block containing the coinbase transaction. This
          prevents reorganization-based reward theft.
        </p>
        <p>
          For example, if you mine a block at height 1,000, your coinbase UTXO
          is spendable starting at height 1,100.
        </p>

        {/* Difficulty adjustment */}
        <h2>Difficulty Adjustment</h2>
        <p>
          Difficulty adjusts every 60 blocks. The new target is computed from
          the ratio of actual elapsed time to expected time:
        </p>
        <CodeBlock language="rust" title="Difficulty adjustment algorithm">
          {`// Every DIFFICULTY_WINDOW (60) blocks:
//
// expected_time = DIFFICULTY_WINDOW × BLOCK_TIME_SECS
//              = 60 × 60 = 3,600 seconds
//
// actual_time = timestamp[tip] - timestamp[tip - DIFFICULTY_WINDOW]
//
// new_target = old_target × actual_time / expected_time
//
// Clamped to [old_target / 4, old_target × 4] to prevent extreme swings.

fn adjust_difficulty(old_target: u32, actual_secs: u64) -> u32 {
    let expected = DIFFICULTY_WINDOW as u64 * BLOCK_TIME_SECS;
    let ratio = actual_secs.clamp(expected / 4, expected * 4);

    // Scale compact nBits target
    compact_to_u256(old_target)
        .saturating_mul(ratio)
        .saturating_div(expected)
        .to_compact()
}`}
        </CodeBlock>

        {/* Block template */}
        <h2>Block Template</h2>
        <p>
          Miners request a block template from a synced node via the{" "}
          <code>getblocktemplate</code> RPC. The template includes all pending
          transactions sorted by fee rate.
        </p>
        <CodeBlock language="json" title="getblocktemplate response">
          {`{
  "version":           1,
  "prev_hash":         "a3f8...64b2",
  "merkle_root":       "d1e2...89ab",  // pre-computed with coinbase
  "timestamp":         1740000000,
  "difficulty_target": 486604799,
  "nonce":             0,              // start here, increment to find solution
  "height":            42381,
  "transactions": [
    {
      "txid":  "7f3a...12cd",
      "data":  "01000000...",          // hex-encoded bincode transaction
      "fee":   1000                   // in rills
    }
  ]
}`}
        </CodeBlock>

        {/* Submit block */}
        <h2>Submitting a Block</h2>
        <p>
          Once a valid nonce is found, serialize the complete block using
          bincode and hex-encode it, then submit via <code>submitblock</code>:
        </p>
        <CodeBlock language="bash" title="Submit block via RPC">
          {`curl -X POST http://127.0.0.1:28332 \\
  -H "Content-Type: application/json" \\
  -d '{
    "jsonrpc": "2.0",
    "method":  "submitblock",
    "params":  ["01000000a3f8..."],
    "id": 1
  }'

# Success response:
# {"jsonrpc":"2.0","result":"ok","id":1}

# Error response (invalid block):
# {"jsonrpc":"2.0","error":{"code":-1,"message":"invalid proof of work"},"id":1}`}
        </CodeBlock>

        {/* Quick start */}
        <h2>Quick Start: rill-miner</h2>
        <p>
          The <code>rill-miner</code> binary provides a CPU miner for testnet
          use. Install and run:
        </p>
        <CodeBlock language="bash" title="Install rill-miner">
          {`# From GitHub releases
wget https://github.com/rillcoin/rill/releases/latest/download/rill-miner-linux-x86_64.tar.gz
tar xzf rill-miner-linux-x86_64.tar.gz
sudo mv rill-miner /usr/local/bin/

# Or build from source (requires Rust 1.85+)
git clone https://github.com/rillcoin/rill
cd rill
cargo build --release -p rill-miner
sudo cp target/release/rill-miner /usr/local/bin/`}
        </CodeBlock>
        <CodeBlock language="bash" title="Run the miner">
          {`# First create a wallet to receive rewards
rill-cli wallet create --network testnet
rill-cli address  # note your trill1... address

# Start mining (testnet)
rill-miner \\
  --rpc     http://127.0.0.1:28332 \\
  --address trill1qw5r3k8d9...    \\
  --threads 4

# Mainnet
rill-miner \\
  --rpc     http://127.0.0.1:18332 \\
  --address rill1qw5r3k8d9...     \\
  --threads $(nproc)`}
        </CodeBlock>

        <h3>Miner Output</h3>
        <CodeBlock language="text">
          {`[2026-02-19 12:00:00] RillCoin Miner v0.1.0
[2026-02-19 12:00:00] Connected to node at 127.0.0.1:28332
[2026-02-19 12:00:00] Mining address: trill1qw5r3k8d9...
[2026-02-19 12:00:00] Threads: 4
[2026-02-19 12:00:01] New template: height=42381 target=0x1d00ffff
[2026-02-19 12:01:03] BLOCK FOUND! height=42381 nonce=0x1a3f8b2c hash=0000001f...
[2026-02-19 12:01:03] Reward: 25.00000000 RILL + 0.12500000 RILL (decay pool)
[2026-02-19 12:01:03] Hashrate: 8.4 MH/s`}
        </CodeBlock>

        {/* Notes */}
        <div
          className="rounded-xl p-5 mt-8"
          style={{
            background: "var(--raised)",
            border: "1px solid var(--border-dim)",
          }}
        >
          <h4 style={{ marginTop: 0 }}>Notes for Miners</h4>
          <ul style={{ marginBottom: 0 }}>
            <li>
              Always mine to a wallet you control — never to an exchange address.
              Coinbase outputs have a 100-block maturity delay.
            </li>
            <li>
              The decay pool release adds a variable component to each block
              reward. A node with more circulating decay yields higher pool
              rewards.
            </li>
            <li>
              CPU mining is only practical on testnet. Mainnet will require
              SHA-256 ASICs for competitive mining.
            </li>
            <li>
              Ensure your node is fully synced before mining. Stale templates
              result in rejected blocks.
            </li>
          </ul>
        </div>
      </div>

      {/* Navigation */}
      <div
        className="flex items-center justify-between mt-12 pt-6"
        style={{ borderTop: "1px solid var(--border-subtle)" }}
      >
        <Link
          href="/decay"
          className="flex items-center gap-2 text-sm transition-colors"
          style={{ color: "var(--text-muted)" }}
        >
          <ArrowLeft size={14} />
          Decay Mechanics
        </Link>
        <Link
          href="/cli"
          className="flex items-center gap-2 text-sm font-medium transition-colors"
          style={{ color: "var(--cyan-400)" }}
        >
          Wallet & CLI
          <ArrowRight size={14} />
        </Link>
      </div>
    </div>
  );
}
