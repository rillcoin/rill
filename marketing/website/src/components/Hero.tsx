"use client";

export default function Hero() {
  return (
    <section
      id="hero"
      style={{
        background:
          "linear-gradient(180deg, #020408 0%, #040B16 50%, #020408 100%)",
      }}
    >
      {/* Mobile layout: stacked content */}
      <div className="lg:hidden flex flex-col items-start gap-7 px-5 py-16 relative overflow-hidden">
        {/* Mobile glow */}
        <div
          className="absolute inset-0 pointer-events-none"
          style={{
            background:
              "radial-gradient(ellipse 100% 50% at 50% 0%, #0C2040 0%, transparent 100%)",
          }}
        />
        <HeroContent />
      </div>

      {/* Desktop layout: absolute-positioned per Pencil spec (1440px canvas) */}
      <div
        className="relative hidden lg:block overflow-hidden"
        style={{ height: 820 }}
      >
        {/* Background radial glow */}
        <div
          className="absolute inset-0 pointer-events-none"
          style={{
            background:
              "radial-gradient(ellipse 65% 85% at 74% 44%, #0C2040 0%, transparent 100%)",
          }}
        />

        {/* Orb */}
        <div
          className="absolute rounded-full pointer-events-none"
          style={{
            width: 620,
            height: 620,
            left: 680,
            top: 60,
            background:
              "radial-gradient(ellipse at 30% 30%, #1B58B0 0%, #0C2040 35%, #040B16 60%, #020408 85%)",
            boxShadow:
              "0 0 120px 60px rgba(59,130,246,0.125), 0 0 240px 120px rgba(34,211,238,0.063)",
          }}
        >
          <div
            className="absolute inset-0 rounded-full"
            style={{
              background:
                "radial-gradient(ellipse at 60% 60%, rgba(34,211,238,0.125) 0%, transparent 50%)",
            }}
          />
        </div>

        {/* Ring 1 */}
        <div
          className="absolute rounded-full pointer-events-none"
          style={{
            width: 664,
            height: 664,
            left: 658,
            top: 38,
            border: "1px solid rgba(59,130,246,0.157)",
          }}
        />

        {/* Ring 2 */}
        <div
          className="absolute rounded-full pointer-events-none"
          style={{
            width: 760,
            height: 760,
            left: 610,
            top: -10,
            border: "1px solid rgba(34,211,238,0.082)",
          }}
        />

        {/* Hero content block */}
        <div
          className="absolute flex flex-col gap-7"
          style={{ width: 550, left: 120, top: 155 }}
        >
          <HeroContent />
        </div>

        {/* HUD Chips */}
        <HudChip
          style={{ left: 1096, top: 144 }}
          borderColor="rgba(34,211,238,0.125)"
          label="THRESHOLD"
          labelColor="rgba(34,211,238,0.376)"
          value="94.12%"
          valueColor="var(--cyan-400)"
        />
        <HudChip
          style={{ left: 1248, top: 408 }}
          borderColor="rgba(59,130,246,0.125)"
          label="DECAY RATE"
          labelColor="rgba(59,130,246,0.376)"
          value="0.024%"
          valueColor="var(--blue-400)"
        />
        <HudChip
          style={{ left: 872, top: 618 }}
          borderColor="rgba(249,115,22,0.125)"
          label="DAILY DRIP"
          labelColor="rgba(249,115,22,0.376)"
          value="10 RILL"
          valueColor="var(--orange-500)"
        />
      </div>
    </section>
  );
}

function HeroContent() {
  return (
    <>
      {/* Badge */}
      <div
        className="inline-flex items-center gap-2 self-start rounded px-3 py-1"
        style={{
          backgroundColor: "rgba(34,211,238,0.055)",
          border: "1px solid rgba(34,211,238,0.188)",
        }}
      >
        <span
          className="block rounded-full flex-shrink-0"
          style={{ width: 5, height: 5, backgroundColor: "var(--cyan-400)" }}
        />
        <span
          className="font-mono font-semibold text-[10px] tracking-[2.5px]"
          style={{ color: "var(--cyan-400)" }}
        >
          TESTNET LIVE
        </span>
      </div>

      {/* Headline */}
      <h1
        className="font-serif leading-none"
        style={{ fontSize: "clamp(72px, 8.9vw, 128px)" }}
      >
        <span className="text-gradient-hero">
          Wealth
          <br />
          flows.
        </span>
      </h1>

      {/* Divider */}
      <div
        className="w-full"
        style={{ height: 1, backgroundColor: "rgba(148,163,184,0.082)" }}
      />

      {/* Sub-headline */}
      <p
        className="font-sans text-[18px] leading-[1.65]"
        style={{ color: "var(--text-muted)", maxWidth: 480 }}
      >
        A proof-of-work cryptocurrency with progressive concentration decay.
        Holdings above thresholds flow back to active miners.
      </p>

      {/* CTA Row */}
      <div className="flex flex-wrap items-center gap-4">
        <a
          href="/wallet"
          className="inline-flex items-center font-sans font-semibold text-[14px] rounded py-3.5 px-7 transition-opacity hover:opacity-90"
          style={{
            color: "#0A0F1A",
            background: "linear-gradient(135deg, #F97316 0%, #FB923C 100%)",
            boxShadow: "0 8px 28px rgba(249,115,22,0.271)",
          }}
        >
          Try the Wallet
        </a>
        <a
          href="https://docs.rillcoin.com"
          className="inline-flex items-center font-sans text-[14px] rounded py-3.5 px-7 transition-opacity hover:opacity-80"
          style={{
            color: "var(--blue-500)",
            border: "1px solid rgba(59,130,246,0.220)",
          }}
        >
          Read the docs
        </a>
      </div>

      {/* Trust line */}
      <p
        className="font-mono text-[11px]"
        style={{ color: "#1E2A38" }}
      >
        Open source&nbsp;&nbsp;&middot;&nbsp;&nbsp;Ed25519&nbsp;&nbsp;&middot;&nbsp;&nbsp;BLAKE3&nbsp;&nbsp;&middot;&nbsp;&nbsp;Rust 2024
      </p>
    </>
  );
}

type HudChipProps = {
  style: React.CSSProperties;
  borderColor: string;
  label: string;
  labelColor: string;
  value: string;
  valueColor: string;
};

function HudChip({
  style,
  borderColor,
  label,
  labelColor,
  value,
  valueColor,
}: HudChipProps) {
  return (
    <div
      className="absolute flex flex-col gap-1 rounded-md px-3.5 py-2.5"
      style={{
        ...style,
        backgroundColor: "#060E1C",
        border: `1px solid ${borderColor}`,
      }}
    >
      <span
        className="font-mono font-medium text-[9px] tracking-[1.5px]"
        style={{ color: labelColor }}
      >
        {label}
      </span>
      <span
        className="font-mono font-bold text-[22px] leading-none"
        style={{ color: valueColor }}
      >
        {value}
      </span>
    </div>
  );
}
