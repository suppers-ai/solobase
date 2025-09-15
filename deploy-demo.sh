#!/bin/bash

# Script to build and deploy the read-only demo to Fly.io
set -e

echo "==================================="
echo "Solobase Read-Only Demo Deployment"
echo "==================================="

# Check if we're in the right directory
if [ ! -f "go.mod" ]; then
    echo "Error: Must run this script from the Solobase root directory"
    exit 1
fi

# Always use the same app name
APP_NAME="solobase-demo"

# Step 1: Create the demo database
echo ""
echo "Step 1: Creating demo database..."
echo "---------------------------------"
./demo/setup-demo-db.sh

if [ ! -f "demo/deployment/demo.db" ]; then
    echo "Error: Failed to create demo database"
    exit 1
fi

echo ""
echo "Step 2: Getting version info..."
echo "-------------------------------"
# Get current git hash for version display
GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "demo")
echo "Version: $GIT_HASH"

echo ""
echo "Step 3: Deploying to Fly.io..."
echo "-------------------------------"

# Deploy from project root with proper context
# This allows Dockerfile to access both the project files and demo.db
fly deploy \
    --app $APP_NAME \
    --config demo/deployment/fly.toml \
    --dockerfile demo/deployment/Dockerfile \
    --build-arg BUILD_VERSION=$GIT_HASH \
    --no-cache

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