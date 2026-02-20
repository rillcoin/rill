import { z } from "zod";
import { sendRill as apiSendRill } from "../clients/faucet.js";

export const sendRillSchema = z.object({
  mnemonic: z.string().describe("BIP-39 mnemonic phrase for the sending wallet"),
  to: z.string().describe("Recipient address (trill1...)"),
  amount_rill: z.number().positive().describe("Amount of RILL to send"),
});

export async function sendRill(args: z.infer<typeof sendRillSchema>) {
  const data = await apiSendRill(args.mnemonic, args.to, args.amount_rill);
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**Transaction sent!**`,
          `- **TxID:** \`${data.txid}\``,
          `- **Amount:** ${data.amount_rill} RILL`,
          `- **Fee:** ${data.fee_rill} RILL`,
          `- **To:** \`${args.to}\``,
        ].join("\n"),
      },
    ],
  };
}
