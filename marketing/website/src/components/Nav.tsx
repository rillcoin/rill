"use client";

import { useState } from "react";
import Image from "next/image";
import Link from "next/link";

const NAV_LINKS = [
  { label: "Protocol", href: "https://docs.rillcoin.com/protocol" },
  { label: "Docs", href: "https://docs.rillcoin.com" },
  { label: "Testnet", href: "https://faucet.rillcoin.com" },
  { label: "Wallet", href: "/wallet" },
];

export default function Nav() {
  const [open, setOpen] = useState(false);

  return (
    <nav
      className="sticky top-0 z-50 w-full"
      style={{ backgroundColor: "var(--void)" }}
    >
      {/* Top bar */}
      <div className="flex items-center justify-between h-[72px] px-5 lg:px-20">
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

        {/* Center: Nav links (desktop) */}
        <div className="hidden md:flex items-center gap-8">
          {NAV_LINKS.map((link) => (
            <Link
              key={link.label}
              href={link.href}
              className="font-sans text-[13px] transition-colors hover:opacity-80"
              style={{ color: "var(--text-dim)" }}
            >
              {link.label}
            </Link>
          ))}
        </div>

        {/* Right: CTA button (desktop) */}
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

        {/* Mobile: hamburger */}
        <button
          className="md:hidden flex flex-col gap-1.5 p-2"
          aria-label={open ? "Close menu" : "Open menu"}
          onClick={() => setOpen(!open)}
        >
          <span
            className="block w-5 h-px transition-transform duration-200"
            style={{
              backgroundColor: "var(--text-secondary)",
              transform: open ? "translateY(3.5px) rotate(45deg)" : "none",
            }}
          />
          <span
            className="block w-5 h-px transition-opacity duration-200"
            style={{
              backgroundColor: "var(--text-secondary)",
              opacity: open ? 0 : 1,
            }}
          />
          <span
            className="block h-px transition-all duration-200"
            style={{
              backgroundColor: "var(--text-secondary)",
              width: open ? 20 : 12,
              transform: open ? "translateY(-3.5px) rotate(-45deg)" : "none",
            }}
          />
        </button>
      </div>

      {/* Mobile menu panel */}
      {open && (
        <div
          className="md:hidden flex flex-col gap-1 px-5 pb-6"
          style={{
            backgroundColor: "var(--void)",
            borderTop: "1px solid var(--border-subtle)",
          }}
        >
          {NAV_LINKS.map((link) => (
            <a
              key={link.label}
              href={link.href}
              onClick={() => setOpen(false)}
              className="font-sans text-[15px] py-3 transition-opacity hover:opacity-80"
              style={{
                color: "var(--text-secondary)",
                borderBottom: "1px solid var(--border-subtle)",
              }}
            >
              {link.label}
            </a>
          ))}
          <a
            href="/wallet"
            onClick={() => setOpen(false)}
            className="inline-flex items-center justify-center font-sans font-medium text-[14px] rounded py-3 mt-3 transition-opacity hover:opacity-90"
            style={{
              color: "var(--cyan-400)",
              backgroundColor: "rgba(34,211,238,0.071)",
              border: "1px solid rgba(34,211,238,0.271)",
            }}
          >
            Try the Wallet
          </a>
        </div>
      )}
    </nav>
  );
}
