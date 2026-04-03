#!/usr/bin/env bash
#
# Local dev environment for Solobase.
# Starts the Stripe mock, Cloudflare Worker (control plane), builds the
# frontend, and runs the server.
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
CF_PORT="${CF_PORT:-8787}"
ADMIN_SECRET="${ADMIN_SECRET:-dev-admin-secret}"

CF_DIR="../solobase-cloudflare"

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

# ── 2. Cloudflare Worker (local control plane) ──────────────────────
HAS_CF=false
if [[ -d "$CF_DIR" && -f "$CF_DIR/wrangler.toml" ]]; then
  # Build the worker if not already built
  if [[ ! -f "$CF_DIR/build/worker/shim.mjs" ]]; then
    echo -e "${cyan}Building Cloudflare Worker (first time, may take a minute)...${reset}"
    (cd "$CF_DIR" && worker-build --release 2>&1 | tail -5)
  fi

  if [[ -f "$CF_DIR/build/worker/shim.mjs" ]]; then
    echo -e "${cyan}Starting Cloudflare Worker (control plane) on :${CF_PORT}...${reset}"

    # Write dev vars for the local worker
    cat > "$CF_DIR/.dev.vars" <<DEVVARS
JWT_SECRET=${JWT_SECRET}
ADMIN_SECRET=${ADMIN_SECRET}
DEVVARS

    # Generate dev config without [build] section (skip slow wasm rebuild)
    sed '/^\[build\]/,/^$/d' "$CF_DIR/wrangler.toml" > "$CF_DIR/.wrangler-dev.toml"

    pushd "$CF_DIR" > /dev/null
    WRANGLER_SEND_METRICS=false npx wrangler dev \
      --config .wrangler-dev.toml \
      --port "$CF_PORT" \
      --log-level warn &
    pids+=($!)
    popd > /dev/null
    HAS_CF=true

    # Wait for worker to be ready
    echo -e "${dim}Waiting for Cloudflare Worker...${reset}"
    for i in $(seq 1 60); do
      if curl -s -H "X-Admin-Secret: ${ADMIN_SECRET}" \
        "http://127.0.0.1:${CF_PORT}/_control/health" 2>/dev/null | grep -q ok; then
        break
      fi
      sleep 1
    done

    if ! curl -s -H "X-Admin-Secret: ${ADMIN_SECRET}" \
      "http://127.0.0.1:${CF_PORT}/_control/health" 2>/dev/null | grep -q ok; then
      echo -e "${yellow}Warning: Cloudflare Worker did not start. Deployments will stay 'pending'.${reset}"
      HAS_CF=false
    fi
  fi
fi

# ── 3. Build frontend (if source exists) ───────────────────────────
if [[ -f frontend/package.json ]] && [[ ! -d data/storage/site ]]; then
  echo -e "${cyan}Building frontend...${reset}"
  (cd frontend && npm install --prefer-offline 2>/dev/null)
  npx vite build --config frontend/vite.config.ts 2>&1 | tail -5
fi

# ── 4. Solobase server ─────────────────────────────────────────────
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

if [[ "$HAS_CF" == "true" ]]; then
  export CONTROL_PLANE_URL="http://127.0.0.1:${CF_PORT}"
  export CONTROL_PLANE_SECRET="${ADMIN_SECRET}"
fi

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
echo -e "  ${bold}Dashboard:${reset}     http://127.0.0.1:${SOLOBASE_PORT}/b/admin/"
echo -e "  ${bold}Stripe mock:${reset}   http://127.0.0.1:${STRIPE_MOCK_PORT}"
if [[ "$HAS_CF" == "true" ]]; then
  echo -e "  ${bold}Control plane:${reset} http://127.0.0.1:${CF_PORT}"
fi
echo ""
echo -e "  ${bold}Admin login:${reset}   ${ADMIN_EMAIL} / ${ADMIN_PASSWORD}"
echo ""
if [[ "$HAS_CF" == "true" ]]; then
  echo -e "  ${dim}Deployments will provision via local Cloudflare Worker.${reset}"
else
  echo -e "  ${dim}No control plane — deployments will stay 'pending'.${reset}"
fi
echo -e "  ${dim}Press Ctrl+C to stop everything.${reset}"
echo ""

# ── Keep alive ──────────────────────────────────────────────────────
wait
