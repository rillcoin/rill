"use client";

export default function CtaSection() {
  return (
    <section
      id="testnet"
      className="relative flex flex-col items-center gap-8 px-5 lg:px-20 py-24 lg:py-[100px] overflow-hidden"
      style={{ backgroundColor: "var(--void)" }}
    >
      {/* Radial glow top-center */}
      <div
        className="absolute top-0 left-1/2 -translate-x-1/2 pointer-events-none"
        style={{
          width: 800,
          height: 500,
          background:
            "radial-gradient(ellipse at 50% 0%, #0C2448 0%, transparent 70%)",
        }}
      />

      {/* Section label */}
      <span
        className="relative font-mono font-semibold text-[11px] tracking-[3px] text-center"
        style={{ color: "rgba(34,211,238,0.314)" }}
      >
        TESTNET IS OPEN
      </span>

      {/* Headline */}
      <h2
        className="relative font-serif text-center leading-none"
        style={{
          fontSize: "clamp(52px, 6.7vw, 96px)",
          color: "var(--text-primary)",
        }}
      >
        Come build.
      </h2>

      {/* Sub-headline */}
      <p
        className="relative font-sans text-[18px] leading-[1.65] text-center max-w-xl"
        style={{ color: "var(--text-dim)" }}
      >
        The testnet is live and open to anyone. Mine blocks, trigger decay
        events, and help stress-test the protocol before mainnet launch.
      </p>

      {/* CTA button */}
      <a
        href="/wallet"
        className="relative inline-flex items-center font-sans font-semibold text-[16px] rounded py-4 px-10 transition-opacity hover:opacity-90"
        style={{
          color: "#0A0F1A",
          background: "linear-gradient(135deg, #F97316 0%, #FB923C 100%)",
          boxShadow: "0 8px 40px rgba(249,115,22,0.314)",
        }}
      >
        Try the Wallet&nbsp;&nbsp;&rarr;
      </a>

      {/* Trust line */}
      <p
        className="relative font-mono text-[12px] text-center"
        style={{ color: "#1A2535" }}
      >
        rillcoin.com/wallet&nbsp;&nbsp;&middot;&nbsp;&nbsp;/faucet on Discord
      </p>
    </section>
  );
}
