# Changelog

## [Unreleased]

### 2026-02-21 - MCP Server & AI Agents

**AI assistants can now interact with RillCoin directly via the Model Context Protocol.**

#### MCP Server (`tools/rill-mcp/`)
- TypeScript MCP server using `@modelcontextprotocol/sdk` with stdio transport
- 10 tools: `create_wallet`, `derive_address`, `check_balance`, `send_rill`, `claim_faucet`, `get_network_status`, `get_block`, `get_transaction`, `search`, `explain_decay`
- HTTP clients wrapping `faucet.rillcoin.com` and `explorer.rillcoin.com` APIs
- BigInt sigmoid decay calculator ported from `rill-decay` engine, cross-validated against Rust test vectors
- 49 tests (formatting, decay math cross-validation, tool handlers with mocked clients)
- Works with Claude Desktop, Cursor, Claude Code, and any MCP-compatible client

#### AI Agent Prompts (`tools/rill-mcp/agents/`)
- **Decay Advisor** — decay mechanics education, impact calculation, holding strategies
- **Wallet Assistant** — wallet lifecycle guidance with security best practices
- **Mining Helper** — setup guides, reward monitoring, troubleshooting
- **Explorer Agent** — natural language blockchain exploration

#### Website
- New `McpSection` component on rillcoin.com homepage (between CLI and CTA sections)
- Shows Claude Desktop config snippet, 10-tool grid, 4 agent cards, "Works with" badges
- MCP Server link added to footer DEVELOPERS column
- Deployed to node0

#### Infrastructure
- Fixed `deploy-landing.sh` certbot detection bug — `grep -c` over SSH was returning filename-prefixed output, causing the integer comparison to fail and overwrite the SSL config
- Re-ran certbot to restore HTTPS after accidental overwrite

### 2026-02-20 - Web Wallet for Testnet

**Browser-based wallet at rillcoin.com/wallet — create wallets, claim faucet, send payments without installing anything.**

#### Backend (rill-faucet)
- `GET /api/wallet/balance?address=trill1...` — address balance + UTXO count lookup
- `POST /api/wallet/send` — send RILL from mnemonic-derived ephemeral wallet with rate limiting
- `POST /api/wallet/derive` — derive address from mnemonic (restore flow)
- `send_from_mnemonic()` — ephemeral keychain with gap-based address scanning, decay-aware coin selection
- Extracted `parse_utxo_json()` shared helper to reduce duplication in `send.rs`
- `fetch_balance_for_address()` — single-address balance + UTXO count query

#### Frontend (rillcoin.com/wallet)
- New `/wallet` page with hero section (radial glow, badge, gradient heading) matching site design
- Create wallet / Restore from mnemonic flow with localStorage persistence
- Balance card with 15s auto-refresh, gradient text, and faucet request button
- Send form with address validation, amount input, and transaction result display
- Mnemonic reveal/hide with numbered grid and copy functionality
- Disconnect button to clear wallet from browser
- Testnet warning banner (orange, prominent)
- Mobile-responsive, matching existing design system (Instrument Serif, Inter, JetBrains Mono)

#### Site-wide link updates
- All CTAs (Hero, Nav, CtaSection, Footer) now point to `/wallet` instead of `faucet.rillcoin.com`
- "Wallet" link added to Nav center links
- Button copy updated: "Get Testnet RILL" → "Try the Wallet"

#### Deployed
- Website deployed to node0 via `deploy-landing.sh` — `/wallet` route live on rillcoin.com
- Faucet deployed to node0 via `deploy-faucet.sh` — 3 new endpoints active on faucet.rillcoin.com
- Smoke tested: `/api/wallet/new` and `/api/wallet/derive` confirmed working

#### Infrastructure fixes
- Fixed nginx `try_files` for static Next.js subroutes (`$uri/index.html` was missing)
- Enabled HTTPS for `rillcoin.com` and `www.rillcoin.com` via certbot
- Separated wallet send rate limiter (30s) from faucet claim rate limiter (24h) — faucet claim was blocking wallet sends from the same IP

**Commits:** `5806303` (backend + frontend), `3f694e3` (CTA links), `d019cfd` (design polish), `88a8f0d` (nginx + HTTPS), `4e69642` (rate limiter fix)

### 2026-02-19 - ci: GitHub Actions CI/CD + Faucet Deploy Fix + Discord Roles

