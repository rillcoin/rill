"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import Nav from "@/components/Nav";
import Footer from "@/components/Footer";
import { rpc } from "@/lib/rpc";

// ---- Types ----------------------------------------------------------------

interface TransactionJson {
  txid: string;
  version: number;
  vin_count: number;
  vout_count: number;
  lock_time: number;
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

function TxDetailInner() {
  const params = useSearchParams();
  const id     = params.get("id");

  const [tx, setTx]         = useState<TransactionJson | null>(null);
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);
  const [error, setError]   = useState<string | null>(null);

  useEffect(() => {
    if (!id) {
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    setNotFound(false);
    rpc<TransactionJson>("gettransaction", [id])
      .then(t => {
        setTx(t);
        setLoading(false);
      })
      .catch(err => {
        const msg: string = err.message ?? "";
        if (msg.toLowerCase().includes("not found")) {
          setNotFound(true);
        } else {
          setError(msg || "Failed to load transaction");
        }
        setLoading(false);
      });
  }, [id]);

  if (!id) {
    return (
      <div
        className="px-5 lg:px-20 py-20 font-mono text-center"
        style={{ color: "var(--text-dim)" }}
      >
        Enter a transaction ID to search.
      </div>
    );
  }

  const truncatedId = id.length > 32
    ? `${id.slice(0, 16)}...${id.slice(-16)}`
    : id;

  if (loading) {
    return (
      <div className="px-5 lg:px-20 py-8">
        <SkeletonBar w="w-20" h="h-3" />
        <div className="mt-2 mb-2">
          <SkeletonBar w="w-64" h="h-8" />
        </div>
        <SkeletonBar w="w-full" h="h-4" />
        <div
          className="rounded-xl p-6 mx-0 mt-8"
          style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
        >
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="flex gap-4 items-center py-3" style={{ borderBottom: "1px solid var(--border-subtle)" }}>
              <SkeletonBar w="w-24" h="h-3" />
              <SkeletonBar w="w-48" h="h-3" />
            </div>
          ))}
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

  // Transaction found only in mempool, or confirmed but not indexed
  if (notFound) {
    return (
      <>
        {/* Header */}
        <div className="px-5 lg:px-20 py-8">
          <p
            className="font-mono uppercase tracking-[3px] mb-2"
            style={{ fontSize: "10px", color: "var(--text-faint)" }}
          >
            TRANSACTION
          </p>
          <p
            className="font-mono"
            style={{ fontSize: "24px", color: "var(--text-primary)" }}
          >
            {truncatedId}
          </p>
          <p
            className="font-mono break-all mt-1"
            style={{ fontSize: "12px", color: "var(--text-dim)" }}
          >
            {id}
          </p>
        </div>

        {/* Confirmed notice */}
        <div
          className="rounded-xl p-6 lg:p-8 mx-5 lg:mx-20 mb-8"
          style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
        >
          <p
            className="font-mono"
            style={{ fontSize: "14px", color: "var(--text-dim)" }}
          >
            This transaction has been confirmed on-chain. Full input/output details require a transaction index (coming soon).
          </p>
        </div>
      </>
    );
  }

  if (!tx) return null;

  const rows: { label: string; value: React.ReactNode }[] = [
    {
      label: "TxID",
      value: (
        <span className="font-mono break-all" style={{ fontSize: "13px", color: "var(--text-dim)" }}>
          {tx.txid}
        </span>
      ),
    },
    {
      label: "Version",
      value: (
        <span className="font-mono" style={{ fontSize: "13px", color: "var(--text-dim)" }}>
          {tx.version}
        </span>
      ),
    },
    {
      label: "Inputs",
      value: (
        <span className="font-mono" style={{ fontSize: "13px", color: "var(--text-dim)" }}>
          {tx.vin_count}
        </span>
      ),
    },
    {
      label: "Outputs",
      value: (
        <span className="font-mono" style={{ fontSize: "13px", color: "var(--text-dim)" }}>
          {tx.vout_count}
        </span>
      ),
    },
    {
      label: "Lock Time",
      value: (
        <span className="font-mono" style={{ fontSize: "13px", color: "var(--text-dim)" }}>
          {tx.lock_time}
        </span>
      ),
    },
    {
      label: "Status",
      value: (
        <span className="font-mono" style={{ fontSize: "13px", color: "#10B981" }}>
          Mempool
        </span>
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
          TRANSACTION
        </p>
        <p
          className="font-mono"
          style={{ fontSize: "24px", color: "var(--text-primary)" }}
        >
          {truncatedId}
        </p>
        <p
          className="font-mono break-all mt-1"
          style={{ fontSize: "12px", color: "var(--text-dim)" }}
        >
          {tx.txid}
        </p>
      </div>

      {/* Details card */}
      <div
        className="rounded-xl p-6 lg:p-8 mx-5 lg:mx-20 mb-12"
        style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
      >
        {rows.map((row, idx) => (
          <div
            key={row.label}
            className="flex flex-col md:flex-row md:items-start gap-2 md:gap-8 py-3"
            style={{
              borderBottom: idx < rows.length - 1 ? "1px solid var(--border-subtle)" : "none",
            }}
          >
            <span
              className="font-mono uppercase tracking-[1px] flex-shrink-0 w-24"
              style={{ fontSize: "11px", color: "var(--text-faint)" }}
            >
              {row.label}
            </span>
            <div className="flex-1">{row.value}</div>
          </div>
        ))}
      </div>
    </>
  );
}

// ---- Page -----------------------------------------------------------------

export default function TxPage() {
  return (
    <div className="min-h-screen flex flex-col" style={{ backgroundColor: "var(--void)" }}>
      <title>Transaction — RillCoin Explorer</title>
      <Nav onSearch={handleSearch} />
      <main className="flex-1">
        <Suspense
          fallback={
            <div className="px-5 lg:px-20 py-20 font-mono text-center" style={{ color: "var(--text-dim)" }}>
              Loading…
            </div>
          }
        >
          <TxDetailInner />
        </Suspense>
      </main>
      <Footer />
    </div>
  );
}
