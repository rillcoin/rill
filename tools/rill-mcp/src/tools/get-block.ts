import { z } from "zod";
import { getBlock as apiGetBlock } from "../clients/explorer.js";

export const getBlockSchema = z.object({
  id: z.string().describe("Block height (number) or block hash (64-char hex)"),
});

export async function getBlock(args: z.infer<typeof getBlockSchema>) {
  const block = await apiGetBlock(args.id);
  return {
    content: [
      {
        type: "text" as const,
        text: `**Block ${block["height"] ?? args.id}**\n\`\`\`json\n${JSON.stringify(block, null, 2)}\n\`\`\``,
      },
    ],
  };
}
