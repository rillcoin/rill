# MonitoRSS Setup Guide — RillCoin Discord

> Step-by-step browser walkthrough for configuring MonitoRSS to deliver RSS feeds
> to #crypto-news and #regulatory-watch in the RillCoin Discord server.
>
> Written for the Community Lead. No developer tools required.
> Estimated time: 30–45 minutes.

---

## Reference

| Item | Value |
|---|---|
| Guild (server) ID | 1473262369546174631 |
| #crypto-news channel ID | 1473390932941340742 |
| #regulatory-watch channel ID | 1473390939757351125 |
| MonitoRSS dashboard | https://monitorss.xyz |

Both channels are already created and set to read-only. MonitoRSS will post via webhook — the bot never needs member posting permissions, only Manage Webhooks in each channel.

---

## Part 1 — Add MonitoRSS to the Server

**1.** Go to https://monitorss.xyz in your browser.

**2.** Click "Add to Discord" (top-right of the page).

**3.** Discord opens an authorization screen. From the "Add to Server" dropdown, select **RillCoin**. Click "Continue."

**4.** On the permissions screen, MonitoRSS requests Manage Webhooks, Send Messages, and Read Message History. Leave all permissions checked. Click "Authorize."

**5.** Complete any CAPTCHA if prompted.

**6.** You are redirected back to the MonitoRSS dashboard. You will see "RillCoin" listed under your servers. Click on it to open the server control panel.

---

## Part 2 — Grant Manage Webhooks in Both Channels

MonitoRSS creates a webhook in each target channel to post feed items. The bot must have Manage Webhooks permission scoped to those channels before you can add feeds.

**7.** In the Discord app (browser or desktop), open the RillCoin server.

**8.** Right-click on **#crypto-news** and select "Edit Channel."

**9.** Go to the "Permissions" tab. Click the plus (+) icon next to "Roles/Members" and search for "MonitoRSS." Select the MonitoRSS role.

**10.** In the MonitoRSS row, explicitly allow "Manage Webhooks." Save changes.

**11.** Repeat steps 8–10 for **#regulatory-watch**.

> Note: If MonitoRSS does not appear in the role list, confirm the bot was added successfully in step 4 and that the MonitoRSS role appears in Server Settings > Roles.

---

## Part 3 — Configure #crypto-news Feeds

You will add four feeds to #crypto-news. Each feed is added separately.

Return to the MonitoRSS dashboard at https://monitorss.xyz and select the RillCoin server.

---

### Feed 1 of 4 — CoinDesk RSS

**12.** Click "Add Feed" (or the "+" button in the feeds panel).

**13.** In the "Feed URL" field, enter:
```
https://www.coindesk.com/arc/outboundfeeds/rss/
```

**14.** In the "Channel" dropdown, select **#crypto-news** (ID: 1473390932941340742).

**15.** Click "Save" or "Add Feed." MonitoRSS will validate the URL by fetching it. If validation succeeds, the feed appears in your feed list.

**16.** Click on the feed to open its settings. Go to the "Filters" or "Message Filters" section.

**17.** Under "Include filters" (words that MUST appear for an article to be posted), add each of the following as separate entries:
- `proof-of-work`
- `mining`
- `consensus`
- `Layer 1`
- `Bitcoin`
- `protocol`
- `monetary policy`
- `PoW`

> In MonitoRSS, include filters use OR logic by default — an article only needs to match one term to pass. Confirm this is the behavior shown in the UI before saving.

**18.** Under "Exclude filters" (articles containing these words are blocked), add each of the following as separate entries:
- `NFT`
- `meme coin`
- `airdrop`
- `presale`
- `token launch`

**19.** Save the filter settings.

---

### Feed 2 of 4 — The Block RSS

**20.** Click "Add Feed" again.

**21.** Feed URL:
```
https://www.theblock.co/rss.xml
```

**22.** Channel: **#crypto-news**.

**23.** Save to add the feed. Then open its settings.

**24.** Apply the same include and exclude filters as Feed 1 (steps 17–18). The filters are not shared between feeds — you must set them individually on each feed.

**25.** Save.

---

### Feed 3 of 4 — Bitcoin Magazine RSS

**26.** Click "Add Feed."

**27.** Feed URL:
```
https://bitcoinmagazine.com/feed
```

**28.** Channel: **#crypto-news**.

**29.** Save to add the feed. Open its settings.

**30.** Apply the same include and exclude filters (steps 17–18).

> Note: Bitcoin Magazine covers Bitcoin exclusively, so the include filters are less likely to block content. The exclude filters are still worth adding to catch any sponsored NFT or presale content that occasionally appears.

**31.** Save.

---

### Feed 4 of 4 — Decrypt RSS

**32.** Click "Add Feed."

**33.** Feed URL:
```
https://decrypt.co/feed
```

**34.** Channel: **#crypto-news**.

**35.** Save to add the feed. Open its settings.

**36.** Apply the same include and exclude filters (steps 17–18).

**37.** Save.

At this point, #crypto-news has four active feeds with filters configured. MonitoRSS will begin checking for new articles on each feed's polling schedule (typically every 10–15 minutes on the free tier).

---

## Part 4 — Configure #regulatory-watch Feeds

You will add four feeds to #regulatory-watch. No keyword filters are needed for this channel — the sources are already regulatory-focused, so all articles from these feeds are relevant.

---

### Feed 1 of 4 — SEC Litigation Releases

**38.** Click "Add Feed."

**39.** Feed URL:
```
https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&type=LIT&dateb=&owner=include&count=40&search_text=&action=getcompany&RSS
```

