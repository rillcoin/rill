#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

import { checkBalanceSchema, checkBalance } from "./tools/check-balance.js";
import { sendRillSchema, sendRill } from "./tools/send-rill.js";
import { getBlockSchema, getBlock } from "./tools/get-block.js";
import { getTransactionSchema, getTransaction } from "./tools/get-transaction.js";
import { networkStatusSchema, networkStatus } from "./tools/network-status.js";
import { claimFaucetSchema, claimFaucet } from "./tools/claim-faucet.js";
import { createWalletSchema, createWallet } from "./tools/create-wallet.js";
import { deriveAddressSchema, deriveAddress } from "./tools/derive-address.js";
import { explainDecaySchema, explainDecay } from "./tools/explain-decay.js";
import { searchSchema, search } from "./tools/search.js";
import { getConductProfileSchema, getConductProfile } from "./tools/get-conduct-profile.js";
import { registerAgentSchema, registerAgent } from "./tools/register-agent.js";
import { vouchForAgentSchema, vouchForAgent } from "./tools/vouch-for-agent.js";
import { createContractSchema, createContract } from "./tools/create-contract.js";
import { fulfilContractSchema, fulfilContract } from "./tools/fulfil-contract.js";
import { submitReviewSchema, submitReview } from "./tools/submit-review.js";

const server = new McpServer({
  name: "rill",
  version: "0.1.0",
});

// -- Wallet tools --

server.tool(
  "create_wallet",
  "Generate a new RillCoin testnet wallet with mnemonic and address",
  createWalletSchema.shape,
  createWallet,
);

server.tool(
  "derive_address",
  "Restore a wallet by deriving the address from an existing mnemonic",
  deriveAddressSchema.shape,
  deriveAddress,
);

server.tool(
  "check_balance",
  "Check the RILL balance and UTXO count for an address",
  checkBalanceSchema.shape,
  checkBalance,
);

server.tool(
  "send_rill",
  "Send RILL from a mnemonic-derived wallet to a recipient address",
  sendRillSchema.shape,
  sendRill,
);

server.tool(
  "claim_faucet",
  "Claim free testnet RILL from the faucet",
  claimFaucetSchema.shape,
  claimFaucet,
);

// -- Explorer tools --

server.tool(
  "get_network_status",
  "Get current RillCoin network status: height, supply, decay pool, peers",
  networkStatusSchema.shape,
  networkStatus,
);

server.tool(
  "get_block",
  "Get block details by height or hash",
  getBlockSchema.shape,
  getBlock,
);

server.tool(
  "get_transaction",
  "Get transaction details by transaction ID",
  getTransactionSchema.shape,
  getTransaction,
);

server.tool(
  "search",
  "Search the blockchain — auto-detects addresses, block heights, hashes, and transaction IDs",
  searchSchema.shape,
  search,
);

// -- Education tools --

server.tool(
  "explain_decay",
  "Calculate and explain how RillCoin's concentration decay affects a given balance. Shows decay amount, effective value, and educational context about the sigmoid decay mechanism.",
  explainDecaySchema.shape,
  explainDecay,
);

// -- Agent tools --

server.tool(
  "get_conduct_profile",
  "Get the Proof of Conduct profile for a RillCoin address — shows conduct score, decay multiplier, and agent status",
  getConductProfileSchema.shape,
  getConductProfile,
);

server.tool(
  "register_agent",
  "Register a wallet as an AI agent on RillCoin — stakes 50 RILL and activates Proof of Conduct tracking",
  registerAgentSchema.shape,
  registerAgent,
);

server.tool(
  "vouch_for_agent",
  "Vouch for another agent wallet — requires conduct score ≥ 700",
  vouchForAgentSchema.shape,
  vouchForAgent,
);

server.tool(
  "create_contract",
  "Create an agent-to-agent contract with escrow value on RillCoin",
  createContractSchema.shape,
  createContract,
);

server.tool(
  "fulfil_contract",
  "Mark an agent contract as fulfilled — both parties get credit",
  fulfilContractSchema.shape,
  fulfilContract,
);

server.tool(
  "submit_review",
  "Submit a peer review (1-10 score) for a completed agent contract",
  submitReviewSchema.shape,
  submitReview,
);

// -- Start server --

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
