# solobase.json Config Refactor Plan

## Goal

Consolidate configuration so the D1/SQLite `variables` table is the single runtime config source for both native and Cloudflare. `solobase.json` becomes minimal infrastructure config with an optional `.env` file for seeding variables.

## Current State

- **Native binary**: reads `solobase.json` for everything (database, storage, JWT, features). ConfigBlock populated from process env vars.
- **Cloudflare user worker**: reads D1 `variables` table for all app config. `solobase.json` not used.
- Two separate config systems.

## Proposed Config

### `solobase.json` (infrastructure only)
```json
{
  "listen": "0.0.0.0:8090",
  "database": { "type": "sqlite", "path": "data/solobase.db" },
  "storage": { "type": "local", "root": "data/storage" },
  "env_file": ".env"
}
```

### `.env` (app config — gitignored)
```
JWT_SECRET=my-secret-key
APP_NAME=My App
ADMIN_EMAIL=admin@example.com
PRIMARY_COLOR=#fe6627
ALLOW_SIGNUP=true
AUTH_ALLOWED_EMAIL_DOMAINS=example.com
STRIPE_SECRET_KEY=sk_test_...
MAILGUN_API_KEY=key-...
PROJECT_CONFIG={"auth":{},"admin":{},"products":{}}
```

## Startup Flows

### Native
1. Read `solobase.json` — parse infrastructure config (database, storage, listen)
2. Load `.env` file into process env vars (if `env_file` is set)
3. Connect to database, run schema migrations
4. Seed variables table from env vars — `INSERT OR IGNORE` so existing DB values take priority
5. Load all variables from DB into ConfigBlock
6. Blocks read config via ConfigBlock

### Cloudflare (unchanged)
1. Worker bindings provide DB, STORAGE, PROJECT_ID
2. Load all variables from `variables` table into ConfigBlock
3. Blocks read config via ConfigBlock

Both paths end at the same place: **variables table → ConfigBlock → blocks**.

## Config Priority (Native)

```
Dashboard edits (variables table)  →  highest priority (already in DB, not overwritten)
.env file                          →  seeds variables table on startup (INSERT OR IGNORE)
Schema seed defaults               →  lowest priority (only if table is empty)
```

Once a value is in the variables table (whether from `.env` seed or dashboard edit), it stays. The `.env` file only populates missing keys.

## What Changes

### `solobase.json` schema (`app_config.rs`)
- Remove: `jwt_secret`, `features`, `web_root`, `app` fields
- Add: `env_file: Option<String>` field
- Keep: `listen`, `database`, `storage`, `$schema`, `version`

### Native binary (`main.rs`)
- Add: load `.env` file using `dotenv` or simple parser
- Add: after migrations, seed variables table from env vars
- Add: load all variables from DB into ConfigBlock (same query as Cloudflare worker)
- Remove: reading JWT_SECRET, features from solobase.json
- Remove: building ConfigBlock from individual env var lookups

### User worker (`solobase-worker/lib.rs`)
- No changes — already reads from variables table

### Features system
- Both native and Cloudflare read `PROJECT_CONFIG` from variables table
- `.env` can set it: `PROJECT_CONFIG={"auth":{},"admin":{}}`
- Default: all features enabled if `PROJECT_CONFIG` not set

### Schema/seeds (`schema.rs`, `settings.rs`)
- No changes — seed defaults still apply when table is empty

## Files to Modify

1. `crates/solobase/src/app_config.rs` — simplify config struct, add `env_file`
2. `crates/solobase/src/main.rs` — new startup flow (load .env, seed variables, load from DB)
3. `crates/solobase/solobase.json` — update to new format
4. `crates/solobase/src/gen_schema.rs` — update JSON schema generation
5. `crates/solobase/Cargo.toml` — add `dotenv` dependency (or simple .env parser)

## Benefits

- One runtime config system for native and Cloudflare (variables table)
- `.env` is a standard format — works with Docker, systemd, CI, etc.
- Secrets stay out of `solobase.json` (`.env` is gitignored)
- Dashboard edits persist and override `.env` values
- Project databases are fully portable
- `solobase.json` is minimal — just infrastructure