**Automated build pipeline, fixed faucet crash, created community roles.**

#### CI/CD Pipeline
- Added `.github/workflows/build-linux.yml` — builds rill-node, rill-miner, rill-faucet, rill-cli for x86_64-unknown-linux-gnu
- Runner: `ubuntu-latest` (4-core), cargo cache keyed on Cargo.lock
- Auto-deploys to node0 on push to main via SSH (secrets: NODE0_SSH_KEY, NODE0_HOST)
- Smoke tests all binaries before deploy; restarts rill-node + rill-faucet (skips miner)
- Artifacts uploaded with 14-day retention

#### Faucet Fix
- Diagnosed rill-faucet systemd 203/EXEC crash loop — binary was macOS arm64 Mach-O, not Linux ELF
- Built rill-faucet from source on node0 (8m 41s), deployed correct ELF binary
- Wallet creation feature live: `GET /api/wallet/new` (BIP-39 mnemonic + testnet address), `/create-wallet` page
- Auto-fund script (`/root/fund_faucet.sh`) running — sends 5000 RILL when height ≥ 101

#### Discord Roles Created
- Testnet Pioneer: `#22D3EE` (cyan), hoisted, mentionable — ID: 1474179312780447754
- Bug Hunter: `#F97316` (orange), hoisted, mentionable — ID: 1474179315137773628
- RillBot granted Manage Roles permission

**Commits:** `60f3b36` (wallet page), `fe04ed0` (CI workflow), `f0169de` (runner fix), `8e261ec` (smoke test fix)

### 2026-02-19 - infra: Full Service Restart + Testnet Chain Reset

**Resolved stale-process conflicts, deployed new binaries, and reset testnet to clean genesis.**

#### What was done
- Stopped the conflicting stale rill-node (PID 31094, old binary, wrong data dir)
- Migrated testnet chain data from `/root/.local/share/rill/testnet/` → `/var/lib/rill/testnet/`
- Updated `/etc/systemd/system/rill-node.service` to add `--testnet --data-dir /var/lib/rill/testnet`
- Updated `/etc/systemd/system/rill-miner.service` to use `Wants=` instead of `Requires=` (prevents miner dying when node restarts)
- Killed old stale miner (PID 31939) running for 16h with wrong mining address
- Installed new binaries: `rill-node` (16.7 MB), `rill-miner` (5.7 MB) from Feb 19 build

#### Testnet chain reset
- Chain at height 452 had difficulty 100x too hard (~220 min/block) due to aggressive early overmining
- All 452 blocks were mined by the stale miner using an unknown address — no recoverable rewards
- Reset chain: `rm -rf /var/lib/rill/testnet/chaindata && systemctl start rill-node rill-miner`
- Fresh genesis at block 0, miner using correct address `trill1qfz8v...nfls3grgem`
- Chain now at height 72, all block rewards going to miner wallet

#### Services status (post-reset)
- `rill-node` — active, testnet, height 72, genesis hash confirmed fresh
- `rill-miner` — active at 57K H/s, target adjusting, ~5 min/block at height 73+
- `rill-faucet` — active on 8080, balance 0 (waiting for coinbase maturity at block 101)
- `rill-explorer` — active on 8081

#### Next: Fund faucet
- Need height ≥ 101 for block-1 coinbase to mature (COINBASE_MATURITY = 100)
- At ~5-10 min/block, maturity expected within ~4 hours
- Then: `rill-cli --testnet send --wallet /root/.rill/miner-new.dat --to <faucet_addr> --amount 5000`

### 2026-02-19 - infra: Mempool Fix Deployed — Services Restored

**Rebuilt rill-node and rill-miner on node0 with the mempool inclusion fix. All four services running.**

#### Bug fixes deployed
- `rill-node` + `rill-miner` rebuilt from commit `a2499c8` (mempool + coinbase maturity fix)
- Binaries installed: `/usr/local/bin/rill-node` (16.7 MB), `/usr/local/bin/rill-miner` (5.7 MB)
- Timestamp: Feb 19 19:09 UTC (verified new build, not cached Feb 18 binaries)
- Build took ~2.5 hours on node0 due to full RocksDB C++ recompilation with `lto = "fat"`

