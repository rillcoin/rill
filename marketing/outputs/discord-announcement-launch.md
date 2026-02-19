# Discord Announcement — Launch Day
**Channel:** #announcements
**Date:** 2026-02-19
**Author:** Social Media agent
**Status:** Ready to publish

---

## Message

> "Wealth should flow like water."

**RillCoin is live on testnet. Everything we have been building is now publicly accessible.**

Starting today, you can read the protocol, run a node, send transactions, observe concentration decay in action on a live chain, and tell us what is broken. That is the whole point of this phase — stress it, break it, and help us fix it before mainnet.

---

**What shipped today**

- **rillcoin.com** — Main site. Start here if you are new.
- **faucet.rillcoin.com** — 10 RILL per address, 24-hour cooldown. Use it.
- **explorer.rillcoin.com** — Live block explorer. Watch decay flow in real time.
- **docs.rillcoin.com** — Whitepaper, full protocol spec, decay mechanics, mining guide, CLI reference, RPC reference, node setup.
- **github.com/rillcoin/rill** — Full source. Rust 2024 edition, six library crates, three binaries. Open to contributions.

---

**The protocol at a glance**

```
Max supply:       21,000,000 RILL
Block reward:     50 RILL
Block time:       60 seconds
Halving:          every 210,000 blocks
Signatures:       Ed25519
Hashing:          BLAKE3 (Merkle) / SHA-256 (PoW headers)
Networking:       libp2p
```

The core mechanic is **concentration decay**. Holdings above defined thresholds decay progressively — the excess does not disappear, it flows into the decay pool and redistributes to active miners each block. Not locked, not destroyed. Circulating. The protocol enforces this at the consensus layer with no floats, no approximations: integer arithmetic, fixed-point at 10^8 precision.

---

**Early adopters**

This phase matters. The people participating now — mining blocks, claiming from the faucet, running nodes, reading the whitepaper — are building the foundation the mainnet will stand on.

A few things worth knowing if you are here early:

- Every address that interacts with the testnet chain is part of a permanent on-chain record
- Miners producing blocks right now are doing real work on a live protocol
- **Bug Hunter** — report a confirmed bug in #bug-reports and you earn the role permanently, along with a credit in the changelog
- **Testnet Pioneer** — claim from the faucet or mine a block before mainnet launches and the role is yours; it marks you as someone who was here before the chain was proven
- The GUI wallet is coming; early CLI users are the ones who know the protocol before most people know it exists

We are not promising anything beyond recognition and a permanent record. But in a network built on circulation rather than accumulation, being an early miner is structurally meaningful — the protocol rewards activity, not tenure.

---

**What is coming next**

GUI wallet, then mainnet. Mainnet follows when the protocol is stable and the testnet community has had a real chance to stress it. Your participation here directly shapes that timeline.

---

**Where to go from here**

- Claim testnet RILL: **faucet.rillcoin.com**
- Read the mechanics: **docs.rillcoin.com/decay**
- Questions, node issues, decay math: **#general** or **#protocol**
- Bugs: **#bug-reports** (use the pinned template)

If something looks wrong on chain, we want to know. That is the entire purpose of running testnet in public.
