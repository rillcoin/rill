#!/bin/bash
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'; NC='\033[0m'
SILENT=false; [[ "$1" == "--silent" ]] && SILENT=true
ERRORS=0

if [[ "$PROJECT" != "rill" ]]; then
  $SILENT || echo -e "${RED}❌ PROJECT='$PROJECT', expected 'rill'. cd into rill directory.${NC}"
  ERRORS=$((ERRORS + 1))
fi

if [[ ! -f "Cargo.toml" ]] || ! grep -q 'rill-core' "Cargo.toml" 2>/dev/null; then
  $SILENT || echo -e "${RED}❌ Not in Rill project root${NC}"
  ERRORS=$((ERRORS + 1))
fi

if [[ -n "$R2_BUCKET_AUDIO" ]] || [[ -n "$CLOUDFLARE_API_TOKEN" ]] || [[ -n "$AWS_ACCESS_KEY_ID" ]]; then
  $SILENT || echo -e "${RED}❌ Subtone credentials detected!${NC}"
  ERRORS=$((ERRORS + 1))
fi

if [[ -n "$POLAR_ACCESS_TOKEN" ]] || [[ -n "$RESEND_API_KEY" ]]; then
  $SILENT || echo -e "${RED}❌ Renewly credentials detected!${NC}"
  ERRORS=$((ERRORS + 1))
fi

if [[ -n "$NEXT_PUBLIC_SUPABASE_URL" ]]; then
  $SILENT || echo -e "${RED}❌ Supabase credentials detected — Rill doesn't use Supabase${NC}"
  ERRORS=$((ERRORS + 1))
fi

if [[ $ERRORS -gt 0 ]]; then
  $SILENT || echo -e "${RED}━━━ $ERRORS isolation error(s) ━━━${NC}"
  exit 1
fi

if ! $SILENT; then
  RUST_INFO=""; command -v rustc &>/dev/null && RUST_INFO=" | Rust $(rustc --version | awk '{print $2}')"
  echo -e "${GREEN}⛓ Rill verified | Cargo workspace${RUST_INFO}${NC}"
fi
exit 0
