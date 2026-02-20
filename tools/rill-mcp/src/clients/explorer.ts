import { config } from "../config.js";

const base = () => config.explorerUrl;

async function fetchJson<T>(url: string): Promise<T> {
  const res = await fetch(url);
  const body = await res.json() as Record<string, unknown>;
  if (!res.ok) {
    const msg = (body as { error?: string }).error ?? res.statusText;
    throw new Error(`Explorer API error (${res.status}): ${msg}`);
  }
  return body as T;
}

export interface NetworkStats {
  height: number;
  best_hash: string;
  circulating_supply: number;
  decay_pool: number;
  utxo_count: number;
  ibd: boolean;
  peer_count: number;
  mempool_size: number;
  mempool_bytes: number;
}

export async function getStats(): Promise<NetworkStats> {
  return fetchJson(`${base()}/api/stats`);
}

export interface BlockSummary {
  hash: string;
  height: number;
  timestamp: number;
  tx_count: number;
  prev_hash: string;
}

export interface BlockList {
  tip: number;
  blocks: BlockSummary[];
}

export async function getRecentBlocks(limit = 20): Promise<BlockList> {
  return fetchJson(`${base()}/api/blocks?limit=${limit}`);
}

export async function getBlock(id: string): Promise<Record<string, unknown>> {
  return fetchJson(`${base()}/api/block/${encodeURIComponent(id)}`);
}

export async function getTransaction(txid: string): Promise<Record<string, unknown>> {
  return fetchJson(`${base()}/api/tx/${encodeURIComponent(txid)}`);
}

export interface SearchResult {
  type: "address" | "block" | "tx";
  value: string;
}

export async function search(query: string): Promise<SearchResult> {
  return fetchJson(`${base()}/api/search?q=${encodeURIComponent(query)}`);
}
