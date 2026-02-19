import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRight, Droplets, Terminal, BookOpen, Cpu, ExternalLink } from "lucide-react";

export const metadata: Metadata = {
  title: "Getting Started",
  description:
    "Everything you need to build on RillCoin — the progressive concentration decay cryptocurrency.",
};

const QUICKSTART_CARDS = [
  {
    icon: Droplets,
    title: "Get testnet RILL",
    description:
      "Request free testnet RILL from the faucet to start experimenting with the protocol.",
    href: "https://faucet.rillcoin.com",
    external: true,
    accent: "var(--cyan-400)",
  },
  {
    icon: Cpu,
    title: "Run a node",
    description:
      "Set up a full RillCoin node on Ubuntu. Sync with testnet in minutes.",
    href: "/node",
    external: false,
    accent: "var(--blue-400)",
  },
  {
    icon: Terminal,
    title: "CLI reference",
    description:
      "Complete reference for rill-cli commands — wallet, balance, send, and more.",
    href: "/cli",
    external: false,
    accent: "var(--orange-400)",
  },
  {
    icon: BookOpen,
    title: "RPC docs",
    description:
      "JSON-RPC 2.0 API reference for interacting with a RillCoin node programmatically.",
    href: "/rpc",
    external: false,
    accent: "var(--cyan-400)",
  },
];

