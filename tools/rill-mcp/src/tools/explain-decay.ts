import { z } from "zod";
import { calculateDecay, COIN, CONCENTRATION_PRECISION, DECAY_C_THRESHOLD_PPB } from "../utils/decay.js";
import { rillToRills } from "../utils/formatting.js";

export const explainDecaySchema = z.object({
  balance_rill: z.number().positive().describe("Balance in RILL to analyze"),
  blocks_held: z.number().int().min(0).describe("Number of blocks the UTXO has been held"),
  concentration_pct: z
    .number()
    .optional()
    .describe("Optional: concentration as percentage of supply (e.g. 0.5 for 0.5%). If omitted, calculated from balance vs 21M max supply."),
});

export async function explainDecay(args: z.infer<typeof explainDecaySchema>) {
  const balanceRills = rillToRills(args.balance_rill as number);
  const blocksHeld = BigInt(args.blocks_held as number);

  let concentrationPpb: bigint | undefined;
  if (args.concentration_pct != null) {
    const pct = args.concentration_pct as number;
    // Convert percentage to PPB: 0.5% → 5_000_000 PPB
    concentrationPpb = BigInt(Math.round(pct * Number(CONCENTRATION_PRECISION) / 100));
  }

  const result = calculateDecay(balanceRills, blocksHeld, concentrationPpb);

  const thresholdPct = Number(DECAY_C_THRESHOLD_PPB * 10000n / CONCENTRATION_PRECISION) / 100;

  let narrative: string;
  if (result.belowThreshold) {
    narrative = [
      `**No decay applies.**`,
      ``,
      `Your ${result.balanceRill} RILL at ${result.concentrationPct} concentration is **below the decay threshold** of ${thresholdPct}% of total supply.`,
      ``,
      `Decay only activates when a single address holds more than ${thresholdPct}% of the circulating supply. Your holdings are safe from decay at this concentration level.`,
      ``,
      `> **How it works:** RillCoin uses a sigmoid (S-curve) decay function. Below ${thresholdPct}%, the decay rate is zero. Above that, the rate increases smoothly up to a maximum of 15% per block at extreme concentrations. This discourages wealth hoarding while leaving normal users unaffected.`,
    ].join("\n");
  } else {
    narrative = [
      `**Decay Analysis for ${result.balanceRill} RILL**`,
      ``,
      `| Metric | Value |`,
      `|--------|-------|`,
      `| Balance | ${result.balanceRill} RILL |`,
      `| Concentration | ${result.concentrationPct} of supply |`,
      `| Decay rate | ${result.decayRatePerBlock} per block |`,
      `| Blocks held | ${result.blocksHeld.toLocaleString()} (~${result.hoursElapsed}) |`,
      `| **Decay amount** | **${result.decayAmountRill} RILL** |`,
      `| **Effective value** | **${result.effectiveValueRill} RILL** |`,
      ``,
      `Over ${result.blocksHeld.toLocaleString()} blocks (~${result.hoursElapsed}), your ${result.balanceRill} RILL would decay by **${result.decayAmountRill} RILL**, leaving an effective balance of **${result.effectiveValueRill} RILL**.`,
      ``,
      `> **Decay flows to the mining pool**, where it is redistributed to miners as block rewards. This is RillCoin's core mechanism: "Wealth should flow like water."`,
      ``,
      `> **Strategies to reduce decay:** Split holdings across multiple addresses, spend or distribute RILL regularly, or keep balances below the ${thresholdPct}% threshold.`,
    ].join("\n");
  }

  return {
    content: [{ type: "text" as const, text: narrative }],
  };
}
