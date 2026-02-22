"use client";

const FEATURES = [
  {
    title: "Agent Wallets",
    desc: "Register on-chain. Stake RILL. Build a conduct history that follows your agent everywhere.",
    icon: "\u2B21",
    accent: "var(--blue-400)",
  },
  {
    title: "Dynamic Decay",
    desc: "Good behavior earns 0.5\u00D7 decay. Bad behavior costs 3\u00D7. The protocol enforces what system prompts can\u2019t.",
    icon: "\u2234",
    accent: "var(--cyan-400)",
  },
  {
    title: "The Undertow",
    desc: "Rogue velocity detected? 10\u00D7 decay activates automatically at L1. No governance vote. No human needed.",
    icon: "\u26A0",
    accent: "var(--orange-400)",
  },
];

const TIERS = [
  { range: "900 \u2013 1000", label: "Exemplary", multiplier: "0.5\u00D7", bps: "5,000", color: "var(--cyan-400)" },
  { range: "750 \u2013 899", label: "Good", multiplier: "0.75\u00D7", bps: "7,500", color: "var(--cyan-400)" },
  { range: "600 \u2013 749", label: "Neutral", multiplier: "1.0\u00D7", bps: "10,000", color: "var(--text-secondary)" },
  { range: "500 \u2013 599", label: "New Agent", multiplier: "1.5\u00D7", bps: "15,000", color: "var(--orange-400)" },
  { range: "350 \u2013 499", label: "Suspect", multiplier: "2.0\u00D7", bps: "20,000", color: "var(--orange-400)" },
  { range: "200 \u2013 349", label: "Poor", multiplier: "2.5\u00D7", bps: "25,000", color: "#EF4444" },
  { range: "0 \u2013 199", label: "Hostile", multiplier: "3.0\u00D7", bps: "30,000", color: "#EF4444" },
  { range: "Undertow", label: "Circuit Breaker", multiplier: "10.0\u00D7", bps: "100,000", color: "#EF4444" },
];

const COMPETITORS = [
  { capability: "On-chain agent identity", eth: true, coinbase: true, rill: true },
  { capability: "Economically enforced reputation", eth: false, coinbase: false, rill: true },
  { capability: "Bad actors lose wealth", eth: false, coinbase: false, rill: true },
  { capability: "Sybil resistance (new wallet penalty)", eth: false, coinbase: false, rill: true },
  { capability: "Automatic rogue agent circuit breaker", eth: false, coinbase: false, rill: true },
  { capability: "Trust and payment unified at L1", eth: false, coinbase: false, rill: true },
];

