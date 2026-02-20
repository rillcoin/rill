"use client";

const MCP_CONFIG = `{
  "mcpServers": {
    "rill": {
      "command": "node",
      "args": ["path/to/rill/tools/rill-mcp/dist/index.js"]
    }
  }
}`;

const TOOLS = [
  { name: "create_wallet", desc: "Generate wallet", icon: "+" },
  { name: "check_balance", desc: "Query balance", icon: "◎" },
  { name: "send_rill", desc: "Send RILL", icon: "→" },
  { name: "claim_faucet", desc: "Claim testnet RILL", icon: "⬡" },
  { name: "get_block", desc: "Inspect blocks", icon: "▣" },
  { name: "get_transaction", desc: "Transaction details", icon: "⬢" },
  { name: "get_network_status", desc: "Network stats", icon: "◈" },
  { name: "search", desc: "Search blockchain", icon: "⌕" },
  { name: "explain_decay", desc: "Decay calculator", icon: "∿" },
  { name: "derive_address", desc: "Restore wallet", icon: "⟲" },
];

const AGENTS = [
  {
    name: "Decay Advisor",
    desc: "Calculates decay impact, explains the sigmoid curve, and suggests strategies to manage holdings.",
    tools: "explain_decay, check_balance, get_network_status",
  },
  {
    name: "Wallet Assistant",
    desc: "Guides you through creating, restoring, and sending from wallets with security best practices.",
    tools: "create_wallet, derive_address, send_rill, claim_faucet",
  },
  {
    name: "Mining Helper",
    desc: "Setup guides, reward monitoring, and troubleshooting for RillCoin miners.",
    tools: "get_network_status, get_block, check_balance",
  },
  {
    name: "Explorer Agent",
    desc: "Natural language blockchain queries — ask about any block, transaction, or address.",
    tools: "search, get_block, get_transaction, check_balance",
  },
];

