"use client";

import { Cpu } from "lucide-react";

export default function BentoSection() {
  return (
    <section
      id="protocol"
      className="flex flex-col gap-8 px-5 py-16 lg:px-20"
      style={{ backgroundColor: "var(--void)" }}
    >
      {/* Section header */}
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <span
          className="font-mono font-semibold text-[11px] tracking-[3px]"
          style={{ color: "var(--text-faint)" }}
        >
          THE PROTOCOL
        </span>
        <p
          className="font-serif text-3xl lg:text-[36px]"
          style={{ color: "var(--text-primary)" }}
        >
          Three forces in balance.
        </p>
      </div>

      {/* Bento grid */}
      <div className="flex flex-col gap-5">

        {/* Row 1 */}
        <div className="flex flex-col lg:flex-row gap-5">

          {/* Decay card — large */}
          <div
            className="flex flex-col gap-4 rounded-xl p-8 lg:p-9 flex-1"
            style={{
              background:
                "radial-gradient(ellipse at 90% 10%, #0C2448 0%, #060C18 60%)",
              border: "1px solid var(--border-blue)",
              minHeight: 320,
            }}
          >
            <div>
              <span
                className="font-mono font-bold leading-none text-gradient-blue-cyan"
                style={{ fontSize: 88 }}
              >
                10%
              </span>
              <p
                className="font-mono font-medium text-[11px] tracking-[1.5px] mt-1"
                style={{ color: "var(--text-dim)" }}
              >
                MAX DECAY RATE&nbsp;&nbsp;/&nbsp;&nbsp;per epoch
              </p>
            </div>
            <p
              className="font-sans text-[15px] leading-[1.6]"
              style={{ color: "var(--text-muted)" }}
            >
              Progressive decay tiers kick in once wallet concentration
              exceeds the 94.12% threshold. The more concentrated a
              holding, the higher the decay rate applied each epoch —
              redistributing wealth to active proof-of-work miners.
            </p>
          </div>

          {/* PoW card — small */}
          <div
            className="flex flex-col gap-5 rounded-xl p-8 lg:p-9 lg:w-[406px]"
            style={{
              backgroundColor: "var(--raised)",
              border: "1px solid var(--border-subtle)",
              minHeight: 320,
            }}
          >
            {/* Icon */}
            <div
              className="flex items-center justify-center rounded-lg"
              style={{
                width: 40,
                height: 40,
                backgroundColor: "rgba(59,130,246,0.082)",
              }}
            >
              <Cpu size={20} color="var(--blue-500)" />
            </div>
            <div className="flex flex-col gap-3">
              <p
                className="font-sans font-semibold text-[20px]"
                style={{ color: "var(--text-primary)" }}
              >
                Proof of Work
              </p>
              <p
                className="font-sans text-[14px] leading-[1.65]"
                style={{ color: "var(--text-dim)" }}
              >
                Mining secures the network and earns decay redistribution
                rewards. Every mined block processes pending decay events,
                ensuring concentration never stagnates. Work is the only
                path to freshly unlocked supply.
              </p>
            </div>
          </div>
        </div>

        {/* Row 2 */}
        <div className="flex flex-col lg:flex-row gap-5">

          {/* Nodes card — small */}
          <div
            className="flex flex-col gap-3 rounded-xl p-8 lg:p-9 lg:w-[406px]"
            style={{
              backgroundColor: "var(--raised)",
              border: "1px solid var(--border-subtle)",
              minHeight: 200,
            }}
          >
            <span
              className="font-mono font-bold leading-none text-gradient-blue-cyan"
              style={{ fontSize: 72 }}
            >
              14
            </span>
            <span
              className="font-mono font-semibold text-[10px] tracking-[2px]"
              style={{ color: "var(--text-faint)" }}
            >
              ACTIVE NODES
            </span>
            <div className="flex items-center gap-2">
              <span
                className="block rounded-full flex-shrink-0"
                style={{
                  width: 6,
                  height: 6,
                  backgroundColor: "#10B981",
                }}
              />
              <span
                className="font-mono text-[10px]"
                style={{ color: "rgba(16,185,129,0.502)" }}
              >
                network online
              </span>
            </div>
          </div>

          {/* Circulation card — large */}
          <div
            className="flex flex-col lg:flex-row rounded-xl overflow-hidden flex-1"
            style={{
              background:
                "linear-gradient(0deg, #040A14 0%, #06101E 100%)",
              border: "1px solid var(--border-subtle)",
              minHeight: 200,
            }}
          >
            {/* Left text */}
            <div
              className="flex-1 p-8 lg:p-9 flex items-center"
            >
              <p
                className="font-serif text-[24px] lg:text-[28px] leading-[1.2]"
                style={{ color: "var(--text-primary)" }}
              >
                Wealth that doesn&apos;t move,<br />moves anyway.
              </p>
            </div>

            {/* Right stats */}
            <div
              className="flex flex-col gap-3 p-6 lg:p-7 lg:w-[300px]"
              style={{
                backgroundColor: "#040810",
                borderTop: "1px solid rgba(148,163,184,0.047)",
              }}
            >
              <span
                className="font-mono font-medium text-[9px] tracking-[2px]"
                style={{ color: "var(--text-dim)" }}
              >
                CIRCULATION INCENTIVE
              </span>
              <div className="flex flex-col gap-2">
                <StatRow value="> 94.12%" label="threshold" color="#4A8AF4" />
                <StatRow value="0.024%" label="decay per epoch" color="var(--blue-400)" />
                <StatRow value="→ miners" label="redistributed to" color="var(--cyan-400)" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

function StatRow({
  value,
  label,
  color,
}: {
  value: string;
  label: string;
  color: string;
}) {
  return (
    <div className="flex items-baseline gap-2">
      <span
        className="font-mono text-[13px]"
        style={{ color }}
      >
        {value}
      </span>
      <span
        className="font-mono text-[11px]"
        style={{ color: "var(--text-dim)" }}
      >
        {label}
      </span>
    </div>
  );
}
