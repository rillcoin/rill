# Rill — Session Memory
*Last updated: 2026-02-19*

## Recent Sessions

### 2026-02-19 — Infra Recovery + Testnet Chain Reset
Resolved stale process conflicts, deployed mempool-fix binaries, reset testnet chain to fresh genesis.

**Key commits:** `a2499c8` (mempool+maturity fix), current session (changelog)

## Active Context
- **Current Phase:** Testnet live. Chain reset to fresh genesis. Faucet needs funding.
- **Server:** DigitalOcean droplet `206.189.202.181` (rill-node0, tag: rill-testnet)
- **DNS provider:** IONOS (rillcoin.com domain)

## Testnet Chain State (as of 2026-02-19 19:34 UTC)
- Height: 72 (fresh genesis, reset due to overmined/too-hard chain)
- Mining address: `trill1qfz8v6clahtl40c738vm8mskt27ka29kvyv60wzressllfs99nfls3grgem`
- Miner wallet: `/root/.rill/miner-new.dat`, password: see credentials.md
- Faucet wallet: `/var/lib/rill/faucet.dat`, balance: 0 (needs funding at height ≥ 101)
- Faucet address: `trill1qnad7yk3l93nd35ddgs0ev5pq85n85qrzyls5zcahx29uxf7w9saq7jzz85`

## Pending Action: Fund Faucet
Once chain height ≥ 101 (COINBASE_MATURITY = 100 blocks), run:
```
python3 -c "
import pty, os, sys
pid = os.fork()
if pid == 0:
    os.execvp('/usr/local/bin/rill-cli', ['/usr/local/bin/rill-cli', '--testnet', 'wallet', 'send',
        '--wallet', '/root/.rill/miner-new.dat',
        '--to', 'trill1qnad7yk3l93nd35ddgs0ev5pq85n85qrzyls5zcahx29uxf7w9saq7jzz85',
        '--amount', '5000'])
else:
    # feed password RillMiner2026xK9qP when prompted
    pass
"
```
(Or use the Python pty approach from previous sessions to supply password non-interactively)

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
- X/Twitter: https://x.com/RillCoin

## Discord Bot
- App ID: 1473971397381456004
- Token: in `/etc/rill/faucet.env` on node0
- Guild ID: 1473262369546174631
- #announcements channel ID: 1473269522092789894
- Role: RillBot (Send Messages, Embed Links, Manage Messages, Mention Everyone)
- IMPORTANT: Must POST via node0 server (local IP is Cloudflare-blocked)
- IMPORTANT: Must include `User-Agent: DiscordBot (https://rillcoin.com, 1.0)` header

## Crate Status (dev workspace — separate from marketing)
- rill-core: complete
- rill-decay: complete
- rill-consensus: complete
- rill-network: complete
- rill-wallet: complete
- rill-node: complete (RPC, storage, sync, mempool)
- rill-faucet: complete (deployed to testnet)
