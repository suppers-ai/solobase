# Cloudflare WAF Rate Limiting Rules

Set up in: Dashboard → Security → WAF → Rate limiting rules

Rules are evaluated top to bottom. More specific rules first.

## Rule 1: Auth brute force protection (PRIORITY)

Prevents password brute force and signup spam.

- **Name:** Auth abuse
- **Expression:** `(http.request.uri.path contains "/auth/login" or http.request.uri.path contains "/auth/signup" or http.request.uri.path contains "/auth/refresh")`
- **Rate:** 10 requests per 1 minute per IP
- **Action:** Block for 300 seconds (5 minutes)
- **Requires:** Pro plan

## Rule 2: API rate limit

Prevents API abuse across all endpoints.

- **Name:** API rate limit
- **Expression:** `(http.request.uri.path contains "/api/" or http.request.uri.path contains "/auth/" or http.request.uri.path contains "/admin/" or http.request.uri.path contains "/b/")`
- **Rate:** 100 requests per 10 seconds per IP
- **Action:** Block for 60 seconds
- **Requires:** Pro plan

## Rule 3: Control plane lockdown

Extra protection for admin endpoints.

- **Name:** Control plane
- **Expression:** `(http.request.uri.path contains "/_control/")`
- **Rate:** 5 requests per 1 minute per IP
- **Action:** Block for 600 seconds (10 minutes)
- **Requires:** Pro plan

## Rule 4: Webhook method restriction

Only POST should reach the billing webhook.

- **Name:** Webhook method
- **Expression:** `(http.request.uri.path contains "/billing/webhook" and http.request.method ne "POST")`
- **Action:** Block
- **Requires:** Free plan (this is a custom rule, not rate limiting)

## Rule 5: Global fallback (FREE PLAN — set this now)

Catches everything else. This is the minimum protection.

- **Name:** Global rate limit
- **Expression:** `(http.request.uri.path wildcard r"*")`
- **Rate:** 1000 requests per 10 seconds per IP
- **Action:** Block for 60 seconds
- **Requires:** Free plan

## Also enable (free, no rules needed)

- **Bot Fight Mode:** Dashboard → Security → Bots → Toggle ON
- **Browser Integrity Check:** Dashboard → Security → Settings → Toggle ON
- **Under Attack Mode:** Dashboard → Security → Settings → Use in emergencies only

## Cost optimization notes

- WAF-blocked requests do NOT invoke your Worker (no cost to you)
- The free plan allows 1 rate limiting rule — use Rule 5 (global)
- Pro plan ($20/mo) unlocks 5 rules — add Rules 1-4
- If you see abuse on specific paths, add targeted rules before the global one
