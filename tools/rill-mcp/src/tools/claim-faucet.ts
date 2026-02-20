import { z } from "zod";
import { claimFaucet as apiClaim } from "../clients/faucet.js";

export const claimFaucetSchema = z.object({
  address: z.string().describe("Testnet address (trill1...) to receive RILL"),
});

export async function claimFaucet(args: z.infer<typeof claimFaucetSchema>) {
  const data = await apiClaim(args.address);
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**Faucet claim successful!**`,
          `- **Amount:** ${data.amount_rill} RILL`,
          `- **Address:** \`${data.address}\``,
          `- **TxID:** \`${data.txid}\``,
        ].join("\n"),
      },
    ],
  };
}
