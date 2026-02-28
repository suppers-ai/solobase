---
title: "Configuration"
description: "Configure Solobase for your environment and requirements"
weight: 20
tags: ["configuration", "setup", "environment"]
---

# Configuration

Solobase is configured through a TOML file (`solobase.toml`) with optional environment variable overrides. This guide covers all available configuration options.

## Configuration File

Create a `solobase.toml` in the working directory where you run Solobase:

```toml
# solobase.toml

[server]
bind = "0.0.0.0:8090"
# log_format = "json"   # "text" (default) or "json"

[database]
type = "sqlite"           # "sqlite" or "postgres"
path = "data/solobase.db" # SQLite file path (when type = "sqlite")
# url = "postgres://user:pass@localhost:5432/solobase"  # (when type = "postgres")

[storage]
type = "local"            # "local" or "s3"
root = "data/storage"     # Local filesystem root (when type = "local")
# bucket = "my-bucket"    # S3 bucket name (when type = "s3")
# region = "us-east-1"    # S3 region
# endpoint = ""           # Custom S3 endpoint for MinIO/Tigris/R2
# prefix = ""             # S3 key prefix for tenant isolation

[auth]
jwt_secret = ""           # Auto-generated if empty (tokens won't survive restarts)
# enable_signup = true

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

Set a custom config file path with the `SOLOBASE_CONFIG` environment variable:

```bash
SOLOBASE_CONFIG=/etc/solobase/config.toml ./solobase
```

## Environment Variable Overrides

Every config option can be overridden by an environment variable. Environment variables take precedence over the TOML file.

| Environment Variable | Config Key | Default | Description |
|---------------------|------------|---------|-------------|
| `BIND_ADDR` | `server.bind` | `0.0.0.0:8090` | Address and port to listen on |
| `DB_TYPE` | `database.type` | `sqlite` | Database backend (`sqlite` or `postgres`) |
| `DATABASE_URL` | `database.url` | - | PostgreSQL connection string |
| `DB_PATH` | `database.path` | `data/solobase.db` | SQLite database file path |
| `STORAGE_TYPE` | `storage.type` | `local` | Storage backend (`local` or `s3`) |
| `STORAGE_ROOT` | `storage.root` | `data/storage` | Local storage directory |
| `S3_BUCKET` | `storage.bucket` | - | S3 bucket name |
| `S3_REGION` | `storage.region` | - | S3 region |
| `S3_ENDPOINT` | `storage.endpoint` | - | Custom S3 endpoint |
| `S3_PREFIX` | `storage.prefix` | - | S3 key prefix |
| `JWT_SECRET` | `auth.jwt_secret` | (auto) | JWT signing secret |

Example:

```bash
DB_TYPE=postgres DATABASE_URL="postgres://user:pass@localhost/solobase" ./solobase
```

## Server

```toml
[server]
bind = "0.0.0.0:8090"
log_format = "text"   # "text" or "json"
```

The `bind` address controls which interface and port Solobase listens on. Use `127.0.0.1:8090` to restrict access to localhost only.

## Database

Solobase supports two database backends.

### SQLite (Default)

Zero-configuration, perfect for development and single-server deployments:

```toml
[database]
type = "sqlite"
path = "data/solobase.db"
```

SQLite uses WAL mode for concurrent read performance. The database file is created automatically.

### PostgreSQL

For production deployments requiring high concurrency or managed database services:

```toml
[database]
type = "postgres"
url = "postgres://user:password@localhost:5432/solobase?sslmode=require"
```

Or via environment variable:

```bash
DB_TYPE=postgres DATABASE_URL="postgres://user:pass@db:5432/solobase" ./solobase
```

## Storage

Solobase supports two storage backends for file uploads.

### Local Storage (Default)

Files are stored on the local filesystem:

```toml
[storage]
type = "local"
root = "data/storage"
```

### S3-Compatible Storage

For production use with AWS S3, Backblaze B2, MinIO, Cloudflare R2, or Tigris:

```toml
[storage]
type = "s3"
bucket = "my-solobase-bucket"
region = "us-east-1"
# endpoint = "https://s3.us-west-002.backblazeb2.com"  # For B2/MinIO/R2
# prefix = "tenant-1/"                                  # Optional key prefix
```

S3 credentials are read from standard AWS environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`) or instance roles.

## Authentication

```toml
[auth]
jwt_secret = "your-secret-key-here"
# enable_signup = true
```

If `jwt_secret` is left empty, Solobase generates a random secret on startup. This means JWT tokens will not survive restarts -- set a persistent secret for production.

Generate a secure secret:

```bash
openssl rand -hex 32
```

## Features

Toggle features on or off:

```toml
[features]
auth = true        # Authentication system
admin = true       # Admin dashboard
files = true       # File storage and management
products = true    # Product catalog
monitoring = true  # System monitoring
legalpages = true  # Legal pages (privacy, terms)
profile = true     # User profiles
system = true      # System settings
userportal = true  # User-facing portal
web = true         # Web frontend
```

Disabled features are excluded from the API and admin dashboard.

## Configuration Examples

### Minimal (Development)

```toml
[server]
bind = "127.0.0.1:8090"
```

Everything else uses defaults: SQLite database, local storage, auto-generated JWT secret.

### Production with PostgreSQL and S3

```toml
[server]
bind = "0.0.0.0:8090"
log_format = "json"

[database]
type = "postgres"
url = "postgres://solobase:${DB_PASSWORD}@db:5432/solobase?sslmode=require"

[storage]
type = "s3"
bucket = "solobase-files"
region = "us-east-1"

[auth]
jwt_secret = "your-256-bit-hex-secret"
```

### Environment Variables Only

You can skip the TOML file entirely and use only environment variables:

```bash
BIND_ADDR=0.0.0.0:8090 \
DB_TYPE=postgres \
DATABASE_URL="postgres://user:pass@db:5432/solobase" \
STORAGE_TYPE=s3 \
S3_BUCKET=my-bucket \
S3_REGION=us-east-1 \
JWT_SECRET=my-secret \
./solobase
```

## Next Steps

- [Quick Start Guide](/docs/quick-start/) - Get your first project running
- [Dashboard Overview](/docs/dashboard/) - Learn about the admin interface
- [Docker Deployment](/docs/deployment/docker/) - Deploy to production