#### Services status (post-restart)
- `rill-node` — active, height 452, chain synced, RPC on 18332
- `rill-miner` — active, mining at height 453, ~20k H/s
- `rill-faucet` — active, balance 0 (funding tx `c26927407a...` in mempool)
- `rill-explorer` — active on port 8081

#### Faucet re-funded
- Mempool cleared on node restart (expected — mempool is in-memory only)
- Re-sent 5000 RILL from miner wallet to faucet: txid `c26927407a51a5d126b3c24afa32bcd55cfb9bd612966671ae28e212ac0f8c38`
- Tx confirmed in mempool (size=1, bytes=13317, fee=51500 rills)

#### Explorer nginx fix
- Added `/api/` proxy block to `nginx-explorer.conf` → port 8081 (Axum backend)
- `https://explorer.rillcoin.com/api/stats` now returns JSON correctly (was serving Next.js HTML)
- Updated `infra/nginx-explorer.conf` to match deployed config

#### Known issue: difficulty adjustment
- Chain mined ~150 blocks rapidly before node restart, LWMA cranked difficulty up
- New block template: `difficulty_target=99951061891` (~100B vs ~6.4T at block 450)
- At ~20k H/s, first block will take ~2.5 hours; LWMA self-corrects with 3x clamp per block
- Difficulty will recover to normal (~2 min blocks) after ~5-6 blocks (~4 hours total)

**Commits:** this session

### 2026-02-19 - marketing: Community Launch — Discord + X Thread

**First public announcement posted across Discord and X.**

#### Discord
- Bot (RillCoin Faucet → renamed to RillCoin) invited to server, `RillBot` role created with Send Messages, Embed Links, Manage Messages, Mention Everyone permissions
- Announcement posted to #announcements in two parts with `@everyone` mention and `— *The RillCoin Core Team*` signature
- Announcement pinned in #announcements
- Bot now posts via DigitalOcean server (local IP was Cloudflare-blocked); User-Agent header required

#### X / Twitter
- Drafted and trimmed 8-tweet thread covering: testnet launch, four live sites, tech stack, concentration decay, open source, early adopter incentives, roadmap, CTA
- Saved to `marketing/outputs/twitter-launch-thread-2026-02-19.md`

#### Links updated
- Discord: `discord.gg/rillcoin` → `discord.com/invite/F3dRVaP8` (Footer.tsx)
- X/Twitter: `x.com/rillcoin` → `x.com/RillCoin` (Footer.tsx)

#### Early adopter programme defined
- **Testnet Pioneer** — claim from faucet or mine a block before mainnet
- **Bug Hunter** — confirmed bug report earns permanent role + changelog credit

**Commits:** `a2e7c9f` (Discord + HTTPS), `1cdc2c6` (X link)

---

### 2026-02-19 - marketing: Full Web Presence Live — Four Sites, All HTTPS

**All four rillcoin.com properties are live with HTTPS and auto-renewing Let's Encrypt certs.**

#### rillcoin.com (main landing site)
- Built from Pencil design — Web 4.0 dark aesthetic with animated orb, bento grid, decay ring, CLI section, CTA, footer
- Next.js 14 App Router, `output: "export"`, Tailwind CSS v3
- Fonts: Instrument Serif (headings), Inter (body), JetBrains Mono (code)
- Deployed to `/var/www/rillcoin` via rsync; HTTPS via certbot

#### faucet.rillcoin.com
- Branded Next.js UI matching rillcoin.com design system
- Connects to Rust faucet backend at port 8080 via nginx proxy
- Address validation (`trill1...` prefix), rate limit feedback, network status bar
- HTTPS cert issued this session (was missing previously)

#### explorer.rillcoin.com
- Block explorer with stats bar (height, supply, decay pool, peers)
- Latest blocks table, search by height/hash/txid/address
- Detail pages: `/block?hash=`, `/tx?id=`, `/address?addr=` (query params for static export)
- nginx proxies `/rpc` → node JSON-RPC at `127.0.0.1:18332` with CORS headers

#### docs.rillcoin.com (new this session)
- 8-page documentation site built from Rust source content
- Pages: Home, Whitepaper, Protocol, Decay Mechanics, Mining, CLI Reference, RPC Reference, Node Setup
- Sidebar navigation with active page highlighting, mobile hamburger nav
- Deployed to `/var/www/rill-docs`; DNS A record added via IONOS; HTTPS cert issued

