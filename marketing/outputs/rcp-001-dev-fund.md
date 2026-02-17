# RCP-001: Developer Fund Block Reward and Community Donation Program

**Status:** Draft
**Author:** RillCoin Core Team
**Date:** 2026-02-17
**Discussion:** #proposals forum thread

---

## Summary

This proposal establishes two funding mechanisms for ongoing RillCoin development and community programs: (1) a 5% allocation from each block reward directed to a transparent developer fund address, and (2) a community donation program with a publicly auditable multi-signature wallet. Together, these mechanisms ensure the project can sustain infrastructure, security, and ecosystem growth without compromising its founding principles: no ICO, no pre-mine, no token sale.

---

## Motivation

RillCoin is a mineable cryptocurrency with no pre-mine, no ICO, and no venture funding. Every coin enters circulation through proof of work. This is a principled design choice that aligns with the project's core thesis: wealth should flow like water, not collect in reservoirs before the current even starts.

But principled design does not eliminate practical needs. Software requires maintenance. Cryptographic implementations require professional security audits. Infrastructure requires upkeep. Developers require compensation. Community programs require funding. Without a sustainable funding model, the project faces three unacceptable outcomes:

1. **Dependency on volunteer labor.** Volunteer-driven development is fragile. Critical security patches and protocol upgrades cannot depend on the availability and goodwill of unpaid contributors indefinitely. Projects that rely exclusively on volunteerism tend toward slow decay or capture by a single well-resourced entity.

2. **Deferred security.** Professional third-party audits of consensus code, cryptographic implementations, and network protocols are expensive. Without dedicated funding, audits are delayed or skipped entirely. For a project handling real value, this is not an acceptable tradeoff.

3. **Informal, opaque funding.** Without a formal mechanism, funding inevitably happens anyway, through side channels, corporate sponsorship with strings attached, or individual benefactors whose influence is disproportionate and invisible. A formal, transparent fund is more honest than pretending the need does not exist.

The Monero model (voluntary donations only) is admirable but has demonstrated real limitations: chronic underfunding of critical infrastructure, difficulty retaining experienced contributors, and multi-month delays on security-critical work. RillCoin can learn from this history without repeating it.

A modest block reward allocation, combined with a voluntary donation program, provides two complementary funding streams: one predictable, one flexible. Both are fully transparent.

---

## Specification

### Part 1: Developer Fund Block Reward (5%)

**Mechanism.** Five percent (5%) of each block's coinbase reward is sent to a designated developer fund address. This allocation is enforced at the consensus level. Blocks that do not include the correct developer fund output are invalid.

**Example.** If the block reward is 50 RILL, the miner receives 47.50 RILL and the developer fund receives 2.50 RILL. The split is calculated using integer arithmetic consistent with RillCoin's consensus math (u64 with 10^8 precision, no floating point).

```
dev_fund_amount = block_reward * 5 / 100
miner_amount = block_reward - dev_fund_amount
```

Integer division truncation favors the miner. Any remainder stays with the miner, not the fund.

**Fund address.** A single, well-known address published in the protocol specification, the project website, and block explorers. The address is hardcoded in consensus rules. Changing it requires a protocol upgrade (hard fork) with community governance approval.

**Permitted uses.** Developer fund disbursements are restricted to:

- Core protocol development and maintenance
- Security audits by qualified third-party firms
- Developer compensation (full-time and contract)
- Infrastructure costs (nodes, CI/CD, monitoring, hosting)
- Bug bounty program payouts
- Community programs (documentation, education, translation)
- Legal and regulatory compliance costs

**Prohibited uses.** Developer fund disbursements must never be used for:

- Speculative investments or trading
- Marketing expenditures that make price predictions or financial promises
- Compensation to individuals who are not actively contributing to the project
- Any purpose not listed under permitted uses without a separate governance vote

**Transparency.** All developer fund transactions are on-chain and visible to anyone. In addition, the Core Team publishes a quarterly transparency report containing:

- Total funds received during the quarter
- Itemized list of all disbursements with recipient, amount, and purpose
- Current fund balance
- Planned expenditures for the following quarter
- A link to on-chain verification of all claimed transactions

Quarterly reports are published to the project website, the Discord #announcements channel, and the project's GitHub repository.

**Governance.** The fund is managed by the Core Team during the pre-mainnet and early mainnet phases. As on-chain governance matures, management authority transitions to the governance system. Specific transition criteria:

