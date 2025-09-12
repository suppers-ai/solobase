#!/bin/bash
cd /home/joris/Projects/suppers-ai/builder/go/solobase

# Generate extension registrations
echo "Discovering extensions..."
go run tools/generate-extensions.go

# Build the application
echo "Building Solobase..."
go build -o solobase cmd/solobase/main.go 2>&1