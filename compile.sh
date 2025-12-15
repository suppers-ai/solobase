#!/bin/bash

# Exit on error
set -e

# Trap errors and provide meaningful messages
trap 'echo "Error: Build failed on line $LINENO" >&2; exit 1' ERR

echo "Building Solobase..."

# Check if Go is installed
if ! command -v go &> /dev/null; then
    echo "Error: Go is not installed or not in PATH" >&2
    exit 1
fi

# Display Go version
echo "Using Go version: $(go version)"

# Build frontend
echo "Building frontend..."
if [ -d "frontend" ]; then
    cd frontend
    if [ ! -d "node_modules" ]; then
        echo "Installing frontend dependencies..."
        npm install
    fi
    npm run build
    cd ..
    echo "✅ Frontend build completed"
else
    echo "Warning: frontend directory not found, skipping frontend build" >&2
fi

# Build Go binary
echo "Building Go binary..."
if go build -o solobase ./cmd/solobase; then
    echo "✅ Build completed successfully"
    echo "Binary created: ./solobase"
else
    echo "❌ Build failed" >&2
    exit 1
fi