#### Sitewide
- HTTP → HTTPS 301 redirect enforced on all four vhosts (verified curl checks)
- Discord link updated: `discord.gg/rillcoin` → `discord.com/invite/F3dRVaP8`
- All social/docs links pointing to real URLs (no `#` placeholders)

**Commits:** `16b44c6` (initial sites), `9f18e8a` (docs), `a2e7c9f` (Discord + HTTPS)

---

### 2026-02-19 - infra: Faucet Deployed to Testnet node0

**Faucet live at `http://206.189.202.181` (DNS `faucet.rillcoin.org` pending propagation)**

- Built `rill-faucet` release binary on node0 (`206.189.202.181`)
- Installed systemd unit (`rill-faucet.service`) — running as `rill` user with hardening
- nginx reverse proxy configured (port 80 → 127.0.0.1:8080)
- DigitalOcean cloud firewall opened for TCP 80 + 443
- Faucet wallet created at `/var/lib/rill/faucet.dat`
- Discord bot token, public key, and application ID added to `/etc/rill/faucet.env`
- `/faucet` slash command registered with Discord at startup
- New miner wallet created (`/root/.rill/miner-new.dat`) — old wallet password lost
- Miner service updated to mine to new accessible wallet address
- Credentials saved to `.claude/memory/credentials.md` (gitignored)

**Pending:**
- Fund faucet wallet (needs coinbase maturity on new miner wallet, ~few blocks)
- DNS propagation for `faucet.rillcoin.org` + certbot HTTPS

### 2026-02-19 - rill-faucet: Testnet Faucet Binary

**New binary: `bins/rill-faucet`**

Built a standalone faucet binary for the live testnet. Community members can claim 10 RILL per address every 24 hours without mining.

**Features:**
- Axum HTTP server (web UI + REST API)
- Embedded single-page web UI at `GET /` — dark theme, #FF8C00 brand colours, mobile-friendly, session-storage for recent claims
- `POST /api/faucet {"address":"trill1..."}` — dispenses RILL, returns txid
- `GET /api/status` — node height, faucet balance, network name
- `POST /discord/interactions` — Discord slash command handler (`/faucet address:<trill1...>`)
- Ed25519 Discord signature verification via `ed25519-dalek`
- Discord slash command registration at startup via `reqwest` REST call
- Per-address + per-IP rate limiting (`RateLimiter`, in-memory `HashMap<_, Instant>`)
- Decay-aware coin selection (reuses `CoinSelector` from `rill-wallet`)
- Wallet mutex held across RPC calls to prevent UTXO double-spend under concurrency
- CORS enabled via `tower-http`

**Configuration (env vars):**
- `FAUCET_WALLET_PATH`, `FAUCET_WALLET_PASSWORD` (required), `FAUCET_RPC_ENDPOINT` (default: `http://127.0.0.1:28332`)
- `FAUCET_BIND_ADDR` (default: `0.0.0.0:8080`), `FAUCET_AMOUNT_RILL` (default: 10), `FAUCET_COOLDOWN_SECS` (default: 86400)
- `DISCORD_BOT_TOKEN`, `DISCORD_PUBLIC_KEY`, `DISCORD_APPLICATION_ID` (all optional)

**Workspace changes:**
- Added `axum = "0.7"`, `tower = "0.4"`, `tower-http = "0.5"` (cors), `reqwest = "0.12"` to workspace deps
- Added `bins/rill-faucet` to workspace members

**Verification:** `cargo build --workspace`, `cargo clippy -p rill-faucet -- -D warnings`, `cargo test --workspace` — all clean, 0 failures.

---

### 2026-02-18 - Marketing: HyperLiquid Launch Planning & Discord Bot Strategy

**HyperLiquid Launch Mechanics:**
- Documented HIP-1 (spot token registration) and HIP-2 (Hyperliquidity seeding) process
- Corrected tokenomics model to match protocol constants: 21,000,000 RILL (not 1B)
- Dev fund premine: 1,050,000 RILL (5%) minted at genesis — effective total ~22,050,000 RILL
- Modeled HIP-2 seeding scenarios: 5% of supply (~1.1M RILL) paired against USDC to set opening price band
- Illustrated FDV range ($4.4M–$22M) depending on USDC seed depth

