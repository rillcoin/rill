#!/usr/bin/env bash
# =============================================================================
# azure-testnet.sh — RillCoin Azure Testnet Provisioner
# =============================================================================
#
# Provisions a 4-node RillCoin testnet on Azure:
#   node0  10.0.1.10  seed node        (public IP, SSH entry point)
#   node1  10.0.1.11  miner
#   node2  10.0.1.12  miner
#   node3  10.0.1.13  wallet / RPC gateway
#
# Usage:
#   ./infra/azure-testnet.sh deploy      # Full deploy (infra + ACR + VMs)
#   ./infra/azure-testnet.sh status      # Show node status and IPs
#   ./infra/azure-testnet.sh ssh [N]     # SSH into node N (default 0)
#   ./infra/azure-testnet.sh tunnel      # Print SSH tunnel commands
#   ./infra/azure-testnet.sh stop        # Deallocate all VMs (save money)
#   ./infra/azure-testnet.sh start       # Start all VMs
#   ./infra/azure-testnet.sh teardown    # Delete everything (destructive)
#   ./infra/azure-testnet.sh cost        # Print estimated monthly cost
#
# Prerequisites:
#   - Azure CLI >= 2.60 installed and logged in (az login)
#   - Docker installed locally (for ACR image build)
#   - An SSH key at SSH_PUBLIC_KEY_PATH
#
# =============================================================================
set -euo pipefail

# =============================================================================
# CONFIG — edit these before running
# =============================================================================

RESOURCE_GROUP="rill-testnet"
LOCATION="eastus"

# Standard_B2s: 2 vCPU, 4 GB RAM — sufficient for RocksDB under testnet load.
# ~$30.37/mo per VM when running 24/7 (eastus, pay-as-you-go).
VM_SIZE="Standard_B2s"

# 1 seed + 2 miners + 1 rpc/wallet
NODE_COUNT=4

# Ubuntu 24.04 LTS — Docker CE supported, long-term maintained.
VM_IMAGE="Canonical:ubuntu-24_04-lts:server:latest"

# The non-root admin user created on each VM.
ADMIN_USER="rill"

# Path to your local SSH public key.
SSH_PUBLIC_KEY_PATH="${HOME}/.ssh/id_ed25519.pub"

# Azure Container Registry name — must be globally unique, alphanumeric only.
# Basic tier: ~$5/mo. Includes 10 GB storage and geo-replication disabled.
ACR_NAME="rillcr"

# Virtual network CIDR blocks.
VNET_CIDR="10.0.0.0/16"
SUBNET_CIDR="10.0.1.0/24"

# Static private IPs assigned to each node.
# node0 = seed, node1-2 = miners, node3 = wallet/RPC.
declare -a NODE_IPS=("10.0.1.10" "10.0.1.11" "10.0.1.12" "10.0.1.13")
declare -a NODE_ROLES=("seed" "miner" "miner" "wallet")

# Docker image tag pushed to ACR.
IMAGE_TAG="latest"
IMAGE_NAME="${ACR_NAME}.azurecr.io/rill-node:${IMAGE_TAG}"

# RillCoin ports.
P2P_PORT=18333
RPC_PORT=18332

# =============================================================================
# ANSI COLORS
# =============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

info()    { printf "${GREEN}[INFO]${RESET}  %s\n" "$*"; }
warn()    { printf "${YELLOW}[WARN]${RESET}  %s\n" "$*"; }
error()   { printf "${RED}[ERROR]${RESET} %s\n" "$*" >&2; }
section() { printf "\n${BOLD}${CYAN}=== %s ===${RESET}\n" "$*"; }

# =============================================================================
# HELPER: resolve the absolute path to the repo root
# =============================================================================

# The script lives at <repo>/infra/azure-testnet.sh.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# =============================================================================
# PREFLIGHT CHECKS
# =============================================================================

