#!/usr/bin/env bash
# =============================================================================
# do-testnet.sh -- RillCoin DigitalOcean Testnet Provisioner
# =============================================================================
#
# Provisions a 2-node RillCoin testnet on DigitalOcean:
#   node0  seed + miner (public IP firewall-restricted, SSH entry point)
#   node1  wallet / RPC gateway (public IP, RPC from caller IP only)
#
# Builds from source on each droplet -- no Docker or container registry needed.
# Only requires: doctl (authenticated) + SSH key on your DO account.
#
# Usage:
#   ./infra/do-testnet.sh deploy      # Full deploy (VPC + firewall + droplets)
#   ./infra/do-testnet.sh status      # Show droplet status and IPs
#   ./infra/do-testnet.sh ssh [N]     # SSH into node N (default 0)
#   ./infra/do-testnet.sh tunnel      # Print SSH tunnel commands
#   ./infra/do-testnet.sh stop        # Power off all droplets (still billed!)
#   ./infra/do-testnet.sh start       # Power on all droplets
#   ./infra/do-testnet.sh teardown    # Delete everything (destructive)
#   ./infra/do-testnet.sh cost        # Print estimated monthly cost
#   ./infra/do-testnet.sh logs [N]    # SSH in and tail rill-node logs
#
# Prerequisites:
#   - doctl >= 1.100 installed and authenticated (doctl auth init)
#   - An SSH key uploaded to your DigitalOcean account
#
# =============================================================================
set -euo pipefail

# =============================================================================
# CONFIG -- edit these before running
# =============================================================================

REGION="nyc1"

# s-1vcpu-2gb: 1 vCPU, 2 GB RAM -- sufficient for testnet node.
# ~$12/mo per droplet. 2 GB swap added for Rust compilation.
DROPLET_SIZE="s-1vcpu-2gb"

# Ubuntu 24.04 LTS
DROPLET_IMAGE="ubuntu-24-04-x64"

# 1 seed/miner + 1 wallet/RPC
NODE_COUNT=2

# VPC and firewall names.
VPC_NAME="rill-testnet-vpc"
FIREWALL_NAME="rill-testnet-fw"

# VPC private address range (must not overlap DO reserved 10.10.0.0/16).
VPC_CIDR="10.20.0.0/24"

# Tag applied to all resources for grouping and easy teardown.
TAG="rill-testnet"

# DO droplets default to root login when provisioned via SSH key.
ADMIN_USER="root"

# SSH key fingerprint -- auto-detected from your doctl account if left empty.
SSH_KEY_FINGERPRINT=""

# GitHub repo to clone on each droplet.
GITHUB_REPO="https://github.com/rillcoin/rill.git"
GITHUB_BRANCH="main"

# RillCoin ports.
P2P_PORT=18333
RPC_PORT=18332

# Node roles (index-aligned with node0..node1).
declare -a NODE_ROLES=("seed" "wallet")

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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# =============================================================================
# HELPERS: doctl wrappers
# =============================================================================

droplet_public_ip() {
    local name="$1"
    doctl compute droplet list \
        --tag-name "${TAG}" \
        --format "Name,PublicIPv4" \
        --no-header 2>/dev/null \
        | awk -v n="${name}" '$1 == n { print $2 }'
}

droplet_id() {
    local name="$1"
    doctl compute droplet list \
        --tag-name "${TAG}" \
        --format "ID,Name" \
        --no-header 2>/dev/null \
        | awk -v n="${name}" '$2 == n { print $1 }'
}

droplet_private_ip() {
    local name="$1"
    local dro_id
    dro_id="$(droplet_id "${name}")"
    if [[ -n "${dro_id}" ]]; then
        doctl compute droplet get "${dro_id}" \
            --format "PrivateIPv4" --no-header 2>/dev/null || echo ""
    fi
}

droplet_status() {
    local name="$1"
    doctl compute droplet list \
        --tag-name "${TAG}" \
        --format "Name,Status" \
        --no-header 2>/dev/null \
        | awk -v n="${name}" '$1 == n { print $2 }'
}

# =============================================================================
# PREFLIGHT CHECKS
# =============================================================================

