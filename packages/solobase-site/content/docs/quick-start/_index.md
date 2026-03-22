---
title: "Quick Start Guide"
description: "Get up and running with Solobase in minutes"
weight: 30
tags: ["quick-start", "tutorial", "getting-started"]
---

# Quick Start Guide

This guide will have Solobase running on your machine in under five minutes.

## Prerequisites

- A terminal
- A web browser

No runtime dependencies are required -- Solobase is a single binary.

## Step 1: Download Solobase

```bash
curl -fsSL https://solobase.dev/install.sh | sh
```

Or download the binary directly from [GitHub Releases](https://github.com/suppers-ai/solobase/releases/latest) and extract it.

## Step 2: Create a Configuration File

Create a `solobase.toml` in your working directory:

```toml
[server]
bind = "0.0.0.0:8090"

[database]
type = "sqlite"
path = "data/solobase.db"

[storage]
type = "local"
root = "data/storage"

[auth]
jwt_secret = "change-me-to-a-random-secret"

[features]
auth = true
admin = true
files = true
products = true
monitoring = true
legalpages = true
profile = true
system = true
userportal = true
web = true
```

## Step 3: Run Solobase

```bash
./solobase
```

You should see:

```
Solobase v0.1.0
Listening on 0.0.0.0:8090
Database: SQLite (data/solobase.db)
Storage: local (data/storage)
Admin panel: http://localhost:8090/admin
```

## Step 4: Access the Admin Dashboard

Open [http://localhost:8090/admin](http://localhost:8090/admin) in your browser.

Log in with the default admin credentials:

- **Email**: `admin@solobase.local`
- **Password**: `solobase`

> Change the default password after your first login.

## Step 5: Explore the Dashboard

The admin dashboard gives you access to:

- **System overview** -- stats, recent activity, health
- **User management** -- create and manage users, set roles
- **Database browser** -- browse tables, view and edit records
- **File storage** -- upload, organize, and manage files
- **Monitoring** -- system metrics and logs

## Step 6: Create a Record via API

Solobase exposes a REST API. Try creating a record:

```bash
# Authenticate and get a token
TOKEN=$(curl -s -X POST http://localhost:8090/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@solobase.local","password":"solobase"}' \
  | jq -r '.token')

# List collections
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8090/api/collections

# Create a record in a collection
curl -X POST http://localhost:8090/api/collections/products/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Laptop","price":1299.99,"description":"High-performance laptop"}'

# Read back the records
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8090/api/collections/products/records
```

## Step 7: Upload a File

```bash
curl -X POST http://localhost:8090/api/storage/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@photo.jpg"
```

## Using Docker Instead

If you prefer Docker:

```bash
docker run -d \
  --name solobase \
  -p 8090:8090 \
  -v solobase_data:/data \
  solobase/solobase
```

Then visit [http://localhost:8090/admin](http://localhost:8090/admin).

## What's Next?

- [Configuration](/docs/configuration/) -- Customize your TOML config, use PostgreSQL or S3
- [Docker Deployment](/docs/deployment/docker/) -- Production deployment with Docker Compose
- [Solobase Cloud](/docs/cloud/) -- Managed hosting with zero ops
- [WASM Blocks](/docs/wasm/) -- Extend Solobase with WebAssembly

## Getting Help

- [GitHub Issues](https://github.com/suppers-ai/solobase/issues)
- [Discord Community](https://discord.gg/jKqMcbrVzm)