**Funding Strategy:**
- Outlined seed round path: sell investor allocation via SAFT pre-launch
- Identified target VCs: Multicoin Capital, Delphi Ventures, Robot Ventures, Dragonfly
- Recommended HyperLiquid Ecosystem Fund grant application as first step
- KOL/angel round proposed for distribution value alongside capital

**Discord Bot Planning:**
- Recommended stack: MEE6 (moderation) + Collab.Land (token gating) + custom RILL bot
- Custom bot spec: `/price`, `/supply`, `/decay`, `/faucet` commands via HyperLiquid public API
- Noted risks of auto-engagement bots (ToS violations, credibility damage, regulatory exposure)

**Milestone:**
- First-ever RillCoin transaction confirmed on testnet — celebrated.

### 2026-02-18 - Testnet Redeployment & Miner Timestamp Fix

**Timestamp Fix:**
- Diagnosed miner stuck loop: `create_block_template` could produce timestamps equal to the parent's when blocks mined faster than 1 per second, causing `TimestampNotAfterParent` validation rejection
- Fixed `create_block_template` to ensure `timestamp >= parent_timestamp + 1`
- Miner no longer enters tight retry loops on fast-mining chains

**Fresh Testnet Deployment:**
- Wiped chain data (correct path: `~/.local/share/rill/testnet/chaindata`)
- Rebuilt and restarted both nodes with timestamp fix
- Chain producing blocks: 72 blocks mined in ~20s (initial convergence), block 73 found at 16min (difficulty overshooting then recovering)
- Difficulty converging: started at 15T, clamped down to ~1.1T, now adjusting back up via LWMA
- Both nodes peered and synced at height 73
- Miner running with `stdbuf -oL` for real-time log output

**Operational Notes:**
- Data directory on Linux: `~/.local/share/rill/testnet/chaindata` (not `~/.rill/`)
- Node flag is `--testnet` (not `--network testnet`)
- New peer IDs generated on each fresh start (keypair not persisted)
- Use `stdbuf -oL` with nohup to avoid log buffering

### 2026-02-18 - DigitalOcean Testnet Deployment & Difficulty Fix

**Testnet Infrastructure:**
- Created `infra/do-testnet.sh` — DigitalOcean provisioner for 2-node testnet (nyc1, $24/mo)
- Deployed rill-node0 (seed/miner) at 206.189.202.181 and rill-node1 (wallet) at 159.223.140.65
- VPC 10.20.0.0/24 with firewall restricting P2P to internal, SSH from anywhere
- Cloud-init builds from source via GitHub deploy key (private repo)
- Nodes peered over libp2p (Kademlia + Gossipsub) with multiaddr bootstrap

**First Blocks & Transaction:**
- Mined 73 blocks on testnet, confirmed block production and UTXO creation
- Sent first-ever RillCoin transaction: 100 RILL from miner to wallet node
- Confirmed balance: 3,650 RILL (73 coinbase × 50 RILL) on miner wallet

**Difficulty Bootstrapping Fix:**
- Diagnosed runaway difficulty: u64::MAX genesis target caused 73 instant blocks, then 4^73 clamp made mining impossible
- Added `TESTNET_INITIAL_TARGET = 15_000_000_000_000` calibrated for ~20K H/s single miner (~60s blocks)
- Updated genesis block, difficulty module, and consensus engine to use calibrated target
- Added `initial_target_override` with `testing` feature flag for cross-crate test access
- All 144 workspace tests passing

**CLI Fixes:**
- Added `RILL_WALLET_PASSWORD` env var for non-interactive wallet operations (SSH/scripts)
- Fixed wallet address persistence — `rill-cli address` now saves after derivation
- Fixed balance scan to auto-derive initial address when address_count is 0
- Removed temporary debug logging from balance command

### 2026-02-18 - Azure Testnet Infrastructure (Partial)
- Created `infra/azure-testnet.sh` -- full Azure CLI provisioner for 4-node testnet
- Created `infra/README.md` with setup guide
- Fixed Dockerfile: added `clang`/`libclang-dev` for bindgen, stub bench files for cargo fetch
- Fixed `.dockerignore` context handling (rsync clean context to avoid 20GB target/ upload)
- Successfully built and pushed x86 Docker image to Azure Container Registry
- Successfully provisioned VNet, NSG, ACR, and public IP in Azure
- Hit Azure limitations: quota restrictions on new subscriptions, ARM64 QEMU emulation too slow
- Resolved quota via `az quota create` API
- Decision: moving to Hetzner for simpler, cheaper testnet hosting
- Azure resources cleaned up (resource groups deleted)

