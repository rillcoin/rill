# RillCoin Discord Server Specification

> Production-ready setup guide. Use this document to configure the server from scratch.

---

## Table of Contents

1. [Server Identity](#1-server-identity)
2. [Channel Structure](#2-channel-structure)
3. [Role Hierarchy](#3-role-hierarchy)
4. [Onboarding Flow](#4-onboarding-flow)
5. [Bot Recommendations](#5-bot-recommendations)
6. [Moderation Policy](#6-moderation-policy)
7. [Community Programs](#7-community-programs)
8. [Integration Points](#8-integration-points)
9. [Launch Checklist](#9-launch-checklist)

---

## 1. Server Identity

### Server Name

**RillCoin**

Avoid appending "Community" or "Official" — the name alone is sufficient. Clarity and restraint are on-brand.

### Server Description (shown in Discovery and invite previews)

> Progressive concentration decay cryptocurrency. Wealth flows to miners, not hoarders. Join us in building a fairer system — technically grounded, community-driven.

Keep this description under 120 characters for mobile display. Do not use price language, financial promises, or hype phrases.

### Vanity URL

**discord.gg/rillcoin**

Register this as soon as the server reaches the 100-member threshold Discord requires. Until then, use a standard invite link pinned in `#welcome` and linked from the website and X/Twitter bio.

### Server Icon

Use the coin icon from `shared/brand-assets/rill-icon.svg` (or the hi-res PNG at `shared/brand-assets/rill-icon-hires.png`). The dark coin on a `#0A1628` background reads clearly at Discord's 512x512 server icon size. Do not use the wordmark — it becomes illegible at small sizes.

Specific guidance for the icon asset:
- Source file: `shared/brand-assets/rill-icon-hires.png`
- Background: `#0A1628` (Dark Navy) — matches Discord's dark theme sidebar
- No padding needed; the coin's internal dark fill already provides visual breathing room
- Export as PNG, 512x512, no transparency (Discord crops to circle)

### Server Banner

The banner appears at the top of the channel list (Nitro-boosted servers, level 2 required). Spec it now so it is ready when boosting is warranted.

- Dimensions: 960x540 px (16:9)
- Background: horizontal gradient from `#0A1628` (left) to `#1A3A5C` (right)
- Centered: wordmark (`rill-wordmark-dark.svg`) at approximately 30% of banner width
- Below wordmark: tagline in Inter, weight 400, `#3B82F6`, "Wealth should flow like water."
- No photography, no abstract renders — keep it typographic and clean

### Accent Color (shown in server boost UI and some embed highlights)

Use `#3B82F6` (Flowing Blue) as the server accent. This is the closest brand match to Discord's color picker options.

---

## 2. Channel Structure

Channels are grouped into categories. Each category is described with its purpose, followed by individual channel entries.

Format per channel:
- **Name** — purpose, posting access, special configuration

### Category: START HERE

Purpose: The first thing a new member sees. Locked for member posting. Sets the tone before they access the rest of the server.

Permissions: All channels in this category are read-only for everyone except Team and Moderators.

| Channel | Purpose | Post Access | Config |
|---|---|---|---|
| `#welcome` | Server introduction, rules, brand explainer. Pinned message links to onboarding thread. | Team only | Read-only |
| `#rules` | Full moderation policy, zero-tolerance items, escalation procedure. Enforces single-message format — one pinned post, kept up to date. | Team only | Read-only |
| `#roles-and-verification` | Explains the role system. Contains the verification button (via bot). Also lists role-earning pathways. | Team only | Read-only; verification component attached |
| `#faq` | Answers to the 10 most common questions about decay mechanics, testnet, wallet setup, and the project roadmap. Updated quarterly. | Team only | Read-only |

### Category: ANNOUNCEMENTS

Purpose: Official project communications. High signal, low volume. Community members watch these channels for real news.

Permissions: Team posts only. Community may react but not post. Crosspost announcements to Telegram and X from here.

| Channel | Purpose | Post Access | Config |
|---|---|---|---|
| `#announcements` | Major releases, milestones, partnership news, critical protocol updates. Expect fewer than 4 posts per month. | Team only | Read-only; Discord Announcement channel type (enables Following) |
| `#dev-updates` | Weekly or bi-weekly technical progress notes from the dev team. Linked to GitHub via webhook. Informal tone, technical depth. | Team only | Read-only; Announcement channel type |
| `#testnet-status` | Live testnet health status. Updated by bot. Shows block height, peer count, last decay event. Manual fallback when bot is offline. | Team only (manual), bot (auto) | Read-only; slowmode 0 (bot-driven) |

### Category: COMMUNITY

Purpose: The main space for conversation, questions, and culture. Open posting for verified members.

Permissions: Verified members post. Unverified (new joins) can read but not post until verification is complete.

| Channel | Purpose | Post Access | Config |
|---|---|---|---|
| `#general` | Open community discussion. On-topic preferred; off-topic tolerated if it does not dominate. | Verified members | Slowmode: 3 seconds |
| `#introductions` | New members introduce themselves. Thread-only: each introduction creates a thread. Keeps the channel clean. | Verified members | Forum channel type (thread-per-post) |
| `#price-and-markets` | Designated space for market discussion. Strictly bounded: no financial advice, no predictions, no coordinated buying language. Exists so this content stays out of #general. | Verified members | Slowmode: 30 seconds; auto-mod filter active |
| `#off-topic` | Non-Rill conversation. Tech, culture, whatever the community builds. | Verified members | Slowmode: 5 seconds |
| `#memes` | Community creativity, memes, art. Must relate to Rill themes. Banned word filter applies. | Verified members | No slowmode; media-heavy |

### Category: TECHNICAL

Purpose: Deep technical discussion. This is the differentiating community space — RillCoin's decay mechanism is technically novel. The channels here should attract developers, researchers, and protocol thinkers.

Permissions: Verified members post. Tag roles apply (see Role section) for highlighting responses from team/contributors.

| Channel | Purpose | Post Access | Config |
|---|---|---|---|
| `#protocol` | Discussion of decay mechanics, consensus rules, fee structure, PoW specifics. High signal expected. | Verified members | Slowmode: 5 seconds; thread-friendly |
| `#development` | Open-source contribution questions, codebase discussion, architecture Q&A. Links to GitHub issues welcome. | Verified members | Slowmode: 3 seconds |
| `#research` | Longer-form technical posts, external paper links, economic modeling of decay. Use threads for extended discussion. | Verified members | Forum channel type; posts require a title |
| `#node-operators` | Running a full node or mining node. Config questions, sync issues, hardware specs. | Verified members | Slowmode: 5 seconds |

### Category: TESTNET

Purpose: Everything related to the active testnet phase. This is the highest-engagement zone while the project is pre-mainnet.

Permissions: Verified members post. Testnet Participant role grants a colored tag here but does not change posting permissions.

| Channel | Purpose | Post Access | Config |
|---|---|---|---|
| `#testnet-general` | General testnet discussion, questions, coordination. | Verified members | Slowmode: 5 seconds |
| `#bug-reports` | Structured bug reports only. Pinned template at top. Each bug becomes a thread. Triaged by dev team. | Verified members | Forum channel type; required fields in template: version, OS, steps to reproduce, expected vs actual |
| `#testnet-wallets` | Share testnet wallet addresses for faucet requests and testing. Mainnet addresses forbidden (enforced by automod pattern). | Verified members | Slowmode: 10 seconds |
| `#faucet` | Bot-served testnet coin requests. One request per user per 24 hours. Command: `/faucet <address>`. | Verified members + bot | Slowmode: 10 seconds; bot command channel |

### Category: GOVERNANCE

Purpose: Forward-looking space for community input on protocol direction. Currently advisory — no on-chain governance yet. Establishes the culture before governance is needed.

Permissions: Verified members read. Posting restricted to Contributor tier and above to keep signal-to-noise high.

| Channel | Purpose | Post Access | Config |
|---|---|---|---|
| `#governance-general` | Discussion of governance ideas, process, and philosophy. | Contributor+ | Slowmode: 10 seconds |
| `#proposals` | Formal improvement proposals. Forum channel. Each proposal gets its own thread with structured format (title, summary, motivation, specification, drawbacks). | Contributor+ | Forum channel type; post guidelines pinned |
| `#voting` | When snapshot.org or on-chain voting is live, results and links posted here. Read-only until governance is active. | Team only (until governance launches) | Read-only initially |

### Category: SUPPORT

Purpose: User support that does not belong in the main channels. Keeps technical conversation from being drowned in setup questions.

Permissions: Verified members post in `#support-general`. Ticket system for private support.

| Channel | Purpose | Post Access | Config |
|---|---|---|---|
| `#support-general` | Public support questions. Wallet setup, sync issues, decay calculation questions. | Verified members | Slowmode: 5 seconds |
| `#create-ticket` | Bot-powered private ticket creation. Users click a button; a private thread opens with the support team. Prevents doxxing of addresses/keys in public channels. | Verified members (button only) | Ticket bot component; no free posting |

### Category: TEAM

Purpose: Internal coordination visible to the team only. Not visible to community members.

Permissions: Team role only.

| Channel | Purpose | Post Access | Config |
|---|---|---|---|
| `#team-general` | Day-to-day coordination | Team only | Private category |
| `#mod-log` | Auto-populated moderation action log from the mod bot | Team + Mods | Private; bot writes here |
| `#incident-response` | Active incident handling (security issues, major bugs, scam waves) | Team + Mods | Private |
| `#community-feedback` | Digested feedback from the community for the dev team | Team only | Private |

### Category: BOTS

Purpose: Bot output and command invocation that would clutter other channels.

Permissions: Verified members post bot commands here. Bots post here.

| Channel | Purpose | Post Access | Config |
|---|---|---|---|
| `#bot-commands` | All non-faucet bot commands invoked here. Keeps `#general` clean. | Verified members | No slowmode |
| `#github-feed` | Automated GitHub commit, PR, and release notifications via webhook. | Bot only (GitHub webhook) | Read-only except webhook |
| `#twitter-feed` | Automated X/Twitter post feed via webhook or bot. | Bot only | Read-only except bot |
| `#crypto-news` | Aggregated crypto industry news from CoinDesk, The Block, Bitcoin Magazine, Decrypt. Filtered to PoW, monetary policy, and protocol design topics. | Bot only (MonitoRSS) | Read-only |
| `#price-ticker` | Auto-updating price data for BTC, ETH, and top 20 by market cap. RILL added post-mainnet listing. | Bot only (CoinGecko Bot) | Read-only |
| `#regulatory-watch` | Low-volume feed of regulatory developments from SEC, CFTC, and industry sources. ~2-5 posts/week. | Bot only (MonitoRSS) | Read-only |
| `#whale-alerts` | Large transaction alerts on BTC, ETH, and major chains. Stub pre-mainnet; RILL tracking added post-mainnet. | Bot only (Whale Alert Bot) | Read-only |

---

## 3. Role Hierarchy

Roles are listed from highest to lowest in the hierarchy. Discord renders them in this order in the member list sidebar.

### Role Design Principles

- Color usage maps to the brand palette. Not every role needs a color — use color sparingly so that colored names in chat are meaningful signals.
- Hoisted roles (shown as separate sections in the member list) are marked as such. Over-hoisting clutters the sidebar; only hoist what matters.
- Bot roles are positioned below all human roles in the hierarchy.

---

### Team Roles

**Founder**
- Color: `#F97316` (Accent Orange)
- Hoisted: Yes
- Permissions: Administrator
- Assignment: Manual, restricted to project founders
- Purpose: Visible authority. The orange name in chat is an immediate signal of authentic project leadership.

**Core Team**
- Color: `#3B82F6` (Flowing Blue)
- Hoisted: Yes
- Permissions: Manage Messages, Manage Threads, Mute/Deafen Members, Move Members, Mention Everyone, Attach Files, Embed Links, all standard member permissions
- Assignment: Manual by Founder. For full-time or core contributors (dev team, ops)
- Purpose: The people building the project day-to-day. Blue name matches the primary brand color.

**Moderator**
- Color: `#1A3A5C` (Deep Water, lightened for visibility — use `#2A5A8C` as Discord role color)
- Hoisted: Yes
- Permissions: Kick Members, Ban Members, Manage Messages, Manage Threads, View Audit Log, Timeout Members, Mute Members
- Assignment: Manual by Core Team after vetting process (see Ambassador Program)
- Purpose: Day-to-day moderation enforcement. Distinguished from Team to clarify who is building vs. who is enforcing rules.

---

### Community Achievement Roles

These roles are earned through participation. They are the primary gamification layer.

**Contributor**
- Color: `#4A8AF4` (blue-light from logo gradient)
- Hoisted: Yes
- Permissions: Standard member permissions plus access to `#governance-general` and `#proposals`
- Assignment: Manual by Core Team or automated via Carl-bot level thresholds. Criteria: submitted a verified bug report OR merged a GitHub contribution OR authored an approved governance proposal OR completed 30 days as an active community member with 500+ messages and no violations.
- Purpose: The first earned role that unlocks governance access. A meaningful milestone.

**Testnet Participant**
- Color: `#5DE0F2` (cyan-light from logo gradient)
- Hoisted: Yes
- Permissions: Standard member permissions; colored tag in testnet channels
- Assignment: Automated via bot when a user submits a valid faucet request and confirms receipt. Revoked if address is flagged for abuse.
- Purpose: Recognizes everyone running and testing the network. This role should be easy to earn during the testnet phase to drive participation.

**Bug Hunter**
- Color: `#F97316` (Accent Orange)
- Hoisted: No
- Permissions: Standard member permissions
- Assignment: Manual by Core Team. Awarded for each confirmed, non-duplicate bug report with a severity of Medium or above. Can be stacked (shown as "Bug Hunter x3" using a bot counter, or simply re-awarded as a separate role tier).
- Purpose: Recognizes quality testnet participation. Developers respond more warmly to named contributors.

**Ambassador**
- Color: `#3B82F6` (Flowing Blue)
- Hoisted: No
- Permissions: Standard member permissions plus ability to post in `#announcements` thread replies
- Assignment: Manual, by application (see Ambassador Program section). Minimum: 60 days active, Contributor role, no moderation history.
- Purpose: Community evangelists. Trusted to represent RillCoin in external spaces.

---

### General Member Roles

**Member**
- Color: No color (default Discord gray/white)
- Hoisted: No
- Permissions: Standard member permissions — read and post in all public community channels, use reactions, attach files, embed links
- Assignment: Automatic after completing verification (see Onboarding Flow)
- Purpose: The baseline verified community member role. All non-verified users are held in the Unverified state below.

**Unverified**
- Color: No color
- Hoisted: No
- Permissions: Read `#welcome`, `#rules`, `#roles-and-verification`, `#faq` only. Cannot post anywhere.
- Assignment: Automatic on join
- Purpose: Gating layer before verification. Prevents bots and drive-by spammers from posting immediately on join.

---

### Opt-In Notification Roles

These are self-assignable via Carl-bot reaction roles or slash commands. They control ping subscriptions.

**Announcements Ping** — Users who want `@here`-style pings for major announcements
**Testnet Ping** — Users who want testnet status and event pings
**Dev Updates Ping** — Users who want pings when `#dev-updates` posts
**AMA Ping** — Users who want reminders for upcoming AMAs and office hours
**Governance Ping** — Users who want pings when new governance proposals open

Self-assignable via `/role` command in `#bot-commands`. No color. Not hoisted.

---

### Bot Roles

**MEE6** — Color: `#000000`. Position: below all human roles. Permissions: Manage Messages, Kick Members, Ban Members, Manage Roles (scoped), View Audit Log.

**Carl-bot** — Color: `#000000`. Position: below all human roles. Permissions: Manage Roles (scoped to roles below it), Manage Messages, Manage Channels (scoped).

**GitHub Feed Bot** — Color: `#000000`. Permissions: Send Messages (scoped to `#github-feed`).

**Twitter Feed Bot** — Color: `#000000`. Permissions: Send Messages (scoped to `#twitter-feed`).

---

## 4. Onboarding Flow

### Step 1: Join

A user accepts an invite link (from rillcoin.com, X/Twitter bio, or a direct invite). They are automatically assigned the **Unverified** role.

They can see and read only the START HERE category: `#welcome`, `#rules`, `#roles-and-verification`, `#faq`.

### Step 2: Welcome DM

Carl-bot sends a DM within 5 seconds of joining. DM copy (approved, no banned words):

---

**Welcome to the RillCoin server.**

RillCoin is a proof-of-work cryptocurrency with progressive concentration decay. Holdings above defined thresholds flow back to active miners. The project is in active development and testnet is live.

To get access to the community:

1. Read the rules in `#rules`
2. Verify your account in `#roles-and-verification`
3. Introduce yourself in `#introductions` once you're in

Start in `#faq` if you want to understand how decay works before anything else.

---

The DM is plain text. No images (many users have DM images disabled and this risks the DM being filtered). No emoji. No price language.

### Step 3: Verification

In `#roles-and-verification`, the user encounters a Carl-bot or Wick verification component. Configuration:

- **Method:** Button click + age verification (account must be at least 14 days old)
- **CAPTCHA:** Enable optional CAPTCHA gate for accounts under 30 days old
- **On success:** Unverified role removed, Member role applied, bot posts confirmation in `#roles-and-verification` (ephemeral, visible only to the user)

Accounts under 14 days old are auto-held and flagged to moderators in `#mod-log`. This is a high-signal scam indicator.

### Step 4: First Experience

After verification, the user can access the full server. The order in which categories and channels appear should guide them naturally:

1. START HERE (they've already been here — now they see the full FAQ)
2. ANNOUNCEMENTS (passive reading — they understand the project's current state)
3. COMMUNITY — `#general` is the natural first landing spot
4. TECHNICAL and TESTNET open up as they explore

No forced pathway. The structure itself guides behavior without being patronizing.

### Step 5: Discovering Decay

The decay mechanism is RillCoin's core differentiator. New members will not understand it immediately. The onboarding path to understanding it:

- `#faq` covers "What is concentration decay?" in plain language — approved copy below
- `#protocol` has a pinned explainer thread from the Core Team
- `#research` has a pinned link to the technical whitepaper section
- `#dev-updates` history shows the decay system being built in real time

**FAQ entry — "What is concentration decay?" (approved copy):**

> RillCoin implements a mechanism called progressive concentration decay. When a wallet's balance exceeds defined thresholds, the portion above each threshold decays over time and flows into the mining pool, where it is redistributed to active miners. The larger your balance, the faster the excess decays. This is not a penalty — it is a circulation incentive. Wealth should flow like water, not pool in reservoirs.

### Ongoing Engagement Signals

After joining, members discover further pathways through:

- Pinned messages in each channel (always up to date — stale pins erode trust)
- Thread activity in `#protocol` and `#research`
- Event announcements in `#announcements` and `#testnet-status`
- The self-serve role assignment in `#bot-commands` (notification opt-ins)

---

## 5. Bot Recommendations

### MEE6 — Primary Moderation and Leveling Bot

**Use:** Auto-moderation, leveling/XP system, command responses

**Why MEE6:** Battle-tested for crypto communities. Has a robust automod suite that handles the patterns crypto scammers use (external links, wallet address solicitation, impersonation). The leveling system feeds into role progression.

**Configuration:**

Automod rules:
- Block messages containing common scam phrases: "DM me for airdrop", "send ETH/BTC to receive", "wallet recovery", "seed phrase", "connect your wallet", "trust wallet", "MetaMask support", any pattern matching `0x[0-9a-fA-F]{40}` (Ethereum addresses — irrelevant to RillCoin but a scam signal)
- Block messages containing banned words from the copy library: moon, lambo, pump, dump, gem, ape, degen, wagmi, ngmi, diamond hands, paper hands, rug, shill
- Block new accounts (under 14 days) from posting external links
- Block messages with more than 3 mentions in a single message (anti-mass-mention spam)
- Block identical messages sent more than twice within 30 seconds (anti-repeat spam)
- Block messages with more than 5 Discord invite links
- Block all messages containing external invite links for accounts under 30 days old
- Block messages containing fake support/verification phishing: "verify on our portal", "complete verification at", "open a ticket on our website" combined with external URLs
- Block messages containing QR code scam patterns: "scan this QR" + "verify", "scan to claim", "scan to receive"
- Block messages containing wallet drainer vectors: "download" + "wallet extension", "install" + "browser extension", "update your wallet to claim"
- Block messages containing social proof manipulation: "100x confirmed", "early investors are up", "insiders are buying", "whales are buying"
- Block Telegram invite links (`t.me/`) from accounts under 60 days old
- Block messages from accounts under 30 days containing "official" + "tool" + URL

Leveling:
- Enable XP in: `#general`, `#protocol`, `#development`, `#research`, `#testnet-general`, `#off-topic`, `#memes`
- Disable XP in: all ANNOUNCEMENTS channels, all START HERE channels, `#bot-commands`, `#github-feed`, `#twitter-feed`, `#faucet`
- Level-up announcements: post in `#bot-commands` only, not in the active channel
- Level-to-role milestones: align with Contributor threshold (Level 15 as a minimum gate, combined with manual review)

**Cost:** Free tier is sufficient for most needs. Premium ($11.95/mo) unlocks more automod rules — worth it for a crypto community.

---

### Carl-bot — Role Management, Reaction Roles, Logging

**Use:** Self-assignable notification roles, welcome DM, comprehensive audit logging, custom commands

**Why Carl-bot:** Best-in-class for role automation and logging. Supports the verification button, DM on join, and reaction role panels without Nitro requirements. The logging is far more granular than MEE6's.

**Configuration:**

Welcome DM: Set up using the Autorole + Welcome module. Fire DM on join with the text specified in the Onboarding section above. Delay: 3 seconds (enough to feel human, not enough to frustrate).

Reaction roles panel in `#roles-and-verification`:
- Create a single embed titled "Notification Roles — Choose Your Pings"
- Five buttons: Announcements Ping, Testnet Ping, Dev Updates Ping, AMA Ping, Governance Ping
- Style: Secondary button (gray) so they do not look like calls to action that could confuse new users
- Users can add/remove freely at any time

Logging module: Log all of the following to `#mod-log`:
- Message edits (before and after)
- Message deletions
- Member joins and leaves
- Role changes
- Channel creation/deletion
- Ban and kick actions
- Timeout actions
- Invite creation and deletion

Custom commands (prefix `!` in `#bot-commands`):
- `!decay` — returns a brief explanation of concentration decay with a link to the documentation
- `!whitepaper` — returns a link to the whitepaper or relevant documentation section
- `!testnet` — returns current testnet status link and faucet instructions
- `!roadmap` — returns a link to the public roadmap
- `!github` — returns the GitHub repository link

**Cost:** Free tier is sufficient. Carl-bot Pro ($5/mo) adds additional embed customization and extended log history — optional.

---

### Wick — Anti-Raid and Account Verification

**Use:** Anti-raid protection, advanced account age verification, alt-account detection

**Why Wick:** Purpose-built for abuse prevention. Handles raid events (mass join attacks common against crypto projects) automatically. Detects suspicious account patterns that MEE6 misses.

**Configuration:**

Anti-raid mode: Enable at threshold of 10 joins per 30 seconds. Action: auto-lockdown (prevent new members from seeing any channels until manually lifted by a Moderator). Send alert to `#incident-response`.

Account age gate:
- Under 14 days: auto-kick with a DM explaining they are welcome to rejoin once their account is older
- 14–30 days: flag to `#mod-log`, apply a temporary "New Account" tag (no permissions impact, just visibility for moderators)

Alt detection: Enable cross-ban sync. When a user is banned, Wick flags any subsequent joins from the same IP or device fingerprint as a potential alt.

**Cost:** Free tier covers the core use cases. Wick Pro adds more granular IP analysis — consider it after the server reaches 1,000 members.

---

### GitHub Webhooks (Native Discord Feature)

**Use:** Automatic posting of commits, pull requests, and releases to `#github-feed`

**Why native webhooks:** No bot dependency, no third-party access to the repository. GitHub's native Discord webhook integration posts formatted embeds for all push events, PR opened/merged, and release published.

**Configuration:**

In the GitHub repository settings (Settings > Webhooks > Add webhook):
- Payload URL: the Discord webhook URL from `#github-feed` channel settings, appended with `/github`
- Content type: `application/json`
- Events to subscribe: Push (to main branch only), Pull Request (opened, closed, merged), Releases
- Branch filter: `main` and `testnet` branches only — do not pipe every feature branch push

In `#github-feed` channel: pin a message explaining what posts here and that it is automated.

---

### Zapier or Make (formerly Integromat) — X/Twitter Feed

**Use:** Automated posting of new X/Twitter posts from @RillCoin to `#twitter-feed`

**Why:** Discord does not natively integrate with X. Zapier or Make can watch the @RillCoin account and post new tweets to `#twitter-feed` via a Discord webhook.

**Configuration:**

Zapier Zap: "Twitter > New Tweet by Specific User" → "Discord > Send Channel Message via Webhook"
- Filter: Only original tweets (exclude replies and retweets) OR include all — decide based on posting volume
- Message format: include the tweet text and a link to the tweet
- Webhook target: `#twitter-feed`

Make is the more cost-effective option for low-volume accounts (free tier covers ~1,000 operations/month). Zapier is easier to configure.

**Note:** Twitter/X API access now requires a paid developer account. Budget approximately $100/month for Basic API access, or use a social listening tool with built-in Discord integration such as Mention or Hootsuite.

---

### MonitoRSS — RSS News Feed Bot

**Use:** Automated delivery of curated crypto industry news and regulatory updates to `#crypto-news` and `#regulatory-watch`

**Why MonitoRSS:** Open-source, self-hostable, 7+ years of uptime, 500M+ articles delivered. Full control over sources and filtering — no spam or shilling risk from bot-curated feeds. The most flexible RSS-to-Discord bot available. One bot instance powers multiple channels with separate feed configurations.

**Configuration:**

`#crypto-news` feeds:
- CoinDesk RSS (filtered to: proof-of-work, Layer 1, monetary policy, protocol)
- The Block RSS (filtered to: Bitcoin, mining, consensus, protocol design)
- Bitcoin Magazine RSS (all articles)
- Decrypt RSS (filtered to: Bitcoin, mining, Layer 1)

`#regulatory-watch` feeds:
- SEC Litigation Releases RSS (official SEC.gov feed)
- CFTC News RSS (official CFTC.gov feed)
- CoinDesk — Policy and Regulation tag RSS
- The Block — Regulation tag RSS

Filtering: MonitoRSS supports keyword filters per feed. Configure include-filters for relevant terms (proof-of-work, mining, consensus, Layer 1, regulation, SEC, CFTC, MiCA) and exclude-filters for noise (NFT launches, meme coins, airdrops).

**Invite:** Add MonitoRSS from https://monitorss.xyz and configure feeds via the web dashboard.

**Cost:** Free for up to 5 feeds. The free tier covers the initial configuration. Self-hosting removes all limits.

---

### CoinGecko Bot — Price Data Feed

**Use:** Automated price summaries for top cryptocurrencies in `#price-ticker`

**Why CoinGecko Bot:** Official bot from CoinGecko, the most widely used crypto data aggregator. Free tier supports 4000+ coins, slash commands, and formatted embeds. No API key required for the Discord bot.

**Configuration:**

- Channel: `#price-ticker`
- Tracked assets: BTC, ETH, and top 20 by market cap
- Schedule: Configure `/price` auto-posting at a regular interval (e.g., every 4-6 hours)
- Post-mainnet: Add RILL to tracked assets once listed on CoinGecko
- Slash commands available in `#bot-commands` for on-demand queries: `/price bitcoin`, `/price ethereum`

**Invite:** Add from the CoinGecko Bot Discord listing. No API key required.

**Cost:** Free.

---

### Whale Alert Bot — Large Transaction Monitoring

**Use:** Alerts for large cryptocurrency transactions on major chains in `#whale-alerts`

**Why Whale Alert:** The standard for on-chain large transaction monitoring. Free tier tracks BTC, ETH, and major chains. Provides context for market movements without requiring custom infrastructure.

**Configuration:**

- Channel: `#whale-alerts`
- Pre-mainnet: Channel is a stub with a pinned explainer. Activate whale tracking when the community reaches sufficient size to benefit from the data.
- Post-mainnet: Add RillCoin-specific large movement alerts via custom bot integration (leveraging the RillCoin node RPC).
- Threshold: Configure minimum transaction size to avoid noise (e.g., BTC > 100 BTC, ETH > 1000 ETH).

**Invite:** Add from the Whale Alert Discord listing.

**Cost:** Free tier covers the core use case. Premium unlocks lower thresholds and more chains — consider after mainnet.

---

### Testnet Faucet Bot (Custom — Future Build)

**Use:** Serve testnet RillCoin to users on request in `#faucet`

**Why custom:** No off-the-shelf bot handles the RillCoin testnet. This will require a small custom bot (Node.js or Python, running on the same infrastructure as the testnet node).

**Planned configuration:**

- Command: `/faucet <testnet-address>`
- Rate limit: one request per Discord user per 24 hours, tracked in a simple database
- Output: posts a message confirming the request and the expected transaction time
- Error handling: invalid address format, rate limit reached, faucet empty (triggers a Core Team alert)
- On successful delivery: auto-assign Testnet Participant role to the requesting user
- Address validation: reject addresses that do not match the RillCoin testnet address format

This bot does not exist yet. Spec it for development once testnet is stable. Estimated build time: 1–2 days for a developer familiar with the codebase.

---

### Decay Calculator Bot (Custom — Future Build)

**Use:** Interactive decay estimation in `#bot-commands` or `#protocol`

**Planned configuration:**

- Command: `/decay <balance>`
- Output: shows the effective decay schedule for the given balance — which thresholds apply, what percentage decays per period, what flows to the mining pool
- Pulls constants from the published protocol specification (not live chain data — this is a calculator, not a live query)
- Disclaimer appended to every response: "This is a model estimate based on published protocol constants. Actual decay depends on current chain state."

---

## 6. Moderation Policy

### Core Principles

1. Protect the community from bad actors without chilling legitimate technical discourse.
2. Moderation decisions should be consistent, documented, and reversible where appropriate.
3. Zero tolerance items are enforced immediately and without discretion.
4. Community members are treated as adults capable of self-regulation when the environment is well-designed.

---

### Zero Tolerance (Immediate Permanent Ban, No Warning)

These actions result in an immediate permanent ban. No warnings. No appeals for 90 days.

- **Scam attempts:** Posting fake giveaways, fake airdrops, fake contract addresses, fake wallet recovery services, or any message designed to extract funds or private keys from community members.
- **Impersonation:** Pretending to be a Core Team member, Moderator, or project representative. This includes similar usernames, copying profile pictures, and claiming to be "official support" in DMs.
- **Phishing links:** Posting links to phishing sites, fake exchange listings, or wallet drainers. Applies even if the user claims it was accidental — the risk to the community is too high to be lenient.
- **Coordinated price manipulation:** Organizing pump schemes, coordinating buy or sell activity across platforms, or explicitly encouraging others to manipulate markets.
- **Doxxing:** Publishing personal identifying information about any community member without their consent.
- **CSAM:** Immediate ban and report to Discord Trust and Safety.

---

### Serious Violations (Escalating Enforcement)

These violations receive escalating enforcement: Warning → 1-hour timeout → 24-hour timeout → 7-day timeout → Permanent ban.

- Repeated spam after a warning
- Hate speech or targeted harassment
- Sharing mainnet wallet seed phrases or private keys (even their own — this is a security risk and likely a scam setup)
- Repeatedly posting price predictions framed as certainties
- Evading a timeout with an alternate account
- Posting content from competitor projects in a promotional context

---

### Minor Violations (Warning, Then Escalation)

- Off-topic posting in channels with specific purposes
- Using banned words from the copy library (moon, lambo, etc.) — a reminder first, then a warning if repeated
- Posting bot commands outside `#bot-commands`
- Excessive meme posting outside `#memes`

---

### Automod Configuration (MEE6)

The following patterns trigger an automatic message deletion and a silent flag to `#mod-log`. A human moderator reviews flagged messages within 24 hours.

**Pattern category: Scam solicitation**
- "DM me" combined with any of: airdrop, giveaway, free, earn, profit
- "send [amount]" combined with any of: ETH, BTC, USDT, wallet address
- Any variation of "double your" or "multiply your"
- "seed phrase", "recovery phrase", "private key", "12 words", "24 words"
- "connect wallet", "verify wallet", "sync wallet"

**Pattern category: Impersonation signals**
- "I am from the team", "I am a developer", "official support", "admin here"
- Messages that include a name similar to any Core Team member's username (fuzzy match threshold: 2 character edits)

**Pattern category: External links from new accounts**
- Any external URL from an account under 30 days old in channels outside START HERE
- Exception: GitHub links and docs.rillcoin.com links are allow-listed

**Pattern category: Banned copy library words**
- moon, lambo, pump (when used in investment context), dump, gem, ape, degen, wagmi, ngmi, diamond hands, paper hands, rug, shill
- Note: "rug" is allowed in technical/development context (e.g., "rugpull vulnerability" in #protocol) — use contextual matching where the automod rule is preceded by a financial/investment keyword

---

### Anti-Scam Measures Specific to Crypto

Crypto communities are targeted aggressively. Beyond automod, implement these structural measures:

1. **DM protection announcement:** Pin a message in `#welcome` and `#general` that reads: "No Core Team member or Moderator will ever DM you first to offer support, airdrops, or giveaways. If someone DMs you claiming to be from the team, it is a scam. Report it using the /report command." This message should be re-posted as an announcement every time a new scam wave is detected.

2. **Team badge verification:** All Core Team members must have the Founder or Core Team Discord role visually verified. When someone without these roles claims to be team in a DM, it is immediately identifiable as false.

3. **No official support in DMs:** Core Team and Moderators use the ticket system in `#create-ticket` for private support. This is stated explicitly in the rules. It removes the plausibility of "I am official support, DM me."

4. **Scam alert channel procedure:** When a new scam variant is detected, post an alert in `#general` with the specific pattern to watch for. Update the automod rules within 24 hours to catch future instances.

5. **Invite link hygiene:** Use Carl-bot to log all invite link creation. Revoke any invite links created by non-Team members that are being distributed externally.

---

### Moderation Escalation Procedure

**Level 0 (Automod):** Message deleted, flag logged. No human intervention needed for clear violations.

**Level 1 (Moderator):** Moderator reviews flagged messages, issues warnings, applies timeouts. Documents action in `#mod-log` with context.

**Level 2 (Core Team):** Bans, appeals, and gray-area decisions escalate to Core Team. All permanent bans require a Core Team sign-off unless they are a Zero Tolerance item.

**Level 3 (Incident):** A scam wave, raid, or impersonation of team members triggers an incident. The `#incident-response` channel activates. Actions: enable anti-raid mode on Wick, post a public advisory in `#general`, review all recent joins in the past 48 hours, manually review any flagged messages from new accounts.

---

### Appeals Process

Users who believe a timeout or ban was applied in error may submit an appeal by:
- Emailing a project contact address (to be established at launch)
- Or using a designated appeal form linked in the ban DM they receive from the bot

Appeals are reviewed by Core Team within 7 days. Permanent bans are not eligible for appeal for 90 days from the date of the ban. Zero tolerance bans are not eligible for appeal.

---

## 7. Community Programs

### Ambassador Program

The Ambassador program recognizes community members who represent RillCoin authentically in external spaces — on X, at crypto events, in other communities, and in educational content.

**Eligibility Criteria:**
- Member for at least 60 days
- Holds the Contributor role
- No active moderation history (no warnings or timeouts in the past 30 days)
- Demonstrable external presence (X account, a blog, a YouTube channel, or active participation in other crypto communities)

**Application Process:**
1. Submit an application form (linked in `#roles-and-verification`) with: Discord username, X handle, why you want to represent RillCoin, one example of content you have created or would create
2. Applications reviewed by Core Team monthly
3. Approved ambassadors receive the Ambassador role and a welcome DM from the Founder

**Ambassador Responsibilities:**
- Represent the project using approved terminology and voice guidelines (no banned words, no price predictions)
- Report scam activity or impersonation they encounter in external spaces
- Provide monthly activity summaries (optional but appreciated)
- Participate in quarterly Ambassador AMAs

**Ambassador Benefits:**
- Early access to announcement drafts for awareness (not for pre-disclosure trading — this is explicitly stated and agreed to)
- Recognition in monthly community recap posts
- Access to a private `#ambassadors` channel (to be created when program has 5+ members)
- Invitation to testnet launch events

---

### Contributor Recognition

Beyond the Ambassador program, contributions to the project are recognized through the Bug Hunter role and a quarterly acknowledgment post.

**Bug Hunter awards:**
- Each confirmed Medium+ severity bug report in `#bug-reports` earns a Bug Hunter role mention
- At 3 confirmed bugs: an additional "Senior Bug Hunter" notation (via bot custom tag or a distinct role)
- At 5 confirmed bugs: acknowledged in the next major announcement post by name

**Community Contribution posts:**
- Published in `#announcements` quarterly
- Lists: active contributors, bug hunters, governance authors, ambassadors
- Authored by Core Team with input from Moderators
- Tone: factual acknowledgment, not effusive praise. "These community members contributed materially to the testnet phase."

---

### Community Events

**AMA (Ask Me Anything) Sessions**

Format: Text-based in a dedicated temporary voice channel or in `#general` with a thread, depending on size. Video AMAs may be scheduled as Discord Stage events.

Frequency: Monthly during active development phases. Quarterly after mainnet.

Structure:
1. Pre-event: collect questions in a dedicated thread in `#general` for 48 hours prior
2. Live session: Core Team answers questions, with Moderators managing thread order
3. Post-event: transcript published in a thread in `#announcements`

Ping: `@AMA Ping` role 48 hours before and 15 minutes before.

**Dev Office Hours**

Format: Informal voice channel (or text in `#development`) where a Core Team developer is available for questions.

Frequency: Bi-weekly during testnet, monthly after mainnet.

Structure: No agenda. Community members join and ask technical questions. Low-pressure, exploratory. This builds trust and surfaces issues that never make it into bug reports.

Ping: `@Dev Updates Ping` role 24 hours before.

**Testnet Launch Events**

When a significant testnet milestone is reached (first block, first decay event, public testnet open), mark it as a community event.

Structure:
1. Announcement in `#announcements` 72 hours prior
2. Live countdown post in `#testnet-status`
3. After the milestone: an open discussion thread in `#testnet-general` where participants share their experience
4. Core Team posts a technical retrospective in `#dev-updates` within 48 hours

These events are not parties — they are technical milestones. The tone is "we built something and it worked." Celebration is appropriate; hype is not.

**Governance Workshops (Future)**

As the governance system matures, schedule monthly workshops to discuss pending proposals. These are educational events — they explain the proposal, the tradeoffs, and how community members can participate in the process.

---

### Engagement-to-Role Mapping

| Activity | Signal | Role Outcome |
|---|---|---|
| Join + verify | Baseline commitment | Member role |
| Request testnet faucet | Testing the network | Testnet Participant role |
| Submit confirmed bug (Medium+) | Quality contribution | Bug Hunter role |
| Active 30 days, 500+ messages, no violations | Sustained engagement | Eligible for Contributor review |
| Contributor + 60 days + external presence | Community advocacy | Eligible for Ambassador application |
| Consistent moderation, trusted judgment | Leadership | Eligible for Moderator invitation |

---

## 8. Integration Points

### X/Twitter (@RillCoin)

- Server description and invite link in the X bio
- Each announcement in `#announcements` should have a corresponding X post (or be a repost of one)
- X posts are mirrored to `#twitter-feed` via Zapier/Make webhook
- X bio links to rillcoin.com/community or directly to the Discord invite

### GitHub

- `#github-feed` receives all commits to `main` and `testnet` branches via native webhook
- `#dev-updates` is written by the dev team manually but references GitHub commit hashes for traceability
- Bug reports in `#bug-reports` that are triaged to confirmed should have a corresponding GitHub issue opened, with the issue number posted back in the Discord thread

### Website (rillcoin.com)

- Footer: Discord link with the RillCoin icon (use `rill-icon.svg`)
- Community page (planned): embed a Discord widget showing online member count and recent activity
- Documentation pages cross-linked from `#faq`, `#protocol` pinned messages, and the decay explainer in the welcome flow

### Future: Testnet Status Bot

When built, the testnet status bot will:
- Update `#testnet-status` with a formatted embed every 10 minutes showing: block height, connected peers, last decay event (block number and amount decayed), network hash rate
- Post an alert in `#testnet-status` when the network is experiencing issues (block time significantly above target, peer count drops below threshold)

The embed format should use Flowing Blue (`#3B82F6`) as the embed color for healthy status and Accent Orange (`#F97316`) for degraded status. This provides immediate visual signal without requiring members to read the text.

### Future: Decay Calculator Bot

When built, the decay calculator bot integrates with the `/decay` command (see Bot Recommendations section). It does not require live chain data — it reads published protocol constants and computes the decay schedule mathematically.

---

## 9. Launch Checklist

Use this checklist when standing up the server for the first time.

### Pre-Launch (Before Invite Goes Public)

- [ ] Create server with correct name and icon
- [ ] Set server description
- [ ] Configure all categories and channels per this specification
- [ ] Set channel permissions for each category (verify Unverified role cannot post outside START HERE)
- [ ] Create all roles with correct colors and permissions
- [ ] Configure MEE6: automod rules, leveling, level-up announcements
- [ ] Configure Carl-bot: welcome DM, reaction roles panel, logging module, custom commands
- [ ] Configure Wick: anti-raid thresholds, account age gates
- [ ] Set up GitHub webhook in `#github-feed`
- [ ] Set up X/Twitter feed (Zapier/Make) in `#twitter-feed`
- [ ] Write and pin the welcome message in `#welcome`
- [ ] Write and pin the full rules in `#rules`
- [ ] Write and pin the FAQ (minimum 10 entries) in `#faq`
- [ ] Write and pin the role explanation in `#roles-and-verification`; attach verification button
- [ ] Create pinned explainer thread in `#protocol`
- [ ] Test the onboarding flow end-to-end with an alt account
- [ ] Verify automod catches scam phrases (test with a sandboxed account)
- [ ] Set vanity URL (once 100-member threshold is hit)
- [ ] Add Discord link to rillcoin.com and X/Twitter bio

### At Launch

- [ ] Invite Core Team members; assign Founder and Core Team roles
- [ ] Invite initial Moderators; assign Moderator roles
- [ ] Post initial `#announcements` message from the Founder introducing the server
- [ ] Post in `#dev-updates` with the current development status
- [ ] Post in `#testnet-status` with current testnet state
- [ ] Open the server to initial invite recipients (beta testers, early community members)

### First 30 Days

- [ ] Schedule the first AMA within 2 weeks of launch
- [ ] Review MEE6 and Wick logs weekly; adjust automod rules based on what gets through
- [ ] Monitor `#bug-reports` and establish the triage workflow
- [ ] Award first Bug Hunter roles to qualifying participants
- [ ] Review member onboarding: are they completing verification? Are they finding `#faq`?
- [ ] Collect feedback on the server structure in `#general` (a simple thread asking "what could be better?")
- [ ] Open the Ambassador application once the server has 50+ active members

---

*Document version 1.0 — February 2026*
*Maintained by the RillCoin Community Lead*
*Review and update quarterly or when a major platform change warrants it*
