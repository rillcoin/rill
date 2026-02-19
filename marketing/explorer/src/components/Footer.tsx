"use client";
export default function Footer() {
  return (
    <footer
      className="px-5 lg:px-20 py-8 font-mono text-[11px] flex flex-col md:flex-row justify-between gap-2"
      style={{ color: "#1A2535", borderTop: "1px solid #0F1A28" }}
    >
      <span>© 2026 RillCoin. Open source. MIT License.</span>
      <span style={{ color: "rgba(59,130,246,0.2)" }}>Built with Rust.</span>
    </footer>
  );
}
