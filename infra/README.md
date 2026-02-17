# RillCoin Azure Testnet

Provisions a 4-node RillCoin testnet on Azure using the Azure CLI.

## Architecture

```
Internet
    |
    | SSH (22) / RPC tunnel (18332)
    v
+----------+   P2P (18333)   +----------+   +----------+   +----------+
|  node0   | <-------------> |  node1   |   |  node2   |   |  node3   |
|  seed    |                 |  miner   |   |  miner   |   |  wallet  |
| (public) |                 | (10.0.1.11)  | (10.0.1.12)  | (10.0.1.13)
| 10.0.1.10|                 +----------+   +----------+   +----------+
+----------+
       ^                              All on VNet 10.0.0.0/16
       |                              NSG blocks all non-SSH/RPC/P2P inbound
```

node0 is the only VM with a public IP. SSH into nodes 1-3 via ProxyJump through node0.

## Prerequisites

### 1. Install the Azure CLI

**macOS (Homebrew)**
```sh
brew install azure-cli
```

**Linux (apt)**
```sh
curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash
```

**Windows**
Download the MSI from https://aka.ms/installazurecliwindows

Verify: `az version` (requires >= 2.60)

### 2. Log in to Azure

```sh
az login
```

Select the subscription you want to use if you have multiple:

```sh
az account list --output table
az account set --subscription "<subscription-id-or-name>"
```

### 3. Install Docker

Docker is required locally only to trigger `az acr build`, which streams the
build context to Azure Container Registry. The actual compilation runs on ACR's
build agents, not on your machine.

Install Docker Desktop (macOS/Windows) or Docker Engine (Linux):
https://docs.docker.com/engine/install/

### 4. SSH Key

The script uses `~/.ssh/id_rsa.pub` by default. Generate one if needed:

```sh
ssh-keygen -t rsa -b 4096 -C "rill-testnet"
```

To use a different path, edit `SSH_PUBLIC_KEY_PATH` at the top of
`infra/azure-testnet.sh`.

## Quick Start

```sh
# Clone the repo (if not already).
git clone <repo-url>
cd rill

# Deploy everything: resource group, VNet, NSG, ACR, 4 VMs.
./infra/azure-testnet.sh deploy
```

The first deploy takes approximately 8-12 minutes:
- Infrastructure (VNet, NSG): ~1 min
- ACR creation + image build (Rust workspace): ~6-8 min
- VM provisioning: ~2-3 min
- cloud-init (Docker install + image pull on each VM): ~3-5 min after VMs boot

## Subcommands

| Command | Description |
|---|---|
| `deploy` | Full deploy: infrastructure, ACR, VMs |
| `status` | Show VM states, IPs, and access commands |
| `ssh [N]` | SSH into node N (0-3). Default: node 0 (seed) |
| `tunnel` | Print SSH tunnel commands for RPC access |
| `stop` | Deallocate all VMs to save compute cost |
| `start` | Start all deallocated VMs |
| `teardown` | Delete everything (prompts for confirmation) |
| `cost` | Print estimated monthly cost breakdown |

## Accessing the Testnet

### SSH

```sh
# Into node0 (seed, direct)
./infra/azure-testnet.sh ssh 0

# Into node3 (wallet, via ProxyJump through node0)
./infra/azure-testnet.sh ssh 3
```

### RPC via SSH Tunnel

The RPC port (18332) is not exposed publicly. Tunnel it through node0:

```sh
# Open a tunnel (runs in the foreground, keep this terminal open)
./infra/azure-testnet.sh tunnel
# ...then follow the printed ssh commands

# Or tunnel all 4 nodes at once in the background
ssh -f -N \
    -L 18332:10.0.1.10:18332 \
    -L 18342:10.0.1.11:18332 \
    -L 18352:10.0.1.12:18332 \
    -L 18362:10.0.1.13:18332 \
    rill@<node0-public-ip>

# Query node0 RPC
rill-cli --rpc-host 127.0.0.1 --rpc-port 18332 getblockcount

# Query node3 wallet RPC
rill-cli --rpc-host 127.0.0.1 --rpc-port 18362 getbalance
```

### Checking Node Logs

SSH into a node and inspect the systemd journal:

```sh
./infra/azure-testnet.sh ssh 0
journalctl -u rill-node -f          # follow live logs
journalctl -u rill-node --since -1h # last hour
```

## Cost Management

Estimated monthly costs (eastus, pay-as-you-go):

| State | Cost |
|---|---|
| All VMs running 24/7 | ~$145/mo |
| All VMs stopped (deallocated) | ~$23/mo |

**Stop VMs when not testing:**
```sh
./infra/azure-testnet.sh stop    # pause compute billing
./infra/azure-testnet.sh start   # resume when needed
```

**Delete everything when done:**
```sh
./infra/azure-testnet.sh teardown
```

## Configuration

All tuneable variables are at the top of `infra/azure-testnet.sh`:

| Variable | Default | Description |
|---|---|---|
| `RESOURCE_GROUP` | `rill-testnet` | Azure resource group name |
| `LOCATION` | `eastus` | Azure region |
| `VM_SIZE` | `Standard_B2s` | VM SKU (2 vCPU, 4 GB RAM) |
| `NODE_COUNT` | `4` | Number of nodes (1 seed + N-1 peers) |
| `VM_IMAGE` | `Canonical:ubuntu-24_04-lts:server:latest` | OS image |
| `ADMIN_USER` | `rill` | SSH admin username on VMs |
| `SSH_PUBLIC_KEY_PATH` | `~/.ssh/id_rsa.pub` | Local public key path |
| `ACR_NAME` | `rillcr` | Azure Container Registry name (globally unique) |

## Security Notes

- SSH (22) is restricted to your public IP detected at deploy time.
- RPC (18332) is restricted to your public IP detected at deploy time.
- P2P (18333) is open within the VNet only; not reachable from the internet.
- The ACR admin password is embedded in cloud-init user-data. For production,
  assign a managed identity to each VM with the `AcrPull` role instead.
- Never commit secrets or ACR credentials to the repository.

## Troubleshooting

**cloud-init not finished yet**
Allow 5 minutes after `deploy` completes before expecting nodes to be healthy.
Check progress:
```sh
./infra/azure-testnet.sh ssh 0
sudo cloud-init status --wait
sudo cloud-init status --long
```

**rill-node not starting**
```sh
./infra/azure-testnet.sh ssh 0
systemctl status rill-node
journalctl -u rill-node -n 100
```

**Can't SSH**
Verify your IP hasn't changed since deploy (common with DHCP ISPs). Update the
NSG rule manually:
```sh
MY_IP=$(curl -sf https://api.ipify.org)
az network nsg rule update \
    --resource-group rill-testnet \
    --nsg-name rill-nsg \
    --name AllowSSH \
    --source-address-prefixes "${MY_IP}"
```

**ACR name already taken**
ACR names are globally unique across all Azure customers. Change `ACR_NAME` in
`infra/azure-testnet.sh` to something unique (e.g., `rillcr$(openssl rand -hex 4)`).
