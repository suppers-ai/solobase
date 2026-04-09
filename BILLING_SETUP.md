# Billing & Stripe Setup Guide

Everything needed to get the subscription, add-on, and usage enforcement system working.

## 1. Stripe Dashboard Setup

### Create Products & Prices

You need to create these in Stripe (Dashboard → Products):

**Base Plans** (recurring):

| Product | Price | Lookup Key |
|---------|-------|------------|
| Starter Plan | $X/month | `starter` |
| Pro Plan | $Y/month | `pro` |

Set the `plan` metadata or lookup_key on each price so the webhook can identify the plan.

**Add-on Items** (recurring, to be added as subscription items):

| Product | Price | Env Var for Price ID |
|---------|-------|---------------------|
| 500K Extra Requests | $5/month | `STRIPE_PRICE_ADDON_REQUESTS_500K` |
| 5GB Extra Object Storage | $3/month | `STRIPE_PRICE_ADDON_R2_5GB` |
| 1GB Extra Database Storage | $2/month | `STRIPE_PRICE_ADDON_D1_1GB` |
| 2 Extra Project Slots | $5/month | `STRIPE_PRICE_ADDON_PROJECTS_2` |

After creating each, copy the Price ID (e.g., `price_1Abc...`) and set it as the corresponding env var.

### Configure Webhook

Stripe Dashboard → Developers → Webhooks → Add Endpoint:

- **URL**: `https://cloud.solobase.dev/b/products/webhooks`
- **Events to listen for**:
  - `checkout.session.completed`
  - `customer.subscription.updated`
  - `customer.subscription.deleted`
  - `invoice.payment_failed`
  - `charge.refunded`
- Copy the **Signing Secret** → set as `STRIPE_WEBHOOK_SECRET`

## 2. Environment Variables

### Products Block (solobase / user workers)

```env
# Required
STRIPE_SECRET_KEY=sk_live_...          # or sk_test_... for dev
STRIPE_WEBHOOK_SECRET=whsec_...        # from webhook endpoint above

# Addon Price IDs (from Stripe Dashboard)
STRIPE_PRICE_ADDON_REQUESTS_500K=price_...
STRIPE_PRICE_ADDON_R2_5GB=price_...
STRIPE_PRICE_ADDON_D1_1GB=price_...
STRIPE_PRICE_ADDON_PROJECTS_2=price_...

# Optional
STRIPE_API_URL=https://api.stripe.com  # default, change for testing
FRONTEND_URL=https://cloud.solobase.dev
PRODUCTS_WEBHOOK_URL=                  # control plane webhook (if used)
PRODUCTS_WEBHOOK_SECRET=               # signing secret for above
```

### CF Dispatch Worker (wrangler.toml / secrets)

```toml
[vars]
DISPATCHER_NAMESPACE = "solobase-workers"
ENVIRONMENT = "production"

[[d1_databases]]
binding = "DB"
database_name = "solobase-platform"
database_id = "..."

[triggers]
crons = ["0 * * * *"]  # hourly storage sync
```

```env
# Secrets (set via wrangler secret put)
CF_ACCOUNT_ID=...
CF_API_TOKEN=...           # needs: D1 read, R2 read, Workers write
STRIPE_SECRET_KEY=...      # same as above
```

### CF API Token Permissions

Create a custom API token at dash.cloudflare.com → My Profile → API Tokens:

- **D1** → Read (for database size queries)
- **R2** → Read (for bucket usage queries)
- **Workers Scripts** → Edit (for deploying user workers)
- **Account** → Workers for Platforms → Edit

## 3. Subscription Flow

### User subscribes to a plan:

```
1. Frontend creates a Stripe Checkout session:
   POST /b/products/checkout
   {purchase_id, success_url, cancel_url}
   → Returns {checkout_url}

2. User completes payment on Stripe

3. Stripe fires webhook: checkout.session.completed
   → metadata must include: {user_id, plan: "starter"}
   → Creates/updates row in subscriptions table
   → Status = "active", plan = "starter"

4. User's projects can now be activated
```

