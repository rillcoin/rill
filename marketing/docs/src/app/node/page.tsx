import type { Metadata } from "next";
import Link from "next/link";
import { ArrowLeft, Server, HardDrive, Wifi, Cpu } from "lucide-react";
import CodeBlock from "@/components/CodeBlock";

export const metadata: Metadata = {
  title: "Node Setup",
  description:
    "Step-by-step guide to running a RillCoin full node on Ubuntu — binary install, source build, systemd service, and port reference.",
};

export default function NodePage() {
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
          Node Setup
        </h1>
        <p className="text-base" style={{ color: "var(--text-muted)" }}>
          Run a RillCoin full node. Sync testnet in minutes. Supports mainnet,
          testnet, and regtest.
        </p>
      </div>

      <div className="doc-prose">
        {/* System requirements */}
        <h2>System Requirements</h2>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-8">
          {[
            { icon: Server, label: "OS", value: "Ubuntu 22.04+" },
            { icon: Cpu, label: "RAM", value: "2 GB minimum" },
            { icon: HardDrive, label: "Storage", value: "20 GB SSD" },
            { icon: Wifi, label: "Network", value: "Stable internet" },
          ].map(({ icon: Icon, label, value }) => (
            <div
              key={label}
              className="rounded-lg p-4"
              style={{
                background: "var(--raised)",
                border: "1px solid var(--border-subtle)",
              }}
            >
              <Icon size={16} className="mb-2" style={{ color: "var(--cyan-400)" }} />
              <span
                className="block text-xs uppercase tracking-wider mb-0.5"
                style={{ color: "var(--text-dim)" }}
              >
                {label}
              </span>
              <span
                className="block text-sm font-medium"
                style={{ color: "var(--text-primary)" }}
              >
                {value}
              </span>
            </div>
          ))}
        </div>

        <p>
          Running a full node on testnet works fine with 2 GB RAM and a basic
          VPS. Mainnet storage requirements will grow over time as the chain
          extends. For production mainnet nodes, 8 GB RAM and 100 GB SSD is
          recommended.
        </p>

        {/* Install from binary */}
        <h2>Option A: Install from Binary</h2>
        <p>
          Pre-built binaries are available for Linux x86_64 and ARM64. This is
          the recommended method for most users.
        </p>
        <CodeBlock language="bash" title="Linux x86_64">
          {`# Download latest release
wget https://github.com/rillcoin/rill/releases/latest/download/rill-node-linux-x86_64.tar.gz

# Verify download
sha256sum rill-node-linux-x86_64.tar.gz
# Compare with SHA256SUMS published in the GitHub release

# Extract and install
tar xzf rill-node-linux-x86_64.tar.gz
sudo mv rill-node /usr/local/bin/
sudo chmod +x /usr/local/bin/rill-node

# Verify
rill-node --version`}
        </CodeBlock>
        <CodeBlock language="bash" title="Linux ARM64 (e.g. Raspberry Pi 4, Oracle Ampere)">
          {`wget https://github.com/rillcoin/rill/releases/latest/download/rill-node-linux-arm64.tar.gz
tar xzf rill-node-linux-arm64.tar.gz
sudo mv rill-node /usr/local/bin/`}
        </CodeBlock>

        {/* Build from source */}
        <h2>Option B: Build from Source</h2>
        <p>
          Building from source requires Rust 1.85+ stable toolchain. This
          ensures you can audit and verify all code before running it.
        </p>
        <CodeBlock language="bash" title="Install Rust (if not already installed)">
          {`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup update stable`}
        </CodeBlock>
        <CodeBlock language="bash" title="Clone and build">
          {`git clone https://github.com/rillcoin/rill
cd rill

# Build the node binary (release mode for production)
cargo build --release -p rill-node

# Install
sudo cp target/release/rill-node /usr/local/bin/
rill-node --version`}
        </CodeBlock>

        {/* Running the node */}
        <h2>Running the Node</h2>

        <h3>Testnet (recommended for first-time setup)</h3>
        <CodeBlock language="bash">
          {`# Start testnet node (P2P: 28333, RPC disabled by default)
rill-node --network testnet --data-dir ~/.rill/testnet

# With RPC enabled (required for rill-cli and mining)
rill-node \\
  --network  testnet \\
  --data-dir ~/.rill/testnet \\
  --rpc-bind 127.0.0.1:28332

# With verbose logging
RUST_LOG=rill_node=debug rill-node \\
  --network  testnet \\
  --data-dir ~/.rill/testnet \\
  --rpc-bind 127.0.0.1:28332`}
        </CodeBlock>

        <h3>Mainnet</h3>
        <CodeBlock language="bash">
          {`# Mainnet node (P2P: 18333, RPC: 18332)
rill-node \\
  --network  mainnet \\
  --data-dir ~/.rill/mainnet \\
  --rpc-bind 127.0.0.1:18332`}
        </CodeBlock>

        <h3>All CLI Flags</h3>
        <div className="space-y-2 mb-6">
          {[
            ["--network <NET>", "mainnet, testnet, or regtest. Default: mainnet"],
            ["--data-dir <PATH>", "Data directory for blocks and state. Default: ~/.rill/{network}"],
            ["--rpc-bind <ADDR>", "Bind address for JSON-RPC server. Example: 127.0.0.1:28332"],
            ["--p2p-port <PORT>", "P2P listen port. Defaults per network (18333/28333/38333)"],
            ["--connect <ADDR>", "Connect to a specific peer on startup (can be repeated)"],
            ["--max-peers <N>", "Maximum number of outbound P2P connections. Default: 16"],
            ["--log-level <LEVEL>", "Logging level: error, warn, info, debug, trace. Default: info"],
          ].map(([flag, desc]) => (
            <div
              key={flag}
              className="flex gap-4 p-3 rounded-lg"
              style={{ border: "1px solid var(--border-subtle)" }}
            >
              <code
                className="text-sm shrink-0"
                style={{
                  fontFamily: "var(--font-jetbrains-mono)",
                  color: "var(--blue-300)",
                  width: "220px",
                }}
              >
                {flag}
              </code>
              <span className="text-sm" style={{ color: "var(--text-muted)" }}>
                {desc}
              </span>
            </div>
          ))}
        </div>

        {/* Systemd */}
        <h2>Systemd Service</h2>
        <p>
          For production nodes, run <code>rill-node</code> as a systemd
          service so it starts automatically and restarts on failure.
        </p>
        <CodeBlock language="bash" title="Create a dedicated user">
          {`sudo useradd --system --home /var/lib/rill --shell /bin/false rill
sudo mkdir -p /var/lib/rill
sudo chown rill:rill /var/lib/rill`}
        </CodeBlock>
        <CodeBlock language="ini" title="/etc/systemd/system/rill-node.service">
          {`[Unit]
Description=RillCoin Full Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=rill
Group=rill
ExecStart=/usr/local/bin/rill-node \\
    --network  testnet \\
    --data-dir /var/lib/rill/testnet \\
    --rpc-bind 127.0.0.1:28332
Restart=on-failure
RestartSec=10
TimeoutStopSec=60
KillSignal=SIGTERM

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/rill

[Install]
WantedBy=multi-user.target`}
        </CodeBlock>
        <CodeBlock language="bash" title="Enable and start">
          {`sudo systemctl daemon-reload
sudo systemctl enable rill-node
sudo systemctl start rill-node

# Check status
sudo systemctl status rill-node

# Follow logs
sudo journalctl -u rill-node -f`}
        </CodeBlock>

        {/* Verify sync */}
        <h2>Verifying Sync</h2>
        <p>
          Once running, check your node&apos;s sync status via CLI or direct RPC:
        </p>
        <CodeBlock language="bash">
          {`# Via rill-cli
rill-cli getsyncstatus --rpc-endpoint http://127.0.0.1:28332

# Via curl
curl -s -X POST http://127.0.0.1:28332 \\
  -H "Content-Type: application/json" \\
  -d '{"jsonrpc":"2.0","method":"getsyncstatus","params":[],"id":1}' | jq .`}
        </CodeBlock>
        <CodeBlock language="text" title="Expected output when synced">
          {`{
  "syncing":         false,
  "current_height":  42381,
  "peer_count":      8,
  "best_block_hash": "a3f8c9d2...64b2"
}`}
        </CodeBlock>

        {/* Port reference */}
        <h2>Port Reference</h2>
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
                <th>Notes</th>
              </tr>
            </thead>
            <tbody>
              {[
                ["Mainnet", "18333", "18332", "Open P2P to internet; keep RPC local"],
                ["Testnet", "28333", "28332", "Open P2P to internet; keep RPC local"],
                ["Regtest", "38333", "38332", "Local development only"],
              ].map(([net, p2p, rpc, note]) => (
                <tr key={net}>
                  <td>{net}</td>
                  <td>{p2p}</td>
                  <td>{rpc}</td>
                  <td style={{ color: "var(--text-dim)" }}>{note}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div
          className="rounded-xl p-5 mb-8"
          style={{
            background: "var(--raised)",
            border: "1px solid var(--border-dim)",
          }}
        >
          <h4 style={{ marginTop: 0 }}>Firewall Configuration</h4>
          <CodeBlock language="bash">
            {`# Allow P2P (required for network participation)
sudo ufw allow 28333/tcp  # testnet
sudo ufw allow 18333/tcp  # mainnet

# RPC should NEVER be exposed to the internet
# Default bind is 127.0.0.1 — do not change to 0.0.0.0`}
          </CodeBlock>
        </div>

        {/* Data directory */}
        <h2>Data Directory</h2>
        <p>
          The data directory stores the blockchain, UTXO set, and peer database.
          Default locations:
        </p>
        <div className="space-y-2 mb-6">
          {[
            ["Mainnet", "~/.rill/mainnet/"],
            ["Testnet", "~/.rill/testnet/"],
            ["Regtest", "~/.rill/regtest/"],
          ].map(([net, path]) => (
            <div
              key={net}
              className="flex gap-4 items-center p-3 rounded-lg"
              style={{ border: "1px solid var(--border-subtle)" }}
            >
              <span
                className="text-sm shrink-0 w-24"
                style={{ color: "var(--text-secondary)" }}
              >
                {net}
              </span>
              <code
                className="text-sm"
                style={{
                  fontFamily: "var(--font-jetbrains-mono)",
                  color: "var(--cyan-300)",
                }}
              >
                {path}
              </code>
            </div>
          ))}
        </div>
        <p>
          Override with <code>--data-dir</code>. The directory contains:
        </p>
        <ul>
          <li>
            <code>db/</code> — RocksDB data files (blocks, UTXOs, clusters)
          </li>
          <li>
            <code>peers.json</code> — Known peer addresses
          </li>
          <li>
            <code>node.log</code> — Node log file (if file logging enabled)
          </li>
        </ul>

        {/* Upgrade */}
        <h2>Upgrading</h2>
        <CodeBlock language="bash">
          {`# Stop the node
sudo systemctl stop rill-node

# Download new binary
wget https://github.com/rillcoin/rill/releases/latest/download/rill-node-linux-x86_64.tar.gz
tar xzf rill-node-linux-x86_64.tar.gz
sudo mv rill-node /usr/local/bin/rill-node

# Restart
sudo systemctl start rill-node
sudo journalctl -u rill-node -f`}
        </CodeBlock>

        <div
          className="rounded-xl p-5"
          style={{
            background: "var(--raised)",
            border: "1px solid var(--border-dim)",
          }}
        >
          <h4 style={{ marginTop: 0 }}>Troubleshooting</h4>
          <ul style={{ marginBottom: 0 }}>
            <li>
              <strong>No peers connecting</strong> — Check that the P2P port is
              open in your firewall. Try adding a bootstrap peer with{" "}
              <code>--connect</code>.
            </li>
            <li>
              <strong>RPC connection refused</strong> — Ensure the node was
              started with <code>--rpc-bind</code>. Check the port matches your
              network (28332 testnet, 18332 mainnet).
            </li>
            <li>
              <strong>Node stuck not syncing</strong> — Check peer count via{" "}
              <code>getsyncstatus</code>. If zero peers, verify firewall and DNS.
            </li>
            <li>
              <strong>Disk full warning</strong> — RocksDB compaction can
              temporarily use extra space. Ensure at least 20% free space at all
              times.
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
          href="/rpc"
          className="flex items-center gap-2 text-sm transition-colors"
          style={{ color: "var(--text-muted)" }}
        >
          <ArrowLeft size={14} />
          RPC Reference
        </Link>
        <Link
          href="/"
          className="flex items-center gap-2 text-sm font-medium transition-colors"
          style={{ color: "var(--cyan-400)" }}
        >
          Back to Getting Started
        </Link>
      </div>
    </div>
  );
}