preflight() {
    section "Preflight checks"

    # Verify doctl is installed.
    if ! command -v doctl &>/dev/null; then
        error "doctl not found. Install it: brew install doctl"
        exit 1
    fi

    local doctl_ver
    doctl_ver="$(doctl version 2>/dev/null | head -1 || echo "unknown")"
    info "doctl version: ${doctl_ver}"

    # Use DIGITALOCEAN_ACCESS_TOKEN from .envrc if set.
    if [[ -n "${DIGITALOCEAN_ACCESS_TOKEN:-}" ]]; then
        export DIGITALOCEAN_ACCESS_TOKEN
        info "Using DIGITALOCEAN_ACCESS_TOKEN from environment."
    fi

    # Verify the user is authenticated.
    if ! doctl account get &>/dev/null; then
        error "Not authenticated with DigitalOcean. Run: doctl auth init"
        error "Or set DIGITALOCEAN_ACCESS_TOKEN in .envrc"
        exit 1
    fi

    local account_email
    account_email="$(doctl account get --format "Email" --no-header 2>/dev/null || echo "unknown")"
    info "Active DO account: ${account_email}"

    # Auto-detect SSH key fingerprint if not set in config.
    if [[ -z "${SSH_KEY_FINGERPRINT}" ]]; then
        info "SSH_KEY_FINGERPRINT not set -- auto-detecting from your DO account..."
        local key_count
        key_count="$(doctl compute ssh-key list --format "FingerPrint" --no-header 2>/dev/null | wc -l | tr -d ' ')"
        if [[ "${key_count}" -eq 0 ]]; then
            error "No SSH keys found in your DigitalOcean account."
            error "Upload one: doctl compute ssh-key import rill --public-key-file ~/.ssh/id_ed25519.pub"
            exit 1
        fi
        if [[ "${key_count}" -gt 1 ]]; then
            warn "Multiple SSH keys found in your DO account. Using the first one."
            warn "Set SSH_KEY_FINGERPRINT in the config section to use a specific key."
        fi
        SSH_KEY_FINGERPRINT="$(doctl compute ssh-key list --format "FingerPrint" --no-header 2>/dev/null | head -1)"
        info "Using SSH key fingerprint: ${SSH_KEY_FINGERPRINT}"
    else
        info "Using SSH key fingerprint: ${SSH_KEY_FINGERPRINT} (from config)"
    fi

    # Detect the caller's public IP for firewall rules.
    MY_IP="$(curl -sf https://api.ipify.org || curl -sf https://ifconfig.me || echo "")"
    if [[ -z "${MY_IP}" ]]; then
        warn "Could not auto-detect your public IP. Firewall rules will allow 0.0.0.0/0."
        MY_IP="0.0.0.0/0"
    else
        info "Your public IP: ${MY_IP} (used for SSH/RPC firewall rules)"
        MY_IP="${MY_IP}/32"
    fi

    info "Preflight passed."
}

# =============================================================================
# PHASE 1: VPC
# =============================================================================

create_vpc() {
    section "Phase 1: VPC"

    local existing_vpc_id
    existing_vpc_id="$(doctl vpcs list \
        --format "ID,Name" \
        --no-header 2>/dev/null \
        | awk -v n="${VPC_NAME}" '$2 == n { print $1 }')"

    if [[ -n "${existing_vpc_id}" ]]; then
        info "VPC '${VPC_NAME}' already exists (ID: ${existing_vpc_id}) -- skipping."
        VPC_ID="${existing_vpc_id}"
        return 0
    fi

    info "Creating VPC '${VPC_NAME}' (${VPC_CIDR}) in ${REGION}..."
    local vpc_json
    vpc_json="$(doctl vpcs create \
        --name "${VPC_NAME}" \
        --region "${REGION}" \
        --ip-range "${VPC_CIDR}" \
        --output json 2>&1)"

    # Parse ID from JSON -- doctl may return [{...}] or {...}
    VPC_ID="$(echo "${vpc_json}" | python3 -c "
import sys, json
data = json.load(sys.stdin)
if isinstance(data, list):
    print(data[0]['id'])
else:
    print(data['id'])
")"

    info "VPC created: ${VPC_ID}"
}

resolve_vpc_id() {
    VPC_ID="$(doctl vpcs list \
        --format "ID,Name" \
        --no-header 2>/dev/null \
        | awk -v n="${VPC_NAME}" '$2 == n { print $1 }')"
    if [[ -z "${VPC_ID}" ]]; then
        error "VPC '${VPC_NAME}' not found. Has the testnet been deployed?"
        exit 1
    fi
}

