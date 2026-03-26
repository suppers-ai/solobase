# Architecture Review — WfP Setup & Config System

## What We Have

- Dispatch worker: routing, CORS, static files, usage tracking, control plane
- User workers: per-project block execution, each with own D1
- Platform ("cloud") is a user worker — same code as all projects
- `variables` table in D1 = single runtime config source
- `.env` file planned for native binary seeding

## Comparison with Other Platforms

### How Supabase handles it
- Per-project Postgres databases (same as our per-project D1)
- Secrets (JWT secret, API keys) stored in **platform management layer**, NOT in the project database
- Has **Vault** — a Postgres extension that encrypts secrets at rest within the DB using a key managed by Supabase's backend
- Config managed via `config.toml` + `.env` for local dev (similar to our plan)

### How Vercel handles it
- Environment variables stored in **Vercel's platform**, encrypted at rest
- Per-environment config (development, preview, production)
- Sensitive vars can't be decrypted once created (write-only)
- Users never see secret values after setting them

### How Firebase/Render/Railway handle it
- All store config in the platform management layer
- Secrets never stored in the user's database
- Injected as env vars at runtime

### Key difference with Solobase
**We store secrets (JWT_SECRET, STRIPE_SECRET_KEY, etc.) directly in the user's D1 database as plain text.** Other platforms keep secrets in a separate management layer.

---

## Issues Found

### 1. Secrets in Plain Text in D1 (High Priority)

**Problem:** `JWT_SECRET`, `STRIPE_SECRET_KEY`, `CONTROL_PLANE_SECRET` are plain text in the `variables` table. Anyone with DB access (or a D1 export) sees all secrets.

**What others do:** Supabase uses Vault (encrypted at rest with separate key). Vercel encrypts at rest and makes sensitive vars write-only.

**Options:**
- **(a) Accept it** — D1 is already encrypted at rest by Cloudflare. The admin dashboard requires auth. For a self-hosted product, the user owns their own secrets. This is actually fine for the use case.
- **(b) Mark sensitive variables** — add a `sensitive` column. The API returns `"*****"` instead of the actual value for sensitive vars. Users can set/update but not read back. Similar to Vercel's approach.
- **(c) Encrypt in DB** — encrypt values with a per-project key derived from the D1 database ID. Adds complexity, and the key has to be stored somewhere.

**Recommendation:** Option (b) is the best balance. The DB values are still plain text (needed for blocks to read them), but the API never exposes them. The dashboard shows masked values with a "regenerate" option.

### 2. CONTROL_PLANE_SECRET in User's DB (Medium Priority)

**Problem:** The platform's control plane API key is stored in the cloud worker's `variables` table. An admin user on the platform can see it via the settings UI and use it to manage all projects.

**What others do:** Platform management credentials are never exposed to end users.

**Recommendation:** Move `CONTROL_PLANE_URL` and `CONTROL_PLANE_SECRET` out of the variables table. These should be Worker env var bindings on the cloud worker only — set during provisioning, not visible in the dashboard. Or better: the deployments block should call the dispatch worker using an internal mechanism (service binding or signed request) instead of a shared secret.

### 3. Secret Rotation — Immediate Revocation (Resolved)

**Decision:** Changing `JWT_SECRET` immediately invalidates all tokens. This is intentional — the secret should only be rotated if compromised, so immediate revocation is the correct behavior.

**Implementation:** The API client intercepts 401 responses globally. When a token becomes invalid (secret rotated, expired, etc.), the client automatically clears auth state and redirects to the login page. No broken UI states, no infinite loops. Users simply re-authenticate with the new secret.

### 4. R2 Data Not Cleaned Up on Project Deletion (Medium Priority)

**Problem:** `delete_project` in `provision.rs` deletes the D1 and user worker, but R2 objects (prefixed by project ID) remain as orphaned data.

**Recommendation:** Add R2 cleanup — list and delete all objects with the project's prefix. Or accept orphaned data and add a cleanup job later.

### 5. No Per-Request Config Caching (Low Priority)

**Problem:** `SELECT key, value FROM variables` runs on every request.

**Reality:** D1 reads are extremely fast (co-located, <1ms). A table with 10-20 rows is microseconds. This is a non-issue for current scale. D1 also has its own caching layer.

**Recommendation:** Leave as-is. Only optimize if profiling shows it's a bottleneck.

### 6. PROJECT_CONFIG as JSON String in Variables (Low Priority)

**Problem:** Feature flags are stored as a JSON string in `PROJECT_CONFIG` variable. Awkward to edit in the dashboard.

**Simpler alternative:** Individual variables like `FEATURE_AUTH=true`, `FEATURE_PRODUCTS=true`. The code already has `is_feature_enabled()` — it just needs to check variables instead of parsing JSON.

**Recommendation:** Consider splitting into individual feature variables. More dashboard-friendly, easier to understand.

---

## What's Good

1. **Per-project D1 isolation** — proper tenant isolation, each project has its own database
2. **Variables table as single config source** — portable, dashboard-editable, same for native and Cloudflare
3. **Auto-generated JWT_SECRET per project** — no shared secrets between tenants
4. **No secrets forwarded during provisioning** — projects start clean
5. **`.env` file plan for native** — standard, works with Docker/systemd
6. **Dispatch worker has zero business logic** — clean separation
7. **Health check after provisioning** — verifies worker is responsive
8. **Rollback on failed provisioning** — cleans up D1 if worker upload fails

## What's Missing / Nice to Have

1. **Project ownership model** — who owns a project? Currently no link between platform users and their projects
2. **Variable change audit log** — track who changed what config when
3. **Backup/export** — users should be able to export their D1 database
4. **Custom domains per project** — `myapp.com` pointing to a user worker
5. **Monitoring/logs** — no way to see user worker logs from the dashboard
6. **Project suspend/resume** — currently control-plane only, not in dashboard

## Simplification Opportunities

1. **Remove `PROJECT_ID` as env var** — store it in the variables table too. Then user workers only need DB + STORAGE bindings. The worker reads PROJECT_ID from variables on first request.

2. **Remove `PROJECT_CONFIG` entirely** — use individual `FEATURE_*` variables instead. Simpler, dashboard-friendly, no JSON parsing.

3. **~~`.env` override instead of seeding~~** — Ruled out. Env var overrides would only work for native (Cloudflare has no env vars for config). This creates inconsistent behavior — dashboard edits silently overridden on native but not Cloudflare. The seeding approach (`INSERT OR IGNORE`) is correct: both runtimes use variables table as the single source, dashboard edits always win.

---

## Summary

The architecture is solid and follows the right patterns. The main gap compared to production SaaS platforms is **secret visibility** — the variables table exposes secrets to admin users. Adding a `sensitive` column with API masking would bring it in line with industry practice. Everything else is either already good or nice-to-have for later.

Sources:
- [Supabase Vault — encrypted secrets in Postgres](https://supabase.com/docs/guides/database/vault)
- [Supabase Config Management](https://supabase.com/docs/guides/local-development/managing-config)
- [Vercel Sensitive Environment Variables](https://vercel.com/docs/environment-variables/sensitive-environment-variables)
- [AWS Multi-Tenant Security Practices](https://aws.amazon.com/blogs/security/security-practices-in-aws-multi-tenant-saas-environments/)
- [Azure Key Vault Multi-Tenant](https://learn.microsoft.com/en-us/azure/architecture/guide/multitenant/service/key-vault)