preflight() {
    section "Preflight checks"

    # Verify Azure CLI is installed.
    if ! command -v az &>/dev/null; then
        error "Azure CLI not found. Install it: https://docs.microsoft.com/cli/azure/install-azure-cli"
        exit 1
    fi

    local az_ver
    az_ver="$(az version --query '"azure-cli"' -o tsv 2>/dev/null || echo "unknown")"
    info "Azure CLI version: ${az_ver}"

    # Verify the user is logged in.
    if ! az account show &>/dev/null; then
        error "Not logged in to Azure. Run: az login"
        exit 1
    fi

    local account_name
    account_name="$(az account show --query 'name' -o tsv)"
    info "Active subscription: ${account_name}"

    # Docker is NOT required locally — az acr build compiles remotely in Azure.
    if command -v docker &>/dev/null; then
        info "Docker found locally (not required — ACR builds remotely)"
    fi

    # Register required Azure resource providers (no-op if already registered).
    info "Registering Azure resource providers (may take a minute on new accounts)..."
    for ns in Microsoft.Network Microsoft.Compute Microsoft.ContainerRegistry; do
        local state
        state="$(az provider show -n "${ns}" --query "registrationState" -o tsv 2>/dev/null || echo "NotRegistered")"
        if [[ "${state}" != "Registered" ]]; then
            az provider register --namespace "${ns}" --wait &>/dev/null
            info "  Registered ${ns}"
        fi
    done
    info "All resource providers ready."

    # Verify the SSH public key exists.
    if [[ ! -f "${SSH_PUBLIC_KEY_PATH}" ]]; then
        error "SSH public key not found at: ${SSH_PUBLIC_KEY_PATH}"
        error "Generate one with: ssh-keygen -t rsa -b 4096"
        exit 1
    fi

    # Detect the caller's public IP for NSG rules (SSH + RPC allow-list).
    MY_IP="$(curl -sf https://api.ipify.org || curl -sf https://ifconfig.me || echo "")"
    if [[ -z "${MY_IP}" ]]; then
        warn "Could not auto-detect your public IP. NSG SSH/RPC rules will allow 0.0.0.0/0."
        warn "Restrict them manually after deployment for security."
        MY_IP="*"
    else
        info "Your public IP: ${MY_IP} (used for SSH/RPC NSG rules)"
    fi

    info "Preflight passed."
}

# =============================================================================
# PHASE 1: INFRASTRUCTURE (resource group, VNet, NSG)
# =============================================================================

create_infra() {
    section "Phase 1: Infrastructure"

    # --- Resource Group ---
    info "Creating resource group '${RESOURCE_GROUP}' in ${LOCATION}..."
    az group create \
        --name  "${RESOURCE_GROUP}" \
        --location "${LOCATION}" \
        --output none

    # --- Virtual Network + Subnet ---
    info "Creating VNet (${VNET_CIDR}) and subnet (${SUBNET_CIDR})..."
    az network vnet create \
        --resource-group "${RESOURCE_GROUP}" \
        --name "rill-vnet" \
        --address-prefix "${VNET_CIDR}" \
        --subnet-name "rill-subnet" \
        --subnet-prefix "${SUBNET_CIDR}" \
        --output none

    # --- Network Security Group ---
    info "Creating Network Security Group 'rill-nsg'..."
    az network nsg create \
        --resource-group "${RESOURCE_GROUP}" \
        --name "rill-nsg" \
        --output none

    # Rule 100 — SSH from caller's IP only.
    # Priority 100 (lowest number = highest precedence).
    local ssh_source="${MY_IP}"
    [[ "${MY_IP}" == "*" ]] && ssh_source="Internet"
    info "NSG: Allow SSH (22) from ${ssh_source}"
    az network nsg rule create \
        --resource-group "${RESOURCE_GROUP}" \
        --nsg-name "rill-nsg" \
        --name "AllowSSH" \
        --priority 100 \
        --protocol Tcp \
        --direction Inbound \
        --source-address-prefixes "${ssh_source}" \
        --source-port-ranges "*" \
        --destination-address-prefixes "*" \
        --destination-port-ranges 22 \
        --access Allow \
        --output none

    # Rule 110 — P2P between VNet nodes only (18333).
    info "NSG: Allow P2P (${P2P_PORT}) within VNet"
    az network nsg rule create \
        --resource-group "${RESOURCE_GROUP}" \
        --nsg-name "rill-nsg" \
        --name "AllowP2PIntranet" \
        --priority 110 \
        --protocol Tcp \
        --direction Inbound \
        --source-address-prefixes "${VNET_CIDR}" \
        --source-port-ranges "*" \
        --destination-address-prefixes "*" \
        --destination-port-ranges "${P2P_PORT}" \
        --access Allow \
        --output none

    # Rule 120 — RPC from caller's IP only (18332).
    local rpc_source="${MY_IP}"
    [[ "${MY_IP}" == "*" ]] && rpc_source="Internet"
    info "NSG: Allow RPC (${RPC_PORT}) from ${rpc_source}"
    az network nsg rule create \
        --resource-group "${RESOURCE_GROUP}" \
        --nsg-name "rill-nsg" \
        --name "AllowRPC" \
        --priority 120 \
        --protocol Tcp \
        --direction Inbound \
        --source-address-prefixes "${rpc_source}" \
        --source-port-ranges "*" \
        --destination-address-prefixes "*" \
        --destination-port-ranges "${RPC_PORT}" \
        --access Allow \
        --output none

    # Rule 4096 — Deny all other inbound traffic.
    info "NSG: Deny all other inbound"
    az network nsg rule create \
        --resource-group "${RESOURCE_GROUP}" \
        --nsg-name "rill-nsg" \
        --name "DenyAllInbound" \
        --priority 4096 \
        --protocol "*" \
        --direction Inbound \
        --source-address-prefixes "*" \
        --source-port-ranges "*" \
        --destination-address-prefixes "*" \
        --destination-port-ranges "*" \
        --access Deny \
        --output none

    # Associate NSG with the subnet.
    info "Associating NSG with subnet..."
    az network vnet subnet update \
        --resource-group "${RESOURCE_GROUP}" \
        --vnet-name "rill-vnet" \
        --name "rill-subnet" \
        --network-security-group "rill-nsg" \
        --output none

    info "Infrastructure ready."
}

