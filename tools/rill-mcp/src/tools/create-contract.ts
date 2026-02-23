import { z } from "zod";
import { createContract as apiCreate } from "../clients/faucet.js";

export const createContractSchema = z.object({
  mnemonic: z.string().describe("BIP-39 mnemonic of the initiating agent"),
  counterparty: z.string().describe("Address of the counterparty agent (trill1...)"),
  value_rill: z.number().describe("Contract value in RILL"),
});

export async function createContract(args: z.infer<typeof createContractSchema>) {
  const data = await apiCreate(args.mnemonic, args.counterparty, args.value_rill);
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**Contract created!**`,
          `- **Contract ID (TxID):** \`${data.txid}\``,
          `- **Counterparty:** \`${args.counterparty}\``,
          `- **Value:** ${args.value_rill} RILL`,
        ].join("\n"),
      },
    ],
  };
}
