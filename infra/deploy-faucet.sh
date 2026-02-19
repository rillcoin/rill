#!/usr/bin/env bash
# =============================================================================
# deploy-faucet.sh — Deploy rill-faucet to the testnet node0 droplet
# =============================================================================
#
# Usage:
#   ./infra/deploy-faucet.sh               # auto-detect node0 IP via doctl
#   ./infra/deploy-faucet.sh 206.189.202.181   # explicit IP
#
# What this does (idempotent — safe to re-run for updates):
#   1. Resolves node0 public IP (via doctl or arg)
#   2. Pulls latest code + builds rill-faucet on the droplet
#   3. Installs binary to /usr/local/bin
#   4. Creates rill user, /var/lib/rill, /etc/rill dirs
#   5. Installs systemd unit + nginx config
#   6. First run: prompts for wallet password, creates env file
#   7. Creates a faucet wallet if none exists
#   8. (Re)starts rill-faucet via systemd
#
# Prerequisites on your local machine:
#   - doctl authenticated (or pass IP explicitly)
#   - SSH key for node0 in your agent / ~/.ssh
#   - rill-cli available locally (for wallet creation step)
#
# =============================================================================
set -euo pipefail

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ADMIN_USER="root"
TAG="rill-testnet"
DROPLET_NAME="rill-node0"
REMOTE_REPO_DIR="/opt/rill"
BINARY_DEST="/usr/local/bin/rill-faucet"
WALLET_PATH="/var/lib/rill/faucet.dat"
ENV_FILE="/etc/rill/faucet.env"
SERVICE_NAME="rill-faucet"
FAUCET_PORT="8080"
FAUCET_BIND="127.0.0.1:${FAUCET_PORT}"
RPC_ENDPOINT="http://127.0.0.1:18332"
AMOUNT_RILL="10"
COOLDOWN_SECS="86400"

# ---------------------------------------------------------------------------
# Colours
# ---------------------------------------------------------------------------
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
info()    { echo -e "${GREEN}[faucet]${NC} $*"; }
warn()    { echo -e "${YELLOW}[faucet]${NC} $*"; }
error()   { echo -e "${RED}[faucet]${NC} $*" >&2; exit 1; }
section() { echo -e "\n${CYAN}══ $* ══${NC}"; }

# ---------------------------------------------------------------------------
# Resolve node0 IP
# ---------------------------------------------------------------------------
section "Resolving node0 IP"

