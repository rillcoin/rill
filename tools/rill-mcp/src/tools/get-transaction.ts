import { z } from "zod";
import { getTransaction as apiGetTx } from "../clients/explorer.js";

export const getTransactionSchema = z.object({
  txid: z.string().describe("Transaction ID (64-char hex hash)"),
});

export async function getTransaction(args: z.infer<typeof getTransactionSchema>) {
  const tx = await apiGetTx(args.txid);
  return {
    content: [
      {
        type: "text" as const,
        text: `**Transaction** \`${args.txid}\`\n\`\`\`json\n${JSON.stringify(tx, null, 2)}\n\`\`\``,
      },
    ],
  };
}