# =============================================================================
# PHASE 2: AZURE CONTAINER REGISTRY + IMAGE BUILD
# =============================================================================

create_acr() {
    section "Phase 2: Container Registry"

    # Basic SKU: ~$5/mo, 10 GB storage, single-region.
    info "Creating ACR '${ACR_NAME}' (Basic tier ~\$5/mo)..."
    az acr create \
        --resource-group "${RESOURCE_GROUP}" \
        --name "${ACR_NAME}" \
        --sku Basic \
        --admin-enabled true \
        --output none

    # Create a clean context directory — az acr build uploads the entire context
    # and may not fully respect .dockerignore. The target/ dir can be 20+ GB
    # so we rsync a clean copy to a temp dir.
    local build_ctx
    build_ctx="$(mktemp -d /tmp/rill-acr-context.XXXXXX)"
    info "Creating clean build context (excluding target/, .git/)..."
    rsync -a \
        --exclude='target' \
        --exclude='.git' \
        --exclude='web/node_modules' \
        --exclude='.DS_Store' \
        "${REPO_ROOT}/" "${build_ctx}/"
    local ctx_size
    ctx_size="$(du -sh "${build_ctx}" | cut -f1)"
    info "Build context: ${ctx_size}"

    info "Building Docker image in ACR (Rust compile — may take 10-15 min)..."
    az acr build \
        --registry "${ACR_NAME}" \
        --image "rill-node:${IMAGE_TAG}" \
        --file "${build_ctx}/Dockerfile" \
        "${build_ctx}"

    rm -rf "${build_ctx}"

    info "Image pushed: ${IMAGE_NAME}"
}

# =============================================================================
# CLOUD-INIT TEMPLATE
# Rendered per-node with variables substituted before passing to az vm create.
# =============================================================================

