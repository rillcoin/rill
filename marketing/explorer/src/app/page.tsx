"use client";

import { useEffect, useState } from "react";
import Nav from "@/components/Nav";
import Footer from "@/components/Footer";
import { rpc } from "@/lib/rpc";
import { timeAgo } from "@/lib/time";

// ---- Types ----------------------------------------------------------------

interface BlockchainInfo {
  height: number;
  best_block_hash: string;
  circulating_supply: number;
  decay_pool_balance: number;
  initial_block_download: boolean;
  utxo_count: number;
  mempool_size: number;
  peer_count: number;
}

interface MempoolInfo {
  size: number;
  bytes: number;
  total_fee: number;
}

interface BlockJson {
  hash: string;
  height: number;
  version: number;
  prev_hash: string;
  merkle_root: string;
  timestamp: number;
  difficulty_target: string;
  nonce: number;
  tx_count: number;
  tx: string[];
}

// ---- Skeleton helpers ------------------------------------------------------

function SkeletonBar({ w, h = "h-7" }: { w: string; h?: string }) {
  return (
    <div
      className={`${h} ${w} rounded animate-pulse`}
      style={{ backgroundColor: "#0F1A28" }}
    />
  );
}

function SkeletonRow() {
  return (
    <div
      className="flex items-center gap-4 py-3"
      style={{ borderBottom: "1px solid var(--border-subtle)" }}
    >
      <SkeletonBar w="w-16" h="h-4" />
      <SkeletonBar w="w-40" h="h-4" />
      <SkeletonBar w="w-8" h="h-4" />
      <SkeletonBar w="w-12" h="h-4" />
    </div>
  );
}

// ---- Search handler -------------------------------------------------------

function handleSearch(q: string) {
  q = q.trim();
  if (!q) return;
  if (/^\d+$/.test(q)) {
    rpc<string>("getblockhash", [parseInt(q)])
      .then(hash => {
        window.location.href = `/block?hash=${hash}`;
      })
      .catch(() => alert("Block not found"));
    return;
  }
  if (q.startsWith("trill1")) {
    window.location.href = `/address?addr=${q}`;
    return;
  }
  if (/^[0-9a-fA-F]{64}$/.test(q)) {
    window.location.href = `/block?hash=${q}`;
    return;
  }
  alert("Enter a block height, block hash, txid, or trill1... address");
}

// ---- Stats Bar -------------------------------------------------------------

function StatsBar({ info }: { info: BlockchainInfo | null }) {
  const circulating = info ? (info.circulating_supply / 1e8).toLocaleString(undefined, { maximumFractionDigits: 0 }) + " RILL" : null;
  const decayPool = info ? (info.decay_pool_balance / 1e8).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + " RILL" : null;

  const stats = [
    { label: "BLOCK HEIGHT", value: info ? info.height.toLocaleString() : null },
    { label: "CIRCULATING",  value: circulating },
    { label: "DECAY POOL",   value: decayPool },
    { label: "PEERS",        value: info ? String(info.peer_count) : null },
  ];

  return (
    <div
      className="flex items-center justify-between w-full px-5 lg:px-20 py-6 gap-6 overflow-x-auto"
      style={{ borderBottom: "1px solid var(--border-subtle)" }}
    >
      {stats.map(stat => (
        <div key={stat.label} className="flex flex-col gap-1.5 flex-shrink-0">
          <span
            className="font-mono uppercase tracking-[2px]"
            style={{ fontSize: "9px", color: "var(--text-faint)" }}
          >
            {stat.label}
          </span>
          {stat.value !== null ? (
            <span
              className="font-mono font-bold text-gradient-blue-cyan"
              style={{ fontSize: "28px", lineHeight: 1 }}
            >
              {stat.value}
            </span>
          ) : (
            <SkeletonBar w="w-32" h="h-8" />
          )}
        </div>
      ))}
    </div>
  );
}

// ---- Latest Blocks --------------------------------------------------------

function truncateHash(hash: string): string {
  return `0x${hash.slice(0, 6)}...${hash.slice(-6)}`;
}