# =============================================================================
# PHASE 2: CLOUD FIREWALL
# =============================================================================

create_firewall() {
    section "Phase 2: Cloud Firewall"

    # Ensure the tag exists before referencing it in the firewall.
    # DO requires tags to exist before they can be used in firewall rules.
    info "Ensuring tag '${TAG}' exists..."
    doctl compute tag create "${TAG}" 2>/dev/null || true

    local existing_fw_id
    existing_fw_id="$(doctl compute firewall list \
        --format "ID,Name" \
        --no-header 2>/dev/null \
        | awk -v n="${FIREWALL_NAME}" '$2 == n { print $1 }')"

    if [[ -n "${existing_fw_id}" ]]; then
        info "Firewall '${FIREWALL_NAME}' already exists (ID: ${existing_fw_id}) -- skipping."
        FIREWALL_ID="${existing_fw_id}"
        return 0
    fi

    info "Creating firewall '${FIREWALL_NAME}'..."
    info "  Allow SSH (22) from ${MY_IP}"
    info "  Allow P2P (${P2P_PORT}) from VPC ${VPC_CIDR}"
    info "  Allow RPC (${RPC_PORT}) from ${MY_IP}"
    info "  Allow all outbound"

    FIREWALL_ID="$(doctl compute firewall create \
        --name "${FIREWALL_NAME}" \
        --tag-names "${TAG}" \
        --inbound-rules "protocol:tcp,ports:22,address:${MY_IP} protocol:tcp,ports:${P2P_PORT},address:${VPC_CIDR} protocol:tcp,ports:${RPC_PORT},address:${MY_IP}" \
        --outbound-rules "protocol:tcp,ports:all,address:0.0.0.0/0 protocol:udp,ports:all,address:0.0.0.0/0 protocol:icmp,address:0.0.0.0/0" \
        --format "ID" \
        --no-header)"

    info "Firewall created: ${FIREWALL_ID}"
    info "Firewall is attached via tag '${TAG}' -- all tagged droplets inherit these rules."
}

# =============================================================================
# CLOUD-INIT TEMPLATE -- builds from source, no Docker needed
# =============================================================================

render_cloud_init() {
    local idx="$1"
    local role="$2"
    local extra_flags="$3"

    cat <<CLOUD_INIT
#cloud-config
# RillCoin node${idx} (${role}) -- generated by do-testnet.sh

package_update: true
package_upgrade: true

packages:
  - build-essential
  - clang
  - libclang-dev
  - pkg-config
  - libssl-dev
  - git
  - curl
  - ca-certificates

write_files:
  # systemd unit for rill-node (runs the binary directly).
  - path: /etc/systemd/system/rill-node.service
    permissions: '0644'
    content: |
      [Unit]
      Description=RillCoin Node (${role})
      After=network-online.target
      Wants=network-online.target

      [Service]
      Type=simple
      User=rill
      Group=rill
      Restart=always
      RestartSec=10
      ExecStart=/usr/local/bin/rill-node \
          --data-dir /var/lib/rill \
          --p2p-listen-addr 0.0.0.0 \
          --p2p-listen-port ${P2P_PORT} \
          --rpc-bind 0.0.0.0 \
          --rpc-port ${RPC_PORT} \
          --log-format json \
          ${extra_flags}
      StandardOutput=journal
      StandardError=journal
      LimitNOFILE=65536

      [Install]
      WantedBy=multi-user.target

  # Build script -- runs once via cloud-init.
  - path: /opt/rill-build.sh
    permissions: '0755'
    content: |
      #!/bin/bash
      set -eo pipefail
      export HOME=/root
      exec > /var/log/rill-build.log 2>&1

      echo "=== RillCoin build started at \$(date) ==="

      # Create 2 GB swap for compilation (2 GB RAM is tight for Rust).
      if [ ! -f /swapfile ]; then
          fallocate -l 2G /swapfile
          chmod 600 /swapfile
          mkswap /swapfile
          swapon /swapfile
          echo '/swapfile none swap sw 0 0' >> /etc/fstab
          echo "Swap enabled."
      fi

      # Install Rust toolchain.
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
          sh -s -- -y --default-toolchain 1.85.0
      source /root/.cargo/env

      echo "Rust version: \$(rustc --version)"

      # Clone the repo.
      cd /opt
      if [ ! -d /opt/rill ]; then
          git clone --branch ${GITHUB_BRANCH} --depth 1 ${GITHUB_REPO}
      fi
      cd /opt/rill

      # Build release binaries.
      echo "Starting cargo build (this takes 15-20 minutes)..."
      cargo build --release --locked \
          --bin rill-node \
          --bin rill-cli \
          --bin rill-miner

      echo "Build complete."

      # Install binaries.
      cp target/release/rill-node  /usr/local/bin/
      cp target/release/rill-cli   /usr/local/bin/
      cp target/release/rill-miner /usr/local/bin/
      chmod +x /usr/local/bin/rill-*

      # Create rill system user and data directory.
      useradd --system --no-create-home --shell /usr/sbin/nologin rill || true
      mkdir -p /var/lib/rill
      chown rill:rill /var/lib/rill

      # Enable and start the service.
      systemctl daemon-reload
      systemctl enable rill-node
      systemctl start rill-node

      echo "=== RillCoin node started at \$(date) ==="

runcmd:
  - /opt/rill-build.sh
CLOUD_INIT
}

