#!/usr/bin/env bash
# One-time script to provision the "cloud" user worker in the dispatch namespace.
# This uploads the built solobase-worker to the dev dispatch namespace with
# the platform D1 database binding.
#
# Prerequisites:
#   - worker-build --release (already done)
#   - CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID env vars set
#   - Dispatch namespace created
#
# Usage: ./provision-cloud.sh [dev|production]

set -euo pipefail

ENV="${1:-dev}"

if [ "$ENV" = "production" ]; then
    NAMESPACE_ID="7a2de1c6-6fbb-4f57-8af1-e054c62e38a2"
    D1_DB_ID="4e2ec8f8-fc62-440a-8406-7bc522045c12"
    R2_BUCKET="solobase-storage"
else
    NAMESPACE_ID="1567de83-433d-48b1-8d38-090323fb3d8b"
    D1_DB_ID="b2da17b4-a047-4e61-b61b-6f5b25613b7c"
    R2_BUCKET="solobase-storage-dev"
fi

SCRIPT_NAME="cloud"
ACCOUNT_ID="${CLOUDFLARE_ACCOUNT_ID:?Set CLOUDFLARE_ACCOUNT_ID}"
API_TOKEN="${CLOUDFLARE_API_TOKEN:?Set CLOUDFLARE_API_TOKEN}"

# Secrets to forward to the user worker (set these env vars or they'll be empty)
JWT_SECRET="${JWT_SECRET:-}"
ADMIN_SECRET="${ADMIN_SECRET:-}"
MAILGUN_API_KEY="${MAILGUN_API_KEY:-}"
MAILGUN_DOMAIN="${MAILGUN_DOMAIN:-}"
MAILGUN_FROM="${MAILGUN_FROM:-}"
MAILGUN_REPLY_TO="${MAILGUN_REPLY_TO:-}"

BUILD_DIR="$(dirname "$0")/build"
JS_FILE="$BUILD_DIR/index.js"
WASM_FILE="$BUILD_DIR/index_bg.wasm"

if [ ! -f "$JS_FILE" ] || [ ! -f "$WASM_FILE" ]; then
    echo "Error: Build artifacts not found. Run 'worker-build --release' first."
    exit 1
fi

echo "Uploading '$SCRIPT_NAME' to namespace $NAMESPACE_ID ($ENV)..."
echo "  D1: $D1_DB_ID"
echo "  R2: $R2_BUCKET"

# Build metadata with bindings
METADATA=$(cat <<'ENDJSON'
{
  "main_module": "index.js",
  "compatibility_date": "2026-03-01",
  "bindings": [
    {"type": "d1", "name": "DB", "id": "D1_DB_ID_PLACEHOLDER"},
    {"type": "r2_bucket", "name": "STORAGE", "bucket_name": "R2_BUCKET_PLACEHOLDER"},
    {"type": "plain_text", "name": "PROJECT_ID", "text": "cloud"},
    {"type": "plain_text", "name": "PROJECT_CONFIG", "text": "{\"version\":1,\"auth\":{},\"admin\":{},\"files\":{},\"products\":{},\"deployments\":{},\"legalpages\":{},\"userportal\":{}}"},
    {"type": "secret_text", "name": "JWT_SECRET", "text": "JWT_SECRET_PLACEHOLDER"},
    {"type": "secret_text", "name": "ADMIN_SECRET", "text": "ADMIN_SECRET_PLACEHOLDER"},
    {"type": "secret_text", "name": "MAILGUN_API_KEY", "text": "MAILGUN_API_KEY_PLACEHOLDER"},
    {"type": "secret_text", "name": "MAILGUN_DOMAIN", "text": "MAILGUN_DOMAIN_PLACEHOLDER"},
    {"type": "secret_text", "name": "MAILGUN_FROM", "text": "MAILGUN_FROM_PLACEHOLDER"},
    {"type": "secret_text", "name": "MAILGUN_REPLY_TO", "text": "MAILGUN_REPLY_TO_PLACEHOLDER"}
  ]
}
ENDJSON
)

# Replace placeholders
METADATA="${METADATA//D1_DB_ID_PLACEHOLDER/$D1_DB_ID}"
METADATA="${METADATA//R2_BUCKET_PLACEHOLDER/$R2_BUCKET}"
METADATA="${METADATA//JWT_SECRET_PLACEHOLDER/$JWT_SECRET}"
METADATA="${METADATA//ADMIN_SECRET_PLACEHOLDER/$ADMIN_SECRET}"
METADATA="${METADATA//MAILGUN_API_KEY_PLACEHOLDER/$MAILGUN_API_KEY}"
METADATA="${METADATA//MAILGUN_DOMAIN_PLACEHOLDER/$MAILGUN_DOMAIN}"
METADATA="${METADATA//MAILGUN_FROM_PLACEHOLDER/$MAILGUN_FROM}"
METADATA="${METADATA//MAILGUN_REPLY_TO_PLACEHOLDER/$MAILGUN_REPLY_TO}"

# Upload via multipart form
RESPONSE=$(curl -s -w "\n%{http_code}" \
    -X PUT \
    "https://api.cloudflare.com/client/v4/accounts/$ACCOUNT_ID/workers/dispatch/namespaces/$NAMESPACE_ID/scripts/$SCRIPT_NAME" \
    -H "Authorization: Bearer $API_TOKEN" \
    -F "metadata=$METADATA;type=application/json" \
    -F "index.js=@$JS_FILE;type=application/javascript+module" \
    -F "index_bg.wasm=@$WASM_FILE;type=application/wasm")

HTTP_CODE=$(echo "$RESPONSE" | tail -1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [ "$HTTP_CODE" -ge 200 ] && [ "$HTTP_CODE" -lt 300 ]; then
    echo "Successfully uploaded '$SCRIPT_NAME' worker ($ENV)"
else
    echo "Error (HTTP $HTTP_CODE):"
    echo "$BODY" | python3 -m json.tool 2>/dev/null || echo "$BODY"
    exit 1
fi
