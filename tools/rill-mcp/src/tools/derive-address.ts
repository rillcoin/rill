import { z } from "zod";
import { deriveAddress as apiDerive } from "../clients/faucet.js";

export const deriveAddressSchema = z.object({
  mnemonic: z.string().describe("BIP-39 mnemonic phrase to derive the address from"),
});

export async function deriveAddress(args: z.infer<typeof deriveAddressSchema>) {
  const data = await apiDerive(args.mnemonic);
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**Wallet Restored**`,
          `- **Address:** \`${data.address}\``,
          ``,
          `> This address was derived from your mnemonic. Use \`check_balance\` to see your funds.`,
        ].join("\n"),
      },
    ],
  };
}
