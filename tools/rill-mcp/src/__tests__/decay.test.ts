import { describe, it, expect } from "vitest";
import {
  sigmoidPositive,
  decayRatePpb,
  fixedPow,
  computeDecay,
  calculateDecay,
  COIN,
  DECAY_C_THRESHOLD_PPB,
  DECAY_R_MAX_PPB,
  DECAY_PRECISION,
  CONCENTRATION_PRECISION,
} from "../utils/decay.js";

// Cross-validation against Rust test vectors from crates/rill-decay/src/sigmoid.rs

describe("sigmoidPositive (cross-validated with Rust)", () => {
  it("sigmoid(0) = 500_000_000", () => {
    expect(sigmoidPositive(0n)).toBe(500_000_000n);
  });

  it("sigmoid(0.5) = 622_459_331", () => {
    expect(sigmoidPositive(500_000_000n)).toBe(622_459_331n);
  });

  it("sigmoid(1.0) = 731_058_579", () => {
    expect(sigmoidPositive(1_000_000_000n)).toBe(731_058_579n);
  });

  it("sigmoid(2.0) = 880_797_078", () => {
    expect(sigmoidPositive(2_000_000_000n)).toBe(880_797_078n);
  });

  it("sigmoid(8.0) = 999_664_650", () => {
    expect(sigmoidPositive(8_000_000_000n)).toBe(999_664_650n);
  });

  it("saturates beyond table", () => {
    expect(sigmoidPositive(100_000_000_000n)).toBe(999_664_650n);
  });

  it("interpolation midpoint matches Rust", () => {
    const val = sigmoidPositive(250_000_000n);
    const expected = 500_000_000n + (622_459_331n - 500_000_000n) / 2n;
    expect(val).toBe(expected);
  });
});

describe("fixedPow (cross-validated with Rust)", () => {
  it("base^0 = precision", () => {
    expect(fixedPow(5_000_000_000n, 0n, DECAY_PRECISION)).toBe(DECAY_PRECISION);
  });

  it("base^1 = base", () => {
    expect(fixedPow(8_500_000_000n, 1n, DECAY_PRECISION)).toBe(8_500_000_000n);
  });

  it("0.8^2 = 0.64", () => {
    expect(fixedPow(8_000_000_000n, 2n, DECAY_PRECISION)).toBe(6_400_000_000n);
  });

  it("0.9^3 = 0.729", () => {
    expect(fixedPow(9_000_000_000n, 3n, DECAY_PRECISION)).toBe(7_290_000_000n);
  });

  it("1.0^1000000 = 1.0", () => {
    expect(fixedPow(DECAY_PRECISION, 1_000_000n, DECAY_PRECISION)).toBe(DECAY_PRECISION);
  });

  it("0^100 = 0", () => {
    expect(fixedPow(0n, 100n, DECAY_PRECISION)).toBe(0n);
  });

  it("0.9999^10000 ≈ e^(-1) ≈ 0.3679", () => {
    const result = fixedPow(9_999_000_000n, 10_000n, DECAY_PRECISION);
    expect(result).toBeGreaterThan(3_600_000_000n);
    expect(result).toBeLessThan(3_800_000_000n);
  });
});

describe("decayRatePpb (cross-validated with Rust)", () => {
  it("zero below threshold", () => {
    expect(decayRatePpb(0n)).toBe(0n);
    expect(decayRatePpb(DECAY_C_THRESHOLD_PPB)).toBe(0n);
    expect(decayRatePpb(DECAY_C_THRESHOLD_PPB / 2n)).toBe(0n);
  });

  it("nonzero above threshold", () => {
    expect(decayRatePpb(DECAY_C_THRESHOLD_PPB + 1n)).toBeGreaterThan(0n);
  });

  it("rate at threshold boundary ≈ 750M (R_MAX * 0.5)", () => {
    const rate = decayRatePpb(DECAY_C_THRESHOLD_PPB + 1n);
    expect(rate).toBeGreaterThanOrEqual(740_000_000n);
    expect(rate).toBeLessThanOrEqual(760_000_000n);
  });

  it("increases with concentration", () => {
    const r1 = decayRatePpb(DECAY_C_THRESHOLD_PPB + 100_000n);
    const r2 = decayRatePpb(DECAY_C_THRESHOLD_PPB + 500_000n);
    const r3 = decayRatePpb(DECAY_C_THRESHOLD_PPB + 1_000_000n);
    expect(r1).toBeLessThan(r2);
    expect(r2).toBeLessThan(r3);
  });

  it("bounded by R_MAX", () => {
    const rate = decayRatePpb(CONCENTRATION_PRECISION);
    expect(rate).toBeLessThanOrEqual(DECAY_R_MAX_PPB);
  });

  it("near R_MAX at 1% concentration", () => {
    const rate = decayRatePpb(10_000_000n);
    expect(rate).toBeGreaterThan(DECAY_R_MAX_PPB * 99n / 100n);
  });
});

describe("computeDecay (cross-validated with Rust)", () => {
  it("zero blocks → zero decay", () => {
    expect(computeDecay(1000n * COIN, DECAY_C_THRESHOLD_PPB + 100_000n, 0n)).toBe(0n);
  });

  it("zero value → zero decay", () => {
    expect(computeDecay(0n, DECAY_C_THRESHOLD_PPB + 100_000n, 100n)).toBe(0n);
  });

  it("below threshold → zero decay", () => {
    expect(computeDecay(1000n * COIN, 0n, 1000n)).toBe(0n);
    expect(computeDecay(1000n * COIN, DECAY_C_THRESHOLD_PPB, 1000n)).toBe(0n);
  });

  it("increases with blocks", () => {
    const conc = DECAY_C_THRESHOLD_PPB + 500_000n;
    const value = 1000n * COIN;
    const d1 = computeDecay(value, conc, 1n);
    const d10 = computeDecay(value, conc, 10n);
    const d100 = computeDecay(value, conc, 100n);
    expect(d1).toBeLessThan(d10);
    expect(d10).toBeLessThan(d100);
  });

  it("never exceeds nominal", () => {
    const value = 1000n * COIN;
    const decay = computeDecay(value, CONCENTRATION_PRECISION, 1_000_000n);
    expect(decay).toBeLessThanOrEqual(value);
  });

  it("compound less than linear", () => {
    const conc = DECAY_C_THRESHOLD_PPB + 500_000n;
    const value = 1000n * COIN;
    const d1 = computeDecay(value, conc, 1n);
    const d10 = computeDecay(value, conc, 10n);
    expect(d10).toBeLessThan(d1 * 10n);
  });
});

describe("calculateDecay (high-level)", () => {
  it("below threshold returns zero decay", () => {
    const result = calculateDecay(100n * COIN, 1000n);
    expect(result.belowThreshold).toBe(true);
    expect(result.decayAmount).toBe(0n);
  });

  it("above threshold returns nonzero decay", () => {
    // 0.5% concentration
    const result = calculateDecay(
      100_000n * COIN,
      1000n,
      5_000_000n, // 0.5% in PPB
    );
    expect(result.belowThreshold).toBe(false);
    expect(result.decayAmount).toBeGreaterThan(0n);
    expect(result.effectiveValue).toBeLessThan(100_000n * COIN);
  });

  it("effective + decay = nominal", () => {
    const result = calculateDecay(500n * COIN, 50n, 5_000_000n);
    expect(result.effectiveValue + result.decayAmount).toBe(500n * COIN);
  });
});
