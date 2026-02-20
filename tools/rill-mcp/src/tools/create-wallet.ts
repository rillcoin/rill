import { z } from "zod";
import { createWallet as apiCreate } from "../clients/faucet.js";

export const createWalletSchema = z.object({});

export async function createWallet() {
  const data = await apiCreate();
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**New RillCoin Wallet Created**`,
          ``,
          `**Address:** \`${data.address}\``,
          ``,
          `**Mnemonic (SAVE THIS — it cannot be recovered):**`,
          `\`\`\``,
          data.mnemonic,
          `\`\`\``,
          ``,
          `> **Security:** Store your mnemonic offline. Anyone with these words can spend your RILL. This is a testnet wallet.`,
        ].join("\n"),
      },
    ],
  };
}