### User adds an addon:

```
1. Frontend lists available addons:
   GET /b/products/addons
   → Returns [{id, name, price_cents, ...}]

2. User subscribes to addon:
   POST /b/products/addons/subscribe
   {addon_id: "addon_requests_500k"}
   → Adds item to existing Stripe subscription
   → Increments subscriptions.addon_requests by 500000

3. Stripe fires webhook: customer.subscription.updated
   → sync_addons_from_stripe() recalculates all addon totals
   → This is the safety net — DB always matches Stripe
```

### User cancels an addon:

```
1. POST /b/products/addons/cancel
   {addon_id: "addon_requests_500k", subscription_item_id: "si_..."}
   → Removes item from Stripe subscription
   → Decrements subscriptions.addon_requests

2. Stripe fires webhook: customer.subscription.updated
   → sync recalculates totals (safety net)
```

### User cancels entire subscription:

```
1. User cancels in Stripe portal or you cancel via API

2. Stripe fires webhook: customer.subscription.deleted
   → Sets status = "cancelled"
   → Resets ALL addon columns to 0
   → Projects become inaccessible (dispatch worker blocks requests)
```

## 4. Usage Enforcement

### How limits are checked (every request):

```
Request → CF Dispatch Worker → check_usage()
  1. Check subscription status (active/past_due/cancelled)
  2. Sum usage across ALL user's projects:
     SELECT SUM(requests), SUM(r2_bytes), SUM(d1_bytes)
     FROM project_usage JOIN projects
     WHERE owner_user_id = ? AND month = ?
  3. Read account addons from subscriptions table
  4. Compare: total_usage vs (plan_limit + addon_amount)
  5. Block if over limit, warn at 80%
```

### How usage is tracked:

- **Requests**: Incremented per-project on every request (via waitUntil, non-blocking)
- **R2 storage**: Synced hourly by cron worker from CF API (`/r2/buckets/{name}/usage`)
- **D1 storage**: Synced hourly by cron worker from CF API (`/d1/database/{id}`)

## 5. TODO Checklist

### Stripe Setup
- [ ] Create Starter product + recurring price in Stripe
- [ ] Create Pro product + recurring price in Stripe
- [ ] Create 4 addon products + recurring prices in Stripe
- [ ] Set `plan` metadata or lookup_key on plan prices
- [ ] Set `addon_id` metadata on addon prices
- [ ] Configure webhook endpoint with correct events
- [ ] Copy webhook signing secret

### Environment Variables
- [ ] Set `STRIPE_SECRET_KEY`
- [ ] Set `STRIPE_WEBHOOK_SECRET`
- [ ] Set all 4 `STRIPE_PRICE_ADDON_*` vars with Stripe Price IDs
- [ ] Set `FRONTEND_URL` to production URL
- [ ] Set CF worker secrets (`CF_ACCOUNT_ID`, `CF_API_TOKEN`)

### CF Worker
- [ ] Deploy dispatch worker with D1 binding
- [ ] Run schema migrations (happens automatically on first request)
- [ ] Configure cron trigger (`0 * * * *`) in wrangler.toml
- [ ] Verify CF API token has D1 + R2 read permissions

### Testing
- [ ] Test plan subscription flow end-to-end (checkout → webhook → activation)
- [ ] Test addon subscribe/cancel flow
- [ ] Test usage enforcement (create project, make requests, hit limit)
- [ ] Test free tier blocking (no subscription → requests blocked)
- [ ] Test subscription cancellation (webhook → addon reset → projects blocked)
- [ ] Test storage sync cron (verify R2/D1 sizes update in project_usage)

### Not Yet Implemented
- [ ] User-facing billing UI (SSR page to view subscription, manage addons, billing portal)
- [ ] Stripe Customer Portal integration (for self-service plan changes)
- [ ] Admin subscription management page
- [ ] Email notifications for usage warnings (80% threshold)
- [ ] Email notifications for payment failures
- [ ] User D1/R2 direct access (scoped API token generation)
