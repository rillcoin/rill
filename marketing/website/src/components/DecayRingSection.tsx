"use client";

export default function DecayRingSection() {
  // SVG ring math
  const radius = 180;
  const strokeWidth = 40;
  const size = 400;
  const center = size / 2;
  const circumference = 2 * Math.PI * radius;
  // ~75% of circumference (approx 270 degree arc)
  const dashArray = `${circumference * 0.75} ${circumference * 0.25}`;
  // Rotate so arc starts top-left (~-225deg)
  const rotation = -225;

  const ring = (
    <div className="relative flex-shrink-0" style={{ width: size, height: size }}>
      {/* Glow behind ring */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          background:
            "radial-gradient(ellipse at 50% 50%, #0C2448 0%, transparent 70%)",
        }}
      />
      <svg
        width={size}
        height={size}
        viewBox={`0 0 ${size} ${size}`}
        fill="none"
        aria-label="Concentration decay ring showing 94% threshold"
      >
        <defs>
          <linearGradient
            id="ringGradient"
            x1="0%"
            y1="0%"
            x2="100%"
            y2="100%"
          >
            <stop offset="0%" stopColor="#1B58B0" />
            <stop offset="60%" stopColor="#22D3EE" />
            <stop offset="100%" stopColor="rgba(34,211,238,0.188)" />
          </linearGradient>
        </defs>

        {/* Faint track ring */}
        <circle
          cx={center}
          cy={center}
          r={radius}
          fill="none"
          stroke="rgba(59,130,246,0.082)"
          strokeWidth={strokeWidth}
        />

        {/* Active arc */}
        <circle
          cx={center}
          cy={center}
          r={radius}
          fill="none"
          stroke="url(#ringGradient)"
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          strokeDasharray={dashArray}
          transform={`rotate(${rotation} ${center} ${center})`}
        />

        {/* Inner fill to make donut */}
        <circle
          cx={center}
          cy={center}
          r={radius - strokeWidth / 2 - 4}
          fill="#020408"
        />

        {/* Center percentage */}
        <text
          x={center}
          y={center - 12}
          textAnchor="middle"
          dominantBaseline="middle"
          fill="#F1F5F9"
          fontFamily="var(--font-instrument-serif), Georgia, serif"
          fontSize="88"
          fontWeight="400"
        >
          94%
        </text>

        {/* Center label */}
        <text
          x={center}
          y={center + 52}
          textAnchor="middle"
          dominantBaseline="middle"
          fill="#334155"
          fontFamily="var(--font-jetbrains-mono), monospace"
          fontSize="12"
          fontWeight="400"
          letterSpacing="1"
        >
          threshold
        </text>
      </svg>
    </div>
  );

  return (
    <section
      id="decay"
      className="overflow-hidden"
      style={{ backgroundColor: "var(--void)" }}
    >
      {/* Mobile layout: stacked */}
      <div className="flex flex-col items-center gap-10 px-5 py-16 lg:hidden">
        {ring}
        <RightBlock />
      </div>

      {/* Desktop layout: absolute-positioned to match Pencil design */}
      <div
        className="relative hidden lg:block"
        style={{ height: 640 }}
      >
        {/* Orb glow */}
        <div
          className="absolute pointer-events-none"
          style={{
            width: 400,
            height: 400,
            left: 520,
            top: 80,
            background:
              "radial-gradient(ellipse at 50% 50%, #0C2448 0%, transparent 70%)",
          }}
        />

        {/* Ring */}
        <div className="absolute" style={{ left: 520, top: 80 }}>
          {ring}
        </div>

        {/* Right text block */}
        <div
          className="absolute flex flex-col gap-6"
          style={{ left: 980, top: 160, width: 380 }}
        >
          <RightBlock />
        </div>
      </div>
    </section>
  );
}

function RightBlock() {
  return (
    <>
      <span
        className="font-mono font-semibold text-[11px] tracking-[3px]"
        style={{ color: "rgba(34,211,238,0.314)" }}
      >
        CONCENTRATION DECAY
      </span>
      <h2
        className="font-serif leading-[1.1]"
        style={{
          fontSize: "clamp(28px, 3vw, 44px)",
          color: "var(--text-primary)",
        }}
      >
        The more you hold,
        <br />
        the more flows out.
      </h2>
      <p
        className="font-sans text-[16px] leading-[1.65]"
        style={{ color: "var(--text-dim)" }}
      >
        Wallets holding more than 94.12% of the total supply trigger
        progressive decay. Each epoch, a portion of excess holdings is
        redistributed directly to active miners as a block reward
        supplement — ensuring supply stays in motion.
      </p>
    </>
  );
}