export default function HomePage() {
  return (
    <div className="max-w-4xl mx-auto px-6 py-12 lg:py-16">
      {/* Hero */}
      <div className="mb-12">
        <div className="flex items-center gap-2 mb-6">
          <span
            className="w-2 h-2 rounded-full animate-pulse"
            style={{ background: "var(--cyan-400)" }}
          />
          <span
            className="text-sm font-medium"
            style={{ color: "var(--cyan-400)" }}
          >
            Testnet Live
          </span>
        </div>

        <h1
          className="font-serif leading-none mb-4"
          style={{ fontSize: "3.5rem", color: "var(--text-primary)" }}
        >
          RillCoin Developer Docs
        </h1>
        <p
          className="text-xl max-w-2xl leading-relaxed mb-2"
          style={{ color: "var(--text-secondary)" }}
        >
          Everything you need to build on RillCoin.
        </p>
        <p
          className="text-base max-w-2xl leading-relaxed"
          style={{ color: "var(--text-muted)" }}
        >
          RillCoin is a proof-of-work cryptocurrency implementing{" "}
          <strong style={{ color: "var(--text-secondary)" }}>
            progressive concentration decay
          </strong>{" "}
          — holdings above concentration thresholds decay over time and flow back
          to the active mining pool. Built in Rust 2024 edition with Ed25519
          signatures, BLAKE3 Merkle trees, and libp2p networking.
        </p>
      </div>

      {/* Quickstart cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-16">
        {QUICKSTART_CARDS.map((card) => {
          const Icon = card.icon;
          return card.external ? (
            <a
              key={card.title}
              href={card.href}
              target="_blank"
              rel="noopener noreferrer"
              className="group flex flex-col gap-3 p-5 rounded-xl transition-all duration-200 hover:translate-y-[-1px]"
              style={{
                background: "var(--raised)",
                border: "1px solid var(--border-subtle)",
              }}
            >
              <div className="flex items-start justify-between">
                <div
                  className="w-9 h-9 rounded-lg flex items-center justify-center"
                  style={{ background: "var(--surface)" }}
                >
                  <Icon size={18} style={{ color: card.accent }} />
                </div>
                <ExternalLink
                  size={14}
                  className="transition-opacity opacity-40 group-hover:opacity-70"
                  style={{ color: "var(--text-muted)" }}
                />
              </div>
              <div>
                <h3
                  className="font-semibold mb-1"
                  style={{ color: "var(--text-primary)" }}
                >
                  {card.title}
                </h3>
                <p className="text-sm leading-relaxed" style={{ color: "var(--text-muted)" }}>
                  {card.description}
                </p>
              </div>
            </a>
          ) : (
            <Link
              key={card.title}
              href={card.href}
              className="group flex flex-col gap-3 p-5 rounded-xl transition-all duration-200 hover:translate-y-[-1px]"
              style={{
                background: "var(--raised)",
                border: "1px solid var(--border-subtle)",
              }}
            >
              <div className="flex items-start justify-between">
                <div
                  className="w-9 h-9 rounded-lg flex items-center justify-center"
                  style={{ background: "var(--surface)" }}
                >
                  <Icon size={18} style={{ color: card.accent }} />
                </div>
                <ArrowRight
                  size={14}
                  className="transition-all opacity-40 group-hover:opacity-70 group-hover:translate-x-0.5"
                  style={{ color: "var(--text-muted)" }}
                />
              </div>
              <div>
                <h3
                  className="font-semibold mb-1"
                  style={{ color: "var(--text-primary)" }}
                >
                  {card.title}
                </h3>
                <p className="text-sm leading-relaxed" style={{ color: "var(--text-muted)" }}>
                  {card.description}
                </p>
              </div>
            </Link>
          );
        })}
      </div>

      {/* Core concepts grid */}
      <div className="mb-12">
        <h2
          className="font-serif text-2xl mb-6"
          style={{ color: "var(--text-primary)" }}
        >
          Core Concepts
        </h2>
        <div className="space-y-3">
          {[
            {
              term: "Concentration Decay",
              def: "Holdings above 0.1% of circulating supply decay at a rate proportional to their concentration. Decayed tokens flow to the mining reward pool.",
              href: "/decay",
            },
            {
              term: "Cluster Tracking",
              def: "Every UTXO is tagged with a cluster_id (BLAKE3 hash). UTXOs sharing a cluster_id are aggregated for decay calculations — preventing decay evasion through address splitting.",
              href: "/decay",
            },
            {
              term: "Effective Value",
              def: "The spendable value of a UTXO after applying decay. Displayed separately from nominal value in wallets and the CLI balance command.",
              href: "/cli",
            },
            {
              term: "Decay Pool",
              def: "Accumulated decayed tokens. 1% of the pool (DECAY_POOL_RELEASE_BPS = 100) is distributed to miners as supplemental block reward each block.",
              href: "/decay",
            },
            {
              term: "Block Reward",
              def: "50 RILL per block, halving every 210,000 blocks (~4 years). Plus variable decay pool release. Coinbase outputs mature after 100 blocks.",
              href: "/mining",
            },
          ].map((item) => (
            <Link
              key={item.term}
              href={item.href}
              className="group flex gap-4 p-4 rounded-lg transition-all"
              style={{
                background: "transparent",
                border: "1px solid var(--border-subtle)",
              }}
            >
              <span
                className="text-sm font-semibold shrink-0 w-44"
                style={{ color: "var(--blue-400)" }}
              >
                {item.term}
              </span>
              <span className="text-sm leading-relaxed" style={{ color: "var(--text-muted)" }}>
                {item.def}
              </span>
            </Link>
          ))}
        </div>
      </div>

      {/* Protocol constants summary */}
      <div
        className="rounded-xl p-6 mb-12"
        style={{
          background: "var(--raised)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        <h2
          className="font-serif text-xl mb-4"
          style={{ color: "var(--text-primary)" }}
        >
          Protocol at a Glance
        </h2>
        <div className="grid grid-cols-2 md:grid-cols-3 gap-x-8 gap-y-3">
          {[
            ["Max Supply", "22,050,000 RILL"],
            ["Block Time", "60 seconds"],
            ["Initial Reward", "50 RILL"],
            ["Halving Interval", "210,000 blocks"],
            ["Signatures", "Ed25519"],
            ["Hash Function", "BLAKE3 / SHA-256"],
            ["Networking", "libp2p"],
            ["Storage", "RocksDB"],
            ["Wire Format", "Bincode"],
            ["Precision", "10\u2078 rills / RILL"],
            ["Address Format", "Bech32m"],
            ["PoW Algorithm", "SHA-256"],
          ].map(([k, v]) => (
            <div key={k}>
              <span
                className="block text-xs uppercase tracking-wider mb-0.5"
                style={{ color: "var(--text-dim)" }}
              >
                {k}
              </span>
              <span
                className="text-sm font-mono font-medium"
                style={{ color: "var(--text-primary)" }}
              >
                {v}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Navigation footer */}
      <div className="flex items-center justify-between">
        <span className="text-sm" style={{ color: "var(--text-dim)" }}>
          Start here
        </span>
        <Link
          href="/whitepaper"
          className="flex items-center gap-2 text-sm font-medium transition-colors"
          style={{ color: "var(--cyan-400)" }}
        >
          Read the Whitepaper
          <ArrowRight size={14} />
        </Link>
      </div>
    </div>
  );
}
