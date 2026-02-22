"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import Nav from "@/components/Nav";
import Footer from "@/components/Footer";
import { rpc } from "@/lib/rpc";

// ---- Types ----------------------------------------------------------------

interface ConductProfile {
  address: string;
  wallet_type: "agent" | "standard";
  conduct_score: number;
  conduct_multiplier_bps: number;
  effective_decay_rate_ppb: number;
  undertow_active: boolean;
  registered_at_block: number;
  wallet_age_blocks: number;
}

// ---- Multiplier bracket table data ----------------------------------------

const BRACKETS = [
  { label: "Exemplary",   range: "900 - 1000", multiplier: "2.00x", bps: 20000 },
  { label: "Excellent",   range: "800 - 899",  multiplier: "1.75x", bps: 17500 },
  { label: "Good",        range: "700 - 799",  multiplier: "1.50x", bps: 15000 },
  { label: "Neutral",     range: "500 - 699",  multiplier: "1.00x", bps: 10000 },
  { label: "Below Avg",   range: "300 - 499",  multiplier: "0.75x", bps: 7500  },
  { label: "Poor",        range: "100 - 299",  multiplier: "0.50x", bps: 5000  },
  { label: "Sanctioned",  range: "0 - 99",     multiplier: "0.00x", bps: 0     },
];

