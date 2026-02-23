import { config } from "../config.js";

const base = () => config.faucetUrl;

async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, init);
  const body = await res.json() as Record<string, unknown>;
  if (!res.ok) {
    const msg = (body as { error?: string }).error ?? res.statusText;
    throw new Error(`Faucet API error (${res.status}): ${msg}`);
  }
  return body as T;
}

export interface WalletNew {
  mnemonic: string;
  address: string;
}

export async function createWallet(): Promise<WalletNew> {
  return fetchJson(`${base()}/api/wallet/new`);
}

export interface WalletBalance {
  address: string;
  balance_rill: number;
  balance_rills: number;
  utxo_count: number;
}

export async function getBalance(address: string): Promise<WalletBalance> {
  return fetchJson(`${base()}/api/wallet/balance?address=${encodeURIComponent(address)}`);
}

export interface SendResult {
  txid: string;
  amount_rill: number;
  fee_rill: number;
}

export async function sendRill(
  mnemonic: string,
  to: string,
  amountRill: number,
): Promise<SendResult> {
  return fetchJson(`${base()}/api/wallet/send`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ mnemonic, to, amount_rill: amountRill }),
  });
}

export interface FaucetResult {
  txid: string;
  amount_rill: number;
  address: string;
}

export async function claimFaucet(address: string): Promise<FaucetResult> {
  return fetchJson(`${base()}/api/faucet`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ address }),
  });
}

export interface DeriveResult {
  address: string;
}

export async function deriveAddress(mnemonic: string): Promise<DeriveResult> {
  return fetchJson(`${base()}/api/wallet/derive`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ mnemonic }),
  });
}

export interface FaucetStatus {
  balance_rill: number;
  height: number;
  network: string;
  amount_per_claim_rill: number;
  cooldown_secs: number;
}

export async function getStatus(): Promise<FaucetStatus> {
  return fetchJson(`${base()}/api/status`);
}

// -- Agent API --

export interface AgentRegisterResult {
  txid: string;
}

export async function registerAgent(mnemonic: string): Promise<AgentRegisterResult> {
  return fetchJson(`${base()}/api/agent/register`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ mnemonic }),
  });
}

export interface ConductProfile {
  address: string;
  wallet_type: string;
  conduct_score: number;
  conduct_multiplier_bps: number;
  effective_decay_rate_ppb: number;
  undertow_active: boolean;
  registered_at_block: number;
  wallet_age_blocks: number;
}

export async function getAgentProfile(address: string): Promise<ConductProfile> {
  return fetchJson(`${base()}/api/agent/profile?address=${encodeURIComponent(address)}`);
}

export interface AgentDirectory {
  agents: Array<{
    address: string;
    conduct_score: number;
    conduct_multiplier_bps: number;
    undertow_active: boolean;
    registered_at_block: number;
  }>;
  total: number;
  offset: number;
  limit: number;
}

export async function getAgentDirectory(offset = 0, limit = 20): Promise<AgentDirectory> {
  return fetchJson(`${base()}/api/agent/directory?offset=${offset}&limit=${limit}`);
}

export interface TxResult {
  txid: string;
}

export async function vouchForAgent(mnemonic: string, targetAddress: string): Promise<TxResult> {
  return fetchJson(`${base()}/api/agent/vouch`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ mnemonic, target_address: targetAddress }),
  });
}

export async function createContract(
  mnemonic: string,
  counterparty: string,
  valueRill: number,
): Promise<TxResult> {
  return fetchJson(`${base()}/api/agent/contract/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ mnemonic, counterparty, value_rill: valueRill }),
  });
}

export async function fulfilContract(mnemonic: string, contractId: string): Promise<TxResult> {
  return fetchJson(`${base()}/api/agent/contract/fulfil`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ mnemonic, contract_id: contractId }),
  });
}

export async function submitReview(
  mnemonic: string,
  subjectAddress: string,
  score: number,
  contractId: string,
): Promise<TxResult> {
  return fetchJson(`${base()}/api/agent/review`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ mnemonic, subject_address: subjectAddress, score, contract_id: contractId }),
  });
}