# Generates a cloud-init YAML document for a given node index.
# Arguments:
#   $1 — node index (0-3)
#   $2 — role: seed | miner | wallet
#   $3 — extra rill-node flags (e.g. "--bootstrap-peers 10.0.1.10:18333")
render_cloud_init() {
    local idx="$1"
    local role="$2"
    local extra_flags="$3"

    # ACR login credentials — retrieved at render time and embedded in the
    # cloud-init document. The password is the ACR admin password; it is
    # passed via cloud-init user-data which is encrypted at rest by Azure.
    # For production use, switch to a managed identity with AcrPull role.
    local acr_password
    acr_password="$(az acr credential show \
        --name "${ACR_NAME}" \
        --query 'passwords[0].value' \
        -o tsv)"

    cat <<CLOUD_INIT
#cloud-config
# RillCoin node${idx} (${role}) — generated by azure-testnet.sh

package_update: true
package_upgrade: true

packages:
  - ca-certificates
  - curl
  - gnupg
  - lsb-release

write_files:
  # systemd unit for rill-node container.
  - path: /etc/systemd/system/rill-node.service
    permissions: '0644'
    content: |
      [Unit]
      Description=RillCoin Node (${role})
      After=docker.service network-online.target
      Requires=docker.service
      Wants=network-online.target

      [Service]
      Type=simple
      Restart=always
      RestartSec=10
      ExecStartPre=-/usr/bin/docker stop rill-node
      ExecStartPre=-/usr/bin/docker rm rill-node
      ExecStart=/usr/bin/docker run \
          --name rill-node \
          --network host \
          --volume rill-data:/data \
          --log-driver journald \
          --log-opt tag=rill-node \
          ${IMAGE_NAME} \
          --data-dir /data \
          --p2p-listen-addr 0.0.0.0 \
          --p2p-listen-port ${P2P_PORT} \
          --rpc-bind 0.0.0.0 \
          --rpc-port ${RPC_PORT} \
          --log-format json \
          ${extra_flags}
      ExecStop=/usr/bin/docker stop rill-node
      StandardOutput=journal
      StandardError=journal

      [Install]
      WantedBy=multi-user.target

  # Docker daemon config — limit log size to avoid filling the OS disk.
  - path: /etc/docker/daemon.json
    permissions: '0644'
    content: |
      {
        "log-driver": "journald",
        "log-opts": {
          "max-size": "100m",
          "max-file": "3"
        }
      }

runcmd:
  # Install Docker CE from the official apt repo.
  - install -m 0755 -d /etc/apt/keyrings
  - curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
  - chmod a+r /etc/apt/keyrings/docker.asc
  - |
    echo \
      "deb [arch=\$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] \
      https://download.docker.com/linux/ubuntu \
      \$(. /etc/os-release && echo \"\${UBUNTU_CODENAME:-\$VERSION_CODENAME}\") stable" \
      | tee /etc/apt/sources.list.d/docker.list > /dev/null
  - apt-get update -y
  - apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin

  # Start and enable Docker.
  - systemctl enable --now docker

  # Log in to ACR using the admin credentials embedded in this cloud-init doc.
  # For production: assign a managed identity and grant it AcrPull instead.
  - docker login ${ACR_NAME}.azurecr.io --username ${ACR_NAME} --password "${acr_password}"

  # Pull the RillCoin image.
  - docker pull ${IMAGE_NAME}

  # Create the named volume for blockchain data persistence.
  - docker volume create rill-data

  # Enable and start the rill-node systemd service.
  - systemctl daemon-reload
  - systemctl enable rill-node
  - systemctl start rill-node
CLOUD_INIT
}

# =============================================================================
# PHASE 3: VIRTUAL MACHINES
# =============================================================================

create_vms() {
    section "Phase 3: Virtual Machines"

    # Public IP for node0 only — all other nodes communicate within the VNet.
    info "Creating public IP for node0 (seed)..."
    az network public-ip create \
        --resource-group "${RESOURCE_GROUP}" \
        --name "rill-node0-pip" \
        --allocation-method Static \
        --sku Standard \
        --output none

    local seed_ip="${NODE_IPS[0]}"

    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        local private_ip="${NODE_IPS[$i]}"
        local role="${NODE_ROLES[$i]}"

        section "Creating VM: ${node_name} (${role}, ${private_ip})"

        # Build the rill-node CLI flags for this role.
        local extra_flags=""
        case "${role}" in
            seed)
                # Seed node: no bootstrap peers — it IS the bootstrap peer.
                extra_flags=""
                ;;
            miner)
                extra_flags="--bootstrap-peers ${seed_ip}:${P2P_PORT}"
                ;;
            wallet)
                # Wallet/RPC node connects to seed; no mining.
                extra_flags="--bootstrap-peers ${seed_ip}:${P2P_PORT}"
                ;;
        esac

        # Write cloud-init to a temp file (az vm create takes a file path).
        local cloudinit_file
        cloudinit_file="$(mktemp /tmp/rill-cloudinit-node${i}-XXXXXX.yaml)"
        # shellcheck disable=SC2064
        trap "rm -f '${cloudinit_file}'" EXIT

        info "Rendering cloud-init for ${node_name}..."
        render_cloud_init "${i}" "${role}" "${extra_flags}" > "${cloudinit_file}"

        # Create a NIC with the static private IP and attach the NSG.
        info "Creating NIC for ${node_name} (${private_ip})..."
        local nic_args=(
            --resource-group "${RESOURCE_GROUP}"
            --name "${node_name}-nic"
            --vnet-name "rill-vnet"
            --subnet "rill-subnet"
            --private-ip-address "${private_ip}"
            --network-security-group "rill-nsg"
            --output none
        )
        # Attach the public IP only to node0.
        if [[ "${i}" -eq 0 ]]; then
            nic_args+=(--public-ip-address "rill-node0-pip")
        else
            nic_args+=(--public-ip-address "")
        fi
        az network nic create "${nic_args[@]}"

        # Create the VM.
        info "Creating VM ${node_name} (${VM_SIZE})..."
        az vm create \
            --resource-group "${RESOURCE_GROUP}" \
            --name "${node_name}" \
            --size "${VM_SIZE}" \
            --image "${VM_IMAGE}" \
            --admin-username "${ADMIN_USER}" \
            --ssh-key-values "${SSH_PUBLIC_KEY_PATH}" \
            --nics "${node_name}-nic" \
            --os-disk-size-gb 32 \
            --storage-sku Premium_LRS \
            --custom-data "${cloudinit_file}" \
            --no-wait \
            --output none

        info "VM ${node_name} creation queued (running in background)."
    done

    # Wait for all VMs to reach the running state.
    section "Waiting for all VMs to provision"
    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        info "Waiting for ${node_name}..."
        az vm wait \
            --resource-group "${RESOURCE_GROUP}" \
            --name "${node_name}" \
            --created
        info "${node_name} is running."
    done

    # Give cloud-init a moment to start Docker and pull the image.
    warn "VMs are running. cloud-init is installing Docker and starting rill-node."
    warn "Allow 3-5 minutes for all nodes to become healthy before querying RPC."
}

