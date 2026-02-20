import { z } from "zod";
import { getBalance } from "../clients/faucet.js";

export const checkBalanceSchema = z.object({
  address: z.string().describe("RillCoin address (trill1... for testnet)"),
});

export async function checkBalance(args: z.infer<typeof checkBalanceSchema>) {
  const data = await getBalance(args.address);
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**Balance for** \`${data.address}\``,
          `- **Balance:** ${data.balance_rill} RILL`,
          `- **Balance (rills):** ${data.balance_rills.toLocaleString()}`,
          `- **UTXOs:** ${data.utxo_count}`,
        ].join("\n"),
      },
    ],
  };
}
