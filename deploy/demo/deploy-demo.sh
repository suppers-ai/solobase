#!/bin/bash

# Script to build and deploy the read-only demo to Fly.io
# Run this from the solobase-demo directory
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "==================================="
echo "Solobase Read-Only Demo Deployment"
echo "==================================="

# Check that the solobase repo exists alongside this repo
SOLOBASE_DIR="${SOLOBASE_DIR:-$(cd .. && pwd)/solobase}"
if [ ! -f "$SOLOBASE_DIR/Cargo.toml" ]; then
    echo "Error: Cannot find solobase repo at $SOLOBASE_DIR"
    echo "Set SOLOBASE_DIR to the path of your solobase checkout"
    exit 1
fi

# Always use the same app name
APP_NAME="solobase-demo"

# Step 1: Create the demo database
echo ""
echo "Step 1: Creating demo database..."
echo "---------------------------------"
./setup-demo-db.sh

if [ ! -f "$SCRIPT_DIR/demo.db" ]; then
    echo "Error: Failed to create demo database"
    exit 1
fi

echo ""
echo "Step 2: Getting version info..."
echo "-------------------------------"
# Get current git hash for version display
GIT_HASH=$(cd "$SOLOBASE_DIR" && git rev-parse --short HEAD 2>/dev/null || echo "demo")
echo "Version: $GIT_HASH"

echo ""
echo "Step 3: Setting secrets..."
echo "--------------------------"
# Set JWT_SECRET if not already set (check if it exists first)
if ! fly secrets list --app $APP_NAME 2>/dev/null | grep -q "JWT_SECRET"; then
    echo "Setting JWT_SECRET..."
    fly secrets set JWT_SECRET="$(openssl rand -base64 32)" --app $APP_NAME
else
    echo "JWT_SECRET already set"
fi

echo ""
echo "Step 4: Deploying to Fly.io..."
echo "-------------------------------"

# Copy demo.db into solobase dir temporarily for Docker build context
cp "$SCRIPT_DIR/demo.db" "$SOLOBASE_DIR/demo.db"

# Deploy from solobase root so Dockerfile has access to the full source
fly deploy \
    --app $APP_NAME \
    --config "$SCRIPT_DIR/fly.toml" \
    --dockerfile "$SCRIPT_DIR/Dockerfile" \
    --build-arg BUILD_VERSION=$GIT_HASH \
    --no-cache \
    "$SOLOBASE_DIR"

# Clean up temporary demo.db copy
rm -f "$SOLOBASE_DIR/demo.db"

echo ""
echo "==================================="
echo "Deployment complete!"
echo ""
echo "Demo URL: https://$APP_NAME.fly.dev"
echo "Login: admin@example.com / admin123"
echo ""
echo "Security features enabled:"
echo "  ✓ Read-only database mode"
echo "  ✓ HTTP write operations blocked"
echo "  ✓ Rate limiting (60 req/min)"
echo "  ✓ Security headers (CSP, X-Frame-Options, etc.)"
echo "===================================="
