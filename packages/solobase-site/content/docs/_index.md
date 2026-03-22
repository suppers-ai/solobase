---
title: "Documentation"
description: "Complete documentation for Solobase"
---

# Documentation

Welcome to the Solobase documentation.

## Quick Start

```bash
# Install
curl -fsSL https://solobase.dev/install.sh | sh

# Or run with Docker
docker run -p 8090:8090 solobase/solobase

# Run the binary
./solobase
```

Visit `http://localhost:8090/admin` to access your admin dashboard.

## Guides

**Getting Started**
[Quick Start](/docs/quick-start/) · [Installation](/docs/installation/) · [Configuration](/docs/configuration/)

**Core Features**
[Dashboard](/docs/dashboard/) · Authentication · Database Management · File Storage

**API Reference**
[Authentication API](/docs/api/auth/) · [Database API](/docs/api/database/)

**Deployment**
[Docker](/docs/deployment/docker/) · [Solobase Cloud](/docs/cloud/)

**Extensions**
[WASM Blocks](/docs/wasm/)

## Features

- Single binary deployment (Rust, no runtime dependencies)
- TOML configuration with environment variable overrides
- SQLite and PostgreSQL database support
- Local filesystem or S3-compatible object storage
- Built-in authentication with JWT
- RESTful API with admin dashboard
- WASM extension system for custom logic
- Managed hosting via [Solobase Cloud](/docs/cloud/)

## Support

[GitHub Issues](https://github.com/suppers-ai/solobase/issues) · [Live Demo](https://solobase-demo.fly.dev/) · [Discord Community](https://discord.gg/jKqMcbrVzm)
