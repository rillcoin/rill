import type { Metadata } from "next";
import Link from "next/link";
import { ArrowLeft, ArrowRight } from "lucide-react";
import CodeBlock from "@/components/CodeBlock";

export const metadata: Metadata = {
  title: "Whitepaper",
  description:
    "RillCoin technical whitepaper — progressive concentration decay, protocol constants, cryptography, and network architecture.",
};

export default function WhitepaperPage() {
  return (
    <div className="max-w-4xl mx-auto px-6 py-12 lg:py-16">
      {/* Header */}
      <div className="mb-10">
        <div className="flex items-center gap-2 mb-4">
          <span
            className="text-xs font-semibold uppercase tracking-widest"
            style={{ color: "var(--text-dim)" }}
          >
            Introduction
          </span>
        </div>
        <h1
          className="font-serif mb-3"
          style={{ fontSize: "2.5rem", lineHeight: 1.15, color: "var(--text-primary)" }}
        >
          RillCoin Whitepaper
        </h1>
        <p className="text-base" style={{ color: "var(--text-muted)" }}>
          Version 1.0 &mdash; Progressive Concentration Decay Cryptocurrency
        </p>
      </div>

      <div className="doc-prose">
        {/* Abstract */}
        <div
          className="rounded-xl p-6 mb-10"
          style={{
            background: "var(--raised)",
            border: "1px solid var(--border-dim)",
          }}
        >
          <h4>Abstract</h4>
          <p style={{ marginBottom: 0 }}>
            RillCoin is a proof-of-work cryptocurrency implementing{" "}
            <strong>progressive concentration decay</strong>. Holdings above
            concentration thresholds decay over time, with decayed tokens
            flowing back to the active mining pool. The protocol is implemented
            in Rust 2024 edition using Ed25519 signatures, BLAKE3 Merkle trees,
            SHA-256 block headers, and libp2p peer-to-peer networking. All
            consensus arithmetic uses integer-only fixed-point arithmetic with
            10<sup>8</sup> precision — no floating point.
          </p>
        </div>

        {/* Section 1 */}
        <h2>1. Introduction</h2>
        <blockquote>
          &ldquo;Wealth should flow like water.&rdquo;
        </blockquote>
        <p>
          Concentrated wealth reduces economic velocity. When a large fraction
          of supply is held statically by a small number of entities, the tokens
          cease to participate in the productive economy. Traditional
          cryptocurrencies offer no mechanism to counteract this tendency —
          early holders and miners accumulate disproportionate balances that
          remain dormant indefinitely.
        </p>
        <p>
          RillCoin&apos;s solution is <strong>algorithmic redistribution</strong> via
          concentration decay. When any cluster of UTXOs exceeds a configurable
          concentration threshold relative to the circulating supply, those
          holdings begin to decay at a rate determined by a sigmoid function of
          their concentration. Decayed tokens accrue to the active mining pool,
          supplementing block rewards and providing an ongoing incentive for
          miners to maintain the network.
        </p>
        <p>
          The name &ldquo;Rill&rdquo; references a small, flowing stream — an apt metaphor
          for the protocol&apos;s design intent: wealth should circulate continuously
          rather than pool in reservoirs.
        </p>

        {/* Section 2 */}
        <h2>2. Protocol Constants</h2>
        <p>
          All constants are defined in <code>rill-core/src/constants.rs</code>.
          Integer-only arithmetic with <code>u64</code> and 10<sup>8</sup>{" "}
          fixed-point precision (1 RILL = 100,000,000 rills, the base unit).
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
                <th>Notes</th>
              </tr>
            </thead>
            <tbody>
              {[
                ["MAX_SUPPLY", "21,000,000 RILL", "Mining rewards only"],
                ["DEV_FUND_PREMINE", "1,050,000 RILL", "5% of max supply"],
                ["TOTAL_SUPPLY", "22,050,000 RILL", "MAX_SUPPLY + DEV_FUND"],
                ["INITIAL_REWARD", "50 RILL", "Per block at genesis"],
                ["HALVING_INTERVAL", "210,000 blocks", "~4 years at 60s blocks"],
                ["BLOCK_TIME_SECS", "60", "Target seconds per block"],
                ["BLOCKS_PER_YEAR", "525,960", "365.25 × 24 × 60"],
                ["COIN", "100,000,000 rills", "1 RILL in base units"],
                ["DEV_FUND_BPS", "500 (5%)", "Basis points of block reward"],
                ["DEV_VEST_BLOCKS", "2,103,840", "BLOCKS_PER_YEAR × 4"],
                ["COINBASE_MATURITY", "100 blocks", "Before coinbase can be spent"],
                ["MIN_TX_FEE", "1,000 rills", "0.00001 RILL minimum"],
                ["MAX_BLOCK_SIZE", "1,048,576 bytes", "1 MiB"],
                ["DIFFICULTY_WINDOW", "60 blocks", "Difficulty adjustment window"],
              ].map(([k, v, n]) => (
                <tr key={k}>
                  <td>
                    <code style={{ color: "var(--cyan-300)" }}>{k}</code>
                  </td>
                  <td>{v}</td>
                  <td style={{ color: "var(--text-dim)" }}>{n}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <h3>Dev Fund</h3>
        <p>
          The dev fund premine of 1,050,000 RILL (5% of MAX_SUPPLY) is subject
          to linear vesting over 2,103,840 blocks (~4 years). At each block, a
          pro-rata portion of the dev fund becomes claimable by the development
          treasury address. This ensures long-term alignment between developers
          and the network.
        </p>

        {/* Section 3 */}
        <h2>3. Cryptography</h2>

        <h3>Signatures — Ed25519</h3>
        <p>
          All transaction inputs are authorized with Ed25519 signatures
          (RFC 8032). Ed25519 provides 128-bit security, deterministic
          signatures, and fast batch verification — all desirable properties for
          a high-throughput blockchain. The public key is 32 bytes; the
          signature is 64 bytes.
        </p>

        <h3>Merkle Trees — BLAKE3</h3>
        <p>
          Block Merkle roots are computed using BLAKE3. BLAKE3&apos;s tree hashing
          mode makes it particularly well-suited for Merkle tree construction,
          providing both speed and parallel verification. Transaction IDs are
          computed as <code>BLAKE3(bincode(transaction))</code>.
        </p>

        <h3>Block Headers — SHA-256</h3>
        <p>
          Block headers use double SHA-256 for the proof-of-work commitment,
          maintaining compatibility with existing SHA-256 ASIC infrastructure.
          The header commits to: version, previous block hash, Merkle root,
          timestamp, difficulty target, and nonce.
        </p>

        <h3>Address Format — Bech32m</h3>
        <p>
          Addresses use Bech32m encoding (BIP-350) with human-readable parts:
        </p>
        <ul>
          <li>
            <code>rill1</code> — Mainnet addresses
          </li>
          <li>
            <code>trill1</code> — Testnet addresses
          </li>
        </ul>
        <p>
          The address payload is the BLAKE3 hash of the Ed25519 public key,
          truncated to 20 bytes. Bech32m provides built-in error detection and
          produces human-readable, copy-paste-friendly strings.
        </p>

        {/* Section 4 */}
        <h2>4. Concentration Decay</h2>
        <p>
          See the <Link href="/decay">Decay Mechanics</Link> reference page for
          full technical detail, including the sigmoid lookup table, fixed-point
          arithmetic, and lineage tracking. The following is a summary.
        </p>

        <h3>Concentration Metric</h3>
        <p>
          Concentration is measured in parts-per-billion (PPB). For a cluster
          with balance <em>B</em> and circulating supply <em>S</em>:
        </p>
        <CodeBlock language="formula">
          {`concentration_ppb = cluster_balance × 1,000,000,000 / circulating_supply`}
        </CodeBlock>

        <h3>Decay Rate</h3>
        <p>
          The decay rate is derived from a fixed-point sigmoid function of the
          concentration:
        </p>
        <CodeBlock language="formula">
          {`decay_rate = (sigmoid(concentration_x) - 0.5) × DECAY_R_MAX_PPB × 2\n\nwhere:\n  DECAY_R_MAX_PPB       = 1,500,000,000  (150% per year at max concentration)\n  DECAY_C_THRESHOLD_PPB = 1,000,000      (0.1% of supply — decay threshold)\n  CONCENTRATION_PRECISION = 1,000,000,000`}
        </CodeBlock>

        <h3>Decay Pool</h3>
        <p>
          Decayed amounts accumulate in the decay pool. Each block,{" "}
          <code>DECAY_POOL_RELEASE_BPS = 100</code> (1%) of the pool is
          released to the block miner as supplemental reward. This creates a
          compounding incentive: as concentration grows, decay rates rise,
          increasing pool size, and increasing miner rewards.
        </p>

        {/* Section 5 */}
        <h2>5. Mining</h2>
        <p>
          RillCoin uses SHA-256 proof-of-work. Difficulty adjusts every{" "}
          <code>DIFFICULTY_WINDOW</code> (60) blocks to maintain the 60-second
          target. The difficulty adjustment uses a clamped ratio of actual vs.
          expected time, preventing extreme swings.
        </p>
        <p>
          Block templates are served via <code>getblocktemplate</code> RPC.
          Miners compute a valid header hash below the target and submit via{" "}
          <code>submitblock</code>. The block reward is:
        </p>
        <CodeBlock language="formula">
          {`block_reward = subsidy(height) + tx_fees + decay_pool_release\n\nwhere subsidy(height) = INITIAL_REWARD >> (height / HALVING_INTERVAL)`}
        </CodeBlock>
        <p>
          Coinbase outputs are subject to a maturity period of{" "}
          <code>COINBASE_MATURITY = 100</code> blocks before they can be spent.
        </p>

        {/* Section 6 */}
        <h2>6. Network</h2>
        <p>
          RillCoin uses <strong>libp2p</strong> for all peer-to-peer
          communication. The network stack provides:
        </p>
        <ul>
          <li>
            <strong>Gossipsub</strong> — Block and transaction propagation
          </li>
          <li>
            <strong>Kademlia DHT</strong> — Peer discovery and routing
          </li>
          <li>
            <strong>TCP transport</strong> — Reliable message delivery
          </li>
        </ul>

        <div
          className="rounded-lg overflow-hidden mb-6"
          style={{ border: "1px solid var(--border-dim)" }}
        >
          <table style={{ marginBottom: 0 }}>
            <thead>
              <tr>
                <th>Network</th>
                <th>P2P Port</th>
                <th>RPC Port</th>
                <th>Magic Bytes</th>
              </tr>
            </thead>
            <tbody>
              {[
                ["Mainnet", "18333", "18332", "RILL"],
                ["Testnet", "28333", "28332", "TEST"],
                ["Regtest", "38333", "38332", "REGT"],
              ].map(([net, p2p, rpc, magic]) => (
                <tr key={net}>
                  <td>{net}</td>
                  <td>{p2p}</td>
                  <td>{rpc}</td>
                  <td>
                    <code>{magic}</code>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Section 7 */}
        <h2>7. Storage</h2>
        <p>
          Node storage uses <strong>RocksDB</strong>, a high-performance LSM
          tree key-value store. The following column families are maintained:
        </p>
        <ul>
          <li>
            <code>blocks</code> — Full block data indexed by hash
          </li>
          <li>
            <code>headers</code> — Block headers indexed by hash and height
          </li>
          <li>
            <code>utxos</code> — Unspent transaction output set
          </li>
          <li>
            <code>clusters</code> — Cluster balance aggregates for decay
            calculation
          </li>
          <li>
            <code>decay_pool</code> — Running decay pool balance
          </li>
          <li>
            <code>mempool</code> — Pending transactions (in-memory, not
            persisted)
          </li>
        </ul>
        <p>
          The wire protocol uses <strong>bincode</strong> for compact,
          deterministic serialization of all on-chain data structures. Bincode
          is not human-readable but provides minimal overhead for
          performance-sensitive consensus paths.
        </p>
      </div>

      {/* Navigation */}
      <div
        className="flex items-center justify-between mt-12 pt-6"
        style={{ borderTop: "1px solid var(--border-subtle)" }}
      >
        <Link
          href="/"
          className="flex items-center gap-2 text-sm transition-colors"
          style={{ color: "var(--text-muted)" }}
        >
          <ArrowLeft size={14} />
          Getting Started
        </Link>
        <Link
          href="/protocol"
          className="flex items-center gap-2 text-sm font-medium transition-colors"
          style={{ color: "var(--cyan-400)" }}
        >
          Protocol Architecture
          <ArrowRight size={14} />
        </Link>
      </div>
    </div>
  );
}
