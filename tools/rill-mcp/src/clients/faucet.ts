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
