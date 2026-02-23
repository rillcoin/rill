import { z } from "zod";
import { registerAgent as apiRegister } from "../clients/faucet.js";

export const registerAgentSchema = z.object({
  mnemonic: z.string().describe("BIP-39 mnemonic phrase for the wallet to register as an agent"),
});

export async function registerAgent(args: z.infer<typeof registerAgentSchema>) {
  const data = await apiRegister(args.mnemonic);
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**Agent registration submitted!**`,
          `- **TxID:** \`${data.txid}\``,
          ``,
          `The wallet is now registered as an agent. It will appear in conduct profiles once the transaction is confirmed.`,
        ].join("\n"),
      },
    ],
  };
}
