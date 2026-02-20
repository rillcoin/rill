/**
 * TypeScript port of the RillCoin sigmoid decay calculation.
 * Uses BigInt throughout for consensus-compatible integer arithmetic.
 * For EDUCATION ONLY — not consensus-critical.
 */

// Protocol constants (from crates/rill-core/src/constants.rs)
export const COIN = 100_000_000n;
export const DECAY_C_THRESHOLD_PPB = 1_000_000n;   // 0.1% of supply triggers decay
export const DECAY_R_MAX_PPB = 1_500_000_000n;      // 15% max rate per block
export const DECAY_PRECISION = 10_000_000_000n;      // Fixed-point denominator
export const DECAY_K = 2000n;                        // Sigmoid steepness
export const CONCENTRATION_PRECISION = 1_000_000_000n;
export const BLOCK_TIME_SECS = 60n;

// Sigmoid lookup table (from crates/rill-decay/src/sigmoid.rs)
const SIGMOID_PRECISION = 1_000_000_000n;
const TABLE_STEP = CONCENTRATION_PRECISION / 2n; // 500_000_000

const SIGMOID_TABLE: bigint[] = [
  500_000_000n, // sigmoid(0.0)
  622_459_331n, // sigmoid(0.5)
  731_058_579n, // sigmoid(1.0)
  817_574_476n, // sigmoid(1.5)
  880_797_078n, // sigmoid(2.0)
  924_141_820n, // sigmoid(2.5)
  952_574_127n, // sigmoid(3.0)
  970_687_769n, // sigmoid(3.5)
  982_013_790n, // sigmoid(4.0)
  989_013_057n, // sigmoid(4.5)
  993_307_149n, // sigmoid(5.0)
  995_929_862n, // sigmoid(5.5)
  997_527_377n, // sigmoid(6.0)
  998_498_883n, // sigmoid(6.5)
  999_088_949n, // sigmoid(7.0)
  999_447_221n, // sigmoid(7.5)
  999_664_650n, // sigmoid(8.0)
];

/** Evaluate sigmoid(x) using the lookup table with linear interpolation. */
export function sigmoidPositive(xScaled: bigint): bigint {
  const index = Number(xScaled / TABLE_STEP);

  if (index >= SIGMOID_TABLE.length - 1) {
    return SIGMOID_TABLE[SIGMOID_TABLE.length - 1];
  }

  const frac = xScaled % TABLE_STEP;
  const lo = SIGMOID_TABLE[index];
  const hi = SIGMOID_TABLE[index + 1];
  const diff = hi - lo;

  return lo + (diff * frac) / TABLE_STEP;
}

/** Compute the decay rate in parts-per-billion for a given concentration. */
export function decayRatePpb(concentrationPpb: bigint): bigint {
  if (concentrationPpb <= DECAY_C_THRESHOLD_PPB) {
    return 0n;
  }

  const diff = concentrationPpb - DECAY_C_THRESHOLD_PPB;
  const argScaled = DECAY_K * diff;
  const sigmoidVal = sigmoidPositive(argScaled);
  return (DECAY_R_MAX_PPB * sigmoidVal) / SIGMOID_PRECISION;
}

/** Fixed-point exponentiation: (base/precision)^exp in fixed-point. */
export function fixedPow(base: bigint, exp: bigint, precision: bigint): bigint {
  if (exp === 0n) return precision;

  let result = precision;
  let b = base;
  let e = exp;

  while (e > 0n) {
    if (e & 1n) {
      result = (result * b) / precision;
    }
    e >>= 1n;
    if (e > 0n) {
      b = (b * b) / precision;
    }
  }

  return result;
}

/** Compute total decay for a UTXO held over `blocksHeld` blocks. */
export function computeDecay(
  nominalValue: bigint,
  concentrationPpb: bigint,
  blocksHeld: bigint,
): bigint {
  if (blocksHeld === 0n || nominalValue === 0n) return 0n;

  const rate = decayRatePpb(concentrationPpb);
  if (rate === 0n) return 0n;
  if (rate >= DECAY_PRECISION) return nominalValue;

  const retention = DECAY_PRECISION - rate;
  const retentionTotal = fixedPow(retention, blocksHeld, DECAY_PRECISION);
  const effective = (nominalValue * retentionTotal) / DECAY_PRECISION;

  const decay = nominalValue - effective;
  return decay < 0n ? 0n : decay;
}

export interface DecayResult {
  balanceRill: string;
  balanceRills: bigint;
  blocksHeld: bigint;
  concentrationPpb: bigint;
  concentrationPct: string;
  decayRatePerBlock: string;
  decayAmount: bigint;
  decayAmountRill: string;
  effectiveValue: bigint;
  effectiveValueRill: string;
  hoursElapsed: string;
  belowThreshold: boolean;
}

/** High-level decay calculation returning all relevant figures. */
export function calculateDecay(
  balanceRills: bigint,
  blocksHeld: bigint,
  concentrationPpb?: bigint,
): DecayResult {
  // If concentration not provided, we can't compute it without knowing total supply.
  // Default to the balance as a fraction of max supply (21M RILL).
  const maxSupply = 21_000_000n * COIN;
  const concPpb = concentrationPpb ??
    (balanceRills * CONCENTRATION_PRECISION) / maxSupply;

  const belowThreshold = concPpb <= DECAY_C_THRESHOLD_PPB;
  const rate = decayRatePpb(concPpb);
  const decayAmount = computeDecay(balanceRills, concPpb, blocksHeld);
  const effectiveValue = balanceRills - decayAmount;

  const concPct = Number(concPpb * 10000n / CONCENTRATION_PRECISION) / 100;
  const ratePerBlock = Number(rate) / Number(DECAY_PRECISION) * 100;
  const hours = Number(blocksHeld * BLOCK_TIME_SECS) / 3600;

  const rillsToStr = (r: bigint): string => {
    const whole = r / COIN;
    const frac = r % COIN;
    if (frac === 0n) return whole.toString();
    return `${whole}.${frac.toString().padStart(8, "0").replace(/0+$/, "")}`;
  };

  return {
    balanceRill: rillsToStr(balanceRills),
    balanceRills,
    blocksHeld,
    concentrationPpb: concPpb,
    concentrationPct: `${concPct}%`,
    decayRatePerBlock: `${ratePerBlock.toFixed(6)}%`,
    decayAmount,
    decayAmountRill: rillsToStr(decayAmount),
    effectiveValue,
    effectiveValueRill: rillsToStr(effectiveValue),
    hoursElapsed: `${hours.toFixed(1)} hours`,
    belowThreshold,
  };
}
