#!/usr/bin/env python3
"""
RillCoin Discord Server Setup Script
Provisions the entire RillCoin Discord server via the Discord REST API v10.

Usage:
    python3 scripts/setup_discord.py
    python3 scripts/setup_discord.py --dry-run

Reads credentials from marketing/.env:
    DISCORD_BOT_TOKEN=your-bot-token
    DISCORD_GUILD_ID=your-guild-id
"""

import argparse
import json
import os
import sys
import time
from typing import Optional

import requests

# ---------------------------------------------------------------------------
# Environment loading
# ---------------------------------------------------------------------------

def load_env(path: str) -> dict:
    """Load key=value pairs from an .env file. Strips quotes and comments."""
    env = {}
    try:
        with open(path, "r") as fh:
            for line in fh:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                if "=" not in line:
                    continue
                key, _, value = line.partition("=")
                key = key.strip()
                value = value.strip().strip('"').strip("'")
                env[key] = value
    except FileNotFoundError:
        print(f"ERROR: .env file not found at {path}")
        sys.exit(1)
    return env


def load_credentials() -> tuple[str, str]:
    """Return (bot_token, guild_id) from environment or .env file."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    marketing_dir = os.path.dirname(script_dir)
    env_path = os.path.join(marketing_dir, ".env")

    try:
        from dotenv import load_dotenv
        load_dotenv(env_path)
    except ImportError:
        env_vars = load_env(env_path)
        for k, v in env_vars.items():
            os.environ.setdefault(k, v)

    token = os.environ.get("DISCORD_BOT_TOKEN", "")
    guild_id = os.environ.get("DISCORD_GUILD_ID", "")

    if not token:
        print("ERROR: DISCORD_BOT_TOKEN is not set in .env or environment.")
        sys.exit(1)
    if not guild_id:
        print("ERROR: DISCORD_GUILD_ID is not set in .env or environment.")
        sys.exit(1)

    return token, guild_id


# ---------------------------------------------------------------------------
# Discord REST client with rate-limit handling
# ---------------------------------------------------------------------------

API_BASE = "https://discord.com/api/v10"


class DiscordClient:
    def __init__(self, token: str, dry_run: bool = False):
        self.token = token
        self.dry_run = dry_run
        self.session = requests.Session()
        self.session.headers.update({
            "Authorization": f"Bot {token}",
            "Content-Type": "application/json",
            "User-Agent": "RillCoinSetup/1.0",
        })

    def _request(self, method: str, path: str, **kwargs) -> Optional[dict]:
        if self.dry_run:
            print(f"  [DRY-RUN] {method.upper()} {path}")
            if kwargs.get("json"):
                print(f"            payload: {json.dumps(kwargs['json'], indent=2)}")
            return {"id": "DRY_RUN_ID", "dry_run": True}

        url = f"{API_BASE}{path}"
        while True:
            resp = self.session.request(method, url, **kwargs)
            if resp.status_code == 429:
                data = resp.json()
                retry_after = data.get("retry_after", 1.0)
                print(f"  [RATE LIMIT] Waiting {retry_after:.2f}s ...")
                time.sleep(retry_after)
                continue
            if resp.status_code in (200, 201):
                return resp.json()
            if resp.status_code == 204:
                return {}
            # Log error but never print response body (may contain token fragments)
            print(f"  [HTTP {resp.status_code}] {method.upper()} {path}")
            return None

    def get(self, path: str) -> Optional[dict]:
        return self._request("get", path)

    def post(self, path: str, payload: dict) -> Optional[dict]:
        return self._request("post", path, json=payload)

    def delete(self, path: str) -> Optional[dict]:
        return self._request("delete", path)

    def put(self, path: str, payload: Optional[dict] = None) -> Optional[dict]:
        kwargs = {}
        if payload is not None:
            kwargs["json"] = payload
        return self._request("put", path, **kwargs)

    def patch(self, path: str, payload: dict) -> Optional[dict]:
        return self._request("patch", path, json=payload)


# ---------------------------------------------------------------------------
# Hex color helper
# ---------------------------------------------------------------------------

def hex_to_int(hex_color: str) -> int:
    """Convert a hex color string like '#F97316' to a decimal integer."""
    return int(hex_color.lstrip("#"), 16)


# ---------------------------------------------------------------------------
# Permission bit constants
# ---------------------------------------------------------------------------

PERM_VIEW_CHANNEL      = 1 << 10
PERM_SEND_MESSAGES     = 1 << 11
PERM_MANAGE_MESSAGES   = 1 << 13
PERM_EMBED_LINKS       = 1 << 14
PERM_ATTACH_FILES      = 1 << 15
PERM_MENTION_EVERYONE  = 1 << 17
PERM_USE_VOICE         = 1 << 21
PERM_MANAGE_CHANNELS   = 1 << 4
PERM_MANAGE_ROLES      = 1 << 28
PERM_KICK_MEMBERS      = 1 << 1
PERM_BAN_MEMBERS       = 1 << 2
PERM_MANAGE_THREADS    = 1 << 34
PERM_TIMEOUT_MEMBERS   = 1 << 40
PERM_MUTE_MEMBERS      = 1 << 22
PERM_DEAFEN_MEMBERS    = 1 << 23
PERM_MOVE_MEMBERS      = 1 << 24
PERM_VIEW_AUDIT_LOG    = 1 << 7
PERM_ADMINISTRATOR     = 1 << 3
PERM_CREATE_INSTANT_INVITE = 1 << 0
PERM_ADD_REACTIONS     = 1 << 6
PERM_READ_MESSAGES_HISTORY = 1 << 16
PERM_USE_APPLICATION_COMMANDS = 1 << 31

# A sensible set of standard member permissions (no admin powers)
STANDARD_MEMBER_PERMS = (
    PERM_VIEW_CHANNEL
    | PERM_SEND_MESSAGES
    | PERM_EMBED_LINKS
    | PERM_ATTACH_FILES
    | PERM_ADD_REACTIONS
    | PERM_READ_MESSAGES_HISTORY
    | PERM_USE_APPLICATION_COMMANDS
    # SECURITY: Invite creation restricted to Moderator+ (HIGH-02)
)

CORE_TEAM_PERMS = (
    STANDARD_MEMBER_PERMS
    | PERM_MANAGE_MESSAGES
    | PERM_MANAGE_THREADS
    | PERM_MUTE_MEMBERS
    | PERM_DEAFEN_MEMBERS
    | PERM_MOVE_MEMBERS
    # SECURITY: @everyone mention restricted to Founder only (HIGH-01)
)

MODERATOR_PERMS = (
    STANDARD_MEMBER_PERMS
    | PERM_KICK_MEMBERS
    | PERM_BAN_MEMBERS
    | PERM_MANAGE_MESSAGES
    | PERM_MANAGE_THREADS
    | PERM_VIEW_AUDIT_LOG
    | PERM_TIMEOUT_MEMBERS
    | PERM_MUTE_MEMBERS
    | PERM_CREATE_INSTANT_INVITE  # Invite creation is Moderator+ only
)

# ---------------------------------------------------------------------------
# Role definitions (highest to lowest in hierarchy)
# ---------------------------------------------------------------------------

ROLES = [
    {
        "name": "Founder",
        "color": hex_to_int("#F97316"),
        "hoist": True,
        "mentionable": False,
        "permissions": str(PERM_ADMINISTRATOR),
    },
    {
        "name": "Core Team",
        "color": hex_to_int("#3B82F6"),
        "hoist": True,
        "mentionable": False,
        "permissions": str(CORE_TEAM_PERMS),
    },
    {
        "name": "Moderator",
        "color": hex_to_int("#2A5A8C"),
        "hoist": True,
        "mentionable": False,
        "permissions": str(MODERATOR_PERMS),
    },
    {
        "name": "Contributor",
        "color": hex_to_int("#4A8AF4"),
        "hoist": True,
        "mentionable": False,
        "permissions": str(STANDARD_MEMBER_PERMS),
    },
    {
        "name": "Testnet Participant",
        "color": hex_to_int("#5DE0F2"),
        "hoist": True,
        "mentionable": False,
        "permissions": str(STANDARD_MEMBER_PERMS),
    },
    {
        "name": "Bug Hunter",
        "color": hex_to_int("#D97706"),  # Amber — distinct from Founder orange (MED-04)
        "hoist": False,
        "mentionable": False,
        "permissions": str(STANDARD_MEMBER_PERMS),
    },
    {
        "name": "Ambassador",
        "color": hex_to_int("#3B82F6"),
        "hoist": False,
        "mentionable": False,
        "permissions": str(STANDARD_MEMBER_PERMS),
    },
    {
        "name": "Member",
        "color": 0,
        "hoist": False,
        "mentionable": False,
        "permissions": str(STANDARD_MEMBER_PERMS),
    },
    {
        "name": "Unverified",
        "color": 0,
        "hoist": False,
        "mentionable": False,
        "permissions": str(PERM_VIEW_CHANNEL | PERM_READ_MESSAGES_HISTORY),
    },
    # Opt-in notification roles
    # SECURITY: mentionable=False — only Core Team/Moderator can ping these (HIGH-03)
    {
        "name": "Announcements Ping",
        "color": 0,
        "hoist": False,
        "mentionable": False,
        "permissions": str(STANDARD_MEMBER_PERMS),
    },
    {
        "name": "Testnet Ping",
        "color": 0,
        "hoist": False,
        "mentionable": False,
        "permissions": str(STANDARD_MEMBER_PERMS),
    },
    {
        "name": "Dev Updates Ping",
        "color": 0,
        "hoist": False,
        "mentionable": False,
        "permissions": str(STANDARD_MEMBER_PERMS),
    },
    {
        "name": "AMA Ping",
        "color": 0,
        "hoist": False,
        "mentionable": False,
        "permissions": str(STANDARD_MEMBER_PERMS),
    },
    {
        "name": "Governance Ping",
        "color": 0,
        "hoist": False,
        "mentionable": False,
        "permissions": str(STANDARD_MEMBER_PERMS),
    },
]

# ---------------------------------------------------------------------------
# Channel type constants
# ---------------------------------------------------------------------------

CH_TEXT        = 0
CH_VOICE       = 2
CH_CATEGORY    = 4
CH_ANNOUNCEMENT = 5
CH_FORUM       = 15

# ---------------------------------------------------------------------------
# Permission overwrite type constants
# ---------------------------------------------------------------------------

OW_ROLE   = 0
OW_MEMBER = 1


def allow_deny(allow: int = 0, deny: int = 0) -> dict:
    return {"allow": str(allow), "deny": str(deny)}


# ---------------------------------------------------------------------------
# Server structure definition
# ---------------------------------------------------------------------------
# Categories and channels.  Permission overwrite templates are applied by
# the setup function once role IDs are known.

SERVER_STRUCTURE = [
    {
        "category": "START HERE",
        "perm_template": "read_only",
        "channels": [
            {
                "name": "welcome",
                "type": CH_TEXT,
                "topic": "Learn what RillCoin is and how to get started in this server.",
                "perm_template": "read_only",
            },
            {
                "name": "rules",
                "type": CH_TEXT,
                "topic": "Community standards and moderation policy. Read before posting.",
                "perm_template": "read_only",
            },
            {
                "name": "roles-and-verification",
                "type": CH_TEXT,
                "topic": "Verify your account to unlock the server. Learn how roles are earned.",
                "perm_template": "read_only",
            },
            {
                "name": "faq",
                "type": CH_TEXT,
                "topic": "Common questions about concentration decay, the testnet, wallets, and the roadmap.",
                "perm_template": "read_only",
            },
        ],
    },
    {
        "category": "ANNOUNCEMENTS",
        "perm_template": "read_only",
        "channels": [
            {
                "name": "announcements",
                "type": CH_ANNOUNCEMENT,
                "topic": "Major releases, milestones, and protocol updates from the Core Team.",
                "perm_template": "read_only",
            },
            {
                "name": "dev-updates",
                "type": CH_ANNOUNCEMENT,
                "topic": "Technical progress notes from developers. Shipped code, not speculation.",
                "perm_template": "read_only",
            },
            {
                "name": "testnet-status",
                "type": CH_TEXT,
                "topic": "Live testnet health: block height, peer count, last decay event.",
                "perm_template": "read_only",
            },
        ],
    },
    {
        "category": "COMMUNITY",
        "perm_template": "community",
        "channels": [
            {
                "name": "general",
                "type": CH_TEXT,
                "topic": "Open discussion about RillCoin. The main gathering channel.",
                "perm_template": "community",
                "slowmode": 3,
            },
            {
                "name": "introductions",
                "type": CH_FORUM,
                "topic": "New here? Start a thread and tell us what brought you to the project.",
                "perm_template": "community",
                "thread_rate_limit": 60,
            },
            {
                "name": "price-and-markets",
                "type": CH_TEXT,
                "topic": "Market discussion only. No financial advice, no predictions, no coordination.",
                "perm_template": "community",
                "slowmode": 30,
            },
            {
                "name": "off-topic",
                "type": CH_TEXT,
                "topic": "Everything unrelated to RillCoin. Tech, culture, whatever flows.",
                "perm_template": "community",
                "slowmode": 5,
            },
            {
                "name": "memes",
                "type": CH_TEXT,
                "topic": "Community art and humor. Keep it related to Rill themes.",
                "perm_template": "community",
            },
        ],
    },
    {
        "category": "TECHNICAL",
        "perm_template": "community",
        "channels": [
            {
                "name": "protocol",
                "type": CH_TEXT,
                "topic": "Decay mechanics, consensus rules, fee structure, and proof-of-work design.",
                "perm_template": "community",
                "slowmode": 5,
            },
            {
                "name": "development",
                "type": CH_TEXT,
                "topic": "Codebase discussion, contribution questions, and architecture decisions.",
                "perm_template": "community",
                "slowmode": 3,
            },
            {
                "name": "research",
                "type": CH_FORUM,
                "topic": "Longer-form technical analysis, external papers, and economic modeling.",
                "perm_template": "community",
                "thread_rate_limit": 300,
            },
            {
                "name": "node-operators",
                "type": CH_TEXT,
                "topic": "Running a node or mining. Configuration, sync issues, hardware discussion.",
                "perm_template": "community",
                "slowmode": 5,
            },
        ],
    },
    {
        "category": "TESTNET",
        "perm_template": "community",
        "channels": [
            {
                "name": "testnet-general",
                "type": CH_TEXT,
                "topic": "General testnet discussion, questions, and coordination.",
                "perm_template": "community",
                "slowmode": 5,
            },
            {
                "name": "bug-reports",
                "type": CH_FORUM,
                "topic": "Structured bug reports only. Use the pinned template. Each report becomes a thread.",
                "perm_template": "community",
                "thread_rate_limit": 300,
            },
            {
                "name": "testnet-wallets",
                "type": CH_TEXT,
                "topic": "Share testnet wallet addresses for testing. Mainnet addresses are not permitted here.",
                "perm_template": "community",
                "slowmode": 10,
            },
            {
                "name": "faucet",
                "type": CH_TEXT,
                "topic": "Request testnet coins with /faucet. One request per user per 24 hours.",
                "perm_template": "community",
                "slowmode": 10,
            },
        ],
    },
    {
        "category": "GOVERNANCE",
        "perm_template": "community",
        "channels": [
            {
                "name": "governance-general",
                "type": CH_TEXT,
                "topic": "Discussion of governance process, philosophy, and community direction.",
                "perm_template": "governance",
                "slowmode": 10,
            },
            {
                "name": "proposals",
                "type": CH_FORUM,
                "topic": "Formal improvement proposals. Use the structured format: title, summary, motivation, specification.",
                "perm_template": "governance",
                "thread_rate_limit": 300,
            },
            {
                "name": "voting",
                "type": CH_TEXT,
                "topic": "Governance vote results and links. Active when on-chain voting launches.",
                "perm_template": "read_only",
            },
        ],
    },
    {
        "category": "SUPPORT",
        "perm_template": "community",
        "channels": [
            {
                "name": "support-general",
                "type": CH_TEXT,
                "topic": "Public support for wallet setup, sync issues, and decay calculation questions.",
                "perm_template": "community",
                "slowmode": 5,
            },
            {
                "name": "create-ticket",
                "type": CH_TEXT,
                "topic": "Open a private support ticket with the team. Click the button below to start.",
                "perm_template": "read_only",
            },
        ],
    },
    {
        "category": "TEAM",
        "perm_template": "team_only",
        "channels": [
            {
                "name": "team-general",
                "type": CH_TEXT,
                "topic": "Day-to-day coordination.",
                "perm_template": "team_only",
            },
            {
                "name": "mod-log",
                "type": CH_TEXT,
                "topic": "Automated moderation action log.",
                "perm_template": "team_only",
            },
            {
                "name": "incident-response",
                "type": CH_TEXT,
                "topic": "Active incident handling.",
                "perm_template": "team_only",
            },
            {
                "name": "community-feedback",
                "type": CH_TEXT,
                "topic": "Digested community feedback for the dev team.",
                "perm_template": "team_only",
            },
        ],
    },
    {
        "category": "BOTS",
        "perm_template": "community",
        "channels": [
            {
                "name": "bot-commands",
                "type": CH_TEXT,
                "topic": "Run bot commands here. Keeps other channels clean.",
                "perm_template": "community",
            },
            {
                "name": "github-feed",
                "type": CH_TEXT,
                "topic": "Automated commits, pull requests, and releases from the RillCoin repository.",
                "perm_template": "read_only",
            },
            {
                "name": "twitter-feed",
                "type": CH_TEXT,
                "topic": "Automated feed of posts from @RillCoin on X.",
                "perm_template": "read_only",
            },
            {
                "name": "crypto-news",
                "type": CH_TEXT,
                "topic": "Aggregated crypto industry news from curated RSS sources. Powered by MonitoRSS.",
                "perm_template": "read_only",
            },
            {
                "name": "price-ticker",
                "type": CH_TEXT,
                "topic": "Auto-updating price data for BTC, ETH, and top 20 by market cap. Powered by CoinGecko Bot.",
                "perm_template": "read_only",
            },
            {
                "name": "regulatory-watch",
                "type": CH_TEXT,
                "topic": "Crypto regulation and legal news from SEC, CFTC, and industry sources. Powered by MonitoRSS.",
                "perm_template": "read_only",
            },
            {
                "name": "whale-alerts",
                "type": CH_TEXT,
                "topic": "Large transaction alerts on major chains. Powered by Whale Alert Bot.",
                "perm_template": "read_only",
            },
        ],
    },
]

# ---------------------------------------------------------------------------
# Pinned message content
# The outer ``` fences in the source document wrap the actual Discord
# message text.  Content below is extracted verbatim from those fences.
# ---------------------------------------------------------------------------

# Each entry: channel_name -> list of message strings (in pin order)
PINNED_MESSAGES: dict[str, list[str]] = {
    "welcome": [
        """\
