import type { Metadata } from "next";
import Link from "next/link";
import { ArrowLeft, ArrowRight } from "lucide-react";
import CodeBlock from "@/components/CodeBlock";

export const metadata: Metadata = {
  title: "Wallet & CLI",
  description:
    "Complete rill-cli reference — wallet create, restore, address, balance, send, getpeerinfo, getblockchaininfo, getsyncstatus, and validateaddress.",
};

function CommandSection({
  name,
  synopsis,
  description,
  options,
  example,
  exampleOutput,
  notes,
}: {
  name: string;
  synopsis: string;
  description: string;
  options?: { flag: string; desc: string }[];
  example?: string;
  exampleOutput?: string;
  notes?: string;
}) {
  return (
    <div
      className="mb-10 rounded-xl overflow-hidden"
      style={{ border: "1px solid var(--border-subtle)" }}
      id={name.replace(/\s/g, "-")}
    >
      <div
        className="px-5 py-4"
        style={{
          background: "var(--raised)",
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        <code
          className="text-lg font-medium"
          style={{
            fontFamily: "var(--font-jetbrains-mono)",
            color: "var(--cyan-300)",
          }}
        >
          {name}
        </code>
        <p
          className="text-sm mt-1"
          style={{ color: "var(--text-muted)", marginBottom: 0 }}
        >
          {synopsis}
        </p>
      </div>
      <div className="px-5 py-4">
        <p className="text-sm leading-relaxed mb-4" style={{ color: "var(--text-secondary)" }}>
          {description}
        </p>

        {options && options.length > 0 && (
          <div className="mb-4">
            <p
              className="text-xs uppercase tracking-wider font-semibold mb-2"
              style={{ color: "var(--text-dim)" }}
            >
              Options
            </p>
            <div className="space-y-2">
              {options.map((opt) => (
                <div key={opt.flag} className="flex gap-4">
                  <code
                    className="text-xs shrink-0 w-72"
                    style={{
                      fontFamily: "var(--font-jetbrains-mono)",
                      color: "var(--blue-300)",
                    }}
                  >
                    {opt.flag}
                  </code>
                  <span className="text-sm" style={{ color: "var(--text-muted)" }}>
                    {opt.desc}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {example && (
          <div className="mb-3">
            <p
              className="text-xs uppercase tracking-wider font-semibold mb-1.5"
              style={{ color: "var(--text-dim)" }}
            >
              Example
            </p>
            <CodeBlock language="bash">{example}</CodeBlock>
          </div>
        )}

        {exampleOutput && (
          <div className="mb-3">
            <p
              className="text-xs uppercase tracking-wider font-semibold mb-1.5"
              style={{ color: "var(--text-dim)" }}
            >
              Output
            </p>
            <CodeBlock language="text">{exampleOutput}</CodeBlock>
          </div>
        )}

        {notes && (
          <div
            className="rounded-lg px-4 py-3 text-sm"
            style={{
              background: "var(--surface)",
              borderLeft: "3px solid var(--orange-400)",
              color: "var(--text-muted)",
            }}
          >
            {notes}
          </div>
        )}
      </div>
    </div>
  );
}

export default function CliPage() {
  return (
    <div className="max-w-4xl mx-auto px-6 py-12 lg:py-16">
      <div className="mb-10">
        <div className="flex items-center gap-2 mb-4">
          <span
            className="text-xs font-semibold uppercase tracking-widest"
            style={{ color: "var(--text-dim)" }}
          >
            Reference
          </span>
        </div>
        <h1
          className="font-serif mb-3"
          style={{ fontSize: "2.5rem", lineHeight: 1.15, color: "var(--text-primary)" }}
        >
          Wallet &amp; CLI Reference
        </h1>
        <p className="text-base" style={{ color: "var(--text-muted)" }}>
          Complete reference for all <code style={{ color: "var(--cyan-300)", fontFamily: "var(--font-jetbrains-mono)", fontSize: "0.875rem" }}>rill-cli</code> commands.
        </p>
      </div>

      <div className="doc-prose">
        {/* Installation */}
        <h2>Installation</h2>
        <CodeBlock language="bash" title="From cargo">
          {`cargo install rill-cli`}
        </CodeBlock>
        <CodeBlock language="bash" title="From binary releases">
          {`# Linux x86_64
wget https://github.com/rillcoin/rill/releases/latest/download/rill-cli-linux-x86_64.tar.gz
tar xzf rill-cli-linux-x86_64.tar.gz
sudo mv rill-cli /usr/local/bin/

# macOS (Apple Silicon)
wget https://github.com/rillcoin/rill/releases/latest/download/rill-cli-macos-arm64.tar.gz
tar xzf rill-cli-macos-arm64.tar.gz
sudo mv rill-cli /usr/local/bin/

rill-cli --version`}
        </CodeBlock>

        {/* Global flags */}
        <h2>Global Flags</h2>
        <div className="space-y-2 mb-8">
          {[
            {
              flag: "--format <table|json>",
              desc: "Output format. table (default) is human-readable; json outputs machine-parseable JSON.",
            },
            {
              flag: "--help, -h",
              desc: "Print help for any command or subcommand.",
            },
            {
              flag: "--version, -V",
              desc: "Print rill-cli version.",
            },
          ].map((opt) => (
            <div key={opt.flag} className="flex gap-4 p-3 rounded-lg" style={{ border: "1px solid var(--border-subtle)" }}>
              <code
                className="text-sm shrink-0"
                style={{
                  fontFamily: "var(--font-jetbrains-mono)",
                  color: "var(--blue-300)",
                  width: "200px",
                }}
              >
                {opt.flag}
              </code>
              <span className="text-sm" style={{ color: "var(--text-muted)" }}>
                {opt.desc}
              </span>
            </div>
          ))}
        </div>

        {/* Environment variables */}
        <div
          className="rounded-xl p-5 mb-10"
          style={{
            background: "var(--raised)",
            border: "1px solid var(--border-dim)",
          }}
        >
          <h4 style={{ marginTop: 0 }}>Environment Variable</h4>
          <div className="flex gap-4 items-start">
            <code style={{ color: "var(--cyan-300)", fontFamily: "var(--font-jetbrains-mono)", fontSize: "0.875rem", whiteSpace: "nowrap" }}>
              RILL_WALLET_PASSWORD
            </code>
            <p style={{ color: "var(--text-muted)", marginBottom: 0, fontSize: "0.875rem" }}>
              If set, used as the wallet decryption password for all commands.
              Intended for automation and CI environments. Store with care —
              wallet files contain encrypted private keys.
            </p>
          </div>
        </div>

        {/* Commands */}
        <h2>Commands</h2>
      </div>

      <CommandSection
        name="wallet create"
        synopsis="Create a new HD wallet with a randomly generated seed"
        description="Generates a new HD wallet backed by a cryptographically random 24-word BIP-39 mnemonic. The wallet file is encrypted with a password you provide. The seed phrase is displayed exactly once at creation time — store it in a secure location. It cannot be recovered from the wallet file."
        options={[
          {
            flag: "-f, --file <PATH>",
            desc: "Path to write the wallet file. Default: ~/.rill/wallet.dat",
          },
          {
            flag: "-n, --network <NET>",
            desc: "mainnet or testnet. Controls address prefix (rill1 vs trill1). Default: testnet",
          },
        ]}
        example={`rill-cli wallet create --network testnet --file ~/.rill/testnet.dat`}
        exampleOutput={`Creating new testnet wallet...
Enter password: ••••••••
Confirm password: ••••••••

✓ Wallet created: ~/.rill/testnet.dat

=== SEED PHRASE (store securely — NOT shown again) ===
abandon ability able about above absent absorb abstract
absurd abuse access accident account accuse achieve acid
acoustic acquire across act action actor actress actual

First address: trill1qw5r3k8d9vf2m7p4...`}
        notes="The seed phrase will NOT be shown again. Write it down and store it offline before proceeding."
      />

      <CommandSection
        name="wallet restore"
        synopsis="Restore a wallet from an existing seed phrase or hex seed"
        description="Restores an HD wallet from a 24-word BIP-39 mnemonic or a hex-encoded seed. Use this to recover a wallet on a new machine or from a backup."
        options={[
          {
            flag: "-f, --file <PATH>",
            desc: "Path to write the restored wallet file. Default: ~/.rill/wallet.dat",
          },
          {
            flag: "-s, --seed <SEED>",
            desc: "24-word mnemonic or hex-encoded seed. If omitted, you will be prompted securely.",
          },
          {
            flag: "-n, --network <NET>",
            desc: "mainnet or testnet. Default: testnet",
          },
        ]}
        example={`# Restore from mnemonic (prompted)
rill-cli wallet restore --network mainnet --file ~/.rill/mainnet.dat

# Restore from mnemonic provided inline
rill-cli wallet restore --seed "abandon ability able ..."`}
        exampleOutput={`Enter seed phrase: ••••••••••••••••••••••••••••••••••••••••
Enter password: ••••••••
Confirm password: ••••••••

✓ Wallet restored: ~/.rill/mainnet.dat
  Network: mainnet
  First address: rill1qw5r3k8d9vf2m7p4...`}
      />

      <CommandSection
        name="address"
        synopsis="Derive and display the next receive address"
        description="Derives the next unused receive address from the wallet's HD key tree. Addresses are deterministic — the same seed always produces the same sequence of addresses."
        options={[
          {
            flag: "-w, --wallet <PATH>",
            desc: "Path to wallet file. Default: ~/.rill/wallet.dat",
          },
        ]}
        example={`rill-cli address --wallet ~/.rill/testnet.dat`}
        exampleOutput={`trill1qw5r3k8d9vf2m7p4xzqna6fjcmlt8ys...`}
      />

      <CommandSection
        name="balance"
        synopsis="Show wallet balance with decay breakdown"
        description="Queries the node for all UTXOs associated with wallet addresses and displays both the nominal balance (face value) and effective balance (after applying concentration decay). Also shows a per-cluster breakdown so you can see which clusters are subject to decay."
        options={[
          {
            flag: "-w, --wallet <PATH>",
            desc: "Path to wallet file. Default: ~/.rill/wallet.dat",
          },
          {
            flag: "-r, --rpc-endpoint <URL>",
            desc: "RPC endpoint URL. Default: http://127.0.0.1:18332",
          },
        ]}
        example={`rill-cli balance \\\n  --wallet ~/.rill/testnet.dat \\\n  --rpc-endpoint http://127.0.0.1:28332`}
        exampleOutput={`=== WALLET BALANCE ===
Nominal:   100.00000000 RILL
Effective:  97.42000000 RILL
Decay:      -2.58000000 RILL (2.58%)

=== CLUSTER BREAKDOWN ===
Cluster  a3f8...12cd   nominal: 100.00000000   effective:  97.42000000   decay: -2.58%
  concentration: 0.42% of supply (above threshold)
  decay rate:    ~1.8% per year`}
        notes="Effective balance is what you can actually spend. The difference (decay) has accrued to the decay pool and will be distributed to miners."
      />

      <CommandSection
        name="send"
        synopsis="Send RILL to an address"
        description="Constructs, signs, and broadcasts a transaction sending RILL to a recipient address. Uses decay-aware coin selection: the wallet prefers to spend UTXOs with the highest decay first, minimizing future decay losses."
        options={[
          {
            flag: "-w, --wallet <PATH>",
            desc: "Path to wallet file. Default: ~/.rill/wallet.dat",
          },
          {
            flag: "-t, --to <ADDRESS>",
            desc: "Recipient rill1... or trill1... address.",
          },
          {
            flag: "-a, --amount <RILL>",
            desc: "Amount to send in RILL (e.g., 10.5). Internally converted to rills.",
          },
          {
            flag: "-f, --fee <RILLS>",
            desc: "Transaction fee in rills. Default: 1000 (0.00001 RILL).",
          },
          {
            flag: "-r, --rpc-endpoint <URL>",
            desc: "RPC endpoint URL. Default: http://127.0.0.1:18332",
          },
        ]}
        example={`rill-cli send \\
  --to    trill1qrecipientaddress... \\
  --amount 10.5 \\
  --fee   1000 \\
  --rpc-endpoint http://127.0.0.1:28332`}
        exampleOutput={`Coin selection: 3 UTXOs (spending highest-decay first)
Transaction size: 412 bytes
Fee: 0.00001000 RILL (1000 rills)

Broadcasting...
✓ Transaction sent: 7f3a9c2b1d4e5f6a...

Track: http://explorer.rillcoin.com/tx/7f3a9c2b1d4e5f6a...`}
      />

      <CommandSection
        name="getpeerinfo"
        synopsis="Show connected peer count"
        description="Queries the node for the current number of connected P2P peers. Useful for verifying your node is connected to the network."
        options={[
          {
            flag: "--rpc-endpoint <URL>",
            desc: "RPC endpoint URL. Default: http://127.0.0.1:18332",
          },
        ]}
        example={`rill-cli getpeerinfo --rpc-endpoint http://127.0.0.1:28332`}
        exampleOutput={`Connected peers: 8`}
      />

      <CommandSection
        name="getblockchaininfo"
        synopsis="Show blockchain state summary"
        description="Returns a comprehensive summary of the current blockchain state including block height, best block hash, circulating supply, decay pool balance, initial block download status, UTXO count, mempool size, and peer count."
        options={[
          {
            flag: "--rpc-endpoint <URL>",
            desc: "RPC endpoint URL. Default: http://127.0.0.1:18332",
          },
        ]}
        example={`rill-cli getblockchaininfo --rpc-endpoint http://127.0.0.1:28332`}
        exampleOutput={`=== BLOCKCHAIN INFO ===
Height:             42,381
Best Block:         a3f8c9d2...64b2
Circulating Supply: 8,432,750.00000000 RILL
Decay Pool:         12,847.32000000 RILL
IBD:                false (fully synced)
UTXO Count:         1,284,920
Mempool Size:       43 transactions
Peers:              8`}
      />

      <CommandSection
        name="getsyncstatus"
        synopsis="Show node sync progress"
        description="Shows whether the node is currently syncing or fully synced, along with current height, peer count, and best known hash."
        options={[
          {
            flag: "--rpc-endpoint <URL>",
            desc: "RPC endpoint URL. Default: http://127.0.0.1:18332",
          },
        ]}
        example={`rill-cli getsyncstatus --rpc-endpoint http://127.0.0.1:28332`}
        exampleOutput={`Status:  synced
Height:  42,381
Peers:   8
Best:    a3f8c9d2...64b2`}
      />

      <CommandSection
        name="validateaddress"
        synopsis="Validate a rill1... or trill1... address"
        description="Validates the checksum and format of a RillCoin address. No RPC connection required — validation is performed client-side using the Bech32m decoding algorithm."
        options={[]}
        example={`rill-cli validateaddress trill1qw5r3k8d9vf2m7p4xzqna6fjcmlt8ys...`}
        exampleOutput={`✓ Valid testnet address
  HRP:     trill1
  Network: testnet
  Payload: 5r3k8d9vf2m7p4xzqna6fjcmlt8ys...`}
        notes="Invalid addresses print an error message and exit with code 1. Use this in scripts to validate user input before broadcasting transactions."
      />

      {/* Navigation */}
      <div
        className="flex items-center justify-between mt-12 pt-6"
        style={{ borderTop: "1px solid var(--border-subtle)" }}
      >
        <Link
          href="/mining"
          className="flex items-center gap-2 text-sm transition-colors"
          style={{ color: "var(--text-muted)" }}
        >
          <ArrowLeft size={14} />
          Mining
        </Link>
        <Link
          href="/rpc"
          className="flex items-center gap-2 text-sm font-medium transition-colors"
          style={{ color: "var(--cyan-400)" }}
        >
          RPC Reference
          <ArrowRight size={14} />
        </Link>
      </div>
    </div>
  );
}
