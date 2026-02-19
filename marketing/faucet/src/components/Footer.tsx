"use client";
export default function Footer() {
  return (
    <footer className="px-5 lg:px-20 py-10" style={{ backgroundColor: "var(--void)" }}>
      <div style={{ height: 1, background: "linear-gradient(90deg, transparent 0%, #1B3A6B 50%, transparent 100%)", marginBottom: 40 }} />
      <div className="flex flex-col md:flex-row justify-between gap-6">
        <div>
          <div className="font-serif text-[24px] mb-1" style={{ color: "var(--text-primary)" }}>RILL</div>
          <div className="font-sans text-[13px]" style={{ color: "var(--text-dim)" }}>Wealth should flow like water.</div>
        </div>
        <div className="flex gap-8 font-sans text-[13px]" style={{ color: "#64748B" }}>
          <a href="https://rillcoin.com" className="hover:opacity-80">Home</a>
          <a href="https://explorer.rillcoin.com" className="hover:opacity-80">Explorer</a>
          <a href="https://github.com/rillcoin/rill" className="hover:opacity-80">GitHub</a>
        </div>
      </div>
      <div className="mt-8 pt-5 font-mono text-[11px]" style={{ color: "#1A2535", borderTop: "1px solid #1A2535" }}>
        © 2026 RillCoin. Open source. MIT License. &nbsp;·&nbsp; <span style={{ color: "rgba(59,130,246,0.2)" }}>Built with Rust.</span>
      </div>
    </footer>
  );
}