# =============================================================================
# STATUS
# =============================================================================

status() {
    section "Testnet Status"

    # Verify resource group exists before querying.
    if ! az group show --name "${RESOURCE_GROUP}" &>/dev/null; then
        warn "Resource group '${RESOURCE_GROUP}' does not exist. Has the testnet been deployed?"
        return 1
    fi

    local seed_public_ip
    seed_public_ip="$(az network public-ip show \
        --resource-group "${RESOURCE_GROUP}" \
        --name "rill-node0-pip" \
        --query 'ipAddress' -o tsv 2>/dev/null || echo "N/A")"

    printf "\n${BOLD}%-12s %-15s %-15s %-10s %-8s${RESET}\n" \
        "Node" "Private IP" "Public IP" "Role" "VM State"
    printf '%s\n' "$(printf '%.0s-' {1..65})"

    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        local private_ip="${NODE_IPS[$i]}"
        local role="${NODE_ROLES[$i]}"
        local public_ip="(internal)"
        [[ "${i}" -eq 0 ]] && public_ip="${seed_public_ip}"

        local vm_state
        vm_state="$(az vm show \
            --resource-group "${RESOURCE_GROUP}" \
            --name "${node_name}" \
            --show-details \
            --query 'powerState' -o tsv 2>/dev/null || echo "unknown")"

        printf "%-12s %-15s %-15s %-10s %-8s\n" \
            "${node_name}" "${private_ip}" "${public_ip}" "${role}" "${vm_state}"
    done

    printf "\n"
    info "Seed node SSH:   ssh ${ADMIN_USER}@${seed_public_ip}"
    info "Seed RPC tunnel: ssh -L 18332:${NODE_IPS[0]}:${RPC_PORT} ${ADMIN_USER}@${seed_public_ip}"
    warn "Run './infra/azure-testnet.sh tunnel' for all tunnel commands."
}

# =============================================================================
# SSH TUNNEL HELPER
# =============================================================================

