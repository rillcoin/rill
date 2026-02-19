"use client";
import Link from "next/link";
import { useState } from "react";

export default function Nav({ onSearch }: { onSearch?: (q: string) => void }) {
  const [q, setQ] = useState("");
  return (
    <nav
      className="sticky top-0 z-50 flex items-center justify-between w-full h-[72px] px-5 lg:px-20 gap-4"
      style={{ backgroundColor: "var(--void)", borderBottom: "1px solid var(--border-subtle)" }}
    >
      <div className="flex items-center gap-3 flex-shrink-0">
        <div
          className="w-7 h-7 rounded-full"
          style={{ background: "linear-gradient(135deg, #4A8AF4 0%, #22D3EE 100%)" }}
        />
        <Link
          href="/"
          className="font-sans font-bold text-[15px] tracking-[5px]"
          style={{ color: "var(--text-primary)" }}
        >
          RILL
        </Link>
        <span
          className="font-mono text-[10px] tracking-[2px] px-2 py-0.5 rounded"
          style={{
            color: "var(--cyan-400)",
            backgroundColor: "rgba(34,211,238,0.08)",
            border: "1px solid rgba(34,211,238,0.2)",
          }}
        >
          EXPLORER
        </span>
      </div>

      {/* Search bar */}
      <form
        className="flex-1 max-w-xl"
        onSubmit={e => {
          e.preventDefault();
          onSearch?.(q);
        }}
      >
        <input
          value={q}
          onChange={e => setQ(e.target.value)}
          placeholder="Search block height, hash, txid, or address…"
          className="w-full font-mono text-[13px] px-4 py-2 rounded outline-none transition-colors"
          style={{
            backgroundColor: "#060E1C",
            border: "1px solid var(--border-subtle)",
            color: "var(--text-primary)",
          }}
          onFocus={e => (e.target.style.borderColor = "rgba(34,211,238,0.3)")}
          onBlur={e => (e.target.style.borderColor = "var(--border-subtle)")}
        />
      </form>

      <a
        href="https://rillcoin.com"
        className="hidden md:inline font-sans text-[13px] transition-opacity hover:opacity-70"
        style={{ color: "var(--text-dim)" }}
      >
        ← rillcoin.com
      </a>
    </nav>
  );
}
