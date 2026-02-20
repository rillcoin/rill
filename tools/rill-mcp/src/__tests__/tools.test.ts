import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the HTTP clients
vi.mock("../clients/faucet.js", () => ({
  createWallet: vi.fn(),
  getBalance: vi.fn(),
  sendRill: vi.fn(),
  claimFaucet: vi.fn(),
  deriveAddress: vi.fn(),
}));

vi.mock("../clients/explorer.js", () => ({
  getStats: vi.fn(),
  getBlock: vi.fn(),
  getTransaction: vi.fn(),
  search: vi.fn(),
}));

import * as faucet from "../clients/faucet.js";
import * as explorer from "../clients/explorer.js";

import { checkBalance } from "../tools/check-balance.js";
import { sendRill } from "../tools/send-rill.js";
import { getBlock } from "../tools/get-block.js";
import { getTransaction } from "../tools/get-transaction.js";
import { networkStatus } from "../tools/network-status.js";
import { claimFaucet } from "../tools/claim-faucet.js";
import { createWallet } from "../tools/create-wallet.js";
import { deriveAddress } from "../tools/derive-address.js";
import { search } from "../tools/search.js";
import { explainDecay } from "../tools/explain-decay.js";

beforeEach(() => {
  vi.clearAllMocks();
});

describe("check_balance", () => {
  it("returns formatted balance", async () => {
    vi.mocked(faucet.getBalance).mockResolvedValue({
      address: "trill1abc",
      balance_rill: 50.5,
      balance_rills: 5_050_000_000,
      utxo_count: 3,
    });

    const result = await checkBalance({ address: "trill1abc" });
    expect(result.content[0].text).toContain("50.5 RILL");
    expect(result.content[0].text).toContain("trill1abc");
    expect(result.content[0].text).toContain("3");
  });
});

describe("send_rill", () => {
  it("returns txid and fee", async () => {
    vi.mocked(faucet.sendRill).mockResolvedValue({
      txid: "abc123",
      amount_rill: 10,
      fee_rill: 0.0001,
    });

    const result = await sendRill({
      mnemonic: "test words",
      to: "trill1dest",
      amount_rill: 10,
    });
    expect(result.content[0].text).toContain("abc123");
    expect(result.content[0].text).toContain("10 RILL");
  });
});

describe("get_block", () => {
  it("returns block JSON", async () => {
    vi.mocked(explorer.getBlock).mockResolvedValue({ height: 42, hash: "deadbeef" });

    const result = await getBlock({ id: "42" });
    expect(result.content[0].text).toContain("42");
    expect(result.content[0].text).toContain("deadbeef");
  });
});

describe("get_transaction", () => {
  it("returns tx JSON", async () => {
    vi.mocked(explorer.getTransaction).mockResolvedValue({ txid: "abc123", outputs: [] });

    const result = await getTransaction({ txid: "abc123" });
    expect(result.content[0].text).toContain("abc123");
  });
});

describe("network_status", () => {
  it("returns formatted stats", async () => {
    vi.mocked(explorer.getStats).mockResolvedValue({
      height: 1234,
      best_hash: "aabbcc",
      circulating_supply: 50000,
      decay_pool: 1000,
      utxo_count: 500,
      ibd: false,
      peer_count: 8,
      mempool_size: 2,
      mempool_bytes: 1024,
    });

    const result = await networkStatus();
    expect(result.content[0].text).toContain("1,234");
    expect(result.content[0].text).toContain("8");
  });
});

describe("claim_faucet", () => {
  it("returns claim result", async () => {
    vi.mocked(faucet.claimFaucet).mockResolvedValue({
      txid: "faucettx",
      amount_rill: 10,
      address: "trill1abc",
    });

    const result = await claimFaucet({ address: "trill1abc" });
    expect(result.content[0].text).toContain("10 RILL");
    expect(result.content[0].text).toContain("faucettx");
  });
});

describe("create_wallet", () => {
  it("returns mnemonic and address", async () => {
    vi.mocked(faucet.createWallet).mockResolvedValue({
      mnemonic: "word1 word2 word3",
      address: "trill1new",
    });

    const result = await createWallet();
    expect(result.content[0].text).toContain("trill1new");
    expect(result.content[0].text).toContain("word1 word2 word3");
    expect(result.content[0].text).toContain("Security");
  });
});

describe("derive_address", () => {
  it("returns derived address", async () => {
    vi.mocked(faucet.deriveAddress).mockResolvedValue({ address: "trill1restored" });

    const result = await deriveAddress({ mnemonic: "word1 word2" });
    expect(result.content[0].text).toContain("trill1restored");
  });
});

describe("search", () => {
  it("returns search result type", async () => {
    vi.mocked(explorer.search).mockResolvedValue({ type: "block", value: "42" });

    const result = await search({ query: "42" });
    expect(result.content[0].text).toContain("Block");
  });
});

describe("explain_decay", () => {
  it("explains no decay below threshold", async () => {
    const result = await explainDecay({
      balance_rill: 100,
      blocks_held: 1000,
      concentration_pct: 0.05,
    });
    expect(result.content[0].text).toContain("No decay applies");
  });

  it("calculates decay above threshold", async () => {
    const result = await explainDecay({
      balance_rill: 100000,
      blocks_held: 1000,
      concentration_pct: 0.5,
    });
    expect(result.content[0].text).toContain("Decay Analysis");
    expect(result.content[0].text).toContain("Effective value");
  });
});
