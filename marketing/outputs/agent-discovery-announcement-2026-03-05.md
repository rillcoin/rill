# Agent Discoverability Announcement — 2026-03-05

## Discord (#announcements)

**@everyone**

**RillCoin Is Now Discoverable by AI Agents Everywhere**

Two weeks ago we shipped the Agent Accessibility Bridge — REST API, MCP tools, SDKs. Agents could use RillCoin, but they had to know where to find us.

Today we are closing that gap. RillCoin is now discoverable across every major AI agent platform. If an agent can search, it can find us.

**What shipped:**

**npm + PyPI**
- `@rillcoin/mcp@0.1.1` published to npm — run `npx @rillcoin/mcp` and you have 16 MCP tools, zero config
- `rill-agent-sdk@0.1.0` published to PyPI — Python SDK with LangChain adapters, `pip install rill-agent-sdk`

**MCP Registry**
- Registered as `io.github.rillcoin/rill` on the official MCP Registry at registry.modelcontextprotocol.io
- Any MCP-aware client — Claude Desktop, Cursor, Windsurf, Claude Code — can now discover RillCoin automatically through the registry

**Agent Protocol Discovery**
- `/.well-known/agent.json` — standard Agent Protocol manifest, live on the faucet
- `/robots.txt` — AI-Agent directives so crawlers and agents know we exist and what we offer
- Both endpoints compiled into the binary and served from the faucet router

**DNS Discovery**
- `_agent.rillcoin.com` TXT record pointing to the discovery endpoint
- Agents that resolve DNS can find us without hitting any website

**GitHub Discoverability**
- 10 repo topics added: `mcp-server`, `ai-agent`, `blockchain`, `cryptocurrency`, `proof-of-conduct`, `concentration-decay`, `langchain`, `llm-tools`, `web3`, `rust`
- GitHub code search, topic search, and Copilot all surface RillCoin now

**Why this matters:**

The bridge gave agents access. Discoverability gives them awareness.

An AI agent searching npm for blockchain MCP servers finds us. An agent browsing the MCP Registry finds us. An agent resolving `_agent.rillcoin.com` finds us. An agent crawling `.well-known/agent.json` finds us. An agent searching GitHub topics finds us.

We are not waiting for agents to be told about RillCoin. We are making sure they can find us on their own.

**Links:**
npm: https://www.npmjs.com/package/@rillcoin/mcp
PyPI: https://pypi.org/project/rill-agent-sdk/
MCP Registry: https://registry.modelcontextprotocol.io
GitHub: https://github.com/rillcoin/rill
Faucet: https://faucet.rillcoin.com

*-- The RillCoin Core Team*

---

## X/Twitter Thread (6 tweets)

**Tweet 1:**
Two weeks ago we gave AI agents access to RillCoin.

Today we made sure they can find us on their own.

RillCoin is now discoverable across npm, PyPI, the MCP Registry, Agent Protocol, DNS, and GitHub. No human has to point the way.

**Tweet 2:**
Published to package registries:

@rillcoin/mcp@0.1.1 on npm — `npx @rillcoin/mcp` for 16 MCP tools
rill-agent-sdk@0.1.0 on PyPI — Python SDK + LangChain adapters

Search "blockchain" or "mcp-server" on either registry. We are there.

**Tweet 3:**
Registered on the official MCP Registry as io.github.rillcoin/rill.

Claude Desktop, Cursor, Windsurf, Claude Code — any MCP-aware client can now discover RillCoin automatically. No manual config. No URLs to copy. Just search and connect.

**Tweet 4:**
Standard discovery endpoints, live on the faucet:

/.well-known/agent.json — Agent Protocol manifest
/robots.txt — AI-Agent directives
_agent.rillcoin.com — DNS TXT record

Three different paths for autonomous agents to find us. Zero human involvement required.

**Tweet 5:**
10 GitHub repo topics added. mcp-server, ai-agent, blockchain, cryptocurrency, proof-of-conduct, concentration-decay, langchain, llm-tools, web3, rust.

GitHub search, Copilot, and topic browsing all surface RillCoin now.

**Tweet 6:**
The accessibility bridge gave agents the tools.
Discoverability gives them awareness.

npm. PyPI. MCP Registry. Agent Protocol. DNS. GitHub.

Every door is open. Agents just have to look.

https://github.com/rillcoin/rill

#RillCoin #MCP #AIAgents #Web3