# =============================================================================
# PHASE 3: DROPLETS
# =============================================================================

create_droplets() {
    section "Phase 3: Droplets"

    local seed_private_ip=""

    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        local role="${NODE_ROLES[$i]}"

        # Skip if the droplet already exists.
        local existing_id
        existing_id="$(droplet_id "${node_name}")"
        if [[ -n "${existing_id}" ]]; then
            info "Droplet '${node_name}' already exists (ID: ${existing_id}) -- skipping."
            if [[ "${i}" -eq 0 ]]; then
                seed_private_ip="$(doctl compute droplet get "${existing_id}" \
                    --format "PrivateIPv4" --no-header 2>/dev/null || echo "")"
                info "Seed private IP: ${seed_private_ip}"
            fi
            continue
        fi

        section "Creating droplet: ${node_name} (${role})"

        # Build the rill-node CLI flags for this role.
        local extra_flags=""
        case "${role}" in
            seed)
                extra_flags=""
                ;;
            wallet)
                extra_flags="--bootstrap-peers ${seed_private_ip}:${P2P_PORT}"
                ;;
        esac

        # Write cloud-init to a temp file.
        local cloudinit_file
        cloudinit_file="$(mktemp /tmp/rill-cloudinit-node${i}-XXXXXX)"
        # shellcheck disable=SC2064
        trap "rm -f '${cloudinit_file}'" EXIT

        info "Rendering cloud-init for ${node_name}..."
        render_cloud_init "${i}" "${role}" "${extra_flags}" > "${cloudinit_file}"

        info "Creating droplet ${node_name} (${DROPLET_SIZE} in ${REGION})..."
        local new_id
        new_id="$(doctl compute droplet create "${node_name}" \
            --size "${DROPLET_SIZE}" \
            --image "${DROPLET_IMAGE}" \
            --region "${REGION}" \
            --vpc-uuid "${VPC_ID}" \
            --ssh-keys "${SSH_KEY_FINGERPRINT}" \
            --tag-names "${TAG}" \
            --user-data-file "${cloudinit_file}" \
            --enable-private-networking \
            --format "ID" \
            --no-header)"

        info "Droplet ${node_name} queued (ID: ${new_id})."

        # For node0, wait until active and retrieve the private IP.
        if [[ "${i}" -eq 0 ]]; then
            info "Waiting for ${node_name} to become active..."
            local attempts=0
            local max_attempts=60  # 5 minutes
            while [[ "${attempts}" -lt "${max_attempts}" ]]; do
                local current_status
                current_status="$(doctl compute droplet get "${new_id}" \
                    --format "Status" --no-header 2>/dev/null || echo "unknown")"
                if [[ "${current_status}" == "active" ]]; then
                    break
                fi
                sleep 5
                attempts=$((attempts + 1))
                info "  ${node_name} status: ${current_status} (attempt ${attempts}/${max_attempts})"
            done

            if [[ "${attempts}" -ge "${max_attempts}" ]]; then
                error "Timed out waiting for ${node_name} to become active."
                exit 1
            fi

            seed_private_ip="$(doctl compute droplet get "${new_id}" \
                --format "PrivateIPv4" --no-header 2>/dev/null || echo "")"

            if [[ -z "${seed_private_ip}" ]]; then
                error "Could not retrieve private IP for ${node_name}."
                exit 1
            fi

            info "${node_name} is active. Private IP: ${seed_private_ip}"
        fi
    done

    # Wait for remaining droplets.
    section "Waiting for all droplets to become active"
    for i in $(seq 1 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        info "Waiting for ${node_name}..."
        local dro_id
        dro_id="$(droplet_id "${node_name}")"
        if [[ -z "${dro_id}" ]]; then
            warn "${node_name} not found -- it may have failed to create."
            continue
        fi
        local attempts=0
        local max_attempts=60
        while [[ "${attempts}" -lt "${max_attempts}" ]]; do
            local current_status
            current_status="$(doctl compute droplet get "${dro_id}" \
                --format "Status" --no-header 2>/dev/null || echo "unknown")"
            if [[ "${current_status}" == "active" ]]; then
                break
            fi
            sleep 5
            attempts=$((attempts + 1))
        done
        local final_status
        final_status="$(doctl compute droplet get "${dro_id}" \
            --format "Status" --no-header 2>/dev/null || echo "unknown")"
        info "${node_name} status: ${final_status}"
    done

    warn "Droplets are active. cloud-init is building from source (~15-20 min)."
    warn "Monitor build progress: ./infra/do-testnet.sh logs 0"
}

