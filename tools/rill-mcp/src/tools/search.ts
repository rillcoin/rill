import { z } from "zod";
import { search as apiSearch } from "../clients/explorer.js";

export const searchSchema = z.object({
  query: z.string().describe("Search query — address, block height, block hash, or transaction ID"),
});

export async function search(args: z.infer<typeof searchSchema>) {
  const result = await apiSearch(args.query);
  const typeLabel = { address: "Address", block: "Block", tx: "Transaction" }[result.type] ?? result.type;
  return {
    content: [
      {
        type: "text" as const,
        text: `**Found:** ${typeLabel} → \`${result.value}\`\n\nUse \`get_block\`, \`get_transaction\`, or \`check_balance\` for details.`,
      },
    ],
  };
}