if [[ $# -ge 1 ]]; then
    NODE0_IP="$1"
    info "Using explicit IP: ${NODE0_IP}"
else
    if ! command -v doctl &>/dev/null; then
        error "doctl not found. Install it or pass the node0 IP as an argument."
    fi
    NODE0_IP="$(doctl compute droplet list --tag-name "${TAG}" --format Name,PublicIPv4 --no-header \
        | awk -v name="${DROPLET_NAME}" '$1 == name { print $2 }')"
    [[ -n "${NODE0_IP}" ]] || error "Could not resolve IP for ${DROPLET_NAME}. Is the testnet deployed?"
    info "Resolved ${DROPLET_NAME} → ${NODE0_IP}"
fi

SSH_OPTS=(-o StrictHostKeyChecking=no -o ConnectTimeout=10)
SSH="${ADMIN_USER}@${NODE0_IP}"

ssh_run() {
    # Run a command on node0, inheriting stdout/stderr.
    ssh "${SSH_OPTS[@]}" "${SSH}" "$@"
}

ssh_run_q() {
    # Run silently (suppress remote stdout).
    ssh "${SSH_OPTS[@]}" "${SSH}" "$@" >/dev/null
}

# ---------------------------------------------------------------------------
# 1. Pull latest source and build
# ---------------------------------------------------------------------------
section "Building rill-faucet on ${NODE0_IP}"

ssh_run bash -s <<'REMOTE'
set -euo pipefail
REPO_DIR="/opt/rill"

# Clone if not present.
if [[ ! -d "${REPO_DIR}/.git" ]]; then
    echo "→ Cloning repository..."
    git clone https://github.com/rillcoin/rill.git "${REPO_DIR}"
fi

cd "${REPO_DIR}"
echo "→ Pulling latest..."
git fetch --all
git reset --hard origin/main

# Ensure Rust toolchain is available.
if ! command -v cargo &>/dev/null; then
    echo "→ Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
fi
source "$HOME/.cargo/env" 2>/dev/null || true

echo "→ Building rill-faucet (release)..."
cargo build --release -p rill-faucet 2>&1
echo "✓ Build complete"
REMOTE

info "Build succeeded."

# ---------------------------------------------------------------------------
# 2. Install binary
# ---------------------------------------------------------------------------
section "Installing binary"

ssh_run bash -s <<REMOTE
set -euo pipefail
source "\$HOME/.cargo/env" 2>/dev/null || true
cp /opt/rill/target/release/rill-faucet ${BINARY_DEST}
chmod 755 ${BINARY_DEST}
echo "✓ Installed to ${BINARY_DEST}"
REMOTE

# ---------------------------------------------------------------------------
# 3. Create rill user and directories
# ---------------------------------------------------------------------------
section "Creating system user and directories"

ssh_run bash -s <<REMOTE
set -euo pipefail

# Create rill system user if missing (no login shell, no home).
if ! id rill &>/dev/null; then
    useradd --system --no-create-home --shell /usr/sbin/nologin rill
    echo "✓ Created rill user"
else
    echo "✓ rill user already exists"
fi

install -d -o rill -g rill -m 750 /var/lib/rill
install -d -o root -g root -m 750 /etc/rill
echo "✓ Directories ready"
REMOTE

# ---------------------------------------------------------------------------
# 4. Install systemd unit
# ---------------------------------------------------------------------------
section "Installing systemd unit"

scp "${SSH_OPTS[@]}" "${REPO_ROOT}/infra/rill-faucet.service" \
    "${SSH}:/etc/systemd/system/rill-faucet.service"

ssh_run systemctl daemon-reload
info "Systemd unit installed."

# ---------------------------------------------------------------------------
# 5. Install nginx config
# ---------------------------------------------------------------------------
section "Configuring nginx"

ssh_run bash -s <<'REMOTE'
set -euo pipefail
if command -v nginx &>/dev/null; then
    echo "✓ nginx found"
else
    echo "→ Installing nginx..."
    apt-get update -qq && apt-get install -y -q nginx
fi
REMOTE

scp "${SSH_OPTS[@]}" "${REPO_ROOT}/infra/nginx-faucet.conf" \
    "${SSH}:/etc/nginx/sites-available/rill-faucet"

ssh_run bash -s <<'REMOTE'
set -euo pipefail
SITES_EN="/etc/nginx/sites-enabled/rill-faucet"
[[ -L "${SITES_EN}" ]] || ln -sf /etc/nginx/sites-available/rill-faucet "${SITES_EN}"
nginx -t
systemctl reload nginx
echo "✓ nginx configured and reloaded"
REMOTE

# ---------------------------------------------------------------------------
# 6. Create env file (first run) or update (subsequent runs)
# ---------------------------------------------------------------------------
section "Faucet environment file"

ENV_EXISTS="$(ssh_run bash -c "test -f '${ENV_FILE}' && echo yes || echo no")"

if [[ "${ENV_EXISTS}" == "yes" ]]; then
    warn "Env file ${ENV_FILE} already exists — skipping password prompt."
    warn "To change the password: sudo nano ${ENV_FILE} on node0, then sudo systemctl restart rill-faucet"
else
    info "First deploy — setting up faucet credentials."
    echo
    read -r -s -p "  Enter faucet wallet password (will be stored in ${ENV_FILE}): " WALLET_PASS
    echo
    read -r -s -p "  Confirm password: " WALLET_PASS2
    echo
    [[ "${WALLET_PASS}" == "${WALLET_PASS2}" ]] || error "Passwords do not match."
    [[ -n "${WALLET_PASS}" ]] || error "Password must not be empty."

    # Optional Discord credentials
    echo
    read -r -p "  Discord bot token (leave blank to skip): " DISCORD_TOKEN
    read -r -p "  Discord public key hex (leave blank to skip): " DISCORD_PUBKEY
    read -r -p "  Discord application ID (leave blank to skip): " DISCORD_APP_ID

    # Write env file on remote (password never touches local disk)
    ssh_run bash -s <<REMOTE
set -euo pipefail
cat > ${ENV_FILE} <<EOF
# RillCoin Faucet — environment variables
# Managed by deploy-faucet.sh — do not commit this file.

FAUCET_WALLET_PATH=${WALLET_PATH}
FAUCET_WALLET_PASSWORD=${WALLET_PASS}
FAUCET_RPC_ENDPOINT=${RPC_ENDPOINT}
FAUCET_BIND_ADDR=${FAUCET_BIND}
FAUCET_AMOUNT_RILL=${AMOUNT_RILL}
FAUCET_COOLDOWN_SECS=${COOLDOWN_SECS}

# Discord (comment out if not using)
$([ -n "${DISCORD_TOKEN}"  ] && echo "DISCORD_BOT_TOKEN=${DISCORD_TOKEN}"        || echo "#DISCORD_BOT_TOKEN=")
$([ -n "${DISCORD_PUBKEY}" ] && echo "DISCORD_PUBLIC_KEY=${DISCORD_PUBKEY}"       || echo "#DISCORD_PUBLIC_KEY=")
$([ -n "${DISCORD_APP_ID}" ] && echo "DISCORD_APPLICATION_ID=${DISCORD_APP_ID}"   || echo "#DISCORD_APPLICATION_ID=")

RUST_LOG=info
EOF
chmod 600 ${ENV_FILE}
chown root:root ${ENV_FILE}
echo "✓ Env file written to ${ENV_FILE}"
REMOTE
fi

# ---------------------------------------------------------------------------
# 7. Create faucet wallet if none exists
# ---------------------------------------------------------------------------
section "Faucet wallet"

WALLET_EXISTS="$(ssh_run bash -c "test -f '${WALLET_PATH}' && echo yes || echo no")"

if [[ "${WALLET_EXISTS}" == "yes" ]]; then
    info "Wallet already exists at ${WALLET_PATH}."
else
    info "No wallet found — creating one on node0..."
    WALLET_PASS_REMOTE="$(ssh_run bash -c "grep ^FAUCET_WALLET_PASSWORD ${ENV_FILE} | cut -d= -f2-")"

    ssh_run bash -s <<REMOTE
set -euo pipefail
source "\$HOME/.cargo/env" 2>/dev/null || true
RILL_CLI=/opt/rill/target/release/rill-cli

if [[ ! -f "\${RILL_CLI}" ]]; then
    cargo build --release -p rill-cli -C /opt/rill 2>&1
fi

# rill-cli wallet create reads password from stdin via rpassword.
# Pipe password twice (create + confirm prompt).
printf '%s\n%s\n' '${WALLET_PASS_REMOTE}' '${WALLET_PASS_REMOTE}' \
    | "\${RILL_CLI}" wallet create --file ${WALLET_PATH} --network testnet 2>&1 || true

# Fix ownership.
chown rill:rill ${WALLET_PATH}
chmod 600 ${WALLET_PATH}
echo "✓ Wallet created at ${WALLET_PATH}"
REMOTE

    echo
    warn "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    warn " IMPORTANT: Fund the faucet wallet before starting service"
    warn "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    info "Faucet address:"
    ssh_run bash -c "source \$HOME/.cargo/env 2>/dev/null; \
        /opt/rill/target/release/rill-cli address --wallet ${WALLET_PATH} 2>/dev/null || echo '(run rill-cli address --wallet ${WALLET_PATH} on node0)'"
    echo
    info "Fund it from the miner wallet, then re-run this script or:"
    info "  sudo systemctl start rill-faucet"
fi

# ---------------------------------------------------------------------------
# 8. Enable and (re)start service
# ---------------------------------------------------------------------------
section "(Re)starting rill-faucet"

ssh_run bash -s <<REMOTE
set -euo pipefail
systemctl enable rill-faucet
systemctl restart rill-faucet
sleep 2
if systemctl is-active --quiet rill-faucet; then
    echo "✓ rill-faucet is running"
else
    echo "✗ rill-faucet failed to start — check logs:"
    journalctl -u rill-faucet -n 30 --no-pager
    exit 1
fi
REMOTE

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
section "Deploy complete"

info "Faucet running on ${NODE0_IP}"
info ""
info "  HTTP (via nginx):  http://faucet.rillcoin.com  (once DNS is set)"
info "  Direct check:      curl http://${NODE0_IP}/api/status"
info ""
info "Useful commands on node0:"
info "  journalctl -u rill-faucet -f          # live logs"
info "  systemctl status rill-faucet           # status"
info "  sudo nano /etc/rill/faucet.env         # edit config"
info "  systemctl restart rill-faucet          # apply config changes"
info ""

if command -v doctl &>/dev/null; then
    info "Enable HTTPS once DNS is pointed at ${NODE0_IP}:"
    info "  $(./infra/do-testnet.sh ssh 0) # SSH in"
    info "  sudo apt-get install -y certbot python3-certbot-nginx"
    info "  sudo certbot --nginx -d faucet.rillcoin.com"
fi