setup_ssh_tunnel() {
    section "SSH Tunnel Commands"

    local seed_public_ip
    seed_public_ip="$(az network public-ip show \
        --resource-group "${RESOURCE_GROUP}" \
        --name "rill-node0-pip" \
        --query 'ipAddress' -o tsv 2>/dev/null || echo "<seed-public-ip>")"

    cat <<TUNNEL

All RPC traffic must be tunnelled through node0 (the only node with a public IP).
Run one of the commands below in a separate terminal, then query the node's RPC
on the forwarded localhost port.

# node0 (seed) — forwards local port 18332 -> node0:18332
ssh -N -L 18332:${NODE_IPS[0]}:${RPC_PORT} ${ADMIN_USER}@${seed_public_ip}

# node1 (miner) — forwards local port 18342 -> node1:18332
ssh -N -L 18342:${NODE_IPS[1]}:${RPC_PORT} ${ADMIN_USER}@${seed_public_ip}

# node2 (miner) — forwards local port 18352 -> node2:18332
ssh -N -L 18352:${NODE_IPS[2]}:${RPC_PORT} ${ADMIN_USER}@${seed_public_ip}

# node3 (wallet) — forwards local port 18362 -> node3:18332
ssh -N -L 18362:${NODE_IPS[3]}:${RPC_PORT} ${ADMIN_USER}@${seed_public_ip}

# All nodes in one command (background tunnel):
ssh -f -N \
    -L 18332:${NODE_IPS[0]}:${RPC_PORT} \
    -L 18342:${NODE_IPS[1]}:${RPC_PORT} \
    -L 18352:${NODE_IPS[2]}:${RPC_PORT} \
    -L 18362:${NODE_IPS[3]}:${RPC_PORT} \
    ${ADMIN_USER}@${seed_public_ip}

After tunnelling, query rill-cli (examples):
  rill-cli --rpc-host 127.0.0.1 --rpc-port 18332 getblockcount  # node0
  rill-cli --rpc-host 127.0.0.1 --rpc-port 18362 getbalance     # node3 wallet

TUNNEL
}

# =============================================================================
# SSH INTO A NODE
# =============================================================================

ssh_node() {
    local idx="${1:-0}"
    if [[ "${idx}" -lt 0 ]] || [[ "${idx}" -ge "${NODE_COUNT}" ]]; then
        error "Node index must be 0-$((NODE_COUNT - 1))"
        exit 1
    fi

    local seed_public_ip
    seed_public_ip="$(az network public-ip show \
        --resource-group "${RESOURCE_GROUP}" \
        --name "rill-node0-pip" \
        --query 'ipAddress' -o tsv)"

    if [[ "${idx}" -eq 0 ]]; then
        info "SSH -> node0 (${seed_public_ip})"
        exec ssh "${ADMIN_USER}@${seed_public_ip}"
    else
        local private_ip="${NODE_IPS[$idx]}"
        info "SSH -> node${idx} via node0 (ProxyJump)"
        exec ssh \
            -J "${ADMIN_USER}@${seed_public_ip}" \
            "${ADMIN_USER}@${private_ip}"
    fi
}

# =============================================================================
# STOP (DEALLOCATE) ALL VMs — saves compute cost while not testing
# =============================================================================

stop_all() {
    section "Stopping (deallocating) all VMs"
    warn "Deallocated VMs incur no compute cost but still charge for:"
    warn "  - OS disk storage (~\$2.40/mo per 32 GB Premium SSD)"
    warn "  - Static public IP (~\$3.65/mo)"
    warn "  - ACR Basic tier (~\$5/mo)"

    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        info "Deallocating ${node_name}..."
        az vm deallocate \
            --resource-group "${RESOURCE_GROUP}" \
            --name "${node_name}" \
            --no-wait \
            --output none
    done
    info "Deallocation queued. VMs will reach 'deallocated' state in ~1-2 minutes."
}

# =============================================================================
# START ALL VMs
# =============================================================================

start_all() {
    section "Starting all VMs"

    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        info "Starting ${node_name}..."
        az vm start \
            --resource-group "${RESOURCE_GROUP}" \
            --name "${node_name}" \
            --no-wait \
            --output none
    done

    info "Start commands queued. VMs will be running in ~1-2 minutes."
    warn "After restart, rill-node systemd service starts automatically."
    warn "Allow ~60 seconds for nodes to re-establish P2P connections."
}

# =============================================================================
# TEARDOWN — deletes the entire resource group (ALL resources)
# =============================================================================

teardown() {
    section "Teardown"

    error "WARNING: This will permanently delete resource group '${RESOURCE_GROUP}'"
    error "and ALL resources within it (VMs, VNet, NSG, ACR, disks, public IPs)."
    printf "\n"
    read -r -p "Type the resource group name to confirm deletion: " confirm

    if [[ "${confirm}" != "${RESOURCE_GROUP}" ]]; then
        info "Teardown cancelled."
        return 0
    fi

    warn "Deleting resource group '${RESOURCE_GROUP}'..."
    az group delete \
        --name "${RESOURCE_GROUP}" \
        --yes \
        --no-wait

    info "Deletion queued. Resource group will be removed in 5-10 minutes."
    info "Verify with: az group show --name '${RESOURCE_GROUP}'"
}

# =============================================================================
# COST ESTIMATE
# =============================================================================