function LatestBlocks({ blocks }: { blocks: BlockJson[] | null }) {
  return (
    <div>
      <p
        className="font-mono font-semibold uppercase tracking-[3px] mb-4"
        style={{ fontSize: "10px", color: "var(--text-faint)" }}
      >
        LATEST BLOCKS
      </p>

      {/* Column headers */}
      <div
        className="grid grid-cols-[80px_1fr_60px_80px] pb-2 mb-1"
        style={{
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        {["HEIGHT", "HASH", "TXS", "TIME"].map((col, i) => (
          <span
            key={col}
            className="font-mono uppercase tracking-[2px]"
            style={{
              fontSize: "9px",
              color: "var(--text-faint)",
              textAlign: i >= 2 ? "right" : "left",
            }}
          >
            {col}
          </span>
        ))}
      </div>

      {blocks === null
        ? Array.from({ length: 10 }).map((_, i) => <SkeletonRow key={i} />)
        : blocks.map(block => (
            <a
              key={block.hash}
              href={`/block?hash=${block.hash}`}
              className="grid grid-cols-[80px_1fr_60px_80px] items-center py-3 cursor-pointer transition-colors"
              style={{
                borderBottom: "1px solid var(--border-subtle)",
                textDecoration: "none",
              }}
              onMouseEnter={e =>
                (e.currentTarget.style.backgroundColor = "rgba(255,255,255,0.02)")
              }
              onMouseLeave={e =>
                (e.currentTarget.style.backgroundColor = "transparent")
              }
            >
              <span
                className="font-mono font-medium"
                style={{ fontSize: "13px", color: "var(--blue-400)" }}
              >
                {block.height.toLocaleString()}
              </span>
              <span
                className="font-mono"
                style={{ fontSize: "13px", color: "var(--text-dim)" }}
              >
                {truncateHash(block.hash)}
              </span>
              <span
                className="font-mono text-right"
                style={{ fontSize: "13px", color: "var(--text-muted)" }}
              >
                {block.tx_count}
              </span>
              <span
                className="font-mono text-right"
                style={{ fontSize: "11px", color: "var(--text-faint)" }}
              >
                {timeAgo(block.timestamp)}
              </span>
            </a>
          ))}

      <div className="mt-4">
        <a
          href="#"
          className="font-mono text-[13px] transition-opacity hover:opacity-70"
          style={{ color: "var(--blue-500)" }}
        >
          View all blocks →
        </a>
      </div>
    </div>
  );
}

// ---- Right sidebar --------------------------------------------------------

function NetworkCard({ info }: { info: BlockchainInfo | null }) {
  return (
    <div
      className="rounded-lg p-5"
      style={{
        backgroundColor: "var(--raised)",
        border: "1px solid var(--border-subtle)",
      }}
    >
      <p
        className="font-mono uppercase tracking-[3px] mb-3"
        style={{ fontSize: "9px", color: "var(--text-faint)" }}
      >
        NETWORK
      </p>

      {/* Status row */}
      <div className="flex items-center gap-2 mb-4">
        <span
          className="w-2 h-2 rounded-full flex-shrink-0"
          style={{ backgroundColor: "#10B981" }}
        />
        <span className="font-sans" style={{ fontSize: "14px", color: "var(--text-primary)" }}>
          Testnet
        </span>
      </div>

      {/* Stats list */}
      <div className="flex flex-col gap-3">
        <StatRow
          label="Best Block"
          value={
            info ? (
              <a
                href={`/block?hash=${info.best_block_hash}`}
                className="font-mono transition-opacity hover:opacity-70"
                style={{ fontSize: "12px", color: "var(--text-dim)" }}
              >
                {truncateHash(info.best_block_hash)}
              </a>
            ) : (
              <SkeletonBar w="w-28" h="h-3" />
            )
          }
        />
        <StatRow
          label="Connections"
          value={
            info ? (
              <span className="font-mono" style={{ fontSize: "12px", color: "var(--text-dim)" }}>
                {info.peer_count}
              </span>
            ) : (
              <SkeletonBar w="w-8" h="h-3" />
            )
          }
        />
        <StatRow
          label="Sync"
          value={
            info ? (
              <span
                className="font-mono"
                style={{
                  fontSize: "12px",
                  color: info.initial_block_download ? "#F59E0B" : "#10B981",
                }}
              >
                {info.initial_block_download ? "Syncing..." : "Synced"}
              </span>
            ) : (
              <SkeletonBar w="w-16" h="h-3" />
            )
          }
        />
        <StatRow
          label="UTXO Set"
          value={
            info ? (
              <span className="font-mono" style={{ fontSize: "12px", color: "var(--text-dim)" }}>
                {info.utxo_count.toLocaleString()}
              </span>
            ) : (
              <SkeletonBar w="w-20" h="h-3" />
            )
          }
        />
      </div>
    </div>
  );
}

function StatRow({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span
        className="font-mono uppercase tracking-[1px] flex-shrink-0"
        style={{ fontSize: "10px", color: "var(--text-faint)" }}
      >
        {label}
      </span>
      <div>{value}</div>
    </div>
  );
}

function MempoolCard({ mempool }: { mempool: MempoolInfo | null }) {
  const stats = mempool
    ? [
        { label: "TXS",   value: String(mempool.size) },
        { label: "BYTES", value: mempool.bytes.toLocaleString() },
        { label: "FEES",  value: (mempool.total_fee / 1e8).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 4 }) + " RILL" },
      ]
    : null;

  return (
    <div
      className="rounded-lg p-5 mt-4"
      style={{
        backgroundColor: "var(--raised)",
        border: "1px solid var(--border-subtle)",
      }}
    >
      <p
        className="font-mono uppercase tracking-[3px] mb-4"
        style={{ fontSize: "9px", color: "var(--text-faint)" }}
      >
        MEMPOOL
      </p>
      <div className="grid grid-cols-3 gap-3">
        {stats
          ? stats.map(s => (
              <div key={s.label} className="flex flex-col gap-1.5">
                <span
                  className="font-mono uppercase tracking-[1px]"
                  style={{ fontSize: "9px", color: "var(--text-faint)" }}
                >
                  {s.label}
                </span>
                <span
                  className="font-mono font-medium"
                  style={{ fontSize: "18px", color: "var(--blue-400)", lineHeight: 1 }}
                >
                  {s.value}
                </span>
              </div>
            ))
          : Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="flex flex-col gap-1.5">
                <SkeletonBar w="w-full" h="h-3" />
                <SkeletonBar w="w-full" h="h-5" />
              </div>
            ))}
      </div>
    </div>
  );
}