# =============================================================================
# STATUS
# =============================================================================

status() {
    section "Testnet Status"

    local droplet_count
    droplet_count="$(doctl compute droplet list \
        --tag-name "${TAG}" \
        --format "ID" \
        --no-header 2>/dev/null | wc -l | tr -d ' ')"

    if [[ "${droplet_count}" -eq 0 ]]; then
        warn "No droplets found with tag '${TAG}'. Has the testnet been deployed?"
        return 1
    fi

    local seed_name="rill-node0"
    local seed_public_ip
    seed_public_ip="$(droplet_public_ip "${seed_name}")"

    printf "\n${BOLD}%-12s %-16s %-16s %-10s %-10s${RESET}\n" \
        "Node" "Public IP" "Private IP" "Role" "Status"
    printf '%s\n' "$(printf '%.0s-' {1..70})"

    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        local role="${NODE_ROLES[$i]}"

        local dro_id
        dro_id="$(droplet_id "${node_name}")"
        if [[ -z "${dro_id}" ]]; then
            printf "%-12s %-16s %-16s %-10s %-10s\n" \
                "${node_name}" "not found" "not found" "${role}" "missing"
            continue
        fi

        local pub_ip priv_ip status_val
        pub_ip="$(doctl compute droplet get "${dro_id}" \
            --format "PublicIPv4" --no-header 2>/dev/null || echo "N/A")"
        priv_ip="$(doctl compute droplet get "${dro_id}" \
            --format "PrivateIPv4" --no-header 2>/dev/null || echo "N/A")"
        status_val="$(doctl compute droplet get "${dro_id}" \
            --format "Status" --no-header 2>/dev/null || echo "unknown")"

        printf "%-12s %-16s %-16s %-10s %-10s\n" \
            "${node_name}" "${pub_ip}" "${priv_ip}" "${role}" "${status_val}"
    done

    printf "\n"
    info "Seed node SSH:   ssh ${ADMIN_USER}@${seed_public_ip}"
    info "Build log:       ssh ${ADMIN_USER}@${seed_public_ip} tail -f /var/log/rill-build.log"
    info "Node log:        ssh ${ADMIN_USER}@${seed_public_ip} journalctl -u rill-node -f"
    warn "Run './infra/do-testnet.sh tunnel' for RPC tunnel commands."
}

# =============================================================================
# SSH TUNNEL HELPER
# =============================================================================

