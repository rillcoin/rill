"use client";

import { useState } from "react";
import Link from "next/link";
import Image from "next/image";
import { usePathname } from "next/navigation";
import { Menu, X } from "lucide-react";

const NAV_SECTIONS = [
  {
    label: "INTRODUCTION",
    items: [
      { label: "Getting Started", href: "/" },
      { label: "Whitepaper", href: "/whitepaper" },
    ],
  },
  {
    label: "PROTOCOL",
    items: [
      { label: "Architecture", href: "/protocol" },
      { label: "Decay Mechanics", href: "/decay" },
      { label: "Mining", href: "/mining" },
    ],
  },
  {
    label: "REFERENCE",
    items: [
      { label: "Wallet & CLI", href: "/cli" },
      { label: "RPC Reference", href: "/rpc" },
      { label: "Node Setup", href: "/node" },
    ],
  },
];

export default function MobileNav() {
  const [open, setOpen] = useState(false);
  const pathname = usePathname();

  const isActive = (href: string) => {
    if (href === "/") return pathname === "/" || pathname === "";
    return pathname === href || pathname === `${href}/`;
  };

  return (
    <>
      {/* Top bar — mobile only */}
      <header
        className="lg:hidden flex items-center justify-between px-4 py-3 sticky top-0 z-40"
        style={{
          background: "var(--base)",
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        <div className="flex items-center gap-2.5">
          <div
            className="w-7 h-7 rounded-md flex items-center justify-center"
            style={{
              background: "linear-gradient(135deg, #4A8AF4 0%, #22D3EE 100%)",
            }}
          >
            <Image
              src="/rill_logo.png"
              alt="RillCoin"
              width={16}
              height={16}
              className="w-4 h-4 object-contain"
            />
          </div>
          <span className="text-sm font-semibold" style={{ color: "var(--text-primary)" }}>
            RillCoin Docs
          </span>
          <span
            className="text-xs px-1.5 py-0.5 rounded"
            style={{
              color: "var(--cyan-400)",
              background: "rgba(34, 211, 238, 0.1)",
            }}
          >
            Testnet Live
          </span>
        </div>
        <button
          onClick={() => setOpen(!open)}
          className="p-1.5 rounded-md transition-colors"
          style={{ color: "var(--text-secondary)" }}
          aria-label="Toggle navigation"
        >
          {open ? <X size={20} /> : <Menu size={20} />}
        </button>
      </header>

      {/* Dropdown drawer */}
      {open && (
        <div
          className="lg:hidden fixed inset-0 top-[52px] z-30 overflow-y-auto"
          style={{ background: "var(--base)" }}
        >
          <nav className="px-4 py-5">
            {NAV_SECTIONS.map((section) => (
              <div key={section.label} className="mb-6">
                <p
                  className="text-xs font-semibold tracking-widest mb-2 px-2"
                  style={{ color: "var(--text-dim)" }}
                >
                  {section.label}
                </p>
                <ul className="space-y-0.5">
                  {section.items.map((item) => {
                    const active = isActive(item.href);
                    return (
                      <li key={item.href}>
                        <Link
                          href={item.href}
                          onClick={() => setOpen(false)}
                          className="flex items-center gap-2 px-3 py-2 rounded-md text-sm transition-all"
                          style={{
                            color: active
                              ? "var(--cyan-400)"
                              : "var(--text-secondary)",
                            background: active
                              ? "rgba(34, 211, 238, 0.075)"
                              : "transparent",
                            fontWeight: active ? "500" : "400",
                          }}
                        >
                          {item.label}
                        </Link>
                      </li>
                    );
                  })}
                </ul>
              </div>
            ))}
          </nav>
        </div>
      )}
    </>
  );
}
