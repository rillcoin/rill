#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

errors=0

# Check we're in marketing directory
if [[ "$PWD" != *"/rill/marketing" ]]; then
  echo -e "${RED}❌ Not in rill/marketing directory${NC}"
  ((errors++))
else
  echo -e "${GREEN}✓ In rill/marketing workspace${NC}"
fi

# Check marketing environment variables
if [[ -z "$RILL_MKT_ROOT" ]]; then
  echo -e "${RED}❌ RILL_MKT_ROOT not set${NC}"
  ((errors++))
else
  echo -e "${GREEN}✓ Marketing environment loaded${NC}"
fi

# Check brand tokens are available
if [[ -z "$RILL_COLOR_NAVY" ]] || [[ -z "$RILL_FONT_HEADLINE" ]]; then
  echo -e "${RED}❌ Brand tokens not loaded${NC}"
  ((errors++))
else
  echo -e "${GREEN}✓ Brand tokens available${NC}"
fi

# Check API access
if [[ -z "$APIFRAME_API_KEY" ]] || [[ -z "$VECTORIZE_API_ID" ]]; then
  echo -e "${RED}❌ API credentials missing${NC}"
  ((errors++))
else
  echo -e "${GREEN}✓ API credentials configured${NC}"
fi

if [[ $errors -eq 0 ]]; then
  echo -e "${BLUE}⛓ Rill Marketing | Ready${NC}"
else
  echo -e "${RED}━━━ $errors error(s) ━━━${NC}"
  return 1
fi
