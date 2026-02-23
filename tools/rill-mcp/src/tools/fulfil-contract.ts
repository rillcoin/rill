import { z } from "zod";
import { fulfilContract as apiFulfil } from "../clients/faucet.js";

export const fulfilContractSchema = z.object({
  mnemonic: z.string().describe("BIP-39 mnemonic of the fulfilling agent"),
  contract_id: z.string().describe("Contract ID (64-character hex txid from contract creation)"),
});

export async function fulfilContract(args: z.infer<typeof fulfilContractSchema>) {
  const data = await apiFulfil(args.mnemonic, args.contract_id);
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**Contract fulfilled!**`,
          `- **TxID:** \`${data.txid}\``,
          `- **Contract:** \`${args.contract_id}\``,
        ].join("\n"),
      },
    ],
  };
}
