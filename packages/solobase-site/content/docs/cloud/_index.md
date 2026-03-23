---
title: "Solobase Cloud"
description: "Managed Solobase hosting on Cloudflare's global edge network"
weight: 20
tags: ["cloud", "hosting", "managed", "cloudflare"]
---

# Solobase Cloud

Solobase Cloud is fully managed hosting on Cloudflare's global edge network. Get a production-ready backend in seconds — no servers, no infrastructure, no ops.

## How It Works

Solobase Cloud runs on Cloudflare Workers with D1 (SQLite at the edge) for database and R2 for file storage. Each project gets:

- A dedicated subdomain (`{name}.solobase.dev`)
- D1 database (SQLite, globally replicated)
- R2 file storage (S3-compatible)
- Automatic TLS certificates
- Always-on serverless — no cold starts, no sleep

## Getting Started

### 1. Sign Up

Visit [solobase.dev](https://solobase.dev) and create an account.

### 2. Choose a Plan

Go to [Pricing](https://solobase.dev/pricing/) and pick a plan:

| Feature | Starter ($5/mo) | Pro ($25/mo) |
|---------|-----------------|--------------|
| Projects | 2 | Unlimited |
| API Requests | 500K/month | 3M/month |
| Database (D1) | 500 MB | 5 GB |
| File Storage (R2) | 2 GB | 20 GB |
| Custom Domain | No | Yes |
| Support | Community | Priority email |

### 3. Create a Project

After payment, create your project from the [dashboard](https://cloud.solobase.dev/blocks/dashboard/). Choose a name — your project will be available at:

```
https://{name}.solobase.dev
```

### 4. Access Your Project

- **Admin panel:** `https://{name}.solobase.dev/blocks/admin/`
- **API:** `https://{name}.solobase.dev/api/...`
- **Dashboard:** `https://cloud.solobase.dev/blocks/dashboard/`

## API Access

Your cloud project exposes the same REST API as a self-hosted Solobase:

```bash
# Sign up / login
TOKEN=$(curl -s -X POST https://{name}.solobase.dev/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"you@example.com","password":"yourpassword"}' \
  | jq -r '.access_token')

# Use the API
curl -H "Authorization: Bearer $TOKEN" \
  https://{name}.solobase.dev/api/admin/users
```

## Custom Domains (Pro)

Pro plan projects support custom domains:

1. Go to your project settings in the admin panel
2. Add your custom domain (e.g., `api.example.com`)
3. Add a CNAME record at your DNS provider:
   ```
   api.example.com  CNAME  {name}.solobase.dev
   ```
4. TLS certificate is provisioned automatically

## Usage & Limits

Your dashboard shows real-time usage:

- **API Requests** — counted per HTTP request to API endpoints (static files don't count)
- **Database Storage** — total D1 storage used
- **File Storage** — total R2 storage used

At 80% usage, you'll see a warning. At 100%, API requests return `429 Too Many Requests`. Add more capacity with add-ons:

| Add-on | Price |
|--------|-------|
| +100K API requests | $1/month |
| +1 GB file storage | $1/month |
| +1 GB database storage | $3/month |

## Billing

- Managed via Stripe
- Upgrade or downgrade anytime (prorated)
- If payment fails, you get a 7-day grace period before service is suspended
- 14-day money-back guarantee
- Manage your subscription from the [dashboard](https://cloud.solobase.dev/blocks/dashboard/#settings)

## Self-Hosting vs Cloud

| | Self-Hosted | Solobase Cloud |
|---|-----------|----------------|
| Infrastructure | You manage | Cloudflare edge |
| Database | SQLite or PostgreSQL | D1 (SQLite) |
| Storage | Local or S3 | R2 |
| Setup time | Minutes | Seconds |
| Maintenance | You | Us |
| Cost | Server costs | $5-25/month |
| Custom WASM blocks | Yes | Coming soon |

Choose **self-hosted** if you need full control or have compliance requirements. Choose **Cloud** for zero-ops deployment.

## Support

- **Starter:** Community support via [Discord](https://discord.gg/jKqMcbrVzm)
- **Pro:** Priority email support
- **Enterprise:** [Contact us](mailto:enterprise@solobase.dev)
