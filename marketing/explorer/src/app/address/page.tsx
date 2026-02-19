"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import Nav from "@/components/Nav";
import Footer from "@/components/Footer";
import { rpc } from "@/lib/rpc";

// ---- Types ----------------------------------------------------------------

interface UtxoJson {
  txid: string;
  index: number;
  value: number;
  block_height: number;
  is_coinbase: boolean;
  cluster_id: number;
  pubkey_hash: string;
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

function truncateHash(hash: string): string {
  if (hash.length <= 16) return hash;
  return `${hash.slice(0, 8)}...${hash.slice(-8)}`;
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

function AddressDetailInner() {
  const params = useSearchParams();
  const addr   = params.get("addr");

  const [utxos, setUtxos]     = useState<UtxoJson[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError]     = useState<string | null>(null);

  useEffect(() => {
    if (!addr) {
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    rpc<UtxoJson[]>("getutxosbyaddress", [addr])
      .then(result => {
        setUtxos(result);
        setLoading(false);
      })
      .catch(err => {
        setError(err.message ?? "Failed to load address");
        setLoading(false);
      });
  }, [addr]);

  if (!addr) {
    return (
      <div
        className="px-5 lg:px-20 py-20 font-mono text-center"
        style={{ color: "var(--text-dim)" }}
      >
        Enter an address to search.
      </div>
    );
  }

  if (loading) {
    return (
      <div className="px-5 lg:px-20 py-8">
        <SkeletonBar w="w-20" h="h-3" />
        <div className="mt-2">
          <SkeletonBar w="w-full" h="h-6" />
        </div>
        <div
          className="rounded-xl p-6 mt-8 mb-6"
          style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
        >
          <SkeletonBar w="w-40" h="h-12" />
          <div className="mt-2">
            <SkeletonBar w="w-24" h="h-4" />
          </div>
        </div>
      </div>
    );
  }

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

  const safeUtxos = utxos ?? [];
  const totalBalance = safeUtxos.reduce((sum, u) => sum + u.value, 0) / 1e8;

  const colHeaders = ["TXID", "INDEX", "VALUE", "HEIGHT", "TYPE"];

  return (
    <>
      {/* Header */}
      <div className="px-5 lg:px-20 py-8">
        <p
          className="font-mono uppercase tracking-[3px] mb-2"
          style={{ fontSize: "10px", color: "var(--text-faint)" }}
        >
          ADDRESS
        </p>
        <p
          className="font-mono break-all"
          style={{ fontSize: "18px", color: "var(--text-primary)" }}
        >
          {addr}
        </p>
      </div>

      {/* Balance card */}
      <div
        className="rounded-xl p-6 mx-5 lg:mx-20 mb-6"
        style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
      >
        <div className="flex items-baseline gap-3 flex-wrap">
          <span
            className="font-serif text-gradient-blue-cyan"
            style={{ fontSize: "48px", lineHeight: 1 }}
          >
            {totalBalance.toLocaleString(undefined, {
              minimumFractionDigits: 2,
              maximumFractionDigits: 8,
            })}
          </span>
          <span
            className="font-mono font-medium"
            style={{ fontSize: "20px", color: "var(--text-dim)" }}
          >
            RILL
          </span>
        </div>
        <p
          className="font-mono mt-2"
          style={{ fontSize: "13px", color: "var(--text-dim)" }}
        >
          {safeUtxos.length} UTXOs
        </p>
      </div>

      {/* UTXOs table */}
      {safeUtxos.length > 0 ? (
        <div className="mx-5 lg:mx-20 mb-12">
          <p
            className="font-mono uppercase tracking-[3px] mb-4"
            style={{ fontSize: "10px", color: "var(--text-faint)" }}
          >
            UTXOS
          </p>

          {/* Column headers */}
          <div
            className="grid grid-cols-[1fr_60px_140px_80px_100px] pb-2"
            style={{ borderBottom: "1px solid var(--border-subtle)" }}
          >
            {colHeaders.map((col, i) => (
              <span
                key={col}
                className="font-mono uppercase tracking-[2px]"
                style={{
                  fontSize: "9px",
                  color: "var(--text-faint)",
                  textAlign: i >= 1 ? "right" : "left",
                }}
              >
                {col}
              </span>
            ))}
          </div>

          {safeUtxos.map(utxo => (
            <div
              key={`${utxo.txid}-${utxo.index}`}
              className="grid grid-cols-[1fr_60px_140px_80px_100px] items-center py-3"
              style={{ borderBottom: "1px solid var(--border-subtle)" }}
            >
              {/* TXID */}
              <a
                href={`/tx?id=${utxo.txid}`}
                className="font-mono transition-opacity hover:opacity-70"
                style={{ fontSize: "13px", color: "var(--blue-400)" }}
              >
                {truncateHash(utxo.txid)}
              </a>

              {/* INDEX */}
              <span
                className="font-mono text-right"
                style={{ fontSize: "13px", color: "var(--text-dim)" }}
              >
                {utxo.index}
              </span>

              {/* VALUE */}
              <span
                className="font-mono text-right"
                style={{ fontSize: "13px", color: "var(--text-primary)" }}
              >
                {(utxo.value / 1e8).toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 8,
                })}{" "}
                RILL
              </span>

              {/* HEIGHT */}
              <span
                className="font-mono text-right"
                style={{ fontSize: "13px", color: "var(--text-dim)" }}
              >
                {utxo.block_height.toLocaleString()}
              </span>

              {/* TYPE */}
              <div className="flex justify-end">
                {utxo.is_coinbase ? (
                  <span
                    className="font-mono text-[10px] px-2 py-0.5 rounded"
                    style={{
                      color: "var(--cyan-400)",
                      backgroundColor: "rgba(34,211,238,0.08)",
                      border: "1px solid rgba(34,211,238,0.2)",
                    }}
                  >
                    COINBASE
                  </span>
                ) : (
                  <span
                    className="font-mono text-[10px] px-2 py-0.5 rounded"
                    style={{
                      color: "var(--blue-400)",
                      backgroundColor: "rgba(59,130,246,0.08)",
                      border: "1px solid rgba(59,130,246,0.2)",
                    }}
                  >
                    TRANSFER
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div
          className="mx-5 lg:mx-20 mb-12 font-mono text-center py-12"
          style={{ color: "var(--text-dim)" }}
        >
          No UTXOs found for this address.
        </div>
      )}
    </>
  );
}

// ---- Page -----------------------------------------------------------------

export default function AddressPage() {
  return (
    <div className="min-h-screen flex flex-col" style={{ backgroundColor: "var(--void)" }}>
      <title>Address — RillCoin Explorer</title>
      <Nav onSearch={handleSearch} />
      <main className="flex-1">
        <Suspense
          fallback={
            <div className="px-5 lg:px-20 py-20 font-mono text-center" style={{ color: "var(--text-dim)" }}>
              Loading…
            </div>
          }
        >
          <AddressDetailInner />
        </Suspense>
      </main>
      <Footer />
    </div>
  );
}
