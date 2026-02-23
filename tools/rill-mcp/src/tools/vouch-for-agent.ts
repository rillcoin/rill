import { z } from "zod";
import { vouchForAgent as apiVouch } from "../clients/faucet.js";

export const vouchForAgentSchema = z.object({
  mnemonic: z.string().describe("BIP-39 mnemonic of the vouching agent"),
  target_address: z.string().describe("Address of the agent to vouch for (trill1...)"),
});

export async function vouchForAgent(args: z.infer<typeof vouchForAgentSchema>) {
  const data = await apiVouch(args.mnemonic, args.target_address);
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**Vouch submitted!**`,
          `- **TxID:** \`${data.txid}\``,
          `- **Target:** \`${args.target_address}\``,
        ].join("\n"),
      },
    ],
  };
}