- Fund disbursements above 500 RILL require a published justification before execution
- Fund disbursements above 5,000 RILL require a 7-day community review period in #governance-general before execution
- Post-mainnet, the community may vote to modify the fund percentage, change the fund address, or dissolve the fund entirely

**Sunset clause.** The 5% allocation is not permanent. At block height corresponding to approximately 4 years post-mainnet launch, a mandatory governance vote is triggered. The community votes on one of four options:

1. Maintain the 5% allocation unchanged
2. Reduce the allocation (specific percentage to be proposed at vote time)
3. Redirect the allocation to a community-governed treasury (DAO or equivalent)
4. Eliminate the allocation entirely, returning 100% of block rewards to miners

If no vote is conducted or quorum is not reached, the allocation automatically reduces to 2.5% and a new vote is triggered one year later. If quorum is again not reached, the allocation drops to 0%.

The sunset clause ensures that the developer fund cannot persist indefinitely without active community consent. Inaction defaults to reduction, not continuation.

**Concentration decay interaction.** The developer fund address is subject to the same concentration decay rules as every other address. There is no exemption. If the fund accumulates beyond decay thresholds, it decays. This creates a natural incentive to disburse funds regularly rather than hoard them, which aligns with both the project's economic philosophy and good treasury management practice.

### Part 2: Community Donation Program

**Donation address.** A separate, dedicated donation address is published on the project website, in the Discord #welcome channel, and in the repository README. This address is distinct from the developer fund address.

**Multi-signature custody.** The donation wallet uses a 2-of-3 multi-signature scheme. The three keyholders are members of the Core Team, publicly identified by their cryptographic keys. Key rotation follows a documented procedure:

- Key rotation is announced 14 days in advance
- The old key remains valid during the transition window
- A signed statement from the outgoing and incoming keyholder is published
- At no point does any single individual hold two of the three keys

**Earmarking.** Donors may earmark their donation for a specific permitted purpose by including a standardized memo field in the transaction:

- `AUDIT` — Security audits
- `BOUNTY` — Bug bounty program
- `INFRA` — Infrastructure
- `DEV` — Developer compensation
- `COMMUNITY` — Community programs and events
- `GENERAL` — No restriction (default if no memo is provided)

Earmarked funds are tracked separately and may only be spent on their designated purpose. If earmarked funds are not needed for their designated purpose within 12 months, the Core Team publishes a notice and the funds are reclassified as general-purpose after a 30-day objection period.

**Transparency.** All donation wallet transactions are on-chain. The Core Team publishes a monthly donation report (shorter and simpler than the quarterly developer fund report) containing:

- Total donations received, broken down by earmark category
- All disbursements with recipient, amount, and purpose
- Current balance by category

**No donor perks.** Donations do not confer any special status, access, or influence. There are no donor tiers, no special Discord roles for donors, no early access to features, no governance weight bonuses. A donation is a contribution to public infrastructure, not a purchase. This is non-negotiable. The moment donations buy influence, the project's credibility on fairness is compromised.

**Relationship to developer fund.** The donation program is supplementary to the block reward allocation, not a replacement. The two mechanisms serve different purposes:

| | Developer Fund | Donation Program |
|---|---|---|
| Source | Block rewards (automatic) | Voluntary contributions |
| Predictability | High (proportional to hashrate) | Variable |
| Governance | Consensus-enforced | Multi-sig managed |
| Reporting | Quarterly | Monthly |
| Earmarking | No (general purpose) | Yes (donor-directed) |

---

## Drawbacks

This section is intentionally thorough. A proposal that does not honestly address its risks is not worth considering.

**Centralization of funds.** A 5% block reward allocation creates a funded entity (the Core Team) within a project that claims to oppose concentration. This is a real tension, not a superficial one. Even with transparency reports and sunset clauses, the team controlling the fund has disproportionate economic influence during the project's formative period. The mitigation (sunset clause, decay applicability, transition to community governance) is genuine but imperfect. The honest answer is that some degree of funded coordination is necessary for a young project, and the alternative (unfunded coordination) has a worse track record.

**Miner tax.** Miners receive 95% of the block reward instead of 100%. Every miner subsidizes the developer fund whether they agree with its expenditures or not. This is a real cost. The counterargument is that miners benefit from a well-maintained, professionally audited, actively developed protocol, and that 5% is a modest premium for that assurance. But the cost is real and should not be minimized.

