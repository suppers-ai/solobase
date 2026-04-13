# Solobase Builder: Unified Runtime Setup

**Date:** 2026-04-13
**Status:** Draft

## Problem

Three deployment targets (native, browser WASM, Cloudflare Workers) each maintain their own copy of the WAFER runtime setup code: registering service blocks, middleware blocks, feature blocks, the router, and the site-main flow. This ~100-150 line orchestration is nearly identical across all three, differing only in which service implementations are injected.

This causes two problems:
1. **Maintenance burden** — adding a new block or middleware requires updating three separate files.
2. **Drift** — the Cloudflare worker is already missing `auth-validator`, `iam-guard`, and `inspector` middleware that the other two platforms register. The browser WASM version inlines a stale copy of the site-main flow JSON.

## Solution

Introduce a `SolobaseBuilder` in the `solobase` crate that encapsulates all common runtime setup. Each platform provides only its service implementations and calls the builder. The `solobase` crate becomes fully platform-agnostic (no tokio), with native-specific code moving to a new `solobase-native` crate.

Additionally, package `solobase-web` as an npm package for easy integration by web developers.

## Crate Restructure

### Before

```
solobase/crates/
  solobase/       -- lib (re-exports core) + binary (tokio, rusqlite) -- mixed concerns
  solobase-core/  -- blocks, routing, shared logic (WASM-compatible)
  solobase-web/   -- browser WASM entry point

solobase-cloud/crates/
  solobase-cloudflare/  -- dispatch worker
  solobase-worker/      -- per-project user worker (duplicated setup)
```

### After

```
solobase/crates/
  solobase/         -- lib ONLY: SolobaseBuilder, flows, router block. No tokio. Platform-agnostic.
  solobase-core/    -- unchanged: blocks, routing, shared logic
  solobase-native/  -- NEW: binary + tokio, rusqlite, tracing, native service impls
  solobase-web/     -- slimmed down: browser service impls + builder call

solobase-cloud/crates/
  solobase-cloudflare/  -- unchanged: dispatch worker
  solobase-worker/      -- slimmed down: CF service impls + builder call
```

### What Moves Where

| Item | From | To |
|------|------|----|
| `main.rs` (binary entry point) | `solobase/` | `solobase-native/` |
| `app_config.rs` (InfraConfig, load_block_settings, load_wrap_grants) | `solobase/` | `solobase-native/` |
| `flows/` (site-main flow JSON + routes) | `solobase/` | stays in `solobase/` (no tokio deps) |
| `blocks/router.rs` (NativeBlockFactory, SolobaseRouterBlock) | `solobase/` | stays in `solobase/` (already platform-agnostic) |
| `builder.rs` (SolobaseBuilder) | new | `solobase/` |

### solobase Cargo.toml Changes

- Remove `default = ["server"]` feature and the `server` feature entirely
- Remove tokio, rusqlite, tracing-subscriber, dotenvy dependencies
- Remove `[[bin]]` section
- Keep: wafer-run, wafer-core, solobase-core, serde, serde_json, async-trait, and all wafer-block-* middleware crates

### solobase-native Cargo.toml (New)

