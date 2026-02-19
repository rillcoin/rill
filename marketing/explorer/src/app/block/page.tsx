"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import Nav from "@/components/Nav";
import Footer from "@/components/Footer";
import { rpc } from "@/lib/rpc";

// ---- Types ----------------------------------------------------------------

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

// ---- Helpers ---------------------------------------------------------------

function SkeletonBar({ w, h = "h-4" }: { w: string; h?: string }) {
  return (
    <div
      className={`${h} ${w} rounded animate-pulse`}
      style={{ backgroundColor: "#0F1A28" }}
    />
  );
}

function formatTimestamp(unix: number): string {
  return new Date(unix * 1000).toISOString().replace("T", " ").slice(0, 19) + " UTC";
}

function handleSearch(q: string) {
  q = q.trim();
  if (!q) return;
  if (/^\d+$/.test(q)) {
    rpc<string>("getblockhash", [parseInt(q)])
      .then(hash => { window.location.href = `/block?hash=${hash}`; })
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

// ---- Inner component (uses useSearchParams) --------------------------------

function BlockDetailInner() {
  const params = useSearchParams();
  const hash   = params.get("hash");

  const [block, setBlock]   = useState<BlockJson | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError]   = useState<string | null>(null);

  useEffect(() => {
    if (!hash) {
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    rpc<BlockJson>("getblock", [hash])
      .then(b => {
        setBlock(b);
        setLoading(false);
      })
      .catch(err => {
        setError(err.message ?? "Failed to load block");
        setLoading(false);
      });
  }, [hash]);

  // No hash provided
  if (!hash) {
    return (
      <div
        className="px-5 lg:px-20 py-20 font-mono text-center"
        style={{ color: "var(--text-dim)" }}
      >
        Enter a block hash to search.
      </div>
    );
  }

  // Loading
  if (loading) {
    return (
      <div className="px-5 lg:px-20 py-8">
        <div className="mb-2">
          <SkeletonBar w="w-20" h="h-3" />
        </div>
        <SkeletonBar w="w-40" h="h-14" />
        <div className="mt-2">
          <SkeletonBar w="w-full" h="h-4" />
        </div>
        <div
          className="rounded-xl p-6 lg:p-8 mt-8 mb-8"
          style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
        >
          <div className="grid grid-cols-1 md:grid-cols-2 gap-x-12 gap-y-4">
            {Array.from({ length: 6 }).map((_, i) => (
              <div key={i} className="flex gap-4 items-center">
                <SkeletonBar w="w-24" h="h-3" />
                <SkeletonBar w="w-32" h="h-3" />
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  // Error
  if (error) {
    return (
      <div
        className="px-5 lg:px-20 py-20 font-mono text-center"
        style={{ color: "var(--text-dim)" }}
      >
        {error}
      </div>
    );
  }

  if (!block) return null;

  const detailRows: { label: string; content: React.ReactNode }[] = [
    {
      label: "Timestamp",
      content: (
        <span className="font-mono" style={{ fontSize: "13px", color: "var(--text-dim)" }}>
          {formatTimestamp(block.timestamp)}
        </span>
      ),
    },
    {
      label: "Transactions",
      content: (
        <span className="font-mono" style={{ fontSize: "13px", color: "var(--text-dim)" }}>
          {block.tx_count}
        </span>
      ),
    },
    {
      label: "Nonce",
      content: (
        <span className="font-mono" style={{ fontSize: "13px", color: "var(--text-dim)" }}>
          {block.nonce.toLocaleString()}
        </span>
      ),
    },
    {
      label: "Difficulty",
      content: (
        <span className="font-mono break-all" style={{ fontSize: "13px", color: "var(--text-dim)" }}>
          {block.difficulty_target}
        </span>
      ),
    },
    {
      label: "Merkle Root",
      content: (
        <span className="font-mono break-all" style={{ fontSize: "13px", color: "var(--text-dim)" }}>
          {block.merkle_root}
        </span>
      ),
    },
    {
      label: "Prev Block",
      content: (
        <a
          href={`/block?hash=${block.prev_hash}`}
          className="font-mono break-all transition-opacity hover:opacity-70 hover:underline"
          style={{ fontSize: "13px", color: "var(--blue-400)" }}
        >
          {block.prev_hash}
        </a>
      ),
    },
  ];

  return (
    <>
      {/* Header */}
      <div className="px-5 lg:px-20 py-8">
        <p
          className="font-mono uppercase tracking-[3px] mb-2"
          style={{ fontSize: "10px", color: "var(--text-faint)" }}
        >
          BLOCK
        </p>
        <p
          className="font-serif text-gradient-blue-cyan"
          style={{ fontSize: "56px", lineHeight: 1.1 }}
        >
          {block.height.toLocaleString()}
        </p>
        <p
          className="font-mono break-all mt-2"
          style={{ fontSize: "13px", color: "var(--text-dim)" }}
        >
          {block.hash}
        </p>
      </div>

      {/* Details card */}
      <div
        className="rounded-xl p-6 lg:p-8 mx-5 lg:mx-20 mb-8"
        style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
      >
        <div className="grid grid-cols-1 md:grid-cols-2 gap-x-12 gap-y-5">
          {detailRows.map(row => (
            <div key={row.label} className="flex flex-col gap-1">
              <span
                className="font-mono uppercase tracking-[1px]"
                style={{ fontSize: "11px", color: "var(--text-faint)" }}
              >
                {row.label}
              </span>
              <div>{row.content}</div>
            </div>
          ))}
        </div>
      </div>

      {/* Transactions */}
      <div className="px-5 lg:px-20 pb-12">
        <p
          className="font-mono uppercase tracking-[3px] mb-4"
          style={{ fontSize: "10px", color: "var(--text-faint)" }}
        >
          TRANSACTIONS
        </p>
        {block.tx.map((txid, idx) => (
          <div
            key={txid}
            className="flex items-center gap-3 py-3"
            style={{ borderBottom: "1px solid var(--border-subtle)" }}
          >
            <span className="font-mono" style={{ color: "var(--blue-400)", fontSize: "13px" }}>
              →
            </span>
            <a
              href={`/tx?id=${txid}`}
              className="font-mono break-all transition-opacity hover:opacity-70"
              style={{ fontSize: "13px", color: "var(--text-dim)" }}
            >
              {txid}
            </a>
            {idx === 0 && (
              <span
                className="font-mono text-[10px] px-2 py-0.5 rounded flex-shrink-0"
                style={{
                  color: "var(--cyan-400)",
                  backgroundColor: "rgba(34,211,238,0.08)",
                  border: "1px solid rgba(34,211,238,0.2)",
                }}
              >
                COINBASE
              </span>
            )}
          </div>
        ))}
      </div>
    </>
  );
}

// ---- Page -----------------------------------------------------------------

export default function BlockPage() {
  return (
    <div className="min-h-screen flex flex-col" style={{ backgroundColor: "var(--void)" }}>
      <title>Block — RillCoin Explorer</title>
      <Nav onSearch={handleSearch} />
      <main className="flex-1">
        <Suspense
          fallback={
            <div className="px-5 lg:px-20 py-20 font-mono text-center" style={{ color: "var(--text-dim)" }}>
              Loading…
            </div>
          }
        >
          <BlockDetailInner />
        </Suspense>
      </main>
      <Footer />
    </div>
  );
}
