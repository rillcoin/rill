# Rill Testnet Guide

This guide covers how to build, run, and interact with a Rill node on the testnet or a local regtest network.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Building from Source](#building-from-source)
- [Network Types](#network-types)
- [Running a Single Node](#running-a-single-node)
- [Connecting Multiple Nodes](#connecting-multiple-nodes)
- [Mining Blocks](#mining-blocks)
- [Using the CLI](#using-the-cli)
- [Running with Docker](#running-with-docker)
- [RPC Reference](#rpc-reference)

---

## Prerequisites

- Rust 1.85 or later (2024 edition)
- Cargo (included with Rust)
- A C compiler (for RocksDB)

Install Rust via rustup:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Building from Source

```bash
git clone https://github.com/rillcoin/rill.git
cd rill

# Build all binaries in release mode
cargo build --release

# Binaries are placed at:
#   ./target/release/rill-node
#   ./target/release/rill-cli
#   ./target/release/rill-miner
```

To verify the build passes all tests:

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

## Network Types

Three network modes are available:

| Mode     | Flag        | P2P Port | RPC Port | Description                                   |
|----------|-------------|----------|----------|-----------------------------------------------|
| mainnet  | (default)   | 18333    | 18332    | Production network                            |
| testnet  | `--testnet` | 28333    | 28332    | Public test network, lower difficulty         |
| regtest  | `--regtest` | 38333    | 38332    | Local regression test, instant blocks         |

Network data is stored in separate subdirectories:
- Mainnet: `~/.local/share/rill/mainnet/` (Linux) or `~/Library/Application Support/rill/mainnet/` (macOS)
- Testnet: same prefix with `testnet/`
- Regtest: same prefix with `regtest/`

You can override the data directory with `--data-dir`.

---

## Running a Single Node

Start a testnet node with default settings:

```bash
./target/release/rill-node --testnet --data-dir /tmp/rill-testnet
```

Start a regtest node (no real PoW, useful for local development):

```bash
./target/release/rill-node --regtest --data-dir /tmp/rill-regtest
```

Disable P2P networking for a fully isolated single-node setup:

```bash
./target/release/rill-node --regtest --data-dir /tmp/rill-regtest --no-network
```

Full set of available flags:

```
USAGE:
    rill-node [OPTIONS]

OPTIONS:
    --data-dir <PATH>              Data directory for blockchain storage
    --rpc-bind <ADDR>              RPC server bind address [default: 127.0.0.1]
    --rpc-port <PORT>              RPC server port [default: 18332]
    --p2p-listen-addr <ADDR>       P2P listen address [default: 0.0.0.0]
    --p2p-listen-port <PORT>       P2P listen port [default: 18333]
    --bootstrap-peers <PEERS>      Bootstrap peers, comma-separated
    --log-level <LEVEL>            Log level: trace, debug, info, warn, error [default: info]
    --log-format <FORMAT>          Log format: text or json [default: text]
    --no-network                   Disable P2P networking (single-node mode)
    --testnet                      Connect to the testnet
    --regtest                      Run in regtest mode (conflicts with --testnet)
```

The node logs the RPC address on startup. By default the RPC server binds to `127.0.0.1:18332` on mainnet.

---

## Connecting Multiple Nodes

To connect multiple nodes manually, pass bootstrap peer addresses using `--bootstrap-peers`.

Start the first node (seed node):

```bash
./target/release/rill-node \
    --regtest \
    --data-dir /tmp/rill-node1 \
    --p2p-listen-addr 0.0.0.0 \
    --p2p-listen-port 38333 \
    --rpc-port 38332
```

Start a second node that peers with the first:

```bash
./target/release/rill-node \
    --regtest \
    --data-dir /tmp/rill-node2 \
    --p2p-listen-addr 0.0.0.0 \
    --p2p-listen-port 38334 \
    --rpc-port 38335 \
    --bootstrap-peers 127.0.0.1:38333
```

Start a third node:

```bash
./target/release/rill-node \
    --regtest \
    --data-dir /tmp/rill-node3 \
    --p2p-listen-addr 0.0.0.0 \
    --p2p-listen-port 38336 \
    --rpc-port 38337 \
    --bootstrap-peers 127.0.0.1:38333
```

On a local network, mDNS discovery is enabled by default, so nodes will find each other automatically without bootstrap peers.

---

## Mining Blocks

`rill-miner` connects to a running node via RPC, fetches block templates, mines them using SHA-256 proof-of-work, and submits found blocks.

A mining address is required to receive coinbase rewards.

First create a wallet and get an address (see the CLI section below), then start the miner:

```bash
./target/release/rill-miner \
    --rpc-endpoint http://127.0.0.1:38332 \
    --mining-address trill1<your-address>
```

Full set of miner flags:

```
USAGE:
    rill-miner [OPTIONS] --mining-address <ADDR>

OPTIONS:
    --rpc-endpoint <URL>       RPC server URL [default: http://127.0.0.1:18332]
    --mining-address <ADDR>    Address to receive block rewards (required)
    --threads <N>              Number of mining threads [default: 1]
    --log-level <LEVEL>        Log level [default: info]
```

The miner logs hashrate and blocks found every 30 seconds. Press Ctrl+C to stop.

---

## Using the CLI

`rill-cli` is the command-line wallet and node query tool. It communicates with a running `rill-node` via JSON-RPC.

### Wallet Management

Create a new HD wallet (generates a 24-word BIP-39 seed phrase):

```bash
./target/release/rill-cli wallet create --network testnet
```

Restore a wallet from an existing seed phrase:

```bash
./target/release/rill-cli wallet restore --network testnet
```

The wallet file is saved to `~/.rill/wallet.dat` by default. Use `--file <PATH>` to specify a different location.

### Get Your Address

```bash
./target/release/rill-cli address
```

Mainnet addresses start with `rill1`. Testnet addresses start with `trill1`.

### Check Balance

```bash
./target/release/rill-cli balance --rpc-endpoint http://127.0.0.1:28332
```

The balance command shows:
- Nominal balance (raw UTXO sum)
- Effective balance (after decay adjustment)
- Per-cluster breakdown if holdings span multiple clusters

### Send a Transaction

```bash
./target/release/rill-cli send \
    --to trill1<recipient-address> \
    --amount 10.5 \
    --fee 1000 \
    --rpc-endpoint http://127.0.0.1:28332
```

Amount is in RILL (supports decimals). Fee is in rills (1 RILL = 100,000,000 rills). The default fee is 1000 rills.

Coin selection automatically spends highest-decay UTXOs first to minimize decay loss.

### Node Queries

Show blockchain state:

```bash
./target/release/rill-cli getblockchaininfo --rpc-endpoint http://127.0.0.1:28332
```

Show sync status:

```bash
./target/release/rill-cli getsyncstatus --rpc-endpoint http://127.0.0.1:28332
```

Show connected peers:

```bash
./target/release/rill-cli getpeerinfo --rpc-endpoint http://127.0.0.1:28332
```

Validate an address (no node required):

```bash
./target/release/rill-cli validateaddress rill1<address>
```

### Output Formats

All node query commands support `--format json` for machine-readable output:

```bash
./target/release/rill-cli getblockchaininfo --format json
```

---

## Running with Docker

A `docker-compose.yml` is provided that starts a 3-node testnet cluster, which is the minimum required for consensus testing.

Build the image and start all three nodes:

```bash
docker-compose up --build
```

This starts:
- `rill-node1`: seed node, RPC on `localhost:18332`, P2P on `localhost:18333`
- `rill-node2`: peers with node1, RPC on `localhost:18334`, P2P on `localhost:18335`
- `rill-node3`: peers with node1, RPC on `localhost:18336`, P2P on `localhost:18337`

All nodes use JSON log format. Blockchain data persists in Docker named volumes (`node1-data`, `node2-data`, `node3-data`).

Query a node from the host:

```bash
curl -s -X POST http://localhost:18332 \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getblockcount","params":[]}'
```

Stop and remove containers (data volumes are preserved):

```bash
docker-compose down
```

Stop and remove containers and all data:

```bash
docker-compose down -v
```

The Docker image uses a multi-stage build:
- Build stage: `rust:1.85-bookworm`
- Runtime stage: `debian:bookworm-slim`

The runtime image runs as a non-root `rillnode` system user. Data is stored in the `/data` volume mount.

---

## RPC Reference

The JSON-RPC server listens on `http://127.0.0.1:<rpc-port>` by default. All requests use HTTP POST with `Content-Type: application/json`.

Example request format:

```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getblockcount",
    "params": []
}
```

### Available Methods

| Method               | Params                     | Returns                     | Description                                              |
|----------------------|----------------------------|-----------------------------|----------------------------------------------------------|
| `getblockcount`      | none                       | `u64`                       | Current chain tip height                                 |
| `getblockhash`       | `height: u64`              | `string` (hex)              | Block hash at given height                               |
| `getblock`           | `hash: string`             | block object                | Full block with header and transaction IDs               |
| `getblockheader`     | `hash: string`             | header object               | Block header fields                                      |
| `gettransaction`     | `txid: string`             | transaction object          | Transaction by ID (mempool or confirmed)                 |
| `sendrawtransaction` | `hex_data: string`         | `string` (txid)             | Submit a hex-encoded bincode-serialized transaction      |
| `getmempoolinfo`     | none                       | mempool info object         | Mempool size, bytes, and total fees                      |
| `getblockchaininfo`  | none                       | blockchain info object      | Height, best hash, supply, decay pool, IBD, UTXO count   |
| `getsyncstatus`      | none                       | sync status object          | Sync state, current height, peer count, best hash        |
| `getpeerinfo`        | none                       | peer info object            | Number of connected peers                                |
| `getinfo`            | none                       | node info object            | Height, best hash, peers, circulating supply, decay pool |
| `getblocktemplate`   | `mining_address: string`   | block template object       | Template for mining a new block                          |
| `submitblock`        | `hex_data: string`         | `string` (hash)             | Submit a mined block (hex-encoded bincode)               |
| `getutxosbyaddress`  | `address: string`          | array of UTXO objects       | UTXOs owned by a given address                           |
| `getclusterbalance`  | `cluster_id: string`       | `u64`                       | Total balance of a decay cluster                         |

### Response Objects

**Blockchain info** (`getblockchaininfo`):
```json
{
    "height": 1000,
    "best_block_hash": "abc123...",
    "circulating_supply": 50000000000000,
    "decay_pool_balance": 123456789,
    "initial_block_download": false,
    "utxo_count": 4200,
    "mempool_size": 3,
    "peer_count": 2
}
```

**Sync status** (`getsyncstatus`):
```json
{
    "syncing": false,
    "current_height": 1000,
    "peer_count": 2,
    "best_block_hash": "abc123..."
}
```

**Peer info** (`getpeerinfo`):
```json
{
    "connected": 2
}
```

**Mempool info** (`getmempoolinfo`):
```json
{
    "size": 3,
    "bytes": 1024,
    "total_fee": 3000
}
```

Monetary values are in rills (1 RILL = 100,000,000 rills) unless otherwise noted.
