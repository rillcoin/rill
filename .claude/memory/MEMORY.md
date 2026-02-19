# Rill — Session Memory
*Last updated: 2026-02-19*

## Recent Sessions

### 2026-02-19 — Full Marketing Web Presence
Built and deployed all four rillcoin.com properties from scratch. All HTTPS, all live.

**Key commits:** `16b44c6`, `9f18e8a`, `a2e7c9f`

## Active Context
- **Current Phase:** Testnet live. Marketing web presence complete.
- **Server:** DigitalOcean droplet `206.189.202.181` (rill-node0, tag: rill-testnet)
- **DNS provider:** IONOS (rillcoin.com domain)

## Live Sites
| Site | URL | Nginx root | Cert |
|---|---|---|---|
| Landing | https://rillcoin.com | /var/www/rillcoin | ✓ auto-renews |
| Faucet | https://faucet.rillcoin.com | /var/www/rill-faucet | ✓ auto-renews |
| Explorer | https://explorer.rillcoin.com | /var/www/rill-explorer | ✓ auto-renews |
| Docs | https://docs.rillcoin.com | /var/www/rill-docs | ✓ auto-renews |

## Marketing Source Layout
```
marketing/
├── website/   — rillcoin.com (Next.js 14)
├── faucet/    — faucet.rillcoin.com (Next.js 14)
├── explorer/  — explorer.rillcoin.com (Next.js 14)
└── docs/      — docs.rillcoin.com (Next.js 14, 8 pages)
```

## Infrastructure Details
- All sites: Next.js 14 `output: "export"`, static HTML rsync'd to server
- Faucet backend: Rust Axum at port 8080, proxied via nginx `/api/` and `/discord/`
- Node RPC: `127.0.0.1:18332` (testnet: `28332`), proxied via nginx `/rpc` on explorer
- HTTPS: Let's Encrypt via certbot, scheduled auto-renewal on all four domains

## Design System (shared across all sites)
- Background: `--void: #020408`
- Blue: `--blue-500: #3B82F6`, Cyan: `--cyan-400: #22D3EE`, Orange: `--orange-500: #F97316`
- Fonts: Instrument Serif (headings), Inter (body), JetBrains Mono (code)

## Key Decisions
See `.claude/skills/architecture/SKILL.md` for ADR log.

## Social / Community Links
- Discord: https://discord.com/invite/F3dRVaP8
- GitHub: https://github.com/rillcoin/rill
- X/Twitter: https://x.com/rillcoin (placeholder — confirm handle)

## Crate Status (dev workspace — separate from marketing)
- rill-core: complete
- rill-decay: complete
- rill-consensus: complete
- rill-network: complete
- rill-wallet: complete
- rill-node: complete (RPC, storage, sync, mempool)
- rill-faucet: complete (deployed to testnet)