### 2026-02-18 - Discord Server Hardening & Onboarding
- Fixed channel permissions: @everyone Send Messages, Create Private Threads, Send in Threads removed at role level
- Audited all 36 channels via API — confirmed read-only, community, governance, and team permissions are correct
- Set up Discord native Onboarding flow (5-step: safety check, default channels, customisation question, server guide, review)
- Onboarding assigns Member role on completion — replaced Carl-bot autorole
- Configured Carl-bot verification reaction role (created, debugged, replaced with onboarding)
- Removed Carl-bot autorole for Member (onboarding handles it now)
- Added @everyone send override on #general for onboarding chattable requirement
- Cleaned up #roles-and-verification: removed broken verify embed, kept notification ping panel
- Deleted Discord messages exposing personal name ("Matt du Jardin achievements")
- Recreated #welcome as text channel (was incorrectly Forum type)

### 2026-02-17 - Privacy & Git Hygiene
- Rewrote entire git history to replace personal author name/email with "rillcoin <dev@rillcoin.com>"
- Set per-repo git config for anonymous commits going forward
- Cleaned up Discord #github-feed webhook messages that contained personal info
- Verified no personal identifiers remain in the codebase

### 2026-02-17 - Phase 5: Testnet Launch Readiness (Complete)

**Phase 5a — Full Sync Integration & Chain Reorg:**
- SyncManager wired into Node event loop with 5s sync tick and 30s timeout tick
- Chain reorganization: disconnect/reconnect with MAX_REORG_DEPTH=100, UTXO+cluster consistency, mempool recovery
- Orphan block pool: HashMap keyed by prev_hash, MAX=100, 10min expiry, cascading reconnection
- Orphan transaction handling: MAX=1000, 5min expiry, retry after block connects
- IBD (Initial Block Download) mode: AtomicBool flag, 144-block threshold, tx suppression during sync

**Phase 5b — Hardening & Security:**
- Peer scoring and banning: penalty/bonus system, BAN_THRESHOLD=-200, 24h ban duration
- DoS protection: per-peer sliding-window rate limiting (blocks 10/min, txs 100/min, headers 5/min), 2MiB max message size
- Header checkpoint verification: parameterized check_checkpoint(), is_below_checkpoint(), reorg protection below checkpoints
- Adversarial proptest suite: 12 property-based tests (256 cases each) covering supply monotonicity, coinbase cap, UTXO consistency, difficulty bounds, merkle/hash determinism
- Block pruning: PruneMode (Full|Pruned(n)), prune_blocks() preserves headers+undo, genesis never pruned

**Phase 5c — Testnet Launch, Performance & Polish:**
- Testnet config: NetworkType enum (Mainnet/Testnet/Regtest), per-network magic bytes/ports/data dirs, --testnet/--regtest CLI flags
- Dev fund vesting: DEV_FUND_PREMINE_AMOUNT, 4-year linear vesting, DEV_FUND_MAX_SPEND_PER_BLOCK
- Criterion benchmark suite: 16 benchmarks across 4 crates (merkle, SHA-256, Ed25519, serialization, sigmoid, decay, block validation, connect_block, UTXO lookup)
- CLI UX polish: getpeerinfo, getblockchaininfo, getsyncstatus, validateaddress commands; --format json|table output
- Storage compaction & optimization: compact() method, optimized find_common_ancestor() and get_headers_after()
- Structured logging: NodeMetrics (blocks_connected, reorgs, mempool_size, peer_count), info_span tracing
- Multi-node E2E tests: 7 scenarios (sync, competing chains, tx propagation, reorg, consistency across 3 nodes, out-of-order delivery, duplicate rejection)
- Docker: multi-stage Dockerfile, 3-node docker-compose testnet, .dockerignore
- Documentation: docs/TESTNET.md, docs/ARCHITECTURE.md, README.md update

**Stats:**
- 927 tests passing (up from 818), zero clippy warnings
- 16 Criterion benchmarks across 4 crates
- Docker testnet infrastructure ready

### 2026-02-17 - Marketing: Telegram, Bridge, Feeds & Governance Proposal