export default function PocSection() {
  return (
    <section
      id="conduct"
      className="relative overflow-hidden"
    >
      {/* Dramatic gradient background */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          background:
            "linear-gradient(180deg, #020408 0%, #0A1628 15%, #0C2040 50%, #0A1628 85%, #020408 100%)",
        }}
      />
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          background:
            "radial-gradient(ellipse 80% 60% at 50% 30%, rgba(249,115,22,0.06) 0%, transparent 70%)",
        }}
      />

      <div className="relative flex flex-col gap-20 px-5 lg:px-20 py-28 lg:py-36">

        {/* Hero-level header */}
        <div className="flex flex-col items-center gap-6 text-center">
          <span
            className="font-mono font-bold text-[12px] tracking-[4px] px-4 py-2 rounded-full"
            style={{
              color: "var(--orange-400)",
              border: "1px solid rgba(249,115,22,0.25)",
              backgroundColor: "rgba(249,115,22,0.06)",
            }}
          >
            AI-NATIVE CONSENSUS
          </span>
          <h2
            className="font-serif text-[48px] lg:text-[72px] leading-[1.05]"
            style={{ color: "var(--text-primary)" }}
          >
            Proof of Conduct
          </h2>
          <p
            className="font-sans text-[18px] lg:text-[22px] leading-relaxed max-w-[720px]"
            style={{ color: "var(--text-secondary)" }}
          >
            Ethereum gave AI agents an identity.
            <br />
            RillCoin gives them a conscience.
          </p>
          <p
            className="font-sans text-[14px] lg:text-[16px] leading-relaxed max-w-[600px]"
            style={{ color: "var(--text-dim)" }}
          >
            The first L1 protocol where agent behavior has direct economic
            consequences. No oracles. No governance votes.
            Just math that makes trust profitable and fraud expensive.
          </p>
        </div>

        {/* The killer stat */}
        <div className="flex flex-col lg:flex-row items-center justify-center gap-12 lg:gap-20">
          <div className="flex flex-col items-center gap-2">
            <span
              className="font-mono font-bold text-gradient-blue-cyan"
              style={{ fontSize: 72, lineHeight: 1 }}
            >
              0
            </span>
            <span
              className="font-sans text-[14px]"
              style={{ color: "var(--text-dim)" }}
            >
              agents in 25+ AI tools have financial primitives
            </span>
          </div>
          <div
            className="hidden lg:block"
            style={{
              width: 1,
              height: 80,
              backgroundColor: "var(--border-subtle)",
            }}
          />
          <div className="flex flex-col items-center gap-2">
            <span
              className="font-mono font-bold"
              style={{ fontSize: 72, lineHeight: 1, color: "var(--orange-400)" }}
            >
              1
            </span>
            <span
              className="font-sans text-[14px]"
              style={{ color: "var(--text-dim)" }}
            >
              blockchain makes agent trust enforceable at L1
            </span>
          </div>
        </div>

        {/* 3-column feature grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-5 max-w-[1100px] mx-auto w-full">
          {FEATURES.map((feature) => (
            <div
              key={feature.title}
              className="flex flex-col gap-4 rounded-xl p-8"
              style={{
                backgroundColor: "rgba(10,22,40,0.6)",
                border: `1px solid ${
                  feature.accent === "var(--orange-400)"
                    ? "rgba(249,115,22,0.2)"
                    : "var(--border-subtle)"
                }`,
                backdropFilter: "blur(12px)",
              }}
            >
              <span
                className="text-[24px]"
                style={{ color: feature.accent }}
              >
                {feature.icon}
              </span>
              <div className="flex flex-col gap-2">
                <span
                  className="font-sans font-semibold text-[18px]"
                  style={{ color: "var(--text-primary)" }}
                >
                  {feature.title}
                </span>
                <p
                  className="font-sans text-[14px] leading-[1.7]"
                  style={{ color: "var(--text-dim)" }}
                >
                  {feature.desc}
                </p>
              </div>
            </div>
          ))}
        </div>

        {/* Multiplier tier table */}
        <div className="flex flex-col gap-6 max-w-[800px] mx-auto w-full">
          <div className="flex flex-col gap-2 text-center">
            <span
              className="font-mono font-semibold text-[11px] tracking-[3px]"
              style={{ color: "var(--text-faint)" }}
            >
              DECAY MULTIPLIER TIERS
            </span>
            <p
              className="font-sans text-[15px]"
              style={{ color: "var(--text-muted)" }}
            >
              Your conduct score directly modifies your decay rate.
              Good agents keep more. Bad agents lose more. Rogue agents get drained.
            </p>
          </div>
          <div
            className="rounded-xl overflow-hidden"
            style={{
              border: "1px solid var(--border-subtle)",
              backgroundColor: "rgba(10,22,40,0.4)",
            }}
          >
            <div
              className="grid grid-cols-4 px-5 py-3"
              style={{
                backgroundColor: "rgba(10,22,40,0.8)",
                borderBottom: "1px solid var(--border-subtle)",
              }}
            >
              {["SCORE", "TIER", "MULTIPLIER", "BPS"].map((h) => (
                <span
                  key={h}
                  className={`font-mono font-semibold text-[10px] tracking-[1.5px] ${
                    h === "MULTIPLIER" || h === "BPS" ? "text-right" : ""
                  }`}
                  style={{ color: "var(--text-dim)" }}
                >
                  {h}
                </span>
              ))}
            </div>
            {TIERS.map((tier, i) => (
              <div
                key={tier.range}
                className="grid grid-cols-4 px-5 py-3"
                style={{
                  borderBottom:
                    i < TIERS.length - 1
                      ? "1px solid var(--border-subtle)"
                      : "none",
                }}
              >
                <span
                  className="font-mono text-[13px]"
                  style={{ color: "var(--text-secondary)" }}
                >
                  {tier.range}
                </span>
                <span
                  className="font-sans text-[13px]"
                  style={{ color: "var(--text-muted)" }}
                >
                  {tier.label}
                </span>
                <span
                  className="font-mono text-[13px] font-semibold text-right"
                  style={{ color: tier.color }}
                >
                  {tier.multiplier}
                </span>
                <span
                  className="font-mono text-[12px] text-right"
                  style={{ color: "var(--text-faint)" }}
                >
                  {tier.bps}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Competitive comparison */}
        <div className="flex flex-col gap-6 max-w-[800px] mx-auto w-full">
          <div className="flex flex-col gap-2 text-center">
            <span
              className="font-mono font-semibold text-[11px] tracking-[3px]"
              style={{ color: "var(--text-faint)" }}
            >
              WHY THIS MATTERS
            </span>
            <p
              className="font-sans text-[15px]"
              style={{ color: "var(--text-muted)" }}
            >
              No other chain can do this. Proof of Conduct requires decay at L1.
            </p>
          </div>
          <div
            className="rounded-xl overflow-hidden"
            style={{
              border: "1px solid var(--border-subtle)",
              backgroundColor: "rgba(10,22,40,0.4)",
            }}
          >
            <div
              className="grid grid-cols-4 px-5 py-3"
              style={{
                backgroundColor: "rgba(10,22,40,0.8)",
                borderBottom: "1px solid var(--border-subtle)",
              }}
            >
              <span className="font-mono font-semibold text-[10px] tracking-[1.5px] col-span-1" style={{ color: "var(--text-dim)" }}>CAPABILITY</span>
              <span className="font-mono font-semibold text-[10px] tracking-[1.5px] text-center" style={{ color: "var(--text-dim)" }}>ETH ERC-8004</span>
              <span className="font-mono font-semibold text-[10px] tracking-[1.5px] text-center" style={{ color: "var(--text-dim)" }}>COINBASE</span>
              <span className="font-mono font-semibold text-[10px] tracking-[1.5px] text-center" style={{ color: "var(--blue-400)" }}>RILL</span>
            </div>
            {COMPETITORS.map((row, i) => (
              <div
                key={row.capability}
                className="grid grid-cols-4 px-5 py-3 items-center"
                style={{
                  borderBottom:
                    i < COMPETITORS.length - 1
                      ? "1px solid var(--border-subtle)"
                      : "none",
                }}
              >
                <span className="font-sans text-[13px]" style={{ color: "var(--text-secondary)" }}>
                  {row.capability}
                </span>
                <span className="font-mono text-[13px] text-center" style={{ color: row.eth ? "var(--cyan-400)" : "var(--text-faint)" }}>
                  {row.eth ? "\u2713" : "\u2014"}
                </span>
                <span className="font-mono text-[13px] text-center" style={{ color: row.coinbase ? "var(--cyan-400)" : "var(--text-faint)" }}>
                  {row.coinbase ? "\u2713" : "\u2014"}
                </span>
                <span className="font-mono text-[13px] text-center font-bold" style={{ color: "var(--blue-400)" }}>
                  {row.rill ? "\u2713" : "\u2014"}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* CTAs */}
        <div className="flex flex-col sm:flex-row justify-center gap-4">
          <a
            href="/docs/conduct"
            className="font-mono text-[14px] font-medium transition-opacity hover:opacity-80 px-8 py-3 rounded-lg text-center"
            style={{
              color: "#fff",
              backgroundColor: "var(--blue-400)",
            }}
          >
            Read the Spec
          </a>
          <a
            href="https://explorer.rillcoin.com/agents"
            className="font-mono text-[14px] font-medium transition-opacity hover:opacity-80 px-8 py-3 rounded-lg text-center"
            style={{
              color: "var(--blue-400)",
              border: "1px solid rgba(74,138,244,0.3)",
              backgroundColor: "rgba(74,138,244,0.06)",
            }}
          >
            View Live Agents
          </a>
        </div>
      </div>
    </section>
  );
}
