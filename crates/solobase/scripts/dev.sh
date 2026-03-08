#!/usr/bin/env bash
#
# Local dev environment for Solobase.
# Starts the Stripe mock, builds the frontend, and runs the server.
#
# Usage:
#   ./scripts/dev.sh          # fresh DB + all services
#   ./scripts/dev.sh --keep   # keep existing DB
#
set -euo pipefail
cd "$(dirname "$0")/.."

# ── Config ──────────────────────────────────────────────────────────
ADMIN_EMAIL="${ADMIN_EMAIL:-admin@example.com}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-admin123}"
JWT_SECRET="${JWT_SECRET:-dev-secret-$(hostname)}"
SOLOBASE_PORT="${SOLOBASE_PORT:-8090}"
STRIPE_MOCK_PORT="${STRIPE_MOCK_PORT:-12111}"

# ── Colors ──────────────────────────────────────────────────────────
bold="\033[1m"
dim="\033[2m"
green="\033[32m"
cyan="\033[36m"
yellow="\033[33m"
reset="\033[0m"

# ── Cleanup on exit ─────────────────────────────────────────────────
pids=()
cleanup() {
  echo ""
  echo -e "${dim}Shutting down...${reset}"
  for pid in "${pids[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
  wait 2>/dev/null
  echo -e "${dim}Done.${reset}"
}
trap cleanup EXIT INT TERM

# ── Fresh DB? ───────────────────────────────────────────────────────
if [[ "${1:-}" != "--keep" ]]; then
  echo -e "${yellow}Clearing database for fresh start...${reset}"
  rm -f data/solobase.db data/solobase.db-shm data/solobase.db-wal
  rm -rf data/storage
  mkdir -p data
else
  echo -e "${dim}Keeping existing database.${reset}"
fi

# ── 1. Stripe mock ──────────────────────────────────────────────────
echo -e "${cyan}Starting Stripe mock on :${STRIPE_MOCK_PORT}...${reset}"
STRIPE_WEBHOOK_SECRET=whsec_test_mock_secret_for_e2e \
SOLOBASE_URL=http://127.0.0.1:${SOLOBASE_PORT} \
STRIPE_MOCK_PORT=${STRIPE_MOCK_PORT} \
  node scripts/stripe-mock.mjs &
pids+=($!)
sleep 0.5

# ── 2. Build frontend (if source exists) ───────────────────────────
if [[ -f frontend/package.json ]] && [[ ! -d frontend/build ]]; then
  echo -e "${cyan}Building frontend...${reset}"
  npm run build 2>&1 | tail -3
fi

# ── 3. Solobase server ─────────────────────────────────────────────
echo -e "${cyan}Starting Solobase on :${SOLOBASE_PORT}...${reset}"
echo ""

export STRIPE_SECRET_KEY=sk_test_mock
export STRIPE_WEBHOOK_SECRET=whsec_test_mock_secret_for_e2e
export STRIPE_API_URL=http://127.0.0.1:${STRIPE_MOCK_PORT}
export ALLOW_PRIVATE_NETWORK=true
export JWT_SECRET
export ADMIN_EMAIL
export ADMIN_PASSWORD
export RATE_LIMIT_AUTH=0
export RUST_LOG="${RUST_LOG:-info,wafer_core::blocks::cors=warn}"

cargo run --bin solobase &
pids+=($!)

# ── Wait for ready ──────────────────────────────────────────────────
echo ""
echo -e "${dim}Waiting for server...${reset}"
for i in $(seq 1 120); do
  if curl -s "http://127.0.0.1:${SOLOBASE_PORT}/health" 2>/dev/null | grep -q ok; then
    break
  fi
  sleep 1
done

if ! curl -s "http://127.0.0.1:${SOLOBASE_PORT}/health" 2>/dev/null | grep -q ok; then
  echo -e "\033[31mServer failed to start. Check logs above.\033[0m"
  exit 1
fi

# ── Ready ───────────────────────────────────────────────────────────
echo ""
echo -e "${bold}${green}══════════════════════════════════════════════════════${reset}"
echo -e "${bold}${green}  Solobase is running!${reset}"
echo -e "${bold}${green}══════════════════════════════════════════════════════${reset}"
echo ""
echo -e "  ${bold}App:${reset}           http://127.0.0.1:${SOLOBASE_PORT}"
echo -e "  ${bold}Dashboard:${reset}     http://127.0.0.1:${SOLOBASE_PORT}/blocks/dashboard/frontend/"
echo -e "  ${bold}Admin:${reset}         http://127.0.0.1:${SOLOBASE_PORT}/blocks/admin/frontend/"
echo -e "  ${bold}Stripe mock:${reset}   http://127.0.0.1:${STRIPE_MOCK_PORT}"
echo ""
echo -e "  ${bold}Admin login:${reset}   ${ADMIN_EMAIL} / ${ADMIN_PASSWORD}"
echo ""
echo -e "  ${dim}Stripe checkout pages will appear at :${STRIPE_MOCK_PORT}/checkout/:id${reset}"
echo -e "  ${dim}Press Ctrl+C to stop everything.${reset}"
echo ""

# ── Keep alive ──────────────────────────────────────────────────────
wait