function getBracketIndex(score: number): number {
  if (score >= 900) return 0;
  if (score >= 800) return 1;
  if (score >= 700) return 2;
  if (score >= 500) return 3;
  if (score >= 300) return 4;
  if (score >= 100) return 5;
  return 6;
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

function scoreColor(score: number): string {
  if (score >= 700) return "#10B981";
  if (score >= 400) return "#F59E0B";
  return "#EF4444";
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

// ---- Conduct Profile Card --------------------------------------------------

function ConductProfileCard({ profile }: { profile: ConductProfile }) {
  const multiplier = (profile.conduct_multiplier_bps / 10000).toFixed(2);
  const scorePct = Math.min(100, Math.max(0, profile.conduct_score / 10));
  const color = scoreColor(profile.conduct_score);
  const activeBracket = getBracketIndex(profile.conduct_score);

  return (
    <div className="flex flex-col gap-6">
      {/* Undertow warning */}
      {profile.undertow_active && (
        <div
          className="rounded-lg px-5 py-4 flex items-center gap-3"
          style={{
            backgroundColor: "rgba(239,68,68,0.08)",
            border: "1px solid rgba(239,68,68,0.3)",
          }}
        >
          <span style={{ fontSize: "18px", color: "#EF4444" }}>!</span>
          <div>
            <p
              className="font-sans font-semibold"
              style={{ fontSize: "14px", color: "#EF4444" }}
            >
              Undertow Active
            </p>
            <p
              className="font-sans"
              style={{ fontSize: "12px", color: "rgba(239,68,68,0.7)" }}
            >
              This wallet is currently subject to undertow penalties. Decay rates are elevated.
            </p>
          </div>
        </div>
      )}

      {/* Main profile card */}
      <div
        className="rounded-xl p-6"
        style={{
          backgroundColor: "var(--raised)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        {/* Address + wallet type badge */}
        <div className="flex items-start justify-between gap-4 flex-wrap mb-6">
          <div>
            <p
              className="font-mono uppercase tracking-[3px] mb-2"
              style={{ fontSize: "10px", color: "var(--text-faint)" }}
            >
              ADDRESS
            </p>
            <p
              className="font-mono break-all"
              style={{ fontSize: "16px", color: "var(--text-primary)" }}
            >
              {profile.address}
            </p>
          </div>
          <span
            className="font-mono text-[10px] uppercase tracking-[2px] px-3 py-1 rounded flex-shrink-0"
            style={
              profile.wallet_type === "agent"
                ? {
                    color: "var(--blue-400)",
                    backgroundColor: "rgba(59,130,246,0.08)",
                    border: "1px solid rgba(59,130,246,0.2)",
                  }
                : {
                    color: "var(--text-dim)",
                    backgroundColor: "rgba(255,255,255,0.04)",
                    border: "1px solid var(--border-subtle)",
                  }
            }
          >
            {profile.wallet_type}
          </span>
        </div>

        {/* Conduct Score bar */}
        <div className="mb-6">
          <div className="flex items-baseline justify-between mb-2">
            <p
              className="font-mono uppercase tracking-[3px]"
              style={{ fontSize: "10px", color: "var(--text-faint)" }}
            >
              CONDUCT SCORE
            </p>
            <span
              className="font-mono font-bold"
              style={{ fontSize: "28px", lineHeight: 1, color }}
            >
              {profile.conduct_score}
              <span
                className="font-normal"
                style={{ fontSize: "14px", color: "var(--text-faint)" }}
              >
                {" "}/ 1000
              </span>
            </span>
          </div>
          <div
            className="w-full rounded-full h-3 overflow-hidden"
            style={{ backgroundColor: "#060E1C" }}
          >
            <div
              className="h-full rounded-full transition-all duration-700"
              style={{
                width: `${scorePct}%`,
                backgroundColor: color,
              }}
            />
          </div>
        </div>

        {/* Stats grid */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-5">
          {/* Decay Multiplier */}
          <div className="flex flex-col gap-1">
            <span
              className="font-mono uppercase tracking-[2px]"
              style={{ fontSize: "9px", color: "var(--text-faint)" }}
            >
              DECAY MULTIPLIER
            </span>
            <span
              className="font-mono font-bold"
              style={{ fontSize: "24px", lineHeight: 1, color: "var(--blue-400)" }}
            >
              {multiplier}x
            </span>
            <span
              className="font-mono"
              style={{ fontSize: "11px", color: "var(--text-faint)" }}
            >
              {profile.conduct_multiplier_bps.toLocaleString()} bps
            </span>
          </div>

          {/* Effective Decay Rate */}
          <div className="flex flex-col gap-1">
            <span
              className="font-mono uppercase tracking-[2px]"
              style={{ fontSize: "9px", color: "var(--text-faint)" }}
            >
              EFFECTIVE DECAY
            </span>
            <span
              className="font-mono font-bold"
              style={{ fontSize: "24px", lineHeight: 1, color: "var(--text-primary)" }}
            >
              {profile.effective_decay_rate_ppb}
            </span>
            <span
              className="font-mono"
              style={{ fontSize: "11px", color: "var(--text-faint)" }}
            >
              ppb
            </span>
          </div>

          {/* Registration Block */}
          <div className="flex flex-col gap-1">
            <span
              className="font-mono uppercase tracking-[2px]"
              style={{ fontSize: "9px", color: "var(--text-faint)" }}
            >
              REGISTERED AT
            </span>
            <a
              href={`/block?hash=${profile.registered_at_block}`}
              className="font-mono font-bold transition-opacity hover:opacity-70"
              style={{ fontSize: "24px", lineHeight: 1, color: "var(--blue-400)" }}
            >
              #{profile.registered_at_block.toLocaleString()}
            </a>
            <span
              className="font-mono"
              style={{ fontSize: "11px", color: "var(--text-faint)" }}
            >
              block
            </span>
          </div>

          {/* Wallet Age */}
          <div className="flex flex-col gap-1">
            <span
              className="font-mono uppercase tracking-[2px]"
              style={{ fontSize: "9px", color: "var(--text-faint)" }}
            >
              WALLET AGE
            </span>
            <span
              className="font-mono font-bold"
              style={{ fontSize: "24px", lineHeight: 1, color: "var(--text-primary)" }}
            >
              {profile.wallet_age_blocks.toLocaleString()}
            </span>
            <span
              className="font-mono"
              style={{ fontSize: "11px", color: "var(--text-faint)" }}
            >
              blocks
            </span>
          </div>
        </div>
      </div>

      {/* Multiplier bracket table */}
      <div
        className="rounded-xl p-6"
        style={{
          backgroundColor: "var(--raised)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        <p
          className="font-mono uppercase tracking-[3px] mb-4"
          style={{ fontSize: "10px", color: "var(--text-faint)" }}
        >
          MULTIPLIER BRACKETS
        </p>

        {/* Table header */}
        <div
          className="grid grid-cols-[1fr_120px_100px_80px] pb-2"
          style={{ borderBottom: "1px solid var(--border-subtle)" }}
        >
          {["TIER", "SCORE RANGE", "MULTIPLIER", "BPS"].map((col, i) => (
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

        {/* Table rows */}
        {BRACKETS.map((bracket, idx) => {
          const isActive = idx === activeBracket;
          return (
            <div
              key={bracket.label}
              className="grid grid-cols-[1fr_120px_100px_80px] items-center py-3"
              style={{
                borderBottom: "1px solid var(--border-subtle)",
                backgroundColor: isActive ? "rgba(59,130,246,0.08)" : "transparent",
                borderLeft: isActive ? "3px solid #3B82F6" : "3px solid transparent",
                paddingLeft: "12px",
                marginLeft: "-12px",
                marginRight: "-12px",
                paddingRight: "12px",
              }}
            >
              <span
                className="font-mono font-medium"
                style={{
                  fontSize: "13px",
                  color: isActive ? "var(--blue-400)" : "var(--text-dim)",
                }}
              >
                {bracket.label}
                {isActive && (
                  <span
                    className="ml-2 text-[10px] px-2 py-0.5 rounded"
                    style={{
                      backgroundColor: "rgba(59,130,246,0.15)",
                      color: "var(--blue-400)",
                    }}
                  >
                    CURRENT
                  </span>
                )}
              </span>
              <span
                className="font-mono text-right"
                style={{
                  fontSize: "13px",
                  color: isActive ? "var(--text-primary)" : "var(--text-dim)",
                }}
              >
                {bracket.range}
              </span>
              <span
                className="font-mono text-right font-medium"
                style={{
                  fontSize: "13px",
                  color: isActive ? "var(--blue-400)" : "var(--text-dim)",
                }}
              >
                {bracket.multiplier}
              </span>
              <span
                className="font-mono text-right"
                style={{
                  fontSize: "13px",
                  color: isActive ? "var(--text-primary)" : "var(--text-faint)",
                }}
              >
                {bracket.bps.toLocaleString()}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ---- Inner component (uses useSearchParams) --------------------------------

function AgentsInner() {
  const params = useSearchParams();
  const router = useRouter();
  const addrParam = params.get("addr") ?? "";

  const [searchInput, setSearchInput] = useState(addrParam);
  const [profile, setProfile] = useState<ConductProfile | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function lookupAddress(addr: string) {
    addr = addr.trim();
    if (!addr) return;
    setLoading(true);
    setError(null);
    setProfile(null);
    router.replace(`/agents?addr=${encodeURIComponent(addr)}`);
    rpc<ConductProfile>("getAgentConductProfile", [addr])
      .then(result => {
        setProfile(result);
        setLoading(false);
      })
      .catch(err => {
        setError(err.message ?? "Failed to load conduct profile");
        setLoading(false);
      });
  }

  useEffect(() => {
    if (addrParam) {
      setSearchInput(addrParam);
      lookupAddress(addrParam);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="px-5 lg:px-20 py-8">
      {/* Page heading */}
      <h1
        className="font-serif mb-2"
        style={{ fontSize: "36px", color: "var(--text-primary)" }}
      >
        Agent Conduct
      </h1>
      <p
        className="font-sans mb-8"
        style={{ fontSize: "14px", color: "var(--text-dim)" }}
      >
        Look up any wallet&apos;s conduct profile, decay multiplier, and undertow status.
      </p>

      {/* Search box */}
      <form
        className="mb-8"
        onSubmit={e => {
          e.preventDefault();
          lookupAddress(searchInput);
        }}
      >
        <div className="flex gap-3">
          <input
            value={searchInput}
            onChange={e => setSearchInput(e.target.value)}
            placeholder="Enter a trill1... address"
            className="flex-1 font-mono text-[13px] px-4 py-3 rounded outline-none transition-colors"
            style={{
              backgroundColor: "#060E1C",
              border: "1px solid var(--border-subtle)",
              color: "var(--text-primary)",
            }}
            onFocus={e => (e.target.style.borderColor = "rgba(34,211,238,0.3)")}
            onBlur={e => (e.target.style.borderColor = "var(--border-subtle)")}
          />
          <button
            type="submit"
            className="font-mono text-[13px] px-6 py-3 rounded transition-opacity hover:opacity-80 flex-shrink-0"
            style={{
              backgroundColor: "#3B82F6",
              color: "#FFFFFF",
              border: "none",
              cursor: "pointer",
            }}
          >
            Look Up
          </button>
        </div>
      </form>

      {/* Loading state */}
      {loading && (
        <div
          className="rounded-xl p-6"
          style={{
            backgroundColor: "var(--raised)",
            border: "1px solid var(--border-subtle)",
          }}
        >
          <SkeletonBar w="w-40" h="h-3" />
          <div className="mt-4">
            <SkeletonBar w="w-full" h="h-3" />
          </div>
          <div className="mt-6">
            <SkeletonBar w="w-32" h="h-3" />
            <div className="mt-2">
              <SkeletonBar w="w-full" h="h-8" />
            </div>
          </div>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-5 mt-6">
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="flex flex-col gap-1">
                <SkeletonBar w="w-20" h="h-3" />
                <SkeletonBar w="w-16" h="h-6" />
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Error state */}
      {error && (
        <div
          className="rounded-lg px-5 py-4 font-mono text-[13px]"
          style={{
            backgroundColor: "rgba(239,68,68,0.06)",
            border: "1px solid rgba(239,68,68,0.2)",
            color: "#EF4444",
          }}
        >
          {error}
        </div>
      )}

      {/* Profile card */}
      {profile && <ConductProfileCard profile={profile} />}

      {/* Empty state */}
      {!loading && !error && !profile && (
        <div
          className="font-mono text-center py-20"
          style={{ color: "var(--text-dim)", fontSize: "14px" }}
        >
          Enter an address above to view its conduct profile.
        </div>
      )}
    </div>
  );
}

// ---- Page -----------------------------------------------------------------

export default function AgentsPage() {
  return (
    <div className="min-h-screen flex flex-col" style={{ backgroundColor: "var(--void)" }}>
      <title>Agents — RillCoin Explorer</title>
      <Nav onSearch={handleSearch} />
      <main className="flex-1">
        <Suspense
          fallback={
            <div
              className="px-5 lg:px-20 py-20 font-mono text-center"
              style={{ color: "var(--text-dim)" }}
            >
              Loading...
            </div>
          }
        >
          <AgentsInner />
        </Suspense>
      </main>
      <Footer />
    </div>
  );
}
