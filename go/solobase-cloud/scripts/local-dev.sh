#!/usr/bin/env bash
# local-dev.sh — Launch the full Solobase stack locally.
# Starts mock-node (9090), solobase-cloud (8080), and solobase-site (5173).
# Usage: bash scripts/local-dev.sh  (from the solobase-cloud directory)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CLOUD_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SITE_DIR="$(cd "$CLOUD_DIR/../solobase-site" && pwd)"

PIDS=()

cleanup() {
    echo ""
    echo "Shutting down..."
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    wait 2>/dev/null || true
    echo "All processes stopped."
}
trap cleanup EXIT INT TERM

echo "=== Solobase Local Dev Stack ==="
echo ""

# 1. Build and start mock-node
echo "[1/3] Building mock-node..."
(cd "$CLOUD_DIR" && go build -o /tmp/mock-node ./cmd/mock-node)

echo "[1/3] Starting mock-node on :9090..."
LISTEN_ADDR=:9090 NODE_SECRET=dev-secret /tmp/mock-node &
PIDS+=($!)

# 2. Build and start solobase-cloud
echo "[2/3] Building solobase-cloud..."
(cd "$CLOUD_DIR" && go build -o /tmp/solobase-cloud ./cmd)

echo "[2/3] Starting solobase-cloud on :8080..."
NODE_0="local-dev,http://localhost:9090,dev-secret,local,127.0.0.1" \
LISTEN_ADDR=:8080 \
API_SECRET=dev-secret \
BASE_URL=http://localhost:8080 \
DEV_MODE=1 \
/tmp/solobase-cloud &
PIDS+=($!)

# 3. Start solobase-site (install deps if needed)
if [ -d "$SITE_DIR" ]; then
    if [ ! -d "$SITE_DIR/node_modules" ]; then
        echo "[3/3] Installing solobase-site dependencies..."
        (cd "$SITE_DIR" && npm install)
    fi
    echo "[3/3] Starting solobase-site on :5173..."
    (cd "$SITE_DIR" && npm run dev -- --port 5173) &
    PIDS+=($!)
else
    echo "[3/3] SKIP: solobase-site not found at $SITE_DIR"
fi

# 4. Wait for services to be healthy
echo ""
echo "Waiting for services..."

for i in $(seq 1 30); do
    if curl -sf http://localhost:9090/api/health >/dev/null 2>&1; then
        echo "  mock-node: ready"
        break
    fi
    sleep 1
done

for i in $(seq 1 30); do
    if curl -sf http://localhost:8080/api/plans >/dev/null 2>&1; then
        echo "  solobase-cloud: ready"
        break
    fi
    sleep 1
done

# 5. Print summary
echo ""
echo "=== Stack is running ==="
echo ""
echo "  Cloud dashboard:  http://localhost:8080"
echo "  Mock node:        http://localhost:9090"
if [ -d "$SITE_DIR" ]; then
echo "  Marketing site:   http://localhost:5173"
fi
echo ""
echo "--- Quick Start ---"
echo ""
echo "  # Get a dev session (sets cookie):"
echo "  curl -c /tmp/cookies.txt http://localhost:8080/api/dev/session"
echo ""
echo "  # List plans:"
echo "  curl http://localhost:8080/api/plans | jq"
echo ""
echo "  # Create a tenant:"
echo '  curl -b /tmp/cookies.txt -X POST http://localhost:8080/api/tenants \'
echo '    -H "Content-Type: application/json" \'
echo '    -d '\''{"subdomain":"myapp","plan":"hobby"}'\'''
echo ""
echo "  # List tenants:"
echo "  curl -b /tmp/cookies.txt http://localhost:8080/api/tenants | jq"
echo ""
echo "  # Run the full test flow:"
echo "  bash scripts/test-flow.sh"
echo ""
echo "Press Ctrl+C to stop all services."
echo ""

# Keep running until interrupted
wait
