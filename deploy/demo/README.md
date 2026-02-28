# Solobase Demo

Demo deployment configurations for [Solobase](https://github.com/suppers-ai/solobase). Deploys a read-only demo instance to Fly.io.

## Structure

```
solobase-demo/
├── Dockerfile          # Docker image for demo deployment
├── fly.toml            # Fly.io configuration
├── deploy-demo.sh      # One-command deploy script
├── setup-demo-db.sh    # Creates pre-populated demo database
├── DEPLOY.md           # Detailed deployment instructions
├── SECURITY.md         # Security configuration docs
└── IAM.md              # IAM & demo features guide
```

## Quick Start

Requires the `solobase` repo checked out alongside this repo:

```
workspace/
├── solobase/          # Solobase source
├── solobase-demo/     # This repo
└── ...
```

Then deploy:

```bash
./deploy-demo.sh
```

Or specify a custom solobase location:

```bash
SOLOBASE_DIR=/path/to/solobase ./deploy-demo.sh
```

## What It Does

1. Builds solobase and creates a pre-populated SQLite database with sample data
2. Packages everything into a Docker image
3. Deploys to Fly.io in read-only mode

## Security Features

- Read-only database (SQLite `?mode=ro`)
- HTTP write operations blocked (POST/PUT/PATCH/DELETE)
- Rate limiting (60 req/min)
- Security headers (CSP, X-Frame-Options, HSTS, etc.)
- Auto-stop when idle

## Demo Credentials

- **Admin**: admin@example.com / admin123
- **Viewer**: viewer@solobase.demo / demo123 (read-only)

See [DEPLOY.md](DEPLOY.md) for detailed deployment instructions and [SECURITY.md](SECURITY.md) for security details.
