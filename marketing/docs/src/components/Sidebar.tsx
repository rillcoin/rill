"use client";

import Link from "next/link";
import Image from "next/image";
import { usePathname } from "next/navigation";

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

export default function Sidebar() {
  const pathname = usePathname();

  const isActive = (href: string) => {
    if (href === "/") return pathname === "/" || pathname === "";
    return pathname === href || pathname === `${href}/`;
  };

  return (
    <aside
      className="hidden lg:flex flex-col w-64 xl:w-72 shrink-0 sticky top-0 h-screen overflow-y-auto scrollbar-thin"
      style={{ borderRight: "1px solid var(--border-subtle)" }}
    >
      {/* Logo */}
      <div
        className="flex items-center gap-3 px-6 py-5"
        style={{ borderBottom: "1px solid var(--border-subtle)" }}
      >
        <div
          className="w-8 h-8 rounded-lg flex items-center justify-center shrink-0"
          style={{
            background: "linear-gradient(135deg, #4A8AF4 0%, #22D3EE 100%)",
          }}
        >
          <Image
            src="/rill_logo.png"
            alt="RillCoin"
            width={20}
            height={20}
            className="w-5 h-5 object-contain"
          />
        </div>
        <div>
          <Link
            href="/"
            className="text-sm font-semibold"
            style={{ color: "var(--text-primary)" }}
          >
            RillCoin Docs
          </Link>
          <div className="flex items-center gap-1.5 mt-0.5">
            <span
              className="w-1.5 h-1.5 rounded-full inline-block"
              style={{ background: "var(--cyan-400)" }}
            />
            <span className="text-xs" style={{ color: "var(--cyan-400)" }}>
              Testnet Live
            </span>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-4 py-5">
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
                      className="flex items-center gap-2 px-2 py-1.5 rounded-md text-sm transition-all duration-150"
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
                      {active && (
                        <span
                          className="w-1 h-1 rounded-full shrink-0"
                          style={{ background: "var(--cyan-400)" }}
                        />
                      )}
                      {!active && <span className="w-1 h-1 shrink-0" />}
                      {item.label}
                    </Link>
                  </li>
                );
              })}
            </ul>
          </div>
        ))}
      </nav>

      {/* Footer */}
      <div
        className="px-6 py-4"
        style={{ borderTop: "1px solid var(--border-subtle)" }}
      >
        <a
          href="https://rillcoin.com"
          target="_blank"
          rel="noopener noreferrer"
          className="text-xs transition-colors"
          style={{ color: "var(--text-muted)" }}
        >
          rillcoin.com &rarr;
        </a>
      </div>
    </aside>
  );
}
