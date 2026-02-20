import { z } from "zod";
import { getStats } from "../clients/explorer.js";

export const networkStatusSchema = z.object({});

export async function networkStatus() {
  const stats = await getStats();
  return {
    content: [
      {
        type: "text" as const,
        text: [
          `**RillCoin Network Status**`,
          `- **Block Height:** ${stats.height.toLocaleString()}`,
          `- **Best Hash:** \`${stats.best_hash}\``,
          `- **Circulating Supply:** ${stats.circulating_supply} RILL`,
          `- **Decay Pool:** ${stats.decay_pool} RILL`,
          `- **UTXO Count:** ${stats.utxo_count?.toLocaleString() ?? "N/A"}`,
          `- **Peers:** ${stats.peer_count}`,
          `- **Mempool:** ${stats.mempool_size} txs (${stats.mempool_bytes?.toLocaleString() ?? 0} bytes)`,
          `- **Initial Block Download:** ${stats.ibd ? "Yes" : "No"}`,
        ].join("\n"),
      },
    ],
  };
}
