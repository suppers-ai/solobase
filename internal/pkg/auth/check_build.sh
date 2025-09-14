#!/bin/bash
# Script to check auth package build

echo "Checking auth package build..."
cd /home/joris/Projects/suppers-ai/builder/go/packages/auth

echo "1. Running go mod tidy..."
go mod tidy

echo "2. Checking for syntax errors..."
go vet ./...

echo "3. Attempting build..."
go build -v ./...

echo "4. Running tests..."
go test ./... -v