**Telegram Community Setup:**
- Telegram channel (@Rillcoinchat) and Rillcoin Community Group created and linked
- Combot added for anti-spam moderation
- Welcome message pinned in group
- Brand icon (rill-icon-512.png) set as profile picture

**Discord-Telegram Bridge:**
- Bridge script (`discord_telegram_bridge.py`) monitors #announcements and #dev-updates
- Forwards new messages to Telegram group with formatted headers
- Cron job runs every 2 minutes via `--once` mode
- State tracking in `.bridge_state.json` prevents duplicate forwards

**Discord Feed Channels:**
- 4 new channels added to BOTS category: #crypto-news, #price-ticker, #regulatory-watch, #whale-alerts
- MonitoRSS configured with 3 feeds: Bitcoin Magazine, Cointelegraph, Cointelegraph Regulation
- CoinTrendzBot deployed for live price voice channels (BTC, ETH, XRP, SOL, BNB + more)
- Incremental deploy script (`add_feed_channels.py`) for non-destructive channel creation
- MonitoRSS setup guide written for community lead handoff

**Governance:**
- RCP-001: Developer Fund Block Reward (5%) & Community Donation Program drafted
- Posted to #proposals forum and #announcements channel on Discord

**Brand Assets:**
- rill-icon.svg and rill-icon-512.png created (three converging streams, brand colors)

**Infrastructure:**
- Dockerfile updated with bench stub fix for cargo fetch
- Azure testnet scripts added (infra/)

### 2026-02-17 - Marketing: Discord Server Fully Configured

**Discord Server Provisioned & Configured:**
- Python setup script (`marketing/scripts/setup_discord.py`) provisions entire server automatically
- Bot created, authorized, and connected to RillCoin Discord server
- 14 roles created with brand-aligned colors and permissions
- 9 categories + 27 channels created with topics, slowmode, and permission overwrites
- 24 pinned messages sent and pinned across 16 channels
- 2 announcement channels (type 5) created after enabling Community features
- Permanent invite link generated: discord.gg/QxBJfvvUaA

**Testnet Incentive Program Announced:**
- 50,000 RILL from dev fund allocated proportionally to testnet participants at mainnet launch
- 24-48 hour early mining access for testnet participants
- Discord announcement posted to #announcements
- Conversation starter posted to #general
- X/Twitter thread drafted (6 tweets) ready for manual posting
- Content saved to `marketing/outputs/testnet-incentive-posts.md`