cost_estimate() {
    section "Estimated Monthly Cost (eastus, pay-as-you-go)"

    cat <<COST

All prices in USD. Based on eastus pay-as-you-go rates as of early 2026.
Actual costs may vary; check https://azure.microsoft.com/pricing/calculator/

--- RUNNING (24/7) ---
${NODE_COUNT}x Standard_B2s VMs      ${NODE_COUNT} x \$30.37/mo   =  \$$(echo "${NODE_COUNT} * 30.37" | bc)
${NODE_COUNT}x Premium SSD P4 32GB   ${NODE_COUNT} x \$2.40/mo    =  \$$(echo "${NODE_COUNT} * 2.40" | bc)
1x Standard Static Public IP         1 x \$3.65/mo    =  \$3.65
1x VNet + Subnet                     Free             =  \$0.00
1x NSG                               Free             =  \$0.00
ACR Basic                            1 x \$5.00/mo    =  \$5.00
Bandwidth (est. 10 GB egress)        10 x \$0.087/GB  =  \$0.87
                                                       ---------
TOTAL (running 24/7)                                   ~\$$(echo "scale=2; ${NODE_COUNT} * 30.37 + ${NODE_COUNT} * 2.40 + 3.65 + 5.00 + 0.87" | bc)/mo

--- STOPPED (deallocated VMs, disks + IP still billed) ---
${NODE_COUNT}x Premium SSD P4 32GB   ${NODE_COUNT} x \$2.40/mo    =  \$$(echo "${NODE_COUNT} * 2.40" | bc)
1x Standard Static Public IP         1 x \$3.65/mo    =  \$3.65
ACR Basic                            1 x \$5.00/mo    =  \$5.00
                                                       ---------
TOTAL (VMs stopped)                                    ~\$$(echo "scale=2; ${NODE_COUNT} * 2.40 + 3.65 + 5.00" | bc)/mo

Tip: Run './infra/azure-testnet.sh stop' when not actively testing.
Tip: Run './infra/azure-testnet.sh teardown' to delete everything when done.
COST
}

# =============================================================================
# FULL DEPLOY — runs all three phases in order
# =============================================================================

deploy() {
    preflight
    create_infra
    create_acr
    create_vms

    section "Deploy Complete"
    info "Testnet is provisioning. Summary:"
    printf "\n"
    status
    printf "\n"
    cost_estimate
    printf "\n"
    warn "REMINDER: Run './infra/azure-testnet.sh stop' when done testing to save money."
}

# =============================================================================
# USAGE / HELP
# =============================================================================

usage() {
    cat <<USAGE
RillCoin Azure Testnet Provisioner

USAGE:
    ./infra/azure-testnet.sh <subcommand> [args]

SUBCOMMANDS:
    deploy        Full deploy: infra + ACR + VMs (idempotent-ish)
    status        Show VM status, IPs, and access commands
    ssh [N]       SSH into node N (default 0). Nodes 1-3 go via ProxyJump.
    tunnel        Print SSH tunnel commands for RPC access
    stop          Deallocate all VMs (saves compute cost, disks still billed)
    start         Start all deallocated VMs
    teardown      Delete EVERYTHING in resource group (with confirmation)
    cost          Print estimated monthly cost breakdown

CONFIGURATION (edit top of script):
    RESOURCE_GROUP  = ${RESOURCE_GROUP}
    LOCATION        = ${LOCATION}
    VM_SIZE         = ${VM_SIZE}
    NODE_COUNT      = ${NODE_COUNT}
    ACR_NAME        = ${ACR_NAME}

PREREQUISITES:
    az login        # Azure CLI authenticated
    docker          # Local Docker for ACR image build
    SSH key at:     ${SSH_PUBLIC_KEY_PATH}

USAGE
}

# =============================================================================
# ENTRY POINT
# =============================================================================

main() {
    local subcommand="${1:-help}"
    shift || true

    case "${subcommand}" in
        deploy)
            deploy
            ;;
        status)
            status
            ;;
        ssh)
            ssh_node "${1:-0}"
            ;;
        tunnel)
            setup_ssh_tunnel
            ;;
        stop)
            stop_all
            ;;
        start)
            start_all
            ;;
        teardown)
            teardown
            ;;
        cost)
            cost_estimate
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            error "Unknown subcommand: ${subcommand}"
            printf "\n"
            usage
            exit 1
            ;;
    esac
}

main "$@"
