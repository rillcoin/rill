#!/usr/bin/env python3
"""
Incremental script to add the 4 new feed channels to the existing BOTS category.

Adds: #crypto-news, #price-ticker, #regulatory-watch, #whale-alerts
Does NOT delete or modify any existing channels.

Usage:
    python3 scripts/add_feed_channels.py
    python3 scripts/add_feed_channels.py --dry-run
"""

import sys
import os
import time

# Reuse the existing setup module for credentials, client, constants, and permissions
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from scripts.setup_discord import (
    load_credentials,
    DiscordClient,
    CH_TEXT,
    PINNED_MESSAGES,
    SERVER_STRUCTURE,
)

import argparse


# ---------------------------------------------------------------------------
# The 4 new channels to add (must match SERVER_STRUCTURE in setup_discord.py)
# ---------------------------------------------------------------------------

NEW_CHANNELS = [
    {
        "name": "crypto-news",
        "type": CH_TEXT,
        "topic": "Aggregated crypto industry news from curated RSS sources. Powered by MonitoRSS.",
        "perm_template": "read_only",
    },
    {
        "name": "price-ticker",
        "type": CH_TEXT,
        "topic": "Auto-updating price data for BTC, ETH, and top 20 by market cap. Powered by CoinTrendzBot.",
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
]


# ---------------------------------------------------------------------------
# Permission helper (simplified from setup_discord.py for read_only only)
# ---------------------------------------------------------------------------

PERM_VIEW_CHANNEL = 1 << 10
PERM_SEND_MESSAGES = 1 << 11
PERM_READ_MESSAGES_HISTORY = 1 << 16
OW_ROLE = 0


def build_read_only_overwrites(everyone_role_id: str, role_ids: dict) -> list[dict]:
    """Build read_only permission overwrites using live role IDs."""
    overwrites = [
        {
            "id": everyone_role_id,
            "type": OW_ROLE,
            "allow": str(PERM_VIEW_CHANNEL | PERM_READ_MESSAGES_HISTORY),
            "deny": str(PERM_SEND_MESSAGES),
        },
    ]
    unverified_id = role_ids.get("Unverified")
    if unverified_id:
        overwrites.append({
            "id": unverified_id,
            "type": OW_ROLE,
            "allow": str(PERM_VIEW_CHANNEL | PERM_READ_MESSAGES_HISTORY),
            "deny": str(PERM_SEND_MESSAGES),
        })
    return overwrites


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Add 4 new feed channels to the existing BOTS category.",
    )
    parser.add_argument("--dry-run", action="store_true",
                        help="Print what would happen without making API calls.")
    args = parser.parse_args()

    token, guild_id = load_credentials()
    client = DiscordClient(token=token, dry_run=args.dry_run)

    print("=== Add Feed Channels to BOTS Category ===")
    print(f"Dry run: {args.dry_run}\n")

    # Step 1: Fetch existing channels to find the BOTS category
    print("Fetching existing channels...")
    channels_result = client.get(f"/guilds/{guild_id}/channels")
    if args.dry_run:
        # Dry-run returns a stub dict; we can't discover real channels
        print("  [DRY-RUN] Would fetch channels, find BOTS category, create 4 channels, pin messages.")
        print("  Channels to create: " + ", ".join(f"#{ch['name']}" for ch in NEW_CHANNELS))
        print("\n=== Dry run complete. No changes made. ===")
        return
    channels = channels_result
    if not channels or not isinstance(channels, list):
        print("ERROR: Could not fetch guild channels.")
        sys.exit(1)

    # Find the BOTS category
    bots_category_id = None
    existing_channel_names = set()
    for ch in channels:
        if ch.get("type") == 4 and ch.get("name", "").upper() == "BOTS":
            bots_category_id = ch["id"]
        existing_channel_names.add(ch.get("name", ""))

    if not bots_category_id:
        print("ERROR: Could not find the BOTS category. Is the server set up?")
        sys.exit(1)

    print(f"Found BOTS category: {bots_category_id}")

    # Step 2: Fetch roles to build permission overwrites
    print("Fetching roles...")
    roles = client.get(f"/guilds/{guild_id}/roles")
    if not roles or not isinstance(roles, list):
        print("ERROR: Could not fetch guild roles.")
        sys.exit(1)

    role_ids = {}
    for role in roles:
        role_ids[role.get("name", "")] = role.get("id", "")

    everyone_role_id = guild_id  # @everyone role ID equals guild ID
    overwrites = build_read_only_overwrites(everyone_role_id, role_ids)

    # Step 3: Create channels that don't already exist
    created_channels = {}  # name -> id
    for ch_def in NEW_CHANNELS:
        name = ch_def["name"]
        if name in existing_channel_names:
            print(f"  SKIP: #{name} already exists.")
            # Find its ID for pinning
            for ch in channels:
                if ch.get("name") == name:
                    created_channels[name] = ch["id"]
                    break
            continue

        payload = {
            "name": name,
            "type": ch_def["type"],
            "parent_id": bots_category_id,
            "topic": ch_def["topic"],
            "permission_overwrites": overwrites,
        }
        print(f"  Creating #{name}...")
        result = client.post(f"/guilds/{guild_id}/channels", payload)
        if result:
            ch_id = result.get("id", "DRY_RUN_ID")
            created_channels[name] = ch_id
            print(f"    Created: {ch_id}")
        else:
            print(f"    ERROR: Failed to create #{name}")
        time.sleep(0.5)

    # Step 4: Pin messages in new channels
    print("\nPinning messages...")
    for name, ch_id in created_channels.items():
        messages = PINNED_MESSAGES.get(name, [])
        if not messages:
            print(f"  SKIP: No pinned messages defined for #{name}")
            continue

        # Check if channel already has pinned messages
        if not args.dry_run:
            existing_pins = client.get(f"/channels/{ch_id}/pins")
            if existing_pins and isinstance(existing_pins, list) and len(existing_pins) > 0:
                print(f"  SKIP: #{name} already has {len(existing_pins)} pinned message(s).")
                continue

        for idx, msg_content in enumerate(messages, start=1):
            if "URL_PLACEHOLDER" in msg_content:
                print(f"  ERROR: #{name} message {idx} contains URL_PLACEHOLDER. Skipping.")
                continue

            print(f"  Pinning message {idx} in #{name}...")
            if args.dry_run:
                print(f"    [DRY-RUN] Would send and pin message {idx}")
                continue

            send_result = client.post(f"/channels/{ch_id}/messages", {"content": msg_content})
            if not send_result:
                print(f"    ERROR: Failed to send message in #{name}")
                continue

            msg_id = send_result.get("id")
            if msg_id:
                time.sleep(0.3)
                client.put(f"/channels/{ch_id}/pins/{msg_id}")
                print(f"    Pinned.")
            time.sleep(0.3)

    print("\n=== Done ===")
    if args.dry_run:
        print("Dry run complete. No changes made.")
    else:
        print("Feed channels added successfully.")


if __name__ == "__main__":
    main()
