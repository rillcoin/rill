"use client";

const FOOTER_LINKS = [
  {
    heading: "PROTOCOL",
    links: [
      { label: "Whitepaper", href: "https://docs.rillcoin.com/whitepaper" },
      { label: "Decay Mechanics", href: "https://docs.rillcoin.com/decay" },
      { label: "Block Explorer", href: "https://explorer.rillcoin.com" },
      { label: "Mining", href: "https://docs.rillcoin.com/mining" },
    ],
  },
  {
    heading: "COMMUNITY",
    links: [
      { label: "Discord", href: "https://discord.com/invite/F3dRVaP8" },
      { label: "X / Twitter", href: "https://x.com/RillCoin" },
      { label: "GitHub", href: "https://github.com/rillcoin/rill" },
      { label: "Faucet", href: "https://faucet.rillcoin.com", accent: true },
    ],
  },
  {
    heading: "DEVELOPERS",
    links: [
      { label: "GitHub", href: "https://github.com/rillcoin/rill" },
      { label: "RPC Docs", href: "https://docs.rillcoin.com/rpc" },
      { label: "Node Setup", href: "https://docs.rillcoin.com/node" },
      { label: "CLI Reference", href: "https://docs.rillcoin.com/cli" },
    ],
  },
] as const;

export default function Footer() {
  return (
    <footer
      className="flex flex-col gap-12 px-5 lg:px-20 py-14 lg:py-[60px]"
      style={{ backgroundColor: "var(--void)" }}
    >
      {/* Top gradient border */}
      <div
        style={{
          height: 1,
          background:
            "linear-gradient(90deg, transparent 0%, #1B3A6B 50%, transparent 100%)",
        }}
      />

      {/* Main row */}
      <div className="flex flex-col lg:flex-row gap-12 lg:gap-0 lg:justify-between">

        {/* Brand column */}
        <div
          className="flex flex-col gap-4"
          style={{ maxWidth: 280 }}
        >
          <span
            className="font-serif text-[28px]"
            style={{ color: "var(--text-primary)" }}
          >
            RILL
          </span>
          <p
            className="font-sans text-[14px]"
            style={{ color: "var(--text-dim)" }}
          >
            Wealth should flow like water.
          </p>
          {/* Network chip */}
          <div
            className="inline-flex items-center gap-2 self-start rounded-full px-2.5 py-1.5"
            style={{
              backgroundColor: "#060E1C",
              border: "1px solid #1B3A6B",
            }}
          >
            <span
              className="block rounded-full flex-shrink-0"
              style={{
                width: 6,
                height: 6,
                backgroundColor: "var(--cyan-400)",
              }}
            />
            <span
              className="font-mono text-[11px]"
              style={{ color: "rgba(34,211,238,0.502)" }}
            >
              Testnet Live
            </span>
          </div>
        </div>

        {/* Links */}
        <div className="flex flex-wrap gap-12 lg:gap-20">
          {FOOTER_LINKS.map((col) => (
            <div key={col.heading} className="flex flex-col gap-4">
              <span
                className="font-mono font-semibold text-[10px] tracking-[2px]"
                style={{ color: "var(--text-dim)" }}
              >
                {col.heading}
              </span>
              <ul className="flex flex-col gap-3">
                {col.links.map((link) => (
                  <li key={link.label}>
                    <a
                      href={link.href}
                      className="font-sans text-[14px] transition-opacity hover:opacity-80"
                      style={{
                        color:
                          "accent" in link && link.accent
                            ? "rgba(34,211,238,0.502)"
                            : "#64748B",
                      }}
                    >
                      {link.label}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      </div>

      {/* Bottom bar */}
      <div
        className="flex flex-col md:flex-row justify-between gap-3 pt-5"
        style={{
          borderTop: "1px solid #1A2535",
        }}
      >
        <span
          className="font-mono text-[12px]"
          style={{ color: "#1A2535" }}
        >
          &copy; 2026 RillCoin. Open source. MIT License.
        </span>
        <span className="font-mono text-[12px] flex gap-3">
          <a
            href="#"
            className="hover:opacity-70 transition-opacity"
            style={{ color: "#1A2535" }}
          >
            Privacy
          </a>
          <span style={{ color: "#1A2535" }}>&middot;</span>
          <a
            href="#"
            className="hover:opacity-70 transition-opacity"
            style={{ color: "#1A2535" }}
          >
            Terms
          </a>
          <span style={{ color: "#1A2535" }}>&middot;</span>
          <span style={{ color: "rgba(59,130,246,0.2)" }}>
            Built with Rust.
          </span>
        </span>
      </div>
    </footer>
  );
}