## Welcome to RillCoin

RillCoin uses progressive concentration decay to prevent whale accumulation. The more you hoard, the more flows back to miners. A cryptocurrency designed for circulation, not concentration.

**"Wealth should flow like water."**

---

**What is this project?**

RillCoin is a proof-of-work cryptocurrency with a built-in concentration decay mechanism that redistributes dormant holdings to active miners. It is currently in active development. The testnet is live and open to participation.

---

**Getting started:**

1. Read the rules → #rules
2. Verify your account → #roles-and-verification
3. Check the FAQ if you want to understand decay before anything else → #faq
4. Join the conversation → #general

---

**Security reminder:**
No Core Team member or Moderator will ever DM you first to offer support, airdrops, or giveaways. If someone DMs you claiming to be from the team, it is a scam. Report it using the `/report` command or open a ticket in #create-ticket.

**Protect yourself from DM spam:**
Go to Server Settings (click the server name) → Privacy Settings → disable "Allow direct messages from server members." This prevents strangers in this server from messaging you directly.""",
        """\
## What is concentration decay?

RillCoin implements progressive concentration decay. When a wallet's balance exceeds defined thresholds, the portion above each threshold decays over time and flows into the mining pool, where it is redistributed to active miners.

The larger your balance, the faster the excess decays. This is not a penalty — it is a circulation incentive. Wealth should flow like water, not pool in reservoirs.