setup_ssh_tunnel() {
    section "SSH Tunnel Commands"

    local seed_name="rill-node0"
    local seed_public_ip
    seed_public_ip="$(droplet_public_ip "${seed_name}")"
    if [[ -z "${seed_public_ip}" ]]; then
        seed_public_ip="<seed-public-ip>"
        warn "Could not resolve seed public IP. Is the testnet deployed?"
    fi

    local priv_ips=()
    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local priv_ip
        priv_ip="$(droplet_private_ip "rill-node${i}")"
        priv_ips+=("${priv_ip:-<node${i}-private-ip>}")
    done

    cat <<TUNNEL

RPC is not exposed publicly. Tunnel through node0:

# node0 (seed/miner) -- local port 18332
ssh -N -L 18332:${priv_ips[0]:-localhost}:${RPC_PORT} ${ADMIN_USER}@${seed_public_ip}

# node1 (wallet/RPC) -- local port 18342
ssh -N -L 18342:${priv_ips[1]:-<node1-private-ip>}:${RPC_PORT} ${ADMIN_USER}@${seed_public_ip}

# Both nodes in one background tunnel:
ssh -f -N \\
    -L 18332:${priv_ips[0]:-localhost}:${RPC_PORT} \\
    -L 18342:${priv_ips[1]:-<node1-private-ip>}:${RPC_PORT} \\
    ${ADMIN_USER}@${seed_public_ip}

Then query:
  curl -s http://localhost:18332 -d '{"jsonrpc":"2.0","id":1,"method":"getblockcount","params":[]}'

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
    seed_public_ip="$(droplet_public_ip "rill-node0")"

    if [[ -z "${seed_public_ip}" ]]; then
        error "Could not resolve node0 public IP. Is the testnet deployed?"
        exit 1
    fi

    if [[ "${idx}" -eq 0 ]]; then
        info "SSH -> node0 (${seed_public_ip})"
        exec ssh "${ADMIN_USER}@${seed_public_ip}"
    else
        local target_priv_ip
        target_priv_ip="$(droplet_private_ip "rill-node${idx}")"
        info "SSH -> node${idx} (${target_priv_ip}) via ProxyJump through node0"
        exec ssh \
            -J "${ADMIN_USER}@${seed_public_ip}" \
            "${ADMIN_USER}@${target_priv_ip}"
    fi
}

# =============================================================================
# LOGS -- tail build or node logs
# =============================================================================

show_logs() {
    local idx="${1:-0}"
    local seed_public_ip
    seed_public_ip="$(droplet_public_ip "rill-node0")"

    if [[ -z "${seed_public_ip}" ]]; then
        error "Could not resolve node0 public IP."
        exit 1
    fi

    if [[ "${idx}" -eq 0 ]]; then
        info "Tailing logs on node0..."
        ssh "${ADMIN_USER}@${seed_public_ip}" \
            "tail -f /var/log/rill-build.log 2>/dev/null || journalctl -u rill-node -f"
    else
        local target_priv_ip
        target_priv_ip="$(droplet_private_ip "rill-node${idx}")"
        info "Tailing logs on node${idx} via ProxyJump..."
        ssh -J "${ADMIN_USER}@${seed_public_ip}" \
            "${ADMIN_USER}@${target_priv_ip}" \
            "tail -f /var/log/rill-build.log 2>/dev/null || journalctl -u rill-node -f"
    fi
}

# =============================================================================
# STOP (POWER OFF) ALL DROPLETS
# =============================================================================

stop_all() {
    section "Stopping (powering off) all droplets"

    warn "IMPORTANT: DigitalOcean charges for powered-off droplets."
    warn "Run 'teardown' to delete everything and stop billing."

    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        local dro_id
        dro_id="$(droplet_id "${node_name}")"
        if [[ -z "${dro_id}" ]]; then
            warn "${node_name} not found -- skipping."
            continue
        fi
        info "Powering off ${node_name} (ID: ${dro_id})..."
        doctl compute droplet-action power-off "${dro_id}" \
            --wait &>/dev/null || true
        info "${node_name} powered off."
    done

    info "All droplets powered off."
}

# =============================================================================
# START ALL DROPLETS
# =============================================================================

start_all() {
    section "Starting all droplets"

    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        local dro_id
        dro_id="$(droplet_id "${node_name}")"
        if [[ -z "${dro_id}" ]]; then
            warn "${node_name} not found -- skipping."
            continue
        fi
        info "Powering on ${node_name} (ID: ${dro_id})..."
        doctl compute droplet-action power-on "${dro_id}" \
            --wait &>/dev/null || true
        info "${node_name} powered on."
    done

    info "All droplets started."
    warn "rill-node systemd service starts automatically on boot."
}

# =============================================================================
# TEARDOWN
# =============================================================================

