"use client";

const CLI_OUTPUT = `$ rill-cli wallet create
  → address: trill1qx7f3k9m2p8...
  → saved to: ~/.rill/wallet.dat

$ rill-cli balance
  → balance:            10.00000000 RILL
  → effective_balance:   9.97341200 RILL
  → decay_applied:       0.02658800 RILL
  → threshold_pct:          94.12%

$ rill-cli send --to trill1qa4n8r2... --amount 5.0
  → txid: a3f8b2c1d4e96f03...
  → status: broadcast`;

export default function CliSection() {
  return (
    <section
      className="flex flex-col items-center gap-12 px-5 lg:px-20 py-24"
      style={{ backgroundColor: "#030710" }}
    >
      {/* Label */}
      <span
        className="font-sans font-semibold text-[12px] tracking-[3px] uppercase"
        style={{ color: "var(--text-muted)" }}
      >
        Built in Rust
      </span>

      {/* Headline */}
      <div className="flex flex-col items-center gap-4 text-center">
        <h2
          className="font-serif text-[40px] lg:text-[52px] leading-none"
          style={{ color: "var(--text-primary)" }}
        >
          Consensus math. No floats. Ever.
        </h2>
        <p
          className="font-sans text-[16px] lg:text-[18px] leading-relaxed max-w-[560px]"
          style={{ color: "var(--text-secondary)" }}
        >
          Integer-only arithmetic with checked operations throughout. No
          rounding errors, no surprises at the consensus layer.
        </p>
      </div>

      {/* Terminal block */}
      <div
        className="w-full max-w-[800px] rounded-xl overflow-hidden"
        style={{
          backgroundColor: "#05080F",
          border: "1px solid var(--border-subtle)",
        }}
      >
        {/* Title bar */}
        <div
          className="flex items-center justify-between h-10 px-4"
          style={{
            backgroundColor: "var(--base)",
            borderBottom: "1px solid var(--border-subtle)",
          }}
        >
          {/* macOS-style dots */}
          <div className="flex items-center gap-2">
            <span
              className="w-2.5 h-2.5 rounded-full"
              style={{ backgroundColor: "rgba(239,68,68,0.376)" }}
            />
            <span
              className="w-2.5 h-2.5 rounded-full"
              style={{ backgroundColor: "rgba(245,158,11,0.376)" }}
            />
            <span
              className="w-2.5 h-2.5 rounded-full"
              style={{ backgroundColor: "rgba(16,185,129,0.376)" }}
            />
          </div>
          {/* Filename */}
          <span
            className="font-mono text-[12px]"
            style={{ color: "rgba(148,163,184,0.376)" }}
          >
            rill-cli&nbsp;&nbsp;·&nbsp;&nbsp;testnet
          </span>
        </div>

        {/* CLI output */}
        <pre
          className="font-mono text-[13px] leading-[1.75] p-6 overflow-x-auto"
          style={{ color: "#94A3B8" }}
        >
          {CLI_OUTPUT.split("\n").map((line, i) => {
            // Colour prompt lines cyan, arrow lines slightly dimmer
            if (line.startsWith("$")) {
              return (
                <span key={i}>
                  <span style={{ color: "var(--cyan-400)" }}>$</span>
                  <span style={{ color: "var(--text-primary)" }}>
                    {line.slice(1)}
                  </span>
                  {"\n"}
                </span>
              );
            }
            if (line.includes("→")) {
              const [before, after] = line.split("→");
              return (
                <span key={i}>
                  {before}
                  <span style={{ color: "var(--blue-400)" }}>→</span>
                  {after}
                  {"\n"}
                </span>
              );
            }
            return <span key={i}>{line}{"\n"}</span>;
          })}
        </pre>
      </div>

      {/* Bottom links */}
      <div className="flex items-center gap-6">
        <a
          href="https://github.com/rillcoin/rill"
          target="_blank"
          rel="noopener noreferrer"
          className="font-mono text-[13px] transition-opacity hover:opacity-80"
          style={{ color: "var(--blue-500)" }}
        >
          View source on GitHub →
        </a>
        <span style={{ color: "var(--text-faint)" }}>·</span>
        <a
          href="https://docs.rillcoin.com"
          className="font-mono text-[13px] transition-opacity hover:opacity-80"
          style={{ color: "var(--text-dim)" }}
        >
          CLI Reference
        </a>
      </div>
    </section>
  );
}