---

**Key links:**
- Documentation: [docs.rillcoin.com](URL_PLACEHOLDER)
- GitHub: [github.com/rillcoin](URL_PLACEHOLDER)
- Website: [rillcoin.com](URL_PLACEHOLDER)
- Testnet guide: [docs.rillcoin.com/testnet](URL_PLACEHOLDER)
- Whitepaper: [docs.rillcoin.com/whitepaper](URL_PLACEHOLDER)

---

**Channels to explore:**
- Technical discussion → #protocol
- Node setup and mining → #node-operators
- Testnet participation → #testnet-general
- Testnet coins → #faucet
- Governance → #governance-general""",
    ],
    "rules": [
        """\
## RillCoin Community Rules

These rules apply to all channels, threads, and interactions in this server.

---

**Zero Tolerance — Immediate Permanent Ban**

The following result in a permanent ban with no warning. These are not negotiable.

- **Scam attempts** — fake giveaways, fake airdrops, fake contract addresses, wallet recovery services, or any message designed to extract funds or private keys
- **Impersonation** — pretending to be a Core Team member, Moderator, or project representative; this includes similar usernames, copied profile pictures, or claiming to be "official support" in DMs
- **Phishing links** — links to phishing sites, fake exchange listings, or wallet drainers
- **Coordinated price manipulation** — organizing buy/sell coordination or explicitly encouraging others to manipulate markets
- **Doxxing** — publishing personal identifying information about any community member without consent

Permanent bans are not eligible for appeal for 90 days. Zero tolerance bans are not eligible for appeal at any time.

---

**Serious Violations — Escalating Enforcement**

Warning → 1-hour timeout → 24-hour timeout → 7-day timeout → Permanent ban.

- Repeated spam after a warning
- Hate speech or targeted harassment
- Sharing wallet seed phrases or private keys (yours or anyone else's)
- Repeatedly posting price predictions framed as certainties
- Evading a timeout with an alternate account
- Posting competitor project content in a promotional context""",
        """\
**Minor Violations — Warning, then escalation**

- Off-topic posting in channels with specific purposes
- Using banned words (see list below)
- Running bot commands outside #bot-commands
- Excessive meme posting outside #memes

---

**Banned words and phrases**
The following are not permitted in this server in an investment or hype context:
moon, lambo, pump, dump, gem, ape, degen, wagmi, ngmi, diamond hands, paper hands, rug, shill

---

**Anti-scam reminder**
No Core Team member or Moderator will ever DM you first. We do not offer support, airdrops, or giveaways via DM. Official support goes through #create-ticket. If someone contacts you claiming to be from the RillCoin team, report it immediately using `/report`.

---

**Appeals**
If you believe a moderation action was applied in error, email appeals@rillcoin.com or use the appeal form linked in your ban notice. Appeals are reviewed by Core Team within 7 days.

---

**Enforcement is consistent and documented.** Moderators follow this policy. If you observe a rule violation, use `/report` or open a ticket in #create-ticket rather than engaging directly.""",
    ],
    "roles-and-verification": [
        """\
## Roles and Verification

**Step 1: Verify your account**
Click the verification button below this message to complete account verification. Your account must be at least 14 days old. Accounts under 30 days old may be asked to complete a CAPTCHA.

On success, you receive the **Member** role and full access to the server.

---

**Role overview — Team**

**Founder** (orange name) — Project founders. Administrator access. If you see this role, you are speaking with project leadership.

**Core Team** (blue name) — Full-time contributors building the protocol. Manage messages, threads, and announcements.

**Moderator** — Community enforcement. Day-to-day moderation. Distinguished from Core Team to clarify who is building vs. who is enforcing.

---

**Role overview — Earned**

**Contributor** (blue-light) — Unlocks access to #governance-general and #proposals. Earned by: submitting a verified bug report, merging a GitHub contribution, authoring an approved governance proposal, or 30 days active with 500+ messages and no violations (reviewed manually).

**Testnet Participant** (cyan) — Automatically assigned when you use the faucet and confirm receipt. Shows you are testing the network.

**Bug Hunter** (orange) — Awarded manually by Core Team for each confirmed Medium+ severity bug report. Can be earned multiple times.

**Ambassador** (blue) — Community evangelists representing RillCoin in external spaces. Requires application. See Pinned Message 2 for eligibility.""",
        """\
**Role overview — General**

**Member** — Verified community member. Access to all public channels. Assigned automatically after verification.

**Unverified** — New joins before verification. Read-only access to #welcome, #rules, #roles-and-verification, and #faq only.

---

**Opt-in notification roles**
Self-assign pings for content you care about. Run `/role` in #bot-commands.

- **Announcements Ping** — Major releases and milestones
- **Testnet Ping** — Testnet events and status alerts
- **Dev Updates Ping** — Technical progress posts
- **AMA Ping** — Upcoming AMAs and office hours
- **Governance Ping** — New governance proposals

---

**Ambassador application**
Eligibility: Member for 60+ days, Contributor role, no moderation history in the past 30 days, and a demonstrable external presence (X account, blog, YouTube, or active participation in other communities).

Apply here: [rillcoin.com/ambassador](URL_PLACEHOLDER)
Applications reviewed monthly by Core Team.

---

**Bug Hunter nominations**
Confirmed Medium+ severity bug reports earn this role automatically. No application needed — do quality work in #bug-reports and it follows.""",
    ],
    "faq": [
        """\
## Frequently Asked Questions

---

**What is RillCoin?**
RillCoin is a proof-of-work cryptocurrency with a built-in concentration decay mechanism that redistributes dormant holdings to active miners. It is designed around a principle of circulation over concentration. The project is open-source and currently in testnet.

---

**What is concentration decay?**
When a wallet's balance exceeds defined thresholds, the portion above each threshold decays over time and flows into the decay pool, where it is redistributed to active miners. The larger your balance, the faster the excess decays. This is not a penalty — it is a circulation incentive. Wealth should flow like water, not pool in reservoirs.

---

**How does decay work technically?**
Decay is computed using a sigmoid curve applied to balances above defined concentration thresholds. Each threshold tier applies a progressively higher decay rate to the excess balance. Decay is applied per block and expressed as a fraction of the effective balance above the threshold. The decayed amount flows into the decay pool and is distributed to miners as part of the block reward.

---

**What are the decay thresholds?**
The specific threshold values and decay rates are defined in the protocol constants. See the technical documentation for the current mainnet parameters: [docs.rillcoin.com/protocol/decay](URL_PLACEHOLDER). Testnet parameters may differ from mainnet parameters during the testing phase.

---

**What is "effective balance"?**
Your effective balance is the portion of your wallet balance that is not subject to active decay — the amount below the first decay threshold. Holdings above the thresholds are subject to decay at rates defined by the protocol.""",
        """\
**How do I run a node?**
The RillCoin node software is available on GitHub: [github.com/rillcoin/rill](URL_PLACEHOLDER). Full node setup documentation is at [docs.rillcoin.com/node](URL_PLACEHOLDER). For questions and troubleshooting, use #node-operators.

Basic requirements: a machine with sufficient disk space for the chain, a stable internet connection, and the ability to open the required ports. Specific hardware recommendations are in the documentation.

---

**How do I get testnet coins?**
Use the faucet in #faucet. Run the command `/faucet <your-testnet-address>`. You can request once every 24 hours per account. Your testnet address must be in the correct RillCoin testnet address format — the bot will reject addresses that do not match.

On successful delivery, you will receive the Testnet Participant role automatically.

---

**What is the roadmap?**
The current public roadmap is at [rillcoin.com/roadmap](URL_PLACEHOLDER). In brief: the project is in active testnet. Mainnet launch follows once the protocol is stable and audited. Governance tooling is planned post-mainnet.

---

**Is there a token sale or ICO?**
No. There is no token sale, no ICO, and no pre-mine. RillCoin is distributed exclusively through proof-of-work mining. Any offer to sell you RillCoin before it is mineable on mainnet is a scam.

---

**How do I contribute to the project?**
Read the contribution guide at [docs.rillcoin.com/contributing](URL_PLACEHOLDER) and the open issues on GitHub: [github.com/rillcoin/rill](URL_PLACEHOLDER). Discuss your approach in #development before opening a large pull request. All contributions must pass `cargo clippy --workspace -- -D warnings` and `cargo test --workspace`.""",
        """\
**Where is the code?**
The full source is on GitHub: [github.com/rillcoin/rill](URL_PLACEHOLDER)

The codebase is a Cargo workspace with the following crate structure:
`rill-core` → `rill-decay` → `rill-consensus` → `rill-network` → `rill-wallet` → `rill-node`

---

**Where can I read the whitepaper?**
[docs.rillcoin.com/whitepaper](URL_PLACEHOLDER)

---

**How do I report a scam or impersonation?**
Use `/report` in any channel, or open a private ticket in #create-ticket. Do not engage with the scammer. No Core Team member or Moderator will ever DM you first — if someone does claiming to be from the team, it is a scam.

---

**I have a question that is not answered here. Where do I ask?**
- General questions → #general
- Technical protocol questions → #protocol
- Node setup and mining → #node-operators
- Testnet participation → #testnet-general
- Bug reports → #bug-reports (use the pinned template)
- Private support → #create-ticket

---

*This FAQ is updated quarterly. Last updated: February 2026.*""",
    ],
    "general": [
        """\
## Welcome to #general

This is the main community discussion channel. On-topic conversation is preferred. Off-topic discussion is tolerated if it does not dominate — for everything unrelated to RillCoin, use #off-topic.

---

**Quick links:**
- New here? Start in #faq and #rules
- Technical questions → #protocol or #development
- Node and mining questions → #node-operators
- Testnet → #testnet-general
- Bot commands → #bot-commands

---

**Anti-scam reminder:**
No Core Team member or Moderator will ever DM you first. If someone DMs you claiming to be from the RillCoin team and offering support, airdrops, or giveaways, it is a scam. Use `/report` or open a ticket in #create-ticket.

Team members are identifiable by their colored names: **Founder** (orange) and **Core Team** (blue) are the only roles that represent the project officially.""",
    ],
    "price-and-markets": [
        """\
## #price-and-markets — Ground Rules

This channel exists so market discussion stays out of #general. It is a designated space, not an endorsement of any particular price view.

---

**What is permitted:**
- Discussion of exchange listings, trading pairs, and market mechanics
- Questions about how the concentration decay mechanism interacts with market dynamics
- Links to price data from established sources
- Factual discussion of market events

**What is not permitted:**
- Price predictions stated as certainties
- Financial advice of any kind ("you should buy/sell")
- Coordinated buy or sell language — this server does not organize market activity
- Banned words: moon, lambo, pump, dump, gem, ape, degen, wagmi, ngmi, diamond hands, paper hands, shill

---

**Standard reminder:**
Nothing posted in this channel constitutes financial advice. RillCoin does not make price predictions or financial promises. Participation in any cryptocurrency involves risk. Make your own decisions.

---

Repeated violations of these ground rules will result in removal from this channel or the server under the standard escalation policy in #rules.""",
    ],
    "protocol": [
        """\
## Concentration Decay — Protocol Explainer

This is the differentiating mechanism of RillCoin. Read this before asking decay questions.

---

**The core idea**
RillCoin implements progressive concentration decay. Wallet balances above defined thresholds decay over time. The decayed amount flows into the decay pool, which is distributed to miners as part of the block reward.

This is a circulation incentive built into the protocol itself — not a fee, not a tax, not a burn. It is a property of holding a large balance.

---

**How the decay rate is determined**
Decay uses a sigmoid curve applied to the balance above each threshold. The sigmoid curve means:
- Small amounts above a threshold decay slowly
- Large amounts above a threshold decay faster
- The decay rate increases progressively across tiers — higher thresholds apply higher rates

This prevents abrupt cliffs while still creating meaningful pressure against large concentration.

---

**What "effective balance" means**
Your effective balance is the portion of your holdings below the first decay threshold. It does not decay. Holdings above the thresholds are subject to decay at the rates defined by the protocol constants.

---

**The decay pool**
Decayed amounts accumulate in the decay pool per block. The pool balance is distributed to miners as part of the block reward for that block. Miners with more hash power receive a proportionally larger share.""",
        """\
**Arithmetic and precision**
All consensus math uses integer arithmetic with fixed-point precision at 10^8 (similar to Bitcoin's satoshi precision). No floating-point arithmetic is used in the protocol. This ensures deterministic results across all implementations.

---

**Proof of work**
RillCoin uses proof-of-work consensus. Block headers are hashed using SHA-256. The current testnet uses a simplified PoW implementation; the mainnet protocol uses RandomX to provide ASIC resistance.

---

**Fee structure**
A minimum fee is required for all transactions. The minimum fee is defined in the protocol constants. Fees are distributed to miners alongside the decay pool distribution.

---

**Where to find the formal specification**
Full protocol specification: [docs.rillcoin.com/protocol](URL_PLACEHOLDER)
Whitepaper (decay mechanism section): [docs.rillcoin.com/whitepaper#decay](URL_PLACEHOLDER)
Source of truth for constants: `rill-core/src/constants.rs` on [GitHub](URL_PLACEHOLDER)

---

**Discussion guidelines for this channel**
High-signal conversation is expected here. If you are asking a basic question, check #faq first. If you are making a technical claim, link your reasoning. Threads are encouraged for extended analysis.""",
    ],
    "development": [
        """\
## Contributing to RillCoin

The codebase is open-source and contributions are welcome.

---

**Before you start:**
- Read the contribution guide: [docs.rillcoin.com/contributing](URL_PLACEHOLDER)
- Check open issues on GitHub: [github.com/rillcoin/rill/issues](URL_PLACEHOLDER)
- For significant changes, discuss your approach here or open a draft PR before writing production code

---

**Repository structure:**
The project is a Cargo workspace — 6 library crates and 3 binaries.
`rill-core` → `rill-decay` → `rill-consensus` → `rill-network` → `rill-wallet` → `rill-node`

---

**Code standards (non-negotiable):**
- Rust 2024 edition, stable toolchain, MSRV 1.85
- `cargo clippy --workspace -- -D warnings` must pass with zero warnings
- `cargo test --workspace` must pass
- All consensus math uses checked arithmetic (`checked_add`, `checked_mul`)
- No floating-point in protocol logic — fixed-point u64 with 10^8 precision
- Public APIs require doc comments and proptest coverage

---

**Commit and branch conventions:**
- Branch: `<crate>/<description>` (e.g., `rill-decay/fix-threshold-calculation`)
- Commit: `<crate>: <description>` (e.g., `rill-core: implement Transaction struct`)

---

**Bug reports go to #bug-reports**, not here. Use the template pinned there.""",
    ],
    "research": [
        """\
## #research — Posting Guidelines

This channel is for longer-form technical content: analysis, external papers, economic modeling of the decay mechanism, and formal arguments about protocol design.

---

**What belongs here:**
- Links to academic papers relevant to proof-of-work, concentration mechanisms, or monetary economics, with a summary of why they are relevant
- Original analysis of the decay mechanism (modeling decay rates, threshold sensitivity, miner incentives)
- Economic arguments for or against specific protocol parameters
- Comparative analysis of RillCoin's approach against other concentration-resistance mechanisms

**What does not belong here:**
- Short questions (use #protocol or #general)
- Bug reports (use #bug-reports with the template)
- Price speculation
- Content without substantive technical or economic content

---

**Format:**
This is a forum channel. Each post requires a title. Use threads for extended discussion. If you are sharing an external paper, include a brief summary of the relevant sections — do not post a link with no context.

---

**Tone:**
Rigorous and direct. Disagreement is welcome; keep it about the ideas, not the people.""",
    ],
    "node-operators": [
        """\
## Running a RillCoin Node — Quick Start

Full documentation: [docs.rillcoin.com/node](URL_PLACEHOLDER)

---

**Prerequisites:**
- Operating system: Linux (Ubuntu 22.04+ recommended), macOS, or Windows (WSL2)
- Disk space: Minimum 20 GB free (testnet); mainnet requirements will be higher
- RAM: Minimum 2 GB
- Network: Stable connection; ability to open inbound TCP port (default: 30333)
- Rust toolchain: stable, MSRV 1.85+ (not required if using pre-built binaries)

---

**Installation (from source):**
```
git clone https://github.com/rillcoin/rill
cd rill
cargo build --release --bin rill-node
```

**Running the node:**
```
./target/release/rill-node --network testnet
```

For full flag reference:
```
./target/release/rill-node --help
```

---

**Connecting to testnet:**
The testnet bootstrap peers are listed in the documentation: [docs.rillcoin.com/testnet/peers](URL_PLACEHOLDER)

Your node will begin syncing from the genesis block. Initial sync time depends on your connection speed and the current chain height.""",
        """\
**Mining (testnet):**
Mining is supported on testnet. To enable mining, provide a wallet address to receive rewards:
```
./target/release/rill-node --network testnet --mine --wallet <your-testnet-address>
```

The miner competes on the current PoW target. On testnet, the RandomX implementation is active. ASIC mining is not advantageous due to the memory-hard algorithm.

---

**Common issues:**

*Node won't connect to peers*
- Verify that your firewall allows inbound connections on the node port
- Check that the bootstrap peer addresses in your config are current — see [docs.rillcoin.com/testnet/peers](URL_PLACEHOLDER)

*Sync is stalled*
- Check the `#testnet-status` channel for network health
- Restart the node with `--resync` flag if blocks are not advancing after 30 minutes

*Decay calculation looks wrong*
- Decay is applied per block. If you are checking balances mid-block, the displayed balance may not yet reflect the latest decay application. Wait for block confirmation.

---

**Reporting node issues:**
If you encounter a bug, use the template in #bug-reports. Include your node version (`rill-node --version`), OS, and the full error output.

For configuration questions, ask in this channel. For suspected bugs, use #bug-reports.""",
    ],
    "testnet-general": [
        """\
## Testnet Participation

The RillCoin testnet is live and open to the public. Your participation matters — the more nodes, wallets, and transactions running on testnet, the more confidently the protocol can be validated before mainnet.

---

**How to get started:**

1. Get the node software: [github.com/rillcoin/rill](URL_PLACEHOLDER)
2. Follow the setup guide: [docs.rillcoin.com/testnet](URL_PLACEHOLDER)
3. Get testnet coins from the faucet in #faucet
4. Run transactions, test the decay mechanism, and report anything unexpected

---

**Roles you can earn:**
- **Testnet Participant** — Automatically assigned when you receive faucet coins
- **Bug Hunter** — Manually awarded for confirmed Medium+ severity bug reports

---

**Channels to use:**
- Setup questions → #node-operators
- Bug reports → #bug-reports (use the template)
- Wallet address sharing for testing → #testnet-wallets
- Testnet coins → #faucet
- Live network status → #testnet-status

---

**Important:** Testnet coins have no monetary value. Testnet parameters (thresholds, decay rates, block time) may differ from the final mainnet parameters. Do not use mainnet wallet addresses on testnet.

Full testnet documentation: [docs.rillcoin.com/testnet](URL_PLACEHOLDER)""",
    ],
    "bug-reports": [
        """\
## Bug Report Template

Every bug report must use this format. Posts without the required fields will be closed without triage.

Each report creates a thread. The dev team triages all reports within 7 days.

---

**Copy and fill in the template below:**

```
**Title:** [One-sentence description of the issue]

**Node version:** [Output of `rill-node --version`]
**OS:** [e.g., Ubuntu 22.04, macOS 14.3, Windows 11 WSL2]
**Network:** [testnet / mainnet]

**Steps to reproduce:**
1.
2.
3.

**Expected behavior:**
[What you expected to happen]

**Actual behavior:**
[What actually happened]

**Logs / error output:**
[Paste relevant log lines here, wrapped in code blocks]

**Severity (your assessment):**
[ ] Low — cosmetic or minor inconvenience
[ ] Medium — incorrect behavior, workaround exists
[ ] High — significant malfunction, no workaround
[ ] Critical — data loss, security issue, consensus failure
```

---

**After submitting:**
A Core Team member will reply in your thread with a triage status. Confirmed Medium+ bugs earn the **Bug Hunter** role. If your bug is linked to a GitHub issue, the issue number will be posted in your thread.

**Security vulnerabilities:** Do not post publicly. Email [security@rillcoin.com](URL_PLACEHOLDER) instead.""",
    ],
    "faucet": [
        """\
## Testnet Faucet

The faucet distributes testnet RillCoin for development and testing.

---

**How to request:**
Run this command in this channel:
```
/faucet <your-testnet-address>
```

Replace `<your-testnet-address>` with your RillCoin testnet wallet address.

---

**Rate limits:**
- One request per Discord account per 24 hours
- One request per wallet address per 24 hours

Both limits apply independently. Creating multiple Discord accounts to bypass the rate limit will result in a ban.

---

**Address format:**
The bot validates address format before processing. Addresses that do not match the RillCoin testnet address format will be rejected. Do not use mainnet addresses here.

To generate a testnet wallet address, see the wallet documentation: [docs.rillcoin.com/wallet](URL_PLACEHOLDER)

---

**After your request:**
The bot will confirm receipt and post the expected transaction time. Once the transaction is confirmed on chain, you will automatically receive the **Testnet Participant** role.

If the faucet does not respond within 10 minutes, check #testnet-status for network health. If the network is healthy and the faucet is unresponsive, report it in #support-general.

---

**Testnet coins have no monetary value.**""",
    ],
    "governance-general": [
        """\
## Governance — Current Status

RillCoin governance is currently in an advisory phase. There is no on-chain voting yet. This channel exists to build the culture and practice of community governance before the formal mechanisms are deployed.

---

**What governance means here, right now:**
- Community members propose and discuss changes to the protocol, parameters, and community direction
- The Core Team reads this channel and takes community input seriously
- No vote here is binding — the Core Team makes final decisions during this phase
- This will change as the project matures and formal governance tooling is deployed

---

**How to participate:**
- Post your thoughts on protocol direction, parameter choices, or process questions here
- To submit a formal proposal, use #proposals with the structured format
- Governance Ping: opt in via `/role` in #bot-commands to receive pings when new proposals open

---

**Access:**
This channel is open to **Contributor** role and above. To earn Contributor, see the requirements in #roles-and-verification.

---

**What comes next:**
On-chain voting is planned post-mainnet. When snapshot.org or on-chain governance launches, results and links will appear in #voting. This channel will transition from advisory to participatory at that point.

Questions about the governance roadmap → [docs.rillcoin.com/governance](URL_PLACEHOLDER)""",
    ],
    "proposals": [
        """\
## Proposal Template and Guidelines

Use this format for every formal proposal. Posts that do not follow the structure will be asked to revise before discussion begins.

This is a forum channel. Each proposal is a separate post with a title.

---

**Proposal template:**

```
**RCP-[number]: [Short title]**

**Summary**
[2–4 sentences. What are you proposing and why?]

**Motivation**
[What problem does this solve? What is the current behavior and why is it insufficient?]

**Specification**
[Precise description of the proposed change. For protocol changes, include: affected constants or parameters, the proposed new values, and the mathematical or logical reasoning. For process changes, describe the new procedure step by step.]

**Drawbacks**
[What are the honest downsides or risks of this proposal? This section is required. Proposals that do not acknowledge tradeoffs are not credible.]

**Alternatives considered**
[What other approaches did you consider and why did you not propose them?]

**Open questions**
[What remains unresolved? What feedback are you specifically seeking?]
```

---

**Numbering:** Use the next sequential RCP (RillCoin Proposal) number. Check existing proposals to avoid duplicates.

**Discussion:** Use the thread on your proposal post. Keep top-level discussion in the thread, not in #governance-general.

**Status:** Core Team will add a status tag to proposals: Draft → Under Review → Accepted / Rejected / Deferred.""",
    ],
    "support-general": [
        """\
## Getting Help

This channel is for public support questions. Use it for wallet setup, sync issues, decay calculation questions, and general troubleshooting.

---

**Before posting, check these resources:**
- FAQ → #faq
- Protocol documentation → [docs.rillcoin.com](URL_PLACEHOLDER)
- Node setup guide → [docs.rillcoin.com/node](URL_PLACEHOLDER)
- #node-operators for node-specific questions
- #bug-reports if you believe you have found a bug (use the template)

---

**When posting a support question, include:**
- What you are trying to do
- What you expected to happen
- What actually happened
- Your node or wallet version, if relevant
- Your operating system, if relevant
- Any error messages (paste the full text or a screenshot)

The more context you provide, the faster someone can help.

---

**For private support** (if your question involves wallet addresses, keys, or sensitive account information):
Use #create-ticket to open a private thread with the support team. Do not post private keys or seed phrases anywhere in this server — not in this channel, not in a ticket.

---

**Security reminder:**
No one from the Core Team will DM you first to offer support. If someone DMs you claiming to be support and asking for your seed phrase, wallet address, or any credentials — it is a scam. Report it using `/report`.""",
    ],
    "bot-commands": [
        """\
## Bot Commands

Run all bot commands here. Running commands in other channels may result in a reminder from moderators.

---

**Notification role management:**
```
/role add <role-name>
/role remove <role-name>
```
Available notification roles: Announcements Ping, Testnet Ping, Dev Updates Ping, AMA Ping, Governance Ping

---

**Information commands:**
```
!decay       — Explanation of concentration decay with documentation link
!whitepaper  — Link to the RillCoin whitepaper
!testnet     — Current testnet status link and faucet instructions
!roadmap     — Link to the public roadmap
!github      — Link to the GitHub repository
```

---

**Decay calculator (coming soon):**
```
/decay <balance>
```
Estimates the decay schedule for a given balance based on published protocol constants. Not live chain data — a model estimate.

---

**Faucet requests go to #faucet**, not here:
```
/faucet <testnet-address>
```

---

**Reporting:**
```
/report
```
Opens a report flow for rule violations, scam attempts, or impersonation. Reports go to the moderation team.""",
    ],
    "crypto-news": [
        """\
## #crypto-news — Industry News Feed

This channel delivers curated crypto industry news from established sources. It is automated and read-only.

---

**Sources:**
- CoinDesk
- The Block
- Bitcoin Magazine
- Decrypt

**Filters:** Content is filtered to topics relevant to RillCoin: proof-of-work, monetary policy, protocol design, and Layer 1 developments. General altcoin news and token launch announcements are excluded.

**Bot:** MonitoRSS — an open-source RSS-to-Discord bot with 7+ years of uptime and 500M+ articles delivered. We use it instead of dedicated crypto news bots to maintain full control over sources and avoid spam or shilling from bot-curated feeds.

---

**This channel is read-only.** Members cannot post here. To discuss a news item, share the link in #general or #price-and-markets.""",
    ],
    "price-ticker": [
        """\
## #price-ticker — Top 20 Crypto Prices

This channel provides auto-updating price data for BTC, ETH, and the top 20 cryptocurrencies by market cap. It is automated and read-only.

---

**Bot:** CoinGecko Bot — the official CoinGecko Discord bot. Free tier, supports 4000+ coins, formatted embeds, and scheduled price summaries.

**What is tracked:** Bitcoin, Ethereum, and the top 20 by market cap. Once RillCoin is listed on exchanges post-mainnet, RILL will be added to the tracked assets.

**Complement:** This channel provides raw price data. For market discussion, use #price-and-markets.

---

**This channel is read-only.** Members cannot post here. Nothing posted in this channel constitutes financial advice. RillCoin does not make price predictions or financial promises.""",
    ],
    "regulatory-watch": [
        """\
## #regulatory-watch — Crypto Regulation and Legal News

This channel delivers low-volume regulatory and legal developments relevant to the cryptocurrency industry. It is automated and read-only.

---

**Sources:**
- SEC Litigation Releases (official RSS)
- CFTC News (official RSS)
- CoinDesk — Policy and Regulation
- The Block — Regulation

**Volume:** Expect 2-5 posts per week. This is intentionally low-noise.

**Bot:** MonitoRSS — the same bot that powers #crypto-news, configured with separate feeds for regulatory content.

---

**Why this channel exists:** RillCoin takes regulatory context seriously. This feed is useful for governance participants, protocol designers, and institutional observers who need to stay informed about the legal landscape affecting proof-of-work cryptocurrencies.

**This channel is read-only.** To discuss regulatory developments, use #governance-general or #general.""",
    ],
    "whale-alerts": [
        """\
## #whale-alerts — Large Transaction Monitoring

This channel will display alerts when large cryptocurrency transactions occur on major chains. It is automated and read-only.

---

**Bot:** Whale Alert Bot — free tier, tracks large transactions on BTC, ETH, and major chains.

**Current status:** This channel is a stub during the pre-mainnet phase. Whale tracking for major chains will be activated when the community finds it useful. Post-mainnet, RillCoin-specific large movement alerts will be added via a custom bot integration.

**Why this channel exists:** Large transaction monitoring provides relevant market intelligence for a proof-of-work community. Tracking whale movements on BTC and ETH helps contextualize broader market dynamics.

---

**This channel is read-only.** To discuss large transactions or market movements, use #price-and-markets.""",
    ],
}