**Governance capture risk.** Until on-chain governance is operational, the Core Team unilaterally decides how the developer fund is spent. Transparency reports reduce information asymmetry but do not eliminate power asymmetry. A small team with access to a growing treasury has both the means and the opportunity to entrench itself. The sunset clause is designed to prevent this, but it is a future mechanism addressing a present risk.

**Donation earmarking complexity.** Earmarked donations add accounting complexity. Edge cases will arise: what if audited code is also infrastructure? What if a community event also serves a developer recruitment purpose? Misclassification accusations are likely. The 12-month reclassification window partially addresses this, but the accounting overhead is real.

**No-perk donation disincentive.** The strict no-perks policy means there is no extrinsic incentive to donate beyond altruism and enlightened self-interest. Donation volume may be low as a result. This is an acceptable tradeoff. The alternative (donor perks) creates exactly the kind of inequality that RillCoin's design is built to prevent.

**Sunset clause gaming.** The automatic reduction on quorum failure could be gamed by parties who benefit from fund elimination simply encouraging voter apathy. Conversely, the Core Team could attempt to set quorum thresholds that favor continuation. The specific quorum parameters are left as an open question for community input.

---

## Alternatives Considered

**0% block reward (Monero model).** Rely entirely on voluntary donations and community crowdfunding campaigns. This is the purest approach and avoids any miner tax. It was rejected because Monero's experience demonstrates that voluntary funding alone results in chronic underfunding of critical work, delayed security audits, and high contributor turnover. RillCoin should learn from this rather than repeat it.

**10% block reward (Dash model).** A larger allocation provides more funding headroom and could support a broader range of initiatives including marketing, partnerships, and ecosystem grants. This was rejected as excessive for a project that has not yet launched its mainnet. A higher percentage increases the centralization concern proportionally. If 5% proves insufficient, the community can vote to increase it. Starting lower is more defensible than starting higher.

**20% block reward (Zcash founders reward model).** Zcash allocated 20% of block rewards to founders and early investors for the first four years. This model was rejected outright. It conflates development funding with founder compensation and investor returns, creating exactly the kind of concentrated benefit that RillCoin's design philosophy opposes. The Zcash community's contentious debates over the founders reward are instructive.

**Time-limited full allocation.** Allocate 10% for the first year, then drop to 0%. This front-loads funding but creates a cliff that could destabilize development mid-stream. A steady 5% with a sunset review is more sustainable.

**Community-voted per-disbursement approval.** Require a governance vote for every individual expenditure from the developer fund. This was rejected as impractical. Routine expenses (server costs, contractor invoices, audit deposits) cannot wait for multi-day voting periods. The tiered threshold system (automatic below 500 RILL, review period above 5,000 RILL) is a pragmatic middle ground.

**Proof-of-burn donation verification.** Allow donors to provably burn coins as donations, removing them from circulation rather than transferring them to a fund. Conceptually interesting but economically wasteful. Burned coins cannot fund development. This mechanism might have a role in future tokenomics proposals but is not appropriate for a development fund.

---

## Open Questions

The following items require community input before this proposal moves from Draft to Under Review:

1. **Sunset vote quorum.** What quorum threshold should the 4-year sunset vote require? Too low and a small minority can eliminate funding. Too high and the status quo persists by default. Proposed starting point for discussion: 20% of active addresses (addresses that have transacted in the prior 90 days) must participate, with a simple majority to decide.

2. **Disbursement threshold values.** The proposed thresholds (500 RILL for justification, 5,000 RILL for community review) are placeholders. They should be calibrated to mainnet economic conditions. What methodology should be used to set and adjust these thresholds?

3. **Core Team keyholder identity.** Should the three multi-sig keyholders for the donation wallet be publicly identified by legal name, or is cryptographic identity (public keys linked to known pseudonymous contributors) sufficient?

4. **Audit firm selection.** Should the community have input on which firms are engaged for security audits funded by the developer fund, or is this an operational decision delegated to the Core Team?

5. **Transition trigger.** What specific criteria should trigger the transition from Core Team management to community governance of the developer fund? A block height? A hashrate threshold? A governance system maturity assessment?

6. **Cross-fund transfers.** Should earmarked donation funds ever be transferable to the developer fund, or must the two pools remain strictly separate?

---

*This proposal follows the RCP (RillCoin Proposal) format established in the #proposals channel guidelines. Discussion should take place in the thread on this post. Governance Ping subscribers have been notified.*
