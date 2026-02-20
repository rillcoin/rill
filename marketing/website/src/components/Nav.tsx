"use client";

import Image from "next/image";
import Link from "next/link";

export default function Nav() {
  return (
    <nav
      className="sticky top-0 z-50 flex items-center justify-between w-full h-[72px] px-5 lg:px-20"
      style={{ backgroundColor: "var(--void)" }}
    >
      {/* Left: Logo group */}
      <div className="flex items-center gap-3">
        <div
          className="relative flex-shrink-0 w-8 h-8 rounded-full overflow-hidden"
          style={{
            background: "linear-gradient(135deg, #4A8AF4 0%, #22D3EE 100%)",
          }}
        >
          <Image
            src="/rill_logo.png"
            alt="RillCoin logo"
            width={32}
            height={32}
            className="object-cover w-full h-full"
            priority
          />
        </div>
        <span
          className="font-sans font-bold text-[15px] tracking-[5px]"
          style={{ color: "var(--text-primary)" }}
        >
          RILL
        </span>
      </div>

      {/* Center: Nav links */}
      <div className="hidden md:flex items-center gap-8">
        <Link
          href="https://docs.rillcoin.com/protocol"
          className="font-sans text-[13px] transition-colors hover:opacity-80"
          style={{ color: "var(--text-dim)" }}
        >
          Protocol
        </Link>
        <Link
          href="https://docs.rillcoin.com"
          className="font-sans text-[13px] transition-colors hover:opacity-80"
          style={{ color: "var(--text-dim)" }}
        >
          Docs
        </Link>
        <Link
          href="https://faucet.rillcoin.com"
          className="font-sans text-[13px] font-medium transition-colors hover:opacity-80"
          style={{ color: "var(--text-secondary)" }}
        >
          Testnet
        </Link>
        <Link
          href="/wallet"
          className="font-sans text-[13px] font-medium transition-colors hover:opacity-80"
          style={{ color: "var(--text-secondary)" }}
        >
          Wallet
        </Link>
      </div>

      {/* Right: CTA button */}
      <a
        href="/wallet"
        className="hidden md:inline-flex items-center font-sans font-medium text-[13px] rounded px-4 py-2 transition-opacity hover:opacity-90"
        style={{
          color: "var(--cyan-400)",
          backgroundColor: "rgba(34,211,238,0.071)",
          border: "1px solid rgba(34,211,238,0.271)",
        }}
      >
        Try the Wallet
      </a>

      {/* Mobile: hamburger placeholder */}
      <button
        className="md:hidden flex flex-col gap-1.5 p-2"
        aria-label="Open menu"
      >
        <span
          className="block w-5 h-px"
          style={{ backgroundColor: "var(--text-secondary)" }}
        />
        <span
          className="block w-5 h-px"
          style={{ backgroundColor: "var(--text-secondary)" }}
        />
        <span
          className="block w-3 h-px"
          style={{ backgroundColor: "var(--text-secondary)" }}
        />
      </button>
    </nav>
  );
}
