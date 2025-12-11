#!/bin/bash

# Exit on error
set -e

# Trap errors and provide meaningful messages
trap 'echo "Error: Build failed on line $LINENO" >&2; exit 1' ERR

# Build the application
echo "Building Solobase..."

# Check if Go is installed
if ! command -v go &> /dev/null; then
    echo "Error: Go is not installed or not in PATH" >&2
    exit 1
fi

# Display Go version
echo "Using Go version: $(go version)"

# Build with error checking
if go build -o solobase ./cmd/solobase; then
    echo "✅ Build completed successfully"
    echo "Binary created: ./solobase"
else
    echo "❌ Build failed" >&2
    exit 1
fi