# ---------------------------------------------------------------------------
# Main setup orchestration
# ---------------------------------------------------------------------------

class ServerSetup:
    def __init__(self, client: DiscordClient, guild_id: str, dry_run: bool):
        self.client = client
        self.guild_id = guild_id
        self.dry_run = dry_run

        # Maps populated during setup, used when building permission overwrites
        self.role_ids: dict[str, str] = {}   # role_name -> role_id
        self.everyone_role_id: str = guild_id  # @everyone role_id equals guild_id

        # channel_name -> channel_id, populated after creation
        self.channel_ids: dict[str, str] = {}

    # ------------------------------------------------------------------
    # Step 1: Delete default channels
    # ------------------------------------------------------------------

    def delete_default_channels(self) -> None:
        print("\n--- Deleting default channels ---")
        result = self.client.get(f"/guilds/{self.guild_id}/channels")
        if not result:
            print("  Could not fetch existing channels.")
            return

        channels = result if isinstance(result, list) else []
        for ch in channels:
            ch_id = ch.get("id", "")
            ch_name = ch.get("name", "unknown")
            if self.dry_run:
                print(f"  [DRY-RUN] Would delete channel: #{ch_name} (id={ch_id})")
                continue
            print(f"  Deleting channel: #{ch_name}")
            self.client.delete(f"/channels/{ch_id}")
            time.sleep(0.3)

    # ------------------------------------------------------------------
    # Step 2: Create roles
    # ------------------------------------------------------------------

    def create_roles(self) -> None:
        print("\n--- Creating roles ---")
        # Roles are created in reverse order of desired position because
        # Discord stacks newly created roles above existing ones by default.
        # We create from bottom to top so the final order is top-to-bottom
        # as specified (highest index = lowest in hierarchy after creation).
        # Then we re-order them using PATCH /guilds/{id}/roles.
        for role_def in ROLES:
            payload = {
                "name": role_def["name"],
                "color": role_def["color"],
                "hoist": role_def["hoist"],
                "mentionable": role_def["mentionable"],
                "permissions": role_def["permissions"],
            }
            print(f"  Creating role: {role_def['name']}")
            result = self.client.post(f"/guilds/{self.guild_id}/roles", payload)
            if result:
                role_id = result.get("id", "DRY_RUN_ID")
                self.role_ids[role_def["name"]] = role_id
            time.sleep(0.3)

        # Re-order roles: Discord expects a list of {id, position} objects.
        # Position 1 = just above @everyone (lowest human role).
        # Higher position number = higher in the hierarchy.
        # We want ROLES[0] (Founder) at the highest position.
        if not self.dry_run and self.role_ids:
            positions = []
            total = len(ROLES)
            for idx, role_def in enumerate(ROLES):
                rid = self.role_ids.get(role_def["name"])
                if rid:
                    # ROLES[0] gets position=total, ROLES[-1] gets position=1
                    positions.append({"id": rid, "position": total - idx})
            if positions:
                print("  Re-ordering roles...")
                self.client.patch(f"/guilds/{self.guild_id}/roles", positions)
                time.sleep(0.5)

    # ------------------------------------------------------------------
    # Step 2b: Validate required roles exist (MED-03)
    # ------------------------------------------------------------------

    def _validate_required_roles(self) -> None:
        """Abort if any role critical to permission overwrites was not created."""
        required = ["Founder", "Core Team", "Moderator", "Contributor",
                     "Member", "Unverified"]
        missing = [r for r in required if r not in self.role_ids]
        if missing and not self.dry_run:
            print(f"\nFATAL: Required roles were not created: {', '.join(missing)}")
            print("Channel permissions depend on these roles. Aborting.")
            sys.exit(1)

    # ------------------------------------------------------------------
    # Step 3: Build permission overwrites
    # ------------------------------------------------------------------

    def _build_overwrites(self, template: str) -> list[dict]:
        """
        Build permission_overwrites array for a channel given a template name.

        Templates:
          read_only   - @everyone can read, cannot send; Unverified can read
          community   - @everyone deny send; Unverified deny view+send; Member allow send
          governance  - same as community but additionally deny send for Member/Unverified,
                        allow send only for Contributor+
          team_only   - @everyone deny view; only Founder/Core Team/Moderator can view+send
        """
        everyone_id = self.everyone_role_id
        unverified_id = self.role_ids.get("Unverified", "")
        member_id = self.role_ids.get("Member", "")
        contributor_id = self.role_ids.get("Contributor", "")
        founder_id = self.role_ids.get("Founder", "")
        core_team_id = self.role_ids.get("Core Team", "")
        moderator_id = self.role_ids.get("Moderator", "")

        overwrites = []

        if template == "read_only":
            # @everyone can view and read history but cannot send messages
            overwrites.append({
                "id": everyone_id,
                "type": OW_ROLE,
                "allow": str(PERM_VIEW_CHANNEL | PERM_READ_MESSAGES_HISTORY),
                "deny": str(PERM_SEND_MESSAGES),
            })
            # Unverified — same (already covered by @everyone but make explicit)
            if unverified_id:
                overwrites.append({
                    "id": unverified_id,
                    "type": OW_ROLE,
                    "allow": str(PERM_VIEW_CHANNEL | PERM_READ_MESSAGES_HISTORY),
                    "deny": str(PERM_SEND_MESSAGES),
                })

        elif template == "community":
            # @everyone: deny send (base)
            overwrites.append({
                "id": everyone_id,
                "type": OW_ROLE,
                "allow": "0",
                "deny": str(PERM_SEND_MESSAGES),
            })
            # Unverified: cannot see or send
            if unverified_id:
                overwrites.append({
                    "id": unverified_id,
                    "type": OW_ROLE,
                    "allow": "0",
                    "deny": str(PERM_VIEW_CHANNEL | PERM_SEND_MESSAGES),
                })
            # Member: can send
            if member_id:
                overwrites.append({
                    "id": member_id,
                    "type": OW_ROLE,
                    "allow": str(PERM_VIEW_CHANNEL | PERM_SEND_MESSAGES | PERM_READ_MESSAGES_HISTORY),
                    "deny": "0",
                })

        elif template == "governance":
            # @everyone: deny send
            overwrites.append({
                "id": everyone_id,
                "type": OW_ROLE,
                "allow": "0",
                "deny": str(PERM_SEND_MESSAGES),
            })
            # Unverified: cannot see or send
            if unverified_id:
                overwrites.append({
                    "id": unverified_id,
                    "type": OW_ROLE,
                    "allow": "0",
                    "deny": str(PERM_VIEW_CHANNEL | PERM_SEND_MESSAGES),
                })
            # Member: can view but not send (governance is Contributor+)
            if member_id:
                overwrites.append({
                    "id": member_id,
                    "type": OW_ROLE,
                    "allow": str(PERM_VIEW_CHANNEL | PERM_READ_MESSAGES_HISTORY),
                    "deny": str(PERM_SEND_MESSAGES),
                })
            # Contributor: can send
            if contributor_id:
                overwrites.append({
                    "id": contributor_id,
                    "type": OW_ROLE,
                    "allow": str(PERM_VIEW_CHANNEL | PERM_SEND_MESSAGES | PERM_READ_MESSAGES_HISTORY),
                    "deny": "0",
                })

        elif template == "team_only":
            # @everyone: deny view entirely
            overwrites.append({
                "id": everyone_id,
                "type": OW_ROLE,
                "allow": "0",
                "deny": str(PERM_VIEW_CHANNEL),
            })
            # Founder, Core Team, Moderator: allow view + send
            for role_id in [founder_id, core_team_id, moderator_id]:
                if role_id:
                    overwrites.append({
                        "id": role_id,
                        "type": OW_ROLE,
                        "allow": str(PERM_VIEW_CHANNEL | PERM_SEND_MESSAGES | PERM_READ_MESSAGES_HISTORY),
                        "deny": "0",
                    })

        # Remove any entries with empty id (roles not yet created)
        return [ow for ow in overwrites if ow.get("id")]

    # ------------------------------------------------------------------
    # Step 3 (continued): Create categories and channels
    # ------------------------------------------------------------------

    def create_channels(self) -> None:
        print("\n--- Creating categories and channels ---")
        for category_def in SERVER_STRUCTURE:
            cat_name = category_def["category"]
            cat_perm_template = category_def.get("perm_template", "community")

            # Build category payload
            cat_payload: dict = {
                "name": cat_name,
                "type": CH_CATEGORY,
                "permission_overwrites": self._build_overwrites(cat_perm_template),
            }
            print(f"\n  Creating category: {cat_name}")
            cat_result = self.client.post(f"/guilds/{self.guild_id}/channels", cat_payload)
            cat_id = cat_result.get("id", "DRY_RUN_CAT_ID") if cat_result else None
            time.sleep(0.4)

            if not cat_id:
                print(f"  ERROR: Failed to create category {cat_name}, skipping its channels.")
                continue

            # Create channels inside this category
            for ch_def in category_def.get("channels", []):
                ch_name = ch_def["name"]
                ch_type = ch_def["type"]
                ch_topic = ch_def.get("topic", "")
                ch_perm_template = ch_def.get("perm_template", cat_perm_template)
                ch_slowmode = ch_def.get("slowmode", 0)

                ch_payload: dict = {
                    "name": ch_name,
                    "type": ch_type,
                    "parent_id": cat_id,
                    "permission_overwrites": self._build_overwrites(ch_perm_template),
                }
                if ch_topic and ch_type not in (CH_CATEGORY, CH_FORUM):
                    ch_payload["topic"] = ch_topic
                if ch_slowmode and ch_type == CH_TEXT:
                    ch_payload["rate_limit_per_user"] = ch_slowmode
                # Forum channels: apply thread creation rate limit (MED-05)
                if ch_type == CH_FORUM:
                    if ch_topic:
                        ch_payload["topic"] = ch_topic
                    # Rate limit between new thread creation (seconds)
                    thread_rate = ch_def.get("thread_rate_limit", 300)
                    ch_payload["default_thread_rate_limit_per_user"] = thread_rate

                print(f"    Creating channel: #{ch_name}")
                ch_result = self.client.post(f"/guilds/{self.guild_id}/channels", ch_payload)
                if ch_result:
                    ch_id = ch_result.get("id", "DRY_RUN_CH_ID")
                    self.channel_ids[ch_name] = ch_id
                time.sleep(0.4)

    # ------------------------------------------------------------------
    # Step 4: Pin messages
    # ------------------------------------------------------------------

    def pin_messages(self) -> None:
        print("\n--- Pinning messages ---")

        # SECURITY: Hard-fail if any message still contains URL_PLACEHOLDER (CRIT-01)
        placeholder_errors = []
        for ch_name, messages in PINNED_MESSAGES.items():
            for idx, msg in enumerate(messages, start=1):
                if "URL_PLACEHOLDER" in msg:
                    placeholder_errors.append(f"  #{ch_name} message {idx}")
        if placeholder_errors:
            print("FATAL: URL_PLACEHOLDER found in pinned messages. Replace all")
            print("placeholders with live URLs before running this script.")
            print("Affected messages:")
            for err in placeholder_errors:
                print(err)
            sys.exit(1)

        for ch_name, messages in PINNED_MESSAGES.items():
            ch_id = self.channel_ids.get(ch_name)
            if not ch_id:
                print(f"  SKIP: No channel id found for #{ch_name} — skipping pins.")
                continue

            # Forum channels cannot have pinned messages in the standard sense
            # (pins live inside threads, not the channel root).  We skip forum
            # channels here; their template messages are posted when the first
            # thread is created manually by the team.
            ch_type = None
            for cat in SERVER_STRUCTURE:
                for ch in cat.get("channels", []):
                    if ch["name"] == ch_name:
                        ch_type = ch["type"]
                        break

            if ch_type == CH_FORUM:
                print(f"  SKIP: #{ch_name} is a forum channel — pin manually via first thread.")
                continue

            for idx, msg_content in enumerate(messages, start=1):
                print(f"  Pinning message {idx}/{len(messages)} in #{ch_name}")

                if self.dry_run:
                    print(f"  [DRY-RUN] Would send and pin message {idx} in #{ch_name}")
                    continue

                # Send the message
                send_result = self.client.post(
                    f"/channels/{ch_id}/messages",
                    {"content": msg_content},
                )
                if not send_result:
                    print(f"  ERROR: Failed to send message {idx} in #{ch_name}")
                    continue

                msg_id = send_result.get("id")
                if not msg_id:
                    print(f"  ERROR: No message id returned for #{ch_name} message {idx}")
                    continue

                time.sleep(0.3)

                # Pin the message
                pin_result = self.client.put(f"/channels/{ch_id}/pins/{msg_id}")
                if pin_result is not None:
                    print(f"    Pinned message {idx} in #{ch_name}")
                else:
                    print(f"  ERROR: Failed to pin message {idx} in #{ch_name}")

                time.sleep(0.3)

    # ------------------------------------------------------------------
    # Run all steps
    # ------------------------------------------------------------------

    def run(self) -> None:
        print(f"\nRillCoin Discord Server Setup")
        print(f"Guild ID : ...{self.guild_id[-4:]}")
        print(f"Dry run  : {self.dry_run}")
        print(f"API base : {API_BASE}")

        self.delete_default_channels()
        self.create_roles()
        self._validate_required_roles()
        self.create_channels()
        self.pin_messages()

        print("\n--- Setup complete ---")
        if self.dry_run:
            print("Dry run finished. No changes were made to Discord.")
        else:
            print("Server provisioning finished.")
            print("Next steps:")
            print("  1. Assign Founder and Core Team roles to the appropriate users.")
            print("  2. Configure MEE6, Carl-bot, and Wick with their respective settings.")
            print("  3. Set up GitHub and X/Twitter webhooks in #github-feed and #twitter-feed.")
            print("     SECURITY: Filter GitHub webhook payloads — suppress commits/PRs")
            print("     containing: security, vulnerability, CVE, exploit, credential, secret.")
            print("  4. Attach the Carl-bot verification button in #roles-and-verification.")
            print("  5. Replace all URL_PLACEHOLDER values in pinned messages with live URLs.")
            print("     (The script will refuse to pin messages containing URL_PLACEHOLDER.)")
            print("  6. Enable '2FA requirement for moderation' in Server Settings > Safety.")
            print("  7. Test the onboarding flow end-to-end with an alt account.")


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Provision the RillCoin Discord server via the Discord REST API v10.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python3 scripts/setup_discord.py
  python3 scripts/setup_discord.py --dry-run

Credentials are read from marketing/.env:
  DISCORD_BOT_TOKEN=your-bot-token
  DISCORD_GUILD_ID=your-guild-id
""",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print what the script would do without making any API calls.",
    )
    args = parser.parse_args()

    token, guild_id = load_credentials()

    if not args.dry_run:
        print("This script will:")
        print("  - Delete ALL existing channels in the server")
        print("  - Create 14 roles")
        print(f"  - Create {sum(len(c['channels']) for c in SERVER_STRUCTURE)} channels across {len(SERVER_STRUCTURE)} categories")
        print("  - Send and pin messages in multiple channels")
        print()
        confirm = input("Type 'yes' to continue, anything else to abort: ").strip().lower()
        if confirm != "yes":
            print("Aborted.")
            sys.exit(0)

    client = DiscordClient(token=token, dry_run=args.dry_run)
    setup = ServerSetup(client=client, guild_id=guild_id, dry_run=args.dry_run)
    setup.run()


if __name__ == "__main__":
    main()
