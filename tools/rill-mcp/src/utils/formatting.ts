/** 1 RILL = 10^8 rills */
export const COIN = 100_000_000n;

/** Convert rills (u64 integer) to RILL (human-readable decimal string). */
export function rillsToRill(rills: bigint): string {
  const whole = rills / COIN;
  const frac = rills % COIN;
  if (frac === 0n) return whole.toString();
  const fracStr = frac.toString().padStart(8, "0").replace(/0+$/, "");
  return `${whole}.${fracStr}`;
}

/** Convert RILL (decimal number or string) to rills. */
export function rillToRills(rill: number | string): bigint {
  const s = typeof rill === "number" ? rill.toFixed(8) : rill;
  const [wholePart, fracPart = ""] = s.split(".");
  const paddedFrac = fracPart.padEnd(8, "0").slice(0, 8);
  return BigInt(wholePart) * COIN + BigInt(paddedFrac);
}

/** Format a rills amount as a human-friendly string, e.g. "12.5 RILL (1,250,000,000 rills)" */
export function formatBalance(rills: bigint): string {
  const rill = rillsToRill(rills);
  return `${rill} RILL (${rills.toLocaleString()} rills)`;
}
