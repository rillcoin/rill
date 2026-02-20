import { describe, it, expect } from "vitest";
import { rillsToRill, rillToRills, formatBalance, COIN } from "../utils/formatting.js";

describe("rillsToRill", () => {
  it("converts zero", () => {
    expect(rillsToRill(0n)).toBe("0");
  });

  it("converts whole RILL", () => {
    expect(rillsToRill(COIN)).toBe("1");
    expect(rillsToRill(50n * COIN)).toBe("50");
  });

  it("converts fractional RILL", () => {
    expect(rillsToRill(150_000_000n)).toBe("1.5");
    expect(rillsToRill(100_000_001n)).toBe("1.00000001");
  });

  it("strips trailing zeros", () => {
    expect(rillsToRill(10_000_000n)).toBe("0.1");
    expect(rillsToRill(1_000_000n)).toBe("0.01");
  });
});

describe("rillToRills", () => {
  it("converts whole RILL", () => {
    expect(rillToRills(1)).toBe(COIN);
    expect(rillToRills(50)).toBe(50n * COIN);
  });

  it("converts fractional RILL", () => {
    expect(rillToRills(1.5)).toBe(150_000_000n);
    expect(rillToRills(0.00000001)).toBe(1n);
  });

  it("converts string input", () => {
    expect(rillToRills("1.5")).toBe(150_000_000n);
    expect(rillToRills("0")).toBe(0n);
  });

  it("handles precision edge case", () => {
    expect(rillToRills("0.12345678")).toBe(12_345_678n);
  });
});

describe("formatBalance", () => {
  it("formats with unit label", () => {
    const result = formatBalance(500n * COIN);
    expect(result).toContain("500 RILL");
    expect(result).toContain("rills");
  });
});
