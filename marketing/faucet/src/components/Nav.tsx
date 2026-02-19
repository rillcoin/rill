"use client";
import Link from "next/link";

export default function Nav() {
  return (
    <nav className="sticky top-0 z-50 flex items-center justify-between w-full h-[72px] px-5 lg:px-20" style={{ backgroundColor: "var(--void)" }}>
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 rounded-full flex-shrink-0" style={{ background: "linear-gradient(135deg, #4A8AF4 0%, #22D3EE 100%)" }} />
        <Link href="https://rillcoin.com" className="font-sans font-bold text-[15px] tracking-[5px]" style={{ color: "var(--text-primary)" }}>RILL</Link>
      </div>
      <div className="hidden md:flex items-center gap-8">
        <Link href="https://rillcoin.com#protocol" className="font-sans text-[13px]" style={{ color: "var(--text-dim)" }}>Protocol</Link>
        <Link href="https://rillcoin.com#docs" className="font-sans text-[13px]" style={{ color: "var(--text-dim)" }}>Docs</Link>
        <Link href="https://explorer.rillcoin.com" className="font-sans text-[13px]" style={{ color: "var(--text-dim)" }}>Explorer</Link>
        <span className="font-mono text-[13px] font-medium" style={{ color: "var(--cyan-400)" }}>Faucet</span>
      </div>
      <div className="hidden md:flex items-center gap-2 rounded-full px-2.5 py-1.5" style={{ backgroundColor: "#060E1C", border: "1px solid #1B3A6B" }}>
        <span className="block w-1.5 h-1.5 rounded-full" style={{ backgroundColor: "var(--cyan-400)" }} />
        <span className="font-mono text-[11px]" style={{ color: "rgba(34,211,238,0.502)" }}>Testnet Live</span>
      </div>
    </nav>
  );
}