**40.** Channel: **#regulatory-watch** (ID: 1473390939757351125).

**41.** Click "Save." MonitoRSS will validate the feed. The SEC feed is a standard Atom/RSS feed and should validate without issue.

**42.** No filters needed. Leave filter settings empty and save.

---

### Feed 2 of 4 — CFTC Press Releases

**43.** Click "Add Feed."

**44.** Feed URL:
```
https://www.cftc.gov/PressRoom/PressReleases/rss.xml
```

**45.** Channel: **#regulatory-watch**.

**46.** Save. No filters needed.

---

### Feed 3 of 4 — CoinDesk Policy Tag

**47.** Click "Add Feed."

**48.** Feed URL:
```
https://www.coindesk.com/arc/outboundfeeds/rss/?outputType=xml&_website=coindesk&category=/policy/
```

**49.** Channel: **#regulatory-watch**.

**50.** Save. This feed is already scoped to CoinDesk's Policy section, so no additional filters are needed.

---

### Feed 4 of 4 — The Block Regulation (Filtered)

**51.** Click "Add Feed."

**52.** Feed URL:
```
https://www.theblock.co/rss.xml
```

**53.** Channel: **#regulatory-watch**.

**54.** Save to add the feed. Open its settings.

**55.** The Block's main feed covers all topics, so apply include filters to scope it to regulatory content. Under "Include filters," add:
- `regulation`
- `regulatory`
- `SEC`
- `CFTC`
- `legal`
- `law`
- `compliance`
- `enforcement`
- `lawsuit`
- `ruling`
- `MiCA`
- `legislation`
- `congress`
- `senate`

**56.** No exclude filters needed for this feed in #regulatory-watch.

**57.** Save.

> Important: This is the same base feed URL as #crypto-news Feed 2 (The Block). MonitoRSS treats each feed-channel pair as a separate subscription, so the two instances will not conflict. The filters ensure that articles about regulation go to #regulatory-watch while articles about PoW and mining go to #crypto-news.

---

## Part 5 — Verify the Setup

**58.** In the MonitoRSS dashboard, open the RillCoin server panel. Confirm you see exactly 8 active feeds:

| Channel | Feed |
|---|---|
| #crypto-news | CoinDesk RSS |
| #crypto-news | The Block RSS |
| #crypto-news | Bitcoin Magazine RSS |
| #crypto-news | Decrypt RSS |
| #regulatory-watch | SEC Litigation Releases |
| #regulatory-watch | CFTC Press Releases |
| #regulatory-watch | CoinDesk Policy Tag |
| #regulatory-watch | The Block Regulation (filtered) |

**59.** Trigger a test delivery. In the MonitoRSS dashboard, find the option to send a test article for each feed (sometimes labeled "Send test" or "Deliver latest article"). Do this for one feed in each channel to confirm messages appear in Discord.

**60.** In Discord, go to **#crypto-news** and confirm a test post appeared. Check that it is formatted as a bot post (not from your account).

**61.** Repeat for **#regulatory-watch**.

**62.** Confirm that neither channel allows community members to reply (these are read-only channels — the channel permissions were set when the channels were created). If members can post, revisit the channel permissions in Discord's channel settings and deny Send Messages for @everyone.

---

## Part 6 — Free Tier Note and Limits

MonitoRSS's free tier supports up to 5 feeds per server. You are configuring 8 feeds total, which exceeds the free limit.

**Options:**

- **MonitoRSS Patron tier ($5–$20/month depending on tier):** Increases the feed limit to 15 or 35 feeds. This is the recommended path. Subscribe at https://monitorss.xyz/patron before completing the setup, or immediately after hitting the 5-feed cap.

- **Self-host MonitoRSS:** MonitoRSS is open-source and can be self-hosted with no feed limits. This requires a server (a $5–10/month VPS is sufficient) and basic Docker familiarity. See https://github.com/synzen/MonitoRSS for the self-hosting guide. Recommended if the team has infrastructure already running for the testnet node.

> The server spec (discord-server-spec.md, section 5) notes self-hosting as an option for removing all limits. Given the testnet infrastructure is already running, self-hosting is worth evaluating.

---

## Part 7 — Post-Setup Maintenance

**When a feed stops delivering:**
- Go to the MonitoRSS dashboard and check the feed status. A red or "failed" indicator means MonitoRSS cannot reach the feed URL.
- Test the URL directly in your browser. SEC and CFTC URLs occasionally change format — check the agency's RSS page for a current URL if the feed fails.
- For The Block and CoinDesk: these publishers occasionally restructure their RSS endpoints. If a feed fails, check the publication's site footer for an updated RSS link.

**When articles slip through the filters:**
- Open the feed settings in the MonitoRSS dashboard and add the unwanted term to the exclude filter list.
- Changes take effect on the next polling cycle (typically within 15 minutes).

**If #crypto-news and #regulatory-watch both receive the same The Block article:**
- This can occur if an article covers both PoW/mining topics and regulation simultaneously.
- It is acceptable behavior — both channels have different audiences. If it becomes a frequent issue, add more specific exclude filters to one of the two The Block feeds to reduce overlap.

**Polling frequency:**
- On the free and Patron tiers, MonitoRSS polls each feed on a schedule it manages (typically every 2–10 minutes). This is not configurable per feed on the standard hosted plan.
- On self-hosted instances, polling frequency is configurable per feed in the dashboard or config file.

---

*Document version 1.0 — February 2026*
*Maintained by the RillCoin Community Lead*
*Review if MonitoRSS changes its dashboard UI, if feed URLs change, or if the channel structure is updated.*
