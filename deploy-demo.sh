#!/bin/bash

# Simple deployment script for Solobase demo
set -e

echo "Deploying Solobase Demo..."

# Always use the same app name
APP_NAME="solobase-demo"

# Deploy with no cache to ensure fresh build
fly deploy \
    --app $APP_NAME \
    --dockerfile demo/deployment/Dockerfile \
    --config demo/deployment/fly.toml \
    --no-cache

echo "Deployment complete!"
echo "URL: https://$APP_NAME.fly.dev"
echo "Login: admin@solobase.demo / admin123"