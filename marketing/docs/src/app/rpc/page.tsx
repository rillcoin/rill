import type { Metadata } from "next";
import Link from "next/link";
import { ArrowLeft, ArrowRight } from "lucide-react";
import CodeBlock from "@/components/CodeBlock";

export const metadata: Metadata = {
  title: "RPC Reference",
  description:
    "JSON-RPC 2.0 API reference for RillCoin nodes — all methods with request and response examples.",
};

function RpcMethod({
  method,
  params,
  returns,
  description,
  request,
  response,
}: {
  method: string;
  params: string;
  returns: string;
  description: string;
  request: string;
  response: string;
}) {
  return (
    <div
      className="mb-8 rounded-xl overflow-hidden"
      style={{ border: "1px solid var(--border-subtle)" }}
      id={method}
    >
      <div
        className="px-5 py-4 flex items-start justify-between gap-4"
        style={{
          background: "var(--raised)",
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        <div>
          <code
            className="text-base font-medium"
            style={{
              fontFamily: "var(--font-jetbrains-mono)",
              color: "var(--cyan-300)",
            }}
          >
            {method}
          </code>
          <p
            className="text-sm mt-1"
            style={{ color: "var(--text-muted)", marginBottom: 0 }}
          >
            {description}
          </p>
        </div>
        <div className="shrink-0 text-right">
          <span
            className="block text-xs"
            style={{ color: "var(--text-dim)" }}
          >
            params
          </span>
          <code
            className="text-xs"
            style={{
              fontFamily: "var(--font-jetbrains-mono)",
              color: "var(--blue-300)",
            }}
          >
            {params || "none"}
          </code>
          <span
            className="block text-xs mt-1"
            style={{ color: "var(--text-dim)" }}
          >
            returns
          </span>
          <code
            className="text-xs"
            style={{
              fontFamily: "var(--font-jetbrains-mono)",
              color: "var(--orange-400)",
            }}
          >
            {returns}
          </code>
        </div>
      </div>
      <div className="px-5 py-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <p
              className="text-xs uppercase tracking-wider font-semibold mb-2"
              style={{ color: "var(--text-dim)" }}
            >
              Request
            </p>
            <CodeBlock language="json">{request}</CodeBlock>
          </div>
          <div>
            <p
              className="text-xs uppercase tracking-wider font-semibold mb-2"
              style={{ color: "var(--text-dim)" }}
            >
              Response
            </p>
            <CodeBlock language="json">{response}</CodeBlock>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function RpcPage() {
  return (
    <div className="max-w-5xl mx-auto px-6 py-12 lg:py-16">
      <div className="mb-10">
        <div className="flex items-center gap-2 mb-4">
          <span
            className="text-xs font-semibold uppercase tracking-widest"
            style={{ color: "var(--text-dim)" }}
          >
            Reference
          </span>
        </div>
        <h1
          className="font-serif mb-3"
          style={{ fontSize: "2.5rem", lineHeight: 1.15, color: "var(--text-primary)" }}
        >
          RPC Reference
        </h1>
        <p className="text-base" style={{ color: "var(--text-muted)" }}>
          JSON-RPC 2.0 API for interacting with a RillCoin node programmatically.
        </p>
      </div>

      <div className="doc-prose">
        {/* Connection */}
        <h2>Connection</h2>
        <div
          className="rounded-lg overflow-hidden mb-6"
          style={{ border: "1px solid var(--border-dim)" }}
        >
          <table style={{ marginBottom: 0 }}>
            <thead>
              <tr>
                <th>Network</th>
                <th>Default Endpoint</th>
              </tr>
            </thead>
            <tbody>
              {[
                ["Mainnet", "http://127.0.0.1:18332"],
                ["Testnet", "http://127.0.0.1:28332"],
                ["Regtest", "http://127.0.0.1:38332"],
              ].map(([net, url]) => (
                <tr key={net}>
                  <td>{net}</td>
                  <td>
                    <code>{url}</code>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <h3>Request Format</h3>
        <p>
          All requests are HTTP POST to the node&apos;s RPC port. The
          Content-Type must be <code>application/json</code>. Requests follow
          the JSON-RPC 2.0 specification.
        </p>
        <CodeBlock language="bash" title="Example curl request">
          {`curl -X POST http://127.0.0.1:28332 \\
  -H "Content-Type: application/json" \\
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'`}
        </CodeBlock>

        <h3>Error Format</h3>
        <CodeBlock language="json">
          {`{
  "jsonrpc": "2.0",
  "error": {
    "code":    -1,
    "message": "block not found"
  },
  "id": 1
}`}
        </CodeBlock>

        <h2>Methods</h2>
      </div>

      <RpcMethod
        method="getblockcount"
        params="[]"
        returns="u64"
        description="Returns the current chain tip block height."
        request={`{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}`}
        response={`{"jsonrpc":"2.0","result":42381,"id":1}`}
      />

      <RpcMethod
        method="getblockhash"
        params="[height: u64]"
        returns="String (hex)"
        description="Returns the block hash at the given height."
        request={`{"jsonrpc":"2.0","method":"getblockhash","params":[42381],"id":1}`}
        response={`{
  "jsonrpc": "2.0",
  "result":  "a3f8c9d2e1b4f7a6...64b2",
  "id": 1
}`}
      />

      <RpcMethod
        method="getblock"
        params="[hash: String]"
        returns="BlockJson"
        description="Returns full block data including header fields and transaction IDs."
        request={`{
  "jsonrpc": "2.0",
  "method":  "getblock",
  "params":  ["a3f8c9d2e1b4f7a6...64b2"],
  "id": 1
}`}
        response={`{
  "jsonrpc": "2.0",
  "result": {
    "hash":              "a3f8c9d2...64b2",
    "height":            42381,
    "version":           1,
    "prev_hash":         "9b2c7e1d...3a4f",
    "merkle_root":       "d1e2f3a4...89ab",
    "timestamp":         1740000000,
    "difficulty_target": 486604799,
    "nonce":             1929374837,
    "tx_count":          47,
    "tx": ["7f3a9c2b...", "4e5d8c1a..."]
  },
  "id": 1
}`}
      />

      <RpcMethod
        method="getblockheader"
        params="[hash: String]"
        returns="HeaderJson"
        description="Returns block header fields only, without transaction data."
        request={`{
  "jsonrpc": "2.0",
  "method":  "getblockheader",
  "params":  ["a3f8c9d2...64b2"],
  "id": 1
}`}
        response={`{
  "jsonrpc": "2.0",
  "result": {
    "hash":              "a3f8c9d2...64b2",
    "version":           1,
    "prev_hash":         "9b2c7e1d...3a4f",
    "merkle_root":       "d1e2f3a4...89ab",
    "timestamp":         1740000000,
    "difficulty_target": 486604799,
    "nonce":             1929374837
  },
  "id": 1
}`}
      />

      <RpcMethod
        method="gettransaction"
        params="[txid: String]"
        returns="TransactionJson"
        description="Returns transaction metadata by TXID (BLAKE3 hash, hex-encoded)."
        request={`{
  "jsonrpc": "2.0",
  "method":  "gettransaction",
  "params":  ["7f3a9c2b1d4e5f6a..."],
  "id": 1
}`}
        response={`{
  "jsonrpc": "2.0",
  "result": {
    "txid":      "7f3a9c2b1d4e5f6a...",
    "version":   1,
    "vin_count": 2,
    "vout_count": 2,
    "lock_time": 0
  },
  "id": 1
}`}
      />

      <RpcMethod
        method="sendrawtransaction"
        params="[hex_data: String]"
        returns="String (txid)"
        description="Submits a hex-encoded bincode-serialized transaction to the mempool and broadcasts it to peers. Returns the transaction ID on success."
        request={`{
  "jsonrpc": "2.0",
  "method":  "sendrawtransaction",
  "params":  ["01000000..."],
  "id": 1
}`}
        response={`{
  "jsonrpc": "2.0",
  "result":  "7f3a9c2b1d4e5f6a...",
  "id": 1
}`}
      />

      <RpcMethod
        method="getmempoolinfo"
        params="[]"
        returns="MempoolInfo"
        description="Returns mempool statistics including transaction count, total size in bytes, and total fees."
        request={`{"jsonrpc":"2.0","method":"getmempoolinfo","params":[],"id":1}`}
        response={`{
  "jsonrpc": "2.0",
  "result": {
    "size":      43,
    "bytes":     18724,
    "total_fee": 43000
  },
  "id": 1
}`}
      />

      <RpcMethod
        method="getpeerinfo"
        params="[]"
        returns="PeerInfo"
        description="Returns the number of currently connected P2P peers."
        request={`{"jsonrpc":"2.0","method":"getpeerinfo","params":[],"id":1}`}
        response={`{
  "jsonrpc": "2.0",
  "result": {
    "connected": 8
  },
  "id": 1
}`}
      />

      <RpcMethod
        method="getinfo"
        params="[]"
        returns="NodeInfo"
        description="Returns a summary of node state: block height, best hash, peer count, circulating supply, and decay pool balance."
        request={`{"jsonrpc":"2.0","method":"getinfo","params":[],"id":1}`}
        response={`{
  "jsonrpc": "2.0",
  "result": {
    "blocks":             42381,
    "bestblockhash":      "a3f8c9d2...64b2",
    "connections":        8,
    "circulating_supply": 8432750.0,
    "decay_pool":         12847.32
  },
  "id": 1
}`}
      />

      <RpcMethod
        method="getblocktemplate"
        params="[mining_address: String]"
        returns="BlockTemplate"
        description="Returns a block template for mining. Includes all pending transactions sorted by fee rate, with a pre-computed Merkle root including the coinbase transaction to mining_address."
        request={`{
  "jsonrpc": "2.0",
  "method":  "getblocktemplate",
  "params":  ["trill1qminingaddress..."],
  "id": 1
}`}
        response={`{
  "jsonrpc": "2.0",
  "result": {
    "version":           1,
    "prev_hash":         "a3f8c9d2...64b2",
    "merkle_root":       "d1e2f3a4...89ab",
    "timestamp":         1740000120,
    "difficulty_target": 486604799,
    "nonce":             0,
    "height":            42382,
    "transactions": [
      {
        "txid": "7f3a9c2b...",
        "data": "01000000...",
        "fee":  1000
      }
    ]
  },
  "id": 1
}`}
      />

      <RpcMethod
        method="submitblock"
        params="[hex_data: String]"
        returns="String"
        description='Submits a hex-encoded bincode-serialized block. Returns "ok" on success. Returns an error if the block is invalid.'
        request={`{
  "jsonrpc": "2.0",
  "method":  "submitblock",
  "params":  ["01000000a3f8..."],
  "id": 1
}`}
        response={`{"jsonrpc":"2.0","result":"ok","id":1}`}
      />

      <RpcMethod
        method="getutxosbyaddress"
        params="[address: String]"
        returns="UTXO[]"
        description="Returns all unspent transaction outputs for a given address. Values are in rills (divide by 100,000,000 for RILL). Includes cluster_id for decay tracking."
        request={`{
  "jsonrpc": "2.0",
  "method":  "getutxosbyaddress",
  "params":  ["trill1qw5r3k8d9..."],
  "id": 1
}`}
        response={`{
  "jsonrpc": "2.0",
  "result": [
    {
      "txid":         "7f3a9c2b...",
      "index":        0,
      "value":        10000000000,
      "block_height": 42100,
      "is_coinbase":  false,
      "cluster_id":   "a3f8c9d2...",
      "pubkey_hash":  "5r3k8d9v..."
    }
  ],
  "id": 1
}`}
      />

      <RpcMethod
        method="getclusterbalance"
        params="[cluster_id: String]"
        returns="u64 (rills)"
        description="Returns the total balance of all UTXOs tagged with the given cluster_id hex string. Divide by 100,000,000 to get RILL."
        request={`{
  "jsonrpc": "2.0",
  "method":  "getclusterbalance",
  "params":  ["a3f8c9d2..."],
  "id": 1
}`}
        response={`{
  "jsonrpc": "2.0",
  "result":  10000000000000,
  "id": 1
}`}
      />

      <RpcMethod
        method="getblockchaininfo"
        params="[]"
        returns="BlockchainInfo"
        description="Returns comprehensive blockchain state including height, circulating supply, decay pool balance, IBD status, UTXO count, mempool size, and peer count."
        request={`{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}`}
        response={`{
  "jsonrpc": "2.0",
  "result": {
    "height":               42381,
    "best_block_hash":      "a3f8c9d2...64b2",
    "circulating_supply":   843275000000000,
    "decay_pool_balance":   1284732000000,
    "initial_block_download": false,
    "utxo_count":           1284920,
    "mempool_size":         43,
    "peer_count":           8
  },
  "id": 1
}`}
      />

      <RpcMethod
        method="getsyncstatus"
        params="[]"
        returns="SyncStatus"
        description="Returns the node's current sync state — whether it is actively syncing or fully caught up."
        request={`{"jsonrpc":"2.0","method":"getsyncstatus","params":[],"id":1}`}
        response={`{
  "jsonrpc": "2.0",
  "result": {
    "syncing":        false,
    "current_height": 42381,
    "peer_count":     8,
    "best_block_hash": "a3f8c9d2...64b2"
  },
  "id": 1
}`}
      />

      {/* Navigation */}
      <div
        className="flex items-center justify-between mt-12 pt-6"
        style={{ borderTop: "1px solid var(--border-subtle)" }}
      >
        <Link
          href="/cli"
          className="flex items-center gap-2 text-sm transition-colors"
          style={{ color: "var(--text-muted)" }}
        >
          <ArrowLeft size={14} />
          Wallet & CLI
        </Link>
        <Link
          href="/node"
          className="flex items-center gap-2 text-sm font-medium transition-colors"
          style={{ color: "var(--cyan-400)" }}
        >
          Node Setup
          <ArrowRight size={14} />
        </Link>
      </div>
    </div>
  );
}
