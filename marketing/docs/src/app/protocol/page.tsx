import type { Metadata } from "next";
import Link from "next/link";
import { ArrowLeft, ArrowRight } from "lucide-react";
import CodeBlock from "@/components/CodeBlock";

export const metadata: Metadata = {
  title: "Protocol Architecture",
  description:
    "RillCoin protocol architecture — crate graph, transaction structure, UTXO model, block format, and address encoding.",
};

export default function ProtocolPage() {
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
          Protocol Architecture
        </h1>
        <p className="text-base" style={{ color: "var(--text-muted)" }}>
          Crate graph, data structures, UTXO model, block format, and network
          layer design.
        </p>
      </div>

      <div className="doc-prose">
        {/* Crate graph */}
        <h2>Crate Graph</h2>
        <p>
          RillCoin is implemented as a Cargo workspace with six library crates
          and three binary crates. Dependencies flow strictly from lower to
          higher layers — no circular dependencies.
        </p>

        <div
          className="rounded-xl p-6 mb-6 font-mono text-sm"
          style={{
            background: "var(--raised)",
            border: "1px solid var(--border-dim)",
            color: "var(--text-secondary)",
          }}
        >
          <div className="flex flex-col gap-1">
            {[
              { name: "rill-core", desc: "Types, constants, errors, crypto primitives", level: 0 },
              { name: "rill-decay", desc: "Sigmoid table, decay rate, cluster tracking", level: 1 },
              { name: "rill-consensus", desc: "Block validation, PoW, difficulty adjustment", level: 2 },
              { name: "rill-network", desc: "libp2p transport, Gossipsub, Kademlia DHT", level: 3 },
              { name: "rill-wallet", desc: "HD wallet, coin selection, signing", level: 4 },
              { name: "rill-node", desc: "RPC server, storage, mempool, sync", level: 5 },
            ].map((crate, i) => (
              <div key={crate.name} className="flex items-center gap-3">
                <div className="flex items-center gap-1" style={{ paddingLeft: `${i * 20}px` }}>
                  {i > 0 && (
                    <span style={{ color: "var(--text-dim)" }}>{"→ "}</span>
                  )}
                  <span style={{ color: "var(--cyan-300)" }}>{crate.name}</span>
                </div>
                <span style={{ color: "var(--text-dim)" }}>—</span>
                <span style={{ color: "var(--text-muted)", fontSize: "0.8125rem" }}>
                  {crate.desc}
                </span>
              </div>
            ))}
          </div>
        </div>

        {[
          {
            crate: "rill-core",
            role: "Foundation layer",
            desc: "Defines all shared types (Transaction, Block, BlockHeader, UTXO, Address, TxInput, TxOutput), protocol constants, error types, and cryptographic primitives. All other crates depend on rill-core. No external blockchain dependencies.",
          },
          {
            crate: "rill-decay",
            role: "Decay engine",
            desc: "Implements the sigmoid lookup table, fixed-point decay rate calculation, cluster balance aggregation, lineage tracking, and the decay pool state machine. Exposes pure functions — no I/O, no side effects.",
          },
          {
            crate: "rill-consensus",
            role: "Consensus rules",
            desc: "Block and transaction validation logic, proof-of-work verification, difficulty adjustment algorithm, Merkle root computation, coinbase maturity checks, and fee validation. The single source of truth for what constitutes a valid block.",
          },
          {
            crate: "rill-network",
            role: "P2P networking",
            desc: "libp2p-based networking: TCP transport, Noise handshake, Gossipsub block/tx propagation, Kademlia peer discovery, and connection management. Handles peer scoring and ban lists.",
          },
          {
            crate: "rill-wallet",
            role: "Wallet logic",
            desc: "HD wallet derivation (BIP-32-style with Ed25519), address generation, decay-aware coin selection (spends highest-decay UTXOs first), transaction construction, and signing.",
          },
          {
            crate: "rill-node",
            role: "Node binary",
            desc: "Integrates all library crates. Hosts the JSON-RPC 2.0 server, manages RocksDB storage, drives the mempool, coordinates block sync, and runs the decay application engine.",
          },
        ].map((item) => (
          <div
            key={item.crate}
            className="flex gap-4 p-4 rounded-lg mb-2"
            style={{ border: "1px solid var(--border-subtle)" }}
          >
            <code
              className="text-sm shrink-0 self-start mt-0.5"
              style={{ color: "var(--cyan-300)", fontFamily: "var(--font-jetbrains-mono)" }}
            >
              {item.crate}
            </code>
            <div>
              <span
                className="text-xs uppercase tracking-wider font-semibold block mb-1"
                style={{ color: "var(--text-dim)" }}
              >
                {item.role}
              </span>
              <p className="text-sm leading-relaxed" style={{ color: "var(--text-muted)", marginBottom: 0 }}>
                {item.desc}
              </p>
            </div>
          </div>
        ))}

        {/* Transaction structure */}
        <h2>Transaction Structure</h2>
        <p>
          Transactions use a standard UTXO model. Each transaction consumes one
          or more UTXOs (inputs) and creates one or more new UTXOs (outputs).
        </p>
        <CodeBlock language="rust" title="Transaction (rill-core/src/types.rs)">
          {`pub struct Transaction {
    pub version:   u64,
    pub inputs:    Vec<TxInput>,
    pub outputs:   Vec<TxOutput>,
    pub lock_time: u64,
}

pub struct TxInput {
    pub prev_txid:    [u8; 32],   // BLAKE3 hash of previous transaction
    pub prev_index:   u32,        // Index of the output being spent
    pub signature:    [u8; 64],   // Ed25519 signature
    pub pubkey:       [u8; 32],   // Ed25519 public key
}

pub struct TxOutput {
    pub value:        u64,        // Amount in rills (1 RILL = 100_000_000)
    pub pubkey_hash:  [u8; 20],   // BLAKE3(pubkey)[..20] — address payload
    pub cluster_id:   [u8; 32],   // BLAKE3 cluster identifier
}`}
        </CodeBlock>

        <h3>Transaction ID</h3>
        <p>
          A transaction ID (txid) is computed as{" "}
          <code>BLAKE3(bincode::serialize(transaction))</code> — the BLAKE3
          hash of the bincode-serialized transaction bytes. This is a 32-byte
          value, displayed as lowercase hex.
        </p>

        <h3>Signing</h3>
        <p>
          Each input is signed independently. The signing message is the
          serialized transaction with all input signatures zeroed out, committed
          to with the signer&apos;s Ed25519 key. The signature and public key are
          embedded in the input itself.
        </p>

        {/* UTXO model */}
        <h2>UTXO Model & Cluster Tracking</h2>
        <p>
          RillCoin extends the standard UTXO model with{" "}
          <strong>cluster tracking</strong>. Every UTXO carries a{" "}
          <code>cluster_id</code> — a 32-byte BLAKE3 hash that groups
          economically related UTXOs for decay calculation. All UTXOs sharing a
          cluster_id are aggregated when computing concentration.
        </p>
        <p>
          The cluster_id is set at UTXO creation time. Wallet implementations
          should assign the same cluster_id to all outputs controlled by the
          same economic actor, or use the default derivation:
        </p>
        <CodeBlock language="formula">
          {`default_cluster_id = BLAKE3(wallet_root_pubkey)`}
        </CodeBlock>
        <p>
          This prevents decay evasion through address splitting: even if a
          holder distributes coins across thousands of addresses, as long as
          they share a cluster_id, the full aggregate is subject to decay.
        </p>

        {/* Block structure */}
        <h2>Block Structure</h2>
        <CodeBlock language="rust" title="Block (rill-core/src/types.rs)">
          {`pub struct Block {
    pub header:       BlockHeader,
    pub transactions: Vec<Transaction>,
}

pub struct BlockHeader {
    pub version:           u32,
    pub prev_hash:         [u8; 32],   // SHA-256 hash of previous header
    pub merkle_root:       [u8; 32],   // BLAKE3 Merkle root of transactions
    pub timestamp:         u64,        // Unix seconds
    pub difficulty_target: u32,        // Compact nBits format
    pub nonce:             u64,        // Proof-of-work nonce
}`}
        </CodeBlock>

        <h3>Block Hash</h3>
        <p>
          The block hash is computed as{" "}
          <code>SHA-256(SHA-256(bincode(block_header)))</code>. A block is
          valid if its hash, interpreted as a 256-bit big-endian integer, is
          less than the target derived from <code>difficulty_target</code>.
        </p>

        <h3>Coinbase Transaction</h3>
        <p>
          The first transaction in every block must be a coinbase transaction.
          Coinbase transactions have no inputs (or a single input with a null
          prevout). The output value must equal the block subsidy plus
          transaction fees plus the decay pool release for that block. Coinbase
          outputs cannot be spent until the UTXO has been buried by{" "}
          <code>COINBASE_MATURITY = 100</code> additional blocks.
        </p>

        {/* Address encoding */}
        <h2>Address Encoding</h2>
        <p>
          RillCoin addresses use Bech32m encoding (BIP-350). The address
          payload is a 20-byte prefix of the BLAKE3 hash of the Ed25519 public
          key.
        </p>
        <CodeBlock language="formula">
          {`address_payload = BLAKE3(ed25519_pubkey_bytes)[0..20]
address_string  = bech32m_encode(hrp, address_payload)

where hrp:
  mainnet → "rill1"
  testnet → "trill1"`}
        </CodeBlock>

        <div
          className="rounded-lg p-4 mb-6"
          style={{
            background: "var(--raised)",
            border: "1px solid var(--border-dim)",
          }}
        >
          <p
            className="text-xs uppercase tracking-wider font-semibold mb-2"
            style={{ color: "var(--text-dim)", marginBottom: "0.5rem" }}
          >
            Example addresses
          </p>
          <div className="space-y-2">
            <div className="flex items-center gap-3">
              <span
                className="text-xs px-2 py-0.5 rounded"
                style={{
                  background: "rgba(34,211,238,0.1)",
                  color: "var(--cyan-400)",
                }}
              >
                testnet
              </span>
              <code
                className="text-sm"
                style={{
                  fontFamily: "var(--font-jetbrains-mono)",
                  color: "var(--text-secondary)",
                }}
              >
                trill1qw5r3k8d9...
              </code>
            </div>
            <div className="flex items-center gap-3">
              <span
                className="text-xs px-2 py-0.5 rounded"
                style={{
                  background: "rgba(59,130,246,0.1)",
                  color: "var(--blue-400)",
                }}
              >
                mainnet
              </span>
              <code
                className="text-sm"
                style={{
                  fontFamily: "var(--font-jetbrains-mono)",
                  color: "var(--text-secondary)",
                }}
              >
                rill1qw5r3k8d9...
              </code>
            </div>
          </div>
        </div>

        {/* Network magic */}
        <h2>Network Magic Bytes</h2>
        <p>
          Each network is identified by a 4-byte magic value prepended to all
          P2P messages. This prevents cross-network contamination.
        </p>
        <div
          className="rounded-lg overflow-hidden mb-6"
          style={{ border: "1px solid var(--border-dim)" }}
        >
          <table style={{ marginBottom: 0 }}>
            <thead>
              <tr>
                <th>Network</th>
                <th>Magic (ASCII)</th>
                <th>Magic (hex)</th>
                <th>P2P Port</th>
                <th>RPC Port</th>
              </tr>
            </thead>
            <tbody>
              {[
                ["Mainnet", "RILL", "0x52494C4C", "18333", "18332"],
                ["Testnet", "TEST", "0x54455354", "28333", "28332"],
                ["Regtest", "REGT", "0x52454754", "38333", "38332"],
              ].map(([net, ascii, hex, p2p, rpc]) => (
                <tr key={net}>
                  <td>{net}</td>
                  <td>
                    <code>{ascii}</code>
                  </td>
                  <td>
                    <code>{hex}</code>
                  </td>
                  <td>{p2p}</td>
                  <td>{rpc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Navigation */}
      <div
        className="flex items-center justify-between mt-12 pt-6"
        style={{ borderTop: "1px solid var(--border-subtle)" }}
      >
        <Link
          href="/whitepaper"
          className="flex items-center gap-2 text-sm transition-colors"
          style={{ color: "var(--text-muted)" }}
        >
          <ArrowLeft size={14} />
          Whitepaper
        </Link>
        <Link
          href="/decay"
          className="flex items-center gap-2 text-sm font-medium transition-colors"
          style={{ color: "var(--cyan-400)" }}
        >
          Decay Mechanics
          <ArrowRight size={14} />
        </Link>
      </div>
    </div>
  );
}
