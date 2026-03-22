---
title: "Solobase Cloud"
description: "Managed Solobase hosting with automatic scaling and zero ops"
weight: 20
tags: ["cloud", "hosting", "managed", "deployment"]
---

# Solobase Cloud

Solobase Cloud is fully managed hosting for Solobase. Get a production-ready backend in seconds without managing servers, databases, or infrastructure.

## What is Solobase Cloud?

Solobase Cloud runs your Solobase instances on shared infrastructure managed by the Solobase team. Each instance gets:

- A dedicated subdomain (`yoursolobase.dev`)
- Managed PostgreSQL database
- S3-compatible file storage
- Automatic TLS certificates
- Scale-to-zero for inactive instances (on applicable plans)

## Getting Started

### 1. Sign Up

Visit [solobase.dev](https://solobase.dev) and sign in with GitHub or Google.

### 2. Create an Instance

After signing in, click **Create Instance** and choose:

- **Subdomain** -- your instance will be available at `yoursolobase.dev`
- **Plan** -- select the plan that fits your needs (see [Plans](#plans) below)

### 3. Access Your Instance

Once created, your instance is immediately available at:

```
https://yoursolobase.dev
```

The admin dashboard is at:

```
https://yoursolobase.dev/admin
```

## Plans

| Feature | Free | Hobby ($5) | Starter ($15) | Professional ($79) | Business ($199) |
|---------|------|-----------|--------------|-------------------|----------------|
| Instances | 1 | 1 | 1 | 3 | 10 |
| Database | 100MB | 500MB | 5GB | 20GB | 100GB dedicated |
| Storage | 512MB | 2GB | 10GB | 50GB | 200GB |
| API Requests | 100K/mo | 1M/mo | 10M/mo | 100M/mo | Unlimited |
| Always-on | No | No | Yes | Yes | Yes |
| Custom domain | No | No | No | Yes | Yes |
| Backups | No | No | No | Daily | Real-time |
| SLA | -- | -- | -- | -- | 99.9% |

See the [Pricing page](https://solobase.dev/pricing/) for full details.

## Custom Domains

Professional and Business plans support custom domains. To set up a custom domain:

1. Go to your instance settings in the dashboard
2. Click **Custom Domain**
3. Enter your domain (e.g., `api.example.com`)
4. Add the CNAME record shown to your DNS provider:
   ```
   api.example.com  CNAME  yoursolobase.dev
   ```
5. Wait for DNS propagation (usually a few minutes)
6. Solobase Cloud automatically provisions a TLS certificate

## Scale-to-Zero

Free and Hobby plan instances automatically sleep after 15 minutes of inactivity. When a request arrives:

1. The request is held briefly while the instance wakes up
2. The instance boots in ~2 seconds
3. The request is forwarded to the running instance
4. Subsequent requests are served instantly

Starter, Professional, and Business plans are **always-on** and never sleep.

## API Access

Your cloud instance exposes the same REST API as a self-hosted Solobase:

```bash
# Authenticate
TOKEN=$(curl -s -X POST https://yoursolobase.dev/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@solobase.local","password":"solobase"}' \
  | jq -r '.token')

# Use the API
curl -H "Authorization: Bearer $TOKEN" \
  https://yoursolobase.dev/api/collections
```

## Managing Instances

From the Solobase Cloud dashboard you can:

- **Pause** an instance to stop it without deleting data
- **Resume** a paused instance
- **Delete** an instance and all its data
- **Upgrade/downgrade** your plan

## Self-Hosting vs Cloud

| | Self-Hosted | Solobase Cloud |
|---|-----------|----------------|
| Control | Full | Managed |
| Setup time | Minutes | Seconds |
| Maintenance | You | Us |
| Scaling | Manual | Automatic |
| Backups | DIY | Included (Pro+) |
| Cost | Server costs | Plan pricing |
| Custom WASM | Yes | Yes |

Choose **self-hosted** if you need full control over infrastructure or have compliance requirements. Choose **Cloud** if you want zero-ops deployment.

## Support

- Free and Hobby: Community support via [Discord](https://discord.gg/jKqMcbrVzm)
- Starter: Email support (48h response)
- Professional: Email support (24h response)
- Business: Priority support (12h response)
- Enterprise: [Contact us](mailto:sales@solobase.dev)
