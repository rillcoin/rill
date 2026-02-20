# rill-mcp

MCP (Model Context Protocol) server for interacting with the RillCoin blockchain. Lets any MCP-compatible AI assistant — Claude Desktop, Cursor, etc. — create wallets, send RILL, explore blocks, and learn about concentration decay.

## Tools

| Tool | Description |
|------|-------------|
| `create_wallet` | Generate a new testnet wallet (mnemonic + address) |
| `derive_address` | Restore wallet from mnemonic |
| `check_balance` | Check address balance and UTXO count |
| `send_rill` | Send RILL from a mnemonic-derived wallet |
| `claim_faucet` | Claim free testnet RILL |
| `get_network_status` | Chain height, supply, decay pool, peers |
| `get_block` | Block details by height or hash |
| `get_transaction` | Transaction details by txid |
| `search` | Auto-detect and search for addresses/blocks/txs |
| `explain_decay` | Calculate and explain concentration decay |

## Quick Start

```bash
cd tools/rill-mcp
npm install
npm run build
```

## Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "rill": {
      "command": "node",
      "args": ["/absolute/path/to/rill/tools/rill-mcp/dist/index.js"]
    }
  }
}
```

## Cursor

Add to `.cursor/mcp.json` in your project root:

```json
{
  "mcpServers": {
    "rill": {
      "command": "node",
      "args": ["/absolute/path/to/rill/tools/rill-mcp/dist/index.js"]
    }
  }
}
```

## Local Development

```bash
# Run in dev mode (auto-reloads)
npm run dev

# Point to a local node instead of live testnet
RILL_FAUCET_URL=http://localhost:8080 RILL_EXPLORER_URL=http://localhost:8081 npm run dev

# Run tests
npm test

# Test with MCP Inspector
npx @modelcontextprotocol/inspector node dist/index.js
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RILL_FAUCET_URL` | `https://faucet.rillcoin.com` | Faucet API base URL |
| `RILL_EXPLORER_URL` | `https://explorer.rillcoin.com` | Explorer API base URL |

## AI Agent Prompts

Four specialized agent system prompts are included in `agents/`:

- **Decay Advisor** (`decay-advisor.md`) — Explains decay mechanics and calculates impact
- **Wallet Assistant** (`wallet-assistant.md`) — Guides wallet creation, restoration, and sending
- **Mining Helper** (`mining-helper.md`) — Mining setup, monitoring, and troubleshooting
- **Explorer Agent** (`explorer-agent.md`) — Natural language blockchain exploration

Use these as system prompts in Claude Desktop projects or custom agent configurations.

## Architecture

```
src/
  index.ts          → Server entry point (stdio transport)
  config.ts         → Environment configuration
  clients/
    faucet.ts       → HTTP client for faucet.rillcoin.com
    explorer.ts     → HTTP client for explorer.rillcoin.com
  tools/            → One file per MCP tool
  utils/
    formatting.ts   → rills↔RILL conversion
    decay.ts        → TypeScript BigInt port of sigmoid decay math
```

The server wraps existing HTTP APIs — no direct Rust calls needed. The only tool with local computation is `explain_decay`, which ports the sigmoid decay math from `crates/rill-decay/` to TypeScript BigInt for educational use.
