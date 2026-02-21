#!/usr/bin/env bash
# =============================================================================
# deploy-landing.sh — Build and deploy the RillCoin landing page to node0
# =============================================================================
#
# Usage:
#   ./infra/deploy-landing.sh                    # auto-detect node0 IP via doctl
#   ./infra/deploy-landing.sh 206.189.202.181    # explicit IP
#
# What this does:
#   1. Builds the Next.js static site (output: export → out/)
#   2. rsync's out/ to /var/www/rillcoin on node0
#   3. Installs nginx config if not already present
#   4. Reloads nginx
# =============================================================================
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MARKETING_DIR="${REPO_ROOT}/marketing"
WEBSITE_DIR="${MARKETING_DIR}/website"
OUT_DIR="${WEBSITE_DIR}/out"
REMOTE_WEB_ROOT="/var/www/rillcoin"
ADMIN_USER="root"
TAG="rill-testnet"
DROPLET_NAME="rill-node0"

# ---------------------------------------------------------------------------
# Colours
# ---------------------------------------------------------------------------
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
info()    { echo -e "${GREEN}[landing]${NC} $*"; }
warn()    { echo -e "${YELLOW}[landing]${NC} $*"; }
error()   { echo -e "${RED}[landing]${NC} $*" >&2; exit 1; }
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

ssh_run() { ssh "${SSH_OPTS[@]}" "${SSH}" "$@"; }

# ---------------------------------------------------------------------------
# 1. Build the static site
# ---------------------------------------------------------------------------
section "Building landing page"

[[ -d "${WEBSITE_DIR}" ]] || error "Website directory not found: ${WEBSITE_DIR}"

cd "${WEBSITE_DIR}"

if [[ ! -d node_modules ]]; then
    info "Installing npm dependencies..."
    npm install
fi

info "Running next build (static export)..."
npm run build

[[ -d "${OUT_DIR}" ]] || error "Build failed — out/ directory not found."
info "Build complete. $(find "${OUT_DIR}" -type f | wc -l | tr -d ' ') files generated."

# ---------------------------------------------------------------------------
# 2. Ensure remote web root exists
# ---------------------------------------------------------------------------
section "Preparing remote web root"

ssh_run bash -s <<REMOTE
set -euo pipefail
mkdir -p ${REMOTE_WEB_ROOT}
chmod 755 ${REMOTE_WEB_ROOT}
echo "✓ ${REMOTE_WEB_ROOT} ready"
REMOTE

# ---------------------------------------------------------------------------
# 3. rsync static files
# ---------------------------------------------------------------------------
section "Deploying to ${NODE0_IP}"

rsync -avz --delete \
    -e "ssh ${SSH_OPTS[*]}" \
    "${OUT_DIR}/" \
    "${SSH}:${REMOTE_WEB_ROOT}/"

info "Files synced to ${REMOTE_WEB_ROOT}"

# ---------------------------------------------------------------------------
# 4. Install nginx config (idempotent)
# ---------------------------------------------------------------------------
section "Configuring nginx"

# Check if certbot has already configured SSL — if so, don't overwrite.
HAS_CERTBOT=$(ssh_run "grep -c 'managed by Certbot' /etc/nginx/sites-available/rill-landing 2>/dev/null || echo 0")

if [[ "${HAS_CERTBOT}" -gt 0 ]]; then
    echo "✓ nginx config already has Certbot SSL — skipping overwrite"
else
    scp "${SSH_OPTS[@]}" \
        "${REPO_ROOT}/infra/nginx-landing.conf" \
        "${SSH}:/etc/nginx/sites-available/rill-landing"
    echo "Installed base nginx config"
fi

ssh_run bash -s <<'REMOTE'
set -euo pipefail
SITES_EN="/etc/nginx/sites-enabled/rill-landing"
[[ -L "${SITES_EN}" ]] || ln -sf /etc/nginx/sites-available/rill-landing "${SITES_EN}"
nginx -t
systemctl reload nginx
echo "✓ nginx configured and reloaded"
REMOTE

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
section "Deploy complete"

info ""
info "  Live:      http://rillcoin.com  (once DNS → ${NODE0_IP})"
info "  Direct:    http://${NODE0_IP}"
info ""
info "To enable HTTPS:"
info "  ssh ${SSH}"
info "  sudo certbot --nginx -d rillcoin.com -d www.rillcoin.com"
info ""
