import { z } from "zod";
import { getAgentProfile } from "../clients/faucet.js";

export const getConductProfileSchema = z.object({
  address: z.string().describe("RillCoin address (trill1...) to look up"),
});

export async function getConductProfile(args: z.infer<typeof getConductProfileSchema>) {
  const p = await getAgentProfile(args.address);
  const multiplier = (p.conduct_multiplier_bps / 10000).toFixed(2);
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**Conduct Profile for** \`${p.address}\``,
          `- **Wallet type:** ${p.wallet_type}`,
          `- **Conduct score:** ${p.conduct_score}/1000`,
          `- **Decay multiplier:** ${multiplier}x (${p.conduct_multiplier_bps} BPS)`,
          `- **Undertow active:** ${p.undertow_active ? "YES" : "no"}`,
          p.wallet_type === "agent"
            ? `- **Registered at block:** ${p.registered_at_block}\n- **Wallet age:** ${p.wallet_age_blocks} blocks`
            : "_(Not registered as agent wallet)_",
        ].join("\n"),
      },
    ],
  };
}