**Bot & Moderation Configuration:**
- Carl-bot: welcome DM, reaction roles (5 notification ping roles via !rr make), logging to #mod-log
- Carl-bot granted permissions in read-only START HERE channels
- MEE6: added but free tier too limited — skipped (premium required for moderation/levels)
- Wick: verification (captcha in #roles-and-verification, 5min duration), join gate (7-day account age, suspicious account kick), anti-nuke (auto quarantine, monitor public roles + channel permissions)
- Discord AutoMod: 4 rules via API (scam phrases, banned copy words, mass mentions, spam detection)
- All violations logged to #mod-log
- GitHub webhook live on #github-feed (push, PR, release events)
- Founder role assigned, role hierarchy ordered
- URL placeholders replaced with GitHub links in all pinned messages

**Forum Channels Configured:**
- #bug-reports: 7 tags (Critical/High/Medium/Low/Confirmed/Fixed/Won't Fix), bug report template post
- #proposals: 10 tags (Draft/Discussion/Under Review/Accepted/Rejected/Implemented + topic tags), RCP template post
- #research: 7 tags (Decay Mechanics/Economic Model/Consensus/Cryptography/External Paper/Analysis/Question), posting guidelines
- #introductions: 4 tags (Developer/Miner/Researcher/Community)

**Next steps:**
- Post X thread manually
- Update X bio re: dev fund transparency ("No premine" vs 5% dev allocation)
- Build testnet faucet bot
- Consider Wick/MEE6 premium if server scales past 500 members

### 2026-02-17 - Marketing: Discord Server Specification & Content

**Discord Server Spec (`marketing/outputs/discord-server-spec.md`):**
- Complete production-ready Discord server specification
- 30+ channels across 9 categories (START HERE, ANNOUNCEMENTS, COMMUNITY, TECHNICAL, TESTNET, GOVERNANCE, SUPPORT, TEAM, BOTS)
- 14-role hierarchy with brand-aligned colors (Founder=Orange, Core Team=Flowing Blue, etc.)
- 6 bot configurations: MEE6 (automod), Carl-bot (roles/logging), Wick (anti-raid), GitHub webhooks, Zapier for X feed, plus specs for custom faucet + decay calculator bots
- Crypto-specific moderation policy with 3-tier escalation
- Ambassador program, Bug Hunter recognition, community event formats
- Full launch checklist (pre-launch, at-launch, first 30 days)

**Channel Descriptions (`marketing/outputs/discord-channel-descriptions.md`):**
- 27 channel topic descriptions, all 60-120 characters
- Copy-library compliant (no banned words, approved terminology only)

**Pinned Messages (`marketing/outputs/discord-pinned-messages.md`):**
- 24 production-ready pinned messages across 16 channels
- All within Discord's 2,000-character limit
- Includes: welcome flow, full rules, 10-entry FAQ, decay explainer, bug report template, proposal template, node operator guide, and anti-scam warnings
- Copy-library compliant throughout

**Next steps:**
- User to create Discord server + bot application
- Build Python setup script to provision server via Discord API
- Draft blog/updates section for rillcoin.com

### 2026-02-17 - Phase 3+4: Production Readiness & Testnet Prep

**Phase 3 — Core Infrastructure:**
- RandomX PoW engine (feature-gated behind `randomx` flag), SHA-256 mock PoW default
- Header-first chain sync protocol with single-peer block download
- UTXO address index with O(k) lookups via RocksDB secondary index
- Cluster balance tracking and RPC query support (`getclusterbalance`)
- Expanded storage layer (533 LOC) with column families for address index
- Argon2id wallet encryption replacing raw AES-256-GCM
- CLI RPC integration for balance, send, and address commands
- Network request-response protocol for block/header fetching

**Phase 4 — Testnet Readiness (5 items):**
- Multi-peer parallel sync: PeerState tracking, timeout detection (30s), ban after 3 failures, round-robin block assignment across peers
- BIP-39 mnemonic backup: 24-word seed phrases for wallet create/restore, hex as fallback
- Decay-aware coin selection: CLI `send` now uses `CoinSelector::select()` (highest-decay UTXOs spent first), fetches cluster balances via RPC
- MIN_TX_FEE enforcement: mempool rejects transactions with fee < 1000 rills
- 15 end-to-end integration tests: mine, spend, difficulty, decay, wallet lifecycle, security regression

**Stats:**
- 818 tests passing (up from 76), zero clippy warnings
- 28 files changed, 3,289 insertions, 5 new files
- Commit: `bfd53c2`, pushed to origin/main

### 2026-02-17 - Blocker Fixes: VULN-COINBASE-TXID & Fee Computation

**VULN-COINBASE-TXID (High — resolved):**
- Coinbase transactions now set `lock_time = height` instead of `lock_time = 0`
- Since `lock_time` is included in the witness-stripped txid, each block height produces a unique coinbase txid
- Updated all `make_coinbase_unique` helpers across chain_state, storage, security_audit, and storage_test
- E2E regression test now verifies 3 blocks to same address produce 3 distinct UTXOs

**Node::process_transaction fee=0 (Medium — resolved):**
- `process_transaction()` now computes actual fee (`input_sum - output_sum`) before mempool insertion
- Uses checked arithmetic (`checked_add`, `checked_sub`) per project conventions
- RPC `sendrawtransaction` now works correctly with MIN_TX_FEE enforcement

**Stats:**
- 818 tests passing, zero clippy warnings, zero active blockers
- 8 files changed, 63 insertions
- Commit: `89f7eca`, pushed to origin/main

### 2026-02-16 - Repository Setup & Documentation

**Infrastructure:**
- Verified SSH access to GitHub (rillcoin account)
- Configured GitHub CLI authentication for rillcoin organization
- Pushed local repository to GitHub at `rillcoin/rill` for the first time
- Set up remote tracking for main branch

**Documentation:**
- Created comprehensive README.md (220 lines)
  - Project overview and philosophy
  - Architecture diagram and crate dependency graph
  - Getting started guide for all three binaries
  - Technical specifications (consensus, decay, network, storage)
  - Development standards and contribution guidelines
  - Build, test, and deployment instructions

**Status:**
- All tests passing (76 tests, 0 failures)
- Build clean across workspace
- Repository publicly accessible at https://github.com/rillcoin/rill
