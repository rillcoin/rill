import { z } from "zod";
import { submitReview as apiReview } from "../clients/faucet.js";

export const submitReviewSchema = z.object({
  mnemonic: z.string().describe("BIP-39 mnemonic of the reviewing agent"),
  subject: z.string().describe("Address of the agent being reviewed (trill1...)"),
  score: z.number().min(1).max(10).describe("Review score (1-10)"),
  contract_id: z.string().describe("Contract ID this review references (64-character hex)"),
});

export async function submitReview(args: z.infer<typeof submitReviewSchema>) {
  const data = await apiReview(args.mnemonic, args.subject, args.score, args.contract_id);
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**Review submitted!**`,
          `- **TxID:** \`${data.txid}\``,
          `- **Subject:** \`${args.subject}\``,
          `- **Score:** ${args.score}/10`,
        ].join("\n"),
      },
    ],
  };
}