- `solobase` (the now-agnostic library)
- tokio (full features)
- rusqlite
- tracing, tracing-subscriber
- dotenvy
- wafer-block-sqlite, wafer-block-local-storage, wafer-block-config, wafer-block-crypto, wafer-block-network, wafer-block-logger, wafer-block-http-listener
- Optional: wafer-block-postgres, wafer-block-s3, opentelemetry (moved from solobase's optional deps)

## SolobaseBuilder API

Located in `solobase/src/builder.rs`.

```rust
pub struct SolobaseBuilder {
    database: Option<Arc<dyn DatabaseService>>,
    storage: Option<Arc<dyn StorageService>>,
    config: Option<Arc<dyn ConfigService>>,
    crypto: Option<Arc<dyn CryptoService>>,
    network: Option<Arc<dyn NetworkService>>,
    logger: Option<Arc<dyn LoggerService>>,
    block_settings: BlockSettings,
    extra_blocks: Vec<(String, Arc<dyn Block>)>,
}

impl SolobaseBuilder {
    pub fn new() -> Self;

    // Required services (one per platform-specific impl)
    pub fn database(mut self, svc: Arc<dyn DatabaseService>) -> Self;
    pub fn storage(mut self, svc: Arc<dyn StorageService>) -> Self;
    pub fn config(mut self, svc: Arc<dyn ConfigService>) -> Self;
    pub fn crypto(mut self, svc: Arc<dyn CryptoService>) -> Self;
    pub fn network(mut self, svc: Arc<dyn NetworkService>) -> Self;
    pub fn logger(mut self, svc: Arc<dyn LoggerService>) -> Self;

    // Feature flags (which blocks are enabled)
    pub fn block_settings(mut self, settings: BlockSettings) -> Self;

    // Escape hatch for platform-specific blocks (e.g. solobase/dispatcher on CF)
    pub fn extra_block(mut self, name: &str, block: Arc<dyn Block>) -> Self;

    // Build the configured Wafer runtime. Does NOT call start().
    pub fn build(self) -> Result<Wafer, String>;
}
```

### build() Internals

`build()` performs the following steps in order:

1. Validate all 6 required services are provided (return `Err` if any missing)
2. Create `Wafer::new()`, set admin block to `"suppers-ai/admin"`
3. Register service blocks via `wafer_core::service_blocks::*::register_with()`:
   - database (+ `"db"` alias)
   - storage (wrapped via `solobase_core::blocks::storage::create()` for namespace isolation + WRAP access control)
   - config
   - crypto
   - network (wrapped via `solobase_core::blocks::network::create()` for request logging)
   - logger
4. Register ALL middleware blocks:
   - `wafer_block_auth_validator::register()`
   - `wafer_block_cors::register()`
   - `wafer_block_iam_guard::register()`
   - `wafer_block_inspector::register()` (with `allow_anonymous: false` config)
   - `wafer_block_readonly_guard::register()`
   - `wafer_block_router::register()`
   - `wafer_block_security_headers::register()`
   - `wafer_block_web::register()`
5. Create feature blocks: `solobase_core::blocks::create_blocks(|name| settings.is_enabled(name))`
6. Register feature blocks: `solobase_core::blocks::register_shared_blocks()`
7. Register email block (always on, not feature-gated)
8. Register any `extra_blocks`
9. Build `SolobaseRouterBlock` with `NativeBlockFactory` + jwt_secret from config + feature settings
10. Register `suppers-ai/router` with routes config
11. Register `site-main` flow via `flows::register_site_main()`
12. Register a post-start hook that injects WRAP grants into the storage block (so callers don't need to manage this themselves)
13. Return the `Wafer` instance (caller calls `start()` or `start_without_bind()`)

### JWT Secret Handling

The builder reads the JWT secret from the config service *before* registering the config block. Since the config service is passed as an `Arc`, the builder can call `.get()` on it before handing it off to `register_with()`. This works because all three platforms populate the config service with variables before calling `build()`.

```rust
// Inside build():
let config = self.config.ok_or("config service required")?;
let jwt_secret = config.get("SUPPERS_AI__AUTH__JWT_SECRET").unwrap_or_default();
// Then register the config block with the same Arc
wafer_core::service_blocks::config::register_with(&mut wafer, config)?;
```

## Platform Usage

### solobase-native (main.rs)

```rust
let mut wafer = SolobaseBuilder::new()
    .database(Arc::new(SQLiteDatabaseService::open(&infra.db_path)?))
    .storage(Arc::new(LocalStorageService::new(&infra.storage_root)?))
    .config(Arc::new(EnvConfigService::from_vars(&vars)))
    .crypto(Arc::new(Argon2JwtCryptoService::new(jwt_secret)))
    .network(Arc::new(HttpNetworkService::new()))
    .logger(Arc::new(TracingLogger))
    .block_settings(features)
    .build()?;

// Native: register http-listener + observability hooks, then start with bind
wafer_block_http_listener::register(&mut wafer)?;
wafer.add_block_config("wafer-run/http-listener", json!({ "flow": "site-main", "listen": infra.listen }));
register_observability_hooks(&mut wafer);
let wafer = wafer.start().await?;
```

Note: `http-listener` is native-only and registered outside the builder. The builder returns a mutable `Wafer` so the caller can add platform-specific blocks/config before starting.

### solobase-web (lib.rs)

```rust
pub async fn initialize() -> Result<(), JsValue> {
    bridge::dbInit().await;
    let vars = config::seed_and_load_variables();
    let features = config::load_block_settings();

    let config_svc = wafer_block_config::service::EnvConfigService::new();
    for (key, value) in &vars { config_svc.set(key, value); }

    let wafer = SolobaseBuilder::new()
        .database(Arc::new(BrowserDatabaseService))
        .storage(Arc::new(BrowserStorageService))
        .config(Arc::new(config_svc))
        .crypto(Arc::new(BrowserCryptoService::new(jwt_secret)))
        .network(Arc::new(BrowserNetworkService))
        .logger(Arc::new(ConsoleLogger))
        .block_settings(features)
        .build()
        .map_err(|e| JsValue::from_str(&e))?;

    wafer.start_without_bind().await.map_err(|e| JsValue::from_str(&e))?;
    RUNTIME.with(|r| *r.borrow_mut() = Some(wafer));
    Ok(())
}
```

Deleted: `register_service_blocks()`, `register_middleware_blocks()`, `register_site_main_flow()`, and the inlined flow JSON copy.

### solobase-cloud worker (lib.rs)

```rust
async fn handle_request(req: &Request, env: &Env) -> Result<Response> {
    let db = env.d1("DB")?;
    let bucket = env.bucket("STORAGE")?;
    let env_vars = load_d1_map(&db, "SELECT key, value FROM suppers_ai__admin__variables").await?;
    let features = load_block_settings_from_d1(&db).await;
    // ... merge worker binding keys ...

    let mut builder = SolobaseBuilder::new()
        .database(Arc::new(D1DatabaseService::new(db)))
        .storage(Arc::new(R2StorageService::new(bucket)))
        .config(Arc::new(HashMapConfigService::new(env_vars)))
        .crypto(Arc::new(SolobaseCryptoService::new(jwt_secret)))
        .network(Arc::new(WorkerFetchService))
        .logger(Arc::new(ConsoleLoggerService))
        .block_settings(features);

    if let Ok(fetcher) = env.service("DISPATCHER") {
        builder = builder.extra_block("solobase/dispatcher", Arc::new(DispatcherBlock::new(fetcher)));
    }

    let wafer = builder.build().map_err(|e| Error::RustError(e))?;
    wafer.start_without_bind().await.map_err(|e| Error::RustError(e))?;

    let mut msg = convert::worker_request_to_message(req).await?;
    let result = wafer.run("site-main", &mut msg).await;
    convert::wafer_result_to_worker_response(result)
}
```

Deleted: `register_blocks!` macro, manual BlockId match loop, all direct middleware/feature block registration.

## solobase-web npm Package

### Package Structure

```
solobase-web/packages/solobase-web/
  package.json
  src/
    index.ts          -- app-side: setupSolobase()
    worker.ts         -- SW-side: re-exports initialize() + handleRequest()
  dist/
    solobase_web_bg.wasm
    solobase_web.js   -- wasm-bindgen glue
```

The `packages/` directory lives alongside the Rust `crates/` in the solobase-web workspace. The WASM binary is built from the Rust crate and copied into `dist/` during the npm build step.

### Mode 1: Batteries-Included

For developers with no existing Service Worker:

```js
import { setupSolobase } from 'solobase-web';

await setupSolobase({
  routes: ['/b/**', '/health', '/api/**'],  // optional, defaults to ['/b/**', '/health']
  scope: '/',                                // optional, defaults to '/'
});
```

`setupSolobase()` does:
1. Registers the bundled `worker.js` as a Service Worker
2. Waits for it to activate
3. The SW calls `initialize()` on install, then intercepts matching fetch events with `handleRequest()`

### Mode 2: Composable

For developers with an existing Service Worker:

```js
// In their existing service-worker.js
import { initialize, handleRequest } from 'solobase-web/worker';

await initialize();

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  if (url.pathname.startsWith('/b/') || url.pathname === '/health') {
    event.respondWith(handleRequest(event.request));
    return;
  }
  // Their existing SW logic
});
```

### Configuration

Both modes accept an optional config for route matching. The batteries-included mode passes config to the SW via `postMessage`. The composable mode lets developers handle routing themselves.

## What Does NOT Change

- **solobase-core/** — no changes. All blocks, routing, shared logic untouched.
- **solobase-cloudflare/** (dispatch worker) — no changes. It has no WAFER dependency.
- **Platform-specific service implementation files** — `database.rs`, `storage.rs`, `crypto_service.rs`, etc. in each platform crate stay as-is.
- **Schema migration / provisioning logic** — stays in solobase-worker (D1-specific) and solobase-native (rusqlite-specific).
- **Custom WASM block loading** — stays in solobase-worker (`custom_blocks.rs`).

## Net Effect

When a new block or middleware is added to solobase-core, the builder is the single place to update. All three platforms (native, browser, Cloudflare) pick it up automatically with no further changes.
