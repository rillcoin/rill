# MCP Server Announcement — 2026-02-21

## Discord (#announcements)

**@everyone**

**AI Agents Can Now Interact With RillCoin**

We just shipped an MCP (Model Context Protocol) server that lets any AI assistant — Claude Desktop, Cursor, Claude Code — interact with the RillCoin blockchain directly.

**10 tools, zero setup friction:**
- `create_wallet` / `derive_address` — generate or restore wallets
- `check_balance` / `send_rill` — check funds, send payments
- `claim_faucet` — grab free testnet RILL
- `get_network_status` / `get_block` / `get_transaction` — explore the chain
- `search` — natural language blockchain search
- `explain_decay` — calculate concentration decay impact on any balance

**4 AI agent prompts included:**
- Decay Advisor — understand how decay affects your holdings
- Wallet Assistant — guided wallet creation with security best practices
- Mining Helper — setup guides and troubleshooting
- Explorer Agent — ask questions about the blockchain in plain English

The decay calculator is ported from the Rust engine with BigInt sigmoid math, cross-validated against our test vectors.

**Get started:** Add to your Claude Desktop config and start asking questions about RillCoin.
Docs: https://docs.rillcoin.com
GitHub: https://github.com/rillcoin/rill/tree/main/tools/rill-mcp

*— The RillCoin Core Team*

---

## X/Twitter Thread (7 tweets)

**Tweet 1:**
AI agents can now interact with RillCoin natively.

We just shipped an MCP server — 10 tools that let Claude, Cursor, or any MCP-compatible assistant create wallets, send RILL, explore blocks, and calculate concentration decay.

Thread

**Tweet 2:**
What's MCP? Model Context Protocol — an open standard that lets AI assistants use external tools.

Instead of copy-pasting RPC commands, just ask your AI: "What's my balance?" or "How much would 50,000 RILL decay over 6 months?"

**Tweet 3:**
The 10 tools:

create_wallet — generate new testnet wallet
check_balance — address balance + UTXOs
send_rill — send payments
claim_faucet — free testnet RILL
get_network_status — chain height, supply, decay pool
get_block / get_transaction / search
explain_decay — sigmoid decay calculator

**Tweet 4:**
The decay calculator is the interesting one.

It's a full BigInt sigmoid implementation ported from our Rust engine. Cross-validated against the same test vectors. Ask it "what happens to 100,000 RILL over a year?" and it walks you through the math.

**Tweet 5:**
We also ship 4 agent prompts:

- Decay Advisor — holding strategy education
- Wallet Assistant — guided wallet lifecycle
- Mining Helper — setup and troubleshooting
- Explorer Agent — natural language blockchain queries

Drop them into Claude Desktop and start exploring.

**Tweet 6:**
Oh, and the repo is now public.

6 library crates, 3 binaries, 920+ tests, integer-only consensus math, Ed25519, BLAKE3 Merkle trees, libp2p networking.

https://github.com/rillcoin/rill

**Tweet 7:**
Try it:
1. Grab the MCP server from GitHub
2. Add to Claude Desktop config
3. Ask: "Create me a testnet wallet and claim some RILL"

Testnet is live. Faucet is live. Wallet is live.

https://rillcoin.com/wallet
https://explorer.rillcoin.com

#RillCoin #MCP #ConcentrationDecay