export default function McpSection() {
  return (
    <section
      id="mcp"
      className="flex flex-col gap-16 px-5 lg:px-20 py-24"
      style={{ backgroundColor: "var(--void)" }}
    >
      {/* Section header */}
      <div className="flex flex-col items-center gap-4 text-center">
        <span
          className="font-mono font-semibold text-[11px] tracking-[3px]"
          style={{ color: "rgba(34,211,238,0.314)" }}
        >
          AI-NATIVE
        </span>
        <h2
          className="font-serif text-[40px] lg:text-[52px] leading-none"
          style={{ color: "var(--text-primary)" }}
        >
          Talk to the blockchain.
        </h2>
        <p
          className="font-sans text-[16px] lg:text-[18px] leading-relaxed max-w-[600px]"
          style={{ color: "var(--text-secondary)" }}
        >
          The RillCoin MCP server lets any AI assistant interact with the
          network directly. Create wallets, send RILL, explore blocks, and
          learn about decay — all through natural language.
        </p>
      </div>

      {/* Two-column: config + tools grid */}
      <div className="flex flex-col lg:flex-row gap-8">
        {/* Config panel */}
        <div
          className="flex flex-col gap-4 rounded-xl overflow-hidden flex-1"
          style={{
            backgroundColor: "#05080F",
            border: "1px solid var(--border-subtle)",
          }}
        >
          {/* Title bar */}
          <div
            className="flex items-center justify-between h-10 px-4"
            style={{
              backgroundColor: "var(--base)",
              borderBottom: "1px solid var(--border-subtle)",
            }}
          >
            <div className="flex items-center gap-2">
              <span
                className="w-2.5 h-2.5 rounded-full"
                style={{ backgroundColor: "rgba(239,68,68,0.376)" }}
              />
              <span
                className="w-2.5 h-2.5 rounded-full"
                style={{ backgroundColor: "rgba(245,158,11,0.376)" }}
              />
              <span
                className="w-2.5 h-2.5 rounded-full"
                style={{ backgroundColor: "rgba(16,185,129,0.376)" }}
              />
            </div>
            <span
              className="font-mono text-[12px]"
              style={{ color: "rgba(148,163,184,0.376)" }}
            >
              claude_desktop_config.json
            </span>
          </div>
          <pre
            className="font-mono text-[13px] leading-[1.75] px-6 pb-6 overflow-x-auto"
            style={{ color: "#94A3B8" }}
          >
            {MCP_CONFIG.split("\n").map((line, i) => {
              if (line.includes('"rill"') || line.includes('"mcpServers"')) {
                return (
                  <span key={i}>
                    {line.split(/("rill"|"mcpServers")/).map((part, j) =>
                      part === '"rill"' || part === '"mcpServers"' ? (
                        <span key={j} style={{ color: "var(--cyan-400)" }}>
                          {part}
                        </span>
                      ) : (
                        <span key={j}>{part}</span>
                      )
                    )}
                    {"\n"}
                  </span>
                );
              }
              if (line.includes('"command"') || line.includes('"args"')) {
                return (
                  <span key={i}>
                    {line.split(/("command"|"args")/).map((part, j) =>
                      part === '"command"' || part === '"args"' ? (
                        <span key={j} style={{ color: "var(--blue-400)" }}>
                          {part}
                        </span>
                      ) : (
                        <span key={j}>{part}</span>
                      )
                    )}
                    {"\n"}
                  </span>
                );
              }
              return <span key={i}>{line}{"\n"}</span>;
            })}
          </pre>
          {/* Setup link */}
          <div
            className="px-6 pb-5"
          >
            <a
              href="https://github.com/rillcoin/rill/tree/main/tools/rill-mcp"
              target="_blank"
              rel="noopener noreferrer"
              className="font-mono text-[13px] transition-opacity hover:opacity-80"
              style={{ color: "var(--blue-500)" }}
            >
              Setup guide →
            </a>
          </div>
        </div>

        {/* Tools grid */}
        <div className="flex-1 flex flex-col gap-4">
          <span
            className="font-mono font-semibold text-[10px] tracking-[2px]"
            style={{ color: "var(--text-faint)" }}
          >
            10 MCP TOOLS
          </span>
          <div className="grid grid-cols-2 gap-3">
            {TOOLS.map((tool) => (
              <div
                key={tool.name}
                className="flex items-center gap-3 rounded-lg px-4 py-3"
                style={{
                  backgroundColor: "var(--raised)",
                  border: "1px solid var(--border-subtle)",
                }}
              >
                <span
                  className="font-mono text-[16px] flex-shrink-0"
                  style={{ color: "var(--blue-400)", width: 20, textAlign: "center" }}
                >
                  {tool.icon}
                </span>
                <div className="flex flex-col gap-0.5 min-w-0">
                  <span
                    className="font-mono text-[12px] truncate"
                    style={{ color: "var(--text-primary)" }}
                  >
                    {tool.name}
                  </span>
                  <span
                    className="font-sans text-[11px] truncate"
                    style={{ color: "var(--text-dim)" }}
                  >
                    {tool.desc}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* AI Agents row */}
      <div className="flex flex-col gap-6">
        <div className="flex flex-col gap-2">
          <span
            className="font-mono font-semibold text-[10px] tracking-[2px]"
            style={{ color: "var(--text-faint)" }}
          >
            AI AGENTS
          </span>
          <p
            className="font-sans text-[15px]"
            style={{ color: "var(--text-muted)" }}
          >
            Specialized system prompts for Claude Desktop projects and custom
            agent configurations.
          </p>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {AGENTS.map((agent) => (
            <div
              key={agent.name}
              className="flex flex-col gap-3 rounded-xl p-6"
              style={{
                backgroundColor: "var(--raised)",
                border: "1px solid var(--border-subtle)",
              }}
            >
              <span
                className="font-sans font-semibold text-[15px]"
                style={{ color: "var(--text-primary)" }}
              >
                {agent.name}
              </span>
              <p
                className="font-sans text-[13px] leading-[1.55]"
                style={{ color: "var(--text-dim)" }}
              >
                {agent.desc}
              </p>
              <span
                className="font-mono text-[10px] mt-auto"
                style={{ color: "var(--text-faint)" }}
              >
                {agent.tools}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Works-with badges */}
      <div className="flex flex-col items-center gap-4">
        <span
          className="font-mono text-[10px] tracking-[2px]"
          style={{ color: "var(--text-faint)" }}
        >
          WORKS WITH
        </span>
        <div className="flex items-center gap-6">
          {["Claude Desktop", "Cursor", "Claude Code", "Any MCP Client"].map(
            (name) => (
              <span
                key={name}
                className="font-sans text-[13px] px-3 py-1.5 rounded-full"
                style={{
                  color: "var(--text-dim)",
                  backgroundColor: "#060E1C",
                  border: "1px solid #1B3A6B",
                }}
              >
                {name}
              </span>
            )
          )}
        </div>
      </div>
    </section>
  );
}