function DecayCard({ info }: { info: BlockchainInfo | null }) {
  const pct =
    info && info.circulating_supply > 0
      ? Math.min(100, (info.decay_pool_balance / info.circulating_supply) * 100)
      : 0;

  const decayLabel = info
    ? `${(info.decay_pool_balance / 1e8).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })} RILL in decay pool`
    : null;

  return (
    <div
      className="rounded-lg p-5 mt-4"
      style={{
        backgroundColor: "var(--raised)",
        border: "1px solid rgba(34,211,238,0.1)",
      }}
    >
      <p
        className="font-mono uppercase tracking-[3px] mb-4"
        style={{ fontSize: "9px", color: "rgba(34,211,238,0.5)" }}
      >
        CONCENTRATION DECAY
      </p>

      {/* Progress bar */}
      <div
        className="w-full rounded h-1.5 overflow-hidden"
        style={{ backgroundColor: "#060E1C" }}
      >
        {info ? (
          <div
            className="h-full rounded transition-all duration-500"
            style={{
              width: `${pct}%`,
              background: "linear-gradient(90deg, #3B82F6 0%, #22D3EE 100%)",
            }}
          />
        ) : (
          <div className="h-full rounded w-1/4 animate-pulse" style={{ backgroundColor: "#0F1A28" }} />
        )}
      </div>

      <div className="mt-2">
        {decayLabel ? (
          <span className="font-mono" style={{ fontSize: "12px", color: "var(--text-dim)" }}>
            {decayLabel}
          </span>
        ) : (
          <SkeletonBar w="w-40" h="h-3" />
        )}
      </div>
    </div>
  );
}

// ---- Page -----------------------------------------------------------------

export default function HomePage() {
  const [chainInfo, setChainInfo] = useState<BlockchainInfo | null>(null);
  const [mempool, setMempool]     = useState<MempoolInfo | null>(null);
  const [blocks, setBlocks]       = useState<BlockJson[] | null>(null);

  useEffect(() => {
    // Fetch chain info
    rpc<BlockchainInfo>("getblockchaininfo")
      .then(setChainInfo)
      .catch(err => console.error("getblockchaininfo error:", err));

    // Fetch mempool info
    rpc<MempoolInfo>("getmempoolinfo")
      .then(setMempool)
      .catch(err => console.error("getmempoolinfo error:", err));

    // Fetch last 10 blocks
    rpc<number>("getblockcount")
      .then(async height => {
        const fetched: BlockJson[] = [];
        const start = Math.max(0, height - 9);
        const heights = Array.from({ length: height - start + 1 }, (_, i) => height - i);
        await Promise.all(
          heights.map(async h => {
            try {
              const hash  = await rpc<string>("getblockhash", [h]);
              const block = await rpc<BlockJson>("getblock", [hash]);
              fetched.push(block);
            } catch {
              // skip failed blocks
            }
          })
        );
        fetched.sort((a, b) => b.height - a.height);
        setBlocks(fetched);
      })
      .catch(err => console.error("block list error:", err));
  }, []);

  return (
    <div className="min-h-screen flex flex-col" style={{ backgroundColor: "var(--void)" }}>
      <title>RillCoin Explorer</title>
      <Nav onSearch={handleSearch} />

      <StatsBar info={chainInfo} />

      <main className="flex-1 px-5 lg:px-20 py-8">
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_400px] gap-6">
          {/* Left: Latest Blocks */}
          <LatestBlocks blocks={blocks} />

          {/* Right: Network + Mempool + Decay */}
          <div>
            <NetworkCard info={chainInfo} />
            <MempoolCard mempool={mempool} />
            <DecayCard info={chainInfo} />
          </div>
        </div>
      </main>

      <Footer />
    </div>
  );
}
