# Changelog

## [Unreleased]

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
