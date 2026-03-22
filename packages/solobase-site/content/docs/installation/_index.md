---
title: "Installation"
description: "Download and install Solobase on your system"
weight: 10
tags: ["installation", "setup", "getting-started"]
---

# Installing Solobase

Solobase is a self-contained Rust binary with no runtime dependencies. Download it, run it, and you're ready to go.

## Quick Install

The fastest way to get started:

```bash
curl -fsSL https://solobase.dev/install.sh | sh
```

This downloads the latest release for your platform and places it in your PATH.

## Pre-built Binaries

Download the latest release for your platform:

| Platform | Architecture | Download |
|----------|-------------|----------|
| Linux | x86_64 | [solobase-linux-amd64.tar.gz](https://github.com/suppers-ai/solobase/releases/latest) |
| Linux | ARM64 | [solobase-linux-arm64.tar.gz](https://github.com/suppers-ai/solobase/releases/latest) |
| macOS | Apple Silicon | [solobase-darwin-arm64.tar.gz](https://github.com/suppers-ai/solobase/releases/latest) |
| macOS | Intel | [solobase-darwin-amd64.tar.gz](https://github.com/suppers-ai/solobase/releases/latest) |
| Windows | x86_64 | [solobase-windows-amd64.zip](https://github.com/suppers-ai/solobase/releases/latest) |

Extract and run:

```bash
tar xzf solobase-linux-amd64.tar.gz
chmod +x solobase
./solobase
```

## Docker

Run Solobase in a container:

```bash
docker run -d \
  --name solobase \
  -p 8090:8090 \
  -v solobase_data:/data \
  solobase/solobase
```

See the [Docker deployment guide](/docs/deployment/docker/) for production setups.

## Build from Source

If you want to build Solobase yourself:

### Prerequisites

- [Rust](https://rustup.rs/) 1.75 or later
- Git

### Clone and Build

```bash
git clone https://github.com/suppers-ai/solobase.git
cd solobase
cargo build --release
```

The binary will be at `target/release/solobase`.

```bash
./target/release/solobase
```

## System Requirements

- **Operating System**: Linux, macOS, or Windows
- **Memory**: 512MB RAM minimum
- **Disk**: 50MB for the binary + space for your data
- **Runtime Dependencies**: None (statically linked)

## Verification

After installing, verify Solobase is working:

```bash
./solobase
```

You should see output similar to:

```
Solobase v0.1.0
Listening on 0.0.0.0:8090
Database: SQLite (data/solobase.db)
Storage: local (data/storage)
Admin panel: http://localhost:8090/admin
```

Visit [http://localhost:8090/admin](http://localhost:8090/admin) to access the admin panel.

## Managed Hosting

Don't want to self-host? [Solobase Cloud](/docs/cloud/) provides fully managed instances with automatic scaling, backups, and custom domains.

## Next Steps

1. [Configure your instance](/docs/configuration/) with a `solobase.toml` file
2. Follow the [Quick Start Guide](/docs/quick-start/) to create your first project
3. Explore the [Dashboard](/docs/dashboard/) to understand the interface

## Troubleshooting

### Permission Denied

```bash
chmod +x solobase
```

### Port Already in Use

Change the bind address in your `solobase.toml`:

```toml
[server]
bind = "0.0.0.0:8091"
```

Or set the environment variable:

```bash
BIND_ADDR=0.0.0.0:8091 ./solobase
```

## Support

- Search existing [GitHub Issues](https://github.com/suppers-ai/solobase/issues)
- Join our [Discord Community](https://discord.gg/jKqMcbrVzm)
