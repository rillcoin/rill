"use client";

const STATS = [
  { number: "12,847", label: "BLOCKS MINED" },
  { number: "1,000,000", label: "CIRCULATING RILL" },
  { number: "2,341", label: "DECAY EVENTS" },
  { number: "14", label: "ACTIVE NODES" },
] as const;

export default function StatsSection() {
  return (
    <section
      className="px-5 lg:px-20"
      style={{ backgroundColor: "var(--void)" }}
    >
      {/* Top divider */}
      <div style={{ height: 1, backgroundColor: "#0F1A28" }} />

      {/* Stats row */}
      <div
        className="flex flex-wrap justify-between items-center gap-8 py-12"
        style={{ minHeight: 200 }}
      >
        {STATS.map((stat) => (
          <div key={stat.label} className="flex flex-col gap-1">
            <span
              className="font-mono font-bold leading-none text-gradient-blue-cyan"
              style={{ fontSize: "clamp(36px, 3.9vw, 56px)" }}
            >
              {stat.number}
            </span>
            <span
              className="font-mono font-medium text-[10px] tracking-[2px]"
              style={{ color: "var(--text-faint)" }}
            >
              {stat.label}
            </span>
          </div>
        ))}
      </div>

      {/* Bottom divider */}
      <div style={{ height: 1, backgroundColor: "#0F1A28" }} />
    </section>
  );
}