teardown() {
    section "Teardown"

    error "WARNING: This will permanently delete ALL testnet resources:"
    error "  - ${NODE_COUNT} droplets (rill-node0 through rill-node$((NODE_COUNT - 1)))"
    error "  - Cloud firewall '${FIREWALL_NAME}'"
    error "  - VPC '${VPC_NAME}'"
    printf "\n"
    warn "This is the only way to stop all billing on DigitalOcean."
    printf "\n"
    read -r -p "Type 'yes' to confirm teardown: " confirm

    if [[ "${confirm}" != "yes" ]]; then
        info "Teardown cancelled."
        return 0
    fi

    # Delete droplets.
    for i in $(seq 0 $((NODE_COUNT - 1))); do
        local node_name="rill-node${i}"
        local dro_id
        dro_id="$(droplet_id "${node_name}")"
        if [[ -n "${dro_id}" ]]; then
            info "Deleting droplet ${node_name} (ID: ${dro_id})..."
            doctl compute droplet delete "${dro_id}" --force
            info "${node_name} deleted."
        else
            info "${node_name} not found -- skipping."
        fi
    done

    # Delete the firewall.
    local fw_id
    fw_id="$(doctl compute firewall list \
        --format "ID,Name" --no-header 2>/dev/null \
        | awk -v n="${FIREWALL_NAME}" '$2 == n { print $1 }')"
    if [[ -n "${fw_id}" ]]; then
        info "Deleting firewall '${FIREWALL_NAME}'..."
        doctl compute firewall delete "${fw_id}" --force
        info "Firewall deleted."
    fi

    # Delete the VPC (wait for droplet cleanup).
    local vpc_id_local
    vpc_id_local="$(doctl vpcs list \
        --format "ID,Name" --no-header 2>/dev/null \
        | awk -v n="${VPC_NAME}" '$2 == n { print $1 }')"
    if [[ -n "${vpc_id_local}" ]]; then
        info "Waiting for droplet cleanup before deleting VPC..."
        sleep 10
        doctl vpcs delete "${vpc_id_local}" --force || \
            warn "VPC deletion failed -- retry in a minute: doctl vpcs delete ${vpc_id_local} --force"
    fi

    info "Teardown complete. All billing stopped."
}

# =============================================================================
# COST ESTIMATE
# =============================================================================

cost_estimate() {
    section "Estimated Monthly Cost"

    cat <<COST

2x s-1vcpu-2gb droplets    2 x \$12/mo    =  \$24/mo
1x VPC                     Free           =  \$0/mo
1x Cloud Firewall          Free           =  \$0/mo
                                             ------
TOTAL                                       \$24/mo

NOTE: DO charges for powered-off droplets. Use 'teardown' to stop billing.
COST
}

# =============================================================================
# FULL DEPLOY
# =============================================================================

deploy() {
    preflight
    create_vpc
    create_firewall
    create_droplets

    section "Deploy Complete"
    info "Droplets are provisioning. Rust is building from source (~15-20 min)."
    printf "\n"
    status
    printf "\n"
    cost_estimate
    printf "\n"
    warn "Monitor build: ./infra/do-testnet.sh logs 0"
    warn "When done testing: ./infra/do-testnet.sh teardown"
}

# =============================================================================
# USAGE
# =============================================================================

usage() {
    cat <<USAGE
RillCoin DigitalOcean Testnet Provisioner

USAGE:
    ./infra/do-testnet.sh <subcommand> [args]

SUBCOMMANDS:
    deploy        Full deploy: VPC + firewall + droplets (builds from source)
    status        Show droplet status, IPs, and access commands
    ssh [N]       SSH into node N (default 0)
    tunnel        Print SSH tunnel commands for RPC access
    logs [N]      Tail build/node logs on node N (default 0)
    stop          Power off droplets (WARNING: still billed!)
    start         Power on all droplets
    teardown      Delete EVERYTHING (stops all billing)
    cost          Print estimated monthly cost

NO DOCKER REQUIRED. Builds Rust from source on each droplet.

PREREQUISITES:
    doctl auth init    # Authenticate with your DO API token
    SSH key on DO      # Upload via DO dashboard or doctl

COST: \$24/mo (2 droplets x \$12). Teardown to stop billing.

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
            MY_IP="${MY_IP:-}"
            status
            ;;
        ssh)
            ssh_node "${1:-0}"
            ;;
        tunnel)
            MY_IP="${MY_IP:-}"
            setup_ssh_tunnel
            ;;
        logs)
            show_logs "${1:-0}"
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
