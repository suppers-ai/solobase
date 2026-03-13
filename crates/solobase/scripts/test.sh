#!/usr/bin/env bash
#
# Run E2E tests against a local Solobase instance.
# Starts everything, runs tests, then tears down.
#
# Usage:
#   ./scripts/test.sh                    # run all tests
#   ./scripts/test.sh stripe-payment     # run only stripe tests
#   ./scripts/test.sh new-developer      # run only click-through tests
#   ./scripts/test.sh --headed           # run with visible browser
#
set -euo pipefail
cd "$(dirname "$0")/.."

# ── Parse args ──────────────────────────────────────────────────────
PW_ARGS=()
SPEC_FILTER=""

for arg in "$@"; do
  case "$arg" in
    --headed|--debug|--ui)
      PW_ARGS+=("$arg")
      ;;
    -*)
      PW_ARGS+=("$arg")
      ;;
    *)
      SPEC_FILTER="$arg"
      ;;
  esac
done

if [[ -n "$SPEC_FILTER" ]]; then
  # Check top-level first, then subdirectories
  if [[ -f "tests/e2e/${SPEC_FILTER}.spec.ts" ]]; then
    PW_ARGS+=("tests/e2e/${SPEC_FILTER}.spec.ts")
  elif found=$(find tests/e2e -name "${SPEC_FILTER}.spec.ts" -print -quit 2>/dev/null) && [[ -n "$found" ]]; then
    PW_ARGS+=("$found")
  else
    # Fall back to grep filter (matches test describe names too)
    PW_ARGS+=("--grep" "$SPEC_FILTER")
  fi
fi

# ── Colors ──────────────────────────────────────────────────────────
dim="\033[2m"
cyan="\033[36m"
green="\033[32m"
red="\033[31m"
reset="\033[0m"

# ── Cleanup on exit ─────────────────────────────────────────────────
pids=()
cleanup() {
  for pid in "${pids[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
  wait 2>/dev/null
}
trap cleanup EXIT INT TERM

# ── Check if server is already running ──────────────────────────────
if curl -s http://127.0.0.1:8090/health 2>/dev/null | grep -q ok; then
  echo -e "${cyan}Server already running on :8090 — running tests directly.${reset}"
  echo ""
  exec ./node_modules/.bin/playwright test "${PW_ARGS[@]}"
fi

# ── Start fresh ─────────────────────────────────────────────────────
echo -e "${cyan}Starting test environment...${reset}"

rm -f data/solobase.db data/solobase.db-shm data/solobase.db-wal
rm -rf data/storage
mkdir -p data

# Stripe mock
node scripts/stripe-mock.mjs > /dev/null 2>&1 &
pids+=($!)
sleep 0.5

# Solobase
STRIPE_SECRET_KEY=sk_test_mock \
STRIPE_WEBHOOK_SECRET=whsec_test_mock_secret_for_e2e \
STRIPE_API_URL=http://127.0.0.1:12111 \
ALLOW_PRIVATE_NETWORK=true \
JWT_SECRET=test-secret-key-for-e2e \
ADMIN_EMAIL=admin@e2e.test \
ADMIN_PASSWORD=AdminE2EPass1234 \
RATE_LIMIT_AUTH=0 \
RATE_LIMIT_API_READ=0 \
RATE_LIMIT_API_WRITE=0 \
RATE_LIMIT_IP=0 \
RUST_LOG=warn \
  cargo run --bin solobase > /tmp/solobase-test.log 2>&1 &
pids+=($!)

# Wait for ready
echo -e "${dim}Waiting for server...${reset}"
for i in $(seq 1 120); do
  if curl -s http://127.0.0.1:8090/health 2>/dev/null | grep -q ok; then
    break
  fi
  sleep 1
done

if ! curl -s http://127.0.0.1:8090/health 2>/dev/null | grep -q ok; then
  echo -e "${red}Server failed to start. Logs:${reset}"
  tail -20 /tmp/solobase-test.log
  exit 1
fi

echo -e "${green}Server ready. Running tests...${reset}"
echo ""

# ── Run tests ───────────────────────────────────────────────────────
./node_modules/.bin/playwright test "${PW_ARGS[@]}"
