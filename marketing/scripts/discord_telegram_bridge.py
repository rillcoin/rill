#!/usr/bin/env python3
"""
Discord-to-Telegram Bridge for RillCoin

Monitors the Discord #announcements and #dev-updates channels and forwards
new messages to the RillCoin Telegram group.

Usage:
    python3 scripts/discord_telegram_bridge.py
    python3 scripts/discord_telegram_bridge.py --dry-run
    python3 scripts/discord_telegram_bridge.py --once  (check once and exit)

Reads from marketing/.env:
    DISCORD_BOT_TOKEN=...
    DISCORD_GUILD_ID=...
    TELEGRAM_BOT_TOKEN=...
    TELEGRAM_CHAT_ID=...

Run as a long-running process or via cron (with --once flag).
"""

import argparse
import json
import os
import sys
import time
from datetime import datetime, timezone

import requests

# ---------------------------------------------------------------------------
# Environment
# ---------------------------------------------------------------------------

def load_env():
    """Load credentials from .env file."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    marketing_dir = os.path.dirname(script_dir)
    env_path = os.path.join(marketing_dir, ".env")

    env = {}
    try:
        with open(env_path, "r") as fh:
            for line in fh:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                if "=" not in line:
                    continue
                key, _, value = line.partition("=")
                env[key.strip()] = value.strip().strip('"').strip("'")
    except FileNotFoundError:
        print(f"ERROR: .env file not found at {env_path}")
        sys.exit(1)

    required = ["DISCORD_BOT_TOKEN", "DISCORD_GUILD_ID",
                "TELEGRAM_BOT_TOKEN", "TELEGRAM_CHAT_ID"]
    missing = [k for k in required if k not in env or not env[k]]
    if missing:
        print(f"ERROR: Missing required .env variables: {', '.join(missing)}")
        print("Add them to your .env file:")
        for k in missing:
            print(f"  {k}=your-value-here")
        sys.exit(1)

    return env


# ---------------------------------------------------------------------------
# Discord client (read-only)
# ---------------------------------------------------------------------------

DISCORD_API = "https://discord.com/api/v10"


def discord_get(path: str, token: str):
    """Make a GET request to the Discord API with rate limit handling."""
    headers = {
        "Authorization": f"Bot {token}",
        "User-Agent": "RillCoinBridge/1.0",
    }
    while True:
        resp = requests.get(f"{DISCORD_API}{path}", headers=headers)
        if resp.status_code == 429:
            retry_after = resp.json().get("retry_after", 1.0)
            time.sleep(retry_after)
            continue
        if resp.status_code == 200:
            return resp.json()
        print(f"  [Discord HTTP {resp.status_code}] GET {path}")
        return None


# ---------------------------------------------------------------------------
# Telegram client (send only)
# ---------------------------------------------------------------------------

TELEGRAM_API = "https://api.telegram.org"


def telegram_send(bot_token: str, chat_id: str, text: str,
                  dry_run: bool = False) -> bool:
    """Send a message to Telegram. Returns True on success."""
    if dry_run:
        print(f"  [DRY-RUN] Would send to Telegram ({len(text)} chars):")
        print(f"    {text[:200]}...")
        return True

    url = f"{TELEGRAM_API}/bot{bot_token}/sendMessage"
    payload = {
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "Markdown",
        "disable_web_page_preview": True,
    }
    resp = requests.post(url, json=payload)
    if resp.status_code == 200:
        return True
    print(f"  [Telegram HTTP {resp.status_code}] {resp.text[:200]}")
    return False


def telegram_get_chat_id(bot_token: str) -> None:
    """Helper: print recent updates to find the chat ID."""
    url = f"{TELEGRAM_API}/bot{bot_token}/getUpdates"
    resp = requests.get(url)
    if resp.status_code == 200:
        data = resp.json()
        print("Recent Telegram updates (look for your group's chat ID):")
        seen = set()
        for update in data.get("result", []):
            msg = update.get("message") or update.get("my_chat_member", {}).get("chat")
            if msg:
                chat = msg.get("chat", msg) if "chat" in msg else msg
                chat_id = chat.get("id")
                title = chat.get("title", chat.get("first_name", "DM"))
                if chat_id not in seen:
                    seen.add(chat_id)
                    print(f"  Chat ID: {chat_id}  Title: {title}")
        if not seen:
            print("  No updates found. Send a message in the group first.")
    else:
        print(f"  ERROR: {resp.status_code}")


# ---------------------------------------------------------------------------
# Message formatting
# ---------------------------------------------------------------------------

def format_for_telegram(content: str, channel_name: str) -> str:
    """Format a Discord message for Telegram delivery."""
    # Add a header indicating the source channel
    header = ""
    if channel_name == "announcements":
        header = "📢 *RillCoin Announcement*\n\n"
    elif channel_name == "dev-updates":
        header = "🔧 *Dev Update*\n\n"
    else:
        header = f"*#{channel_name}*\n\n"

    # Strip Discord-specific markdown that doesn't translate well
    text = content
    # Discord ## headers -> Telegram bold
    text = text.replace("## ", "*")
    # Clean up any triple backticks (code blocks)
    # Keep them as-is, Telegram supports ```

    # Telegram message limit is 4096 chars
    max_len = 4096 - len(header)
    if len(text) > max_len:
        text = text[:max_len - 20] + "\n\n_(truncated)_"

    return header + text


# ---------------------------------------------------------------------------
# State management (track last forwarded message)
# ---------------------------------------------------------------------------

def get_state_path() -> str:
    script_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(script_dir, ".bridge_state.json")


def load_state() -> dict:
    path = get_state_path()
    try:
        with open(path, "r") as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


def save_state(state: dict) -> None:
    path = get_state_path()
    with open(path, "w") as f:
        json.dump(state, f, indent=2)


# ---------------------------------------------------------------------------
# Bridge logic
# ---------------------------------------------------------------------------

# Channels to monitor
BRIDGE_CHANNELS = ["announcements", "dev-updates"]


def find_channel_ids(guild_id: str, token: str) -> dict:
    """Find channel IDs for the channels we want to bridge."""
    channels = discord_get(f"/guilds/{guild_id}/channels", token)
    if not channels or not isinstance(channels, list):
        return {}

    result = {}
    for ch in channels:
        if ch.get("name") in BRIDGE_CHANNELS:
            result[ch["name"]] = ch["id"]
    return result


def check_and_forward(env: dict, dry_run: bool = False) -> int:
    """Check for new messages and forward them. Returns count forwarded."""
    discord_token = env["DISCORD_BOT_TOKEN"]
    guild_id = env["DISCORD_GUILD_ID"]
    tg_token = env["TELEGRAM_BOT_TOKEN"]
    tg_chat_id = env["TELEGRAM_CHAT_ID"]

    state = load_state()
    channel_ids = find_channel_ids(guild_id, discord_token)

    if not channel_ids:
        print("ERROR: Could not find bridge channels in Discord.")
        return 0

    forwarded = 0

    for ch_name, ch_id in channel_ids.items():
        last_id = state.get(ch_name)

        # Fetch recent messages (up to 10)
        path = f"/channels/{ch_id}/messages?limit=10"
        if last_id:
            path += f"&after={last_id}"

        messages = discord_get(path, discord_token)
        if not messages or not isinstance(messages, list):
            continue

        # Messages come newest-first, reverse to forward in chronological order
        messages.reverse()

        for msg in messages:
            msg_id = msg.get("id", "")
            content = msg.get("content", "")

            # Skip empty messages and bot messages (avoid echo loops)
            if not content:
                continue
            author = msg.get("author", {})
            if author.get("bot", False):
                continue

            formatted = format_for_telegram(content, ch_name)
            print(f"  Forwarding from #{ch_name}: {content[:80]}...")
            success = telegram_send(tg_token, tg_chat_id, formatted, dry_run)

            if success:
                state[ch_name] = msg_id
                forwarded += 1
                time.sleep(1)  # Rate limit Telegram

        # Even if no new messages, update state to latest
        if messages:
            latest_id = messages[-1].get("id", last_id)
            state[ch_name] = latest_id

    if not dry_run:
        save_state(state)

    return forwarded


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Bridge Discord announcements to Telegram.",
    )
    parser.add_argument("--dry-run", action="store_true",
                        help="Print what would be sent without sending.")
    parser.add_argument("--once", action="store_true",
                        help="Check once and exit (for cron use).")
    parser.add_argument("--get-chat-id", action="store_true",
                        help="Print recent Telegram updates to find chat ID.")
    parser.add_argument("--test", action="store_true",
                        help="Send a test message to Telegram and exit.")
    args = parser.parse_args()

    env = load_env()

    if args.get_chat_id:
        telegram_get_chat_id(env["TELEGRAM_BOT_TOKEN"])
        return

    if args.test:
        print("Sending test message to Telegram...")
        success = telegram_send(
            env["TELEGRAM_BOT_TOKEN"],
            env["TELEGRAM_CHAT_ID"],
            "📢 *RillCoin Bridge Test*\n\nThis is a test message from the Discord-Telegram bridge. If you see this, the bridge is working.",
            dry_run=args.dry_run,
        )
        print("Success." if success else "Failed.")
        return

    print("=== RillCoin Discord → Telegram Bridge ===")
    print(f"Monitoring: {', '.join(BRIDGE_CHANNELS)}")
    print(f"Dry run: {args.dry_run}")

    if args.once:
        count = check_and_forward(env, args.dry_run)
        print(f"Forwarded {count} message(s).")
        return

    # Long-running mode: check every 60 seconds
    print("Running in continuous mode (Ctrl+C to stop)...")
    print(f"Polling interval: 60 seconds\n")

    while True:
        try:
            count = check_and_forward(env, args.dry_run)
            if count:
                print(f"  [{datetime.now(timezone.utc).strftime('%H:%M:%S')}] Forwarded {count} message(s).")
            time.sleep(60)
        except KeyboardInterrupt:
            print("\nBridge stopped.")
            break
        except Exception as e:
            print(f"  ERROR: {e}")
            time.sleep(60)


if __name__ == "__main__":
    main()
