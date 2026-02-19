# RillCoin Launch Thread — 2026-02-19

First public-facing thread. Covers what shipped today and what is coming next.
Approved against copy library voice checklist before publishing.

Hashtags: #RillCoin #ConcentrationDecay #CryptoForCirculation

---

[1/8]
Most cryptocurrencies are designed to be held.
We built one designed to flow.

RillCoin is live on testnet today — a proof-of-work chain where concentrated holdings
decay back to miners over time.

Wealth should flow like water.

---

[2/8]
Four sites went live today, all HTTPS:

rillcoin.com — main site
faucet.rillcoin.com — claim 10 RILL (24h cooldown)
explorer.rillcoin.com — live chain data
docs.rillcoin.com — whitepaper, protocol spec, CLI reference, node setup

Everything is up. Go break something.

---

[3/8]
The technical stack:

- Rust 2024 edition, proof-of-work
- Ed25519 signatures + BLAKE3 Merkle trees
- SHA-256 PoW, 60-second block time
- libp2p networking (Gossipsub + Kademlia DHT)
- 21,000,000 RILL max supply, 50 RILL block reward, halving every 210,000 blocks

#RillCoin

---

[4/8]
The differentiator: concentration decay.

Holdings above a threshold lose a small percentage each cycle. That amount flows
directly to active miners — not burned, not locked.

Your effective balance reflects circulation, not accumulation. The more you hoard,
the more the current pulls away.

#ConcentrationDecay

---

[5/8]
Everything is open source.

github.com/rillcoin/rill

The decay logic, consensus rules, wallet, node — all of it. Audit the mechanism.
Run the math. If you find a flaw, we want to know before mainnet does.

#CryptoForCirculation

---

[6/8]
Early adopters matter here more than most projects.

Testnet miners are producing real blocks on a live chain. Every address that
interacts with the protocol now is part of the permanent record.

The people who help prove the mechanism work before mainnet are the foundation
of the network.

---

[7/8]
What is coming next:

- GUI wallet (beyond rill-cli)
- Mainnet launch

We are building in sequence. Testnet integrity before mainnet.

If you find bugs, report them. Confirmed reports earn the Bug Hunter role
and a permanent place in the changelog.

---

[8/8]
If this resonates, two things to do right now:

1. Claim testnet RILL at faucet.rillcoin.com
2. Join the Discord: discord.com/invite/F3dRVaP8

The testnet is where we prove the mechanism works. Come help stress-test it.

Wealth should flow like water. @RillCoin
