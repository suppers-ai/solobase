#!/bin/bash

# Build the application
echo "Building Solobase..."
go build -o solobase backend/cmd/solobase/main.go