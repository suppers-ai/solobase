# solobase-browser Framework Design

**Date:** 2026-04-19
**Status:** Design approved; pending implementation plan
**Sub-project:** 1 of 4 in the solobase framework refactor

## Problem

`crates/solobase-web/` today is structured as an application, not a framework. It bundles:

- Rust-side platform services (bridge, database, storage, network, crypto, logger, asset_loader) that any WAFER-in-browser project needs.
- JS-side static assets (sw.js, loader.js, bridge.js, sql.js integration, HTML shell) that any browser project needs.
- WAFER runtime wiring specific to Solobase.
- The Solobase admin app itself (block registrations, feature-specific routing).

Third-party projects (gizza-ai being the concrete example) want to build WAFER-in-browser applications without duplicating all of this. Today they do:

- Copy-paste `src/bridge.rs`, `src/database.rs`, `src/storage.rs`, `src/network.rs` verbatim (`gizza-ai/src/bridge.rs:5` literally says "Copied from solobase-web").
- Vendor `site/sw.js`, `site/loader.js`, `site/bridge.js` and hand-edit for their project.
- Reach across the filesystem at build time: `cp ../solobase/crates/solobase-web/pkg/sql-wasm-esm.js dist/`.

Consequences:

- Bug fixes propagate by hand, or not at all.
- Cross-repo filesystem coupling breaks any build that isn't a local sibling checkout (CI, npm consumers).
- Changes to the browser platform layer can't be made confidently because we don't know who has copied what.

## Non-Goals

- CLI tool `solobase build --target {wasm,native}` (sub-project 3).
- Native framework extraction (sub-project 2).
- Migration of gizza-ai onto the framework (sub-project 4).
- Unifying `App` into a target-agnostic `solobase::App` facade. Deferred until both browser and native frameworks exist.
- Changes to the npm package `packages/solobase-web/`.
- Feature-block extraction. `solobase-core` feature blocks stay in `solobase-core`; the framework does not depend on them.
- **LLM service extraction.** `BrowserLlmService` (`crates/solobase-web/src/llm.rs`) and its JS bridge `webllm-engine.js` stay in `solobase-web`. A future sub-project (likely paired with gizza-ai's migration) moves them into `solobase-browser` once the second consumer's needs are known.

## Chosen Approach

**Extract browser platform services into a new crate `solobase-browser` that owns the reusable Rust code, vendors the JS/HTML/sql.js assets via `include_bytes!`, and exposes a toolbox of explicit factory functions plus Service-Worker plumbing helpers. Migrate `solobase-web` onto the new framework as the first validation consumer.**

The framework is a *library of browser-adapted services*, not an app framework. It does not know about blocks, routing, or block bundles. Consumers (solobase-web today, gizza-ai tomorrow) compose the factory functions with their own app-level builder (e.g., `SolobaseBuilder` for solobase-web; direct `wafer-run` usage for gizza-ai) and write their own `#[wasm_bindgen]` entrypoints.

User code is plain Rust with no macros, inventory tricks, or hidden control flow. This mirrors how axum users hand-compose `Router` + middleware + `serve(listener, app)` — idiomatic, explicit, inspectable. When the CLI ships in sub-project 3, it can generate the boilerplate for users who want zero-setup, but the boilerplate is always optional.

Alternative approaches considered and rejected:

- **Closure-based `framework_init(register_fn)` with an `App` wrapper.** Earlier shape considered during brainstorming. Rejected because app-specific config (JWT secret from DB, CSP string, feature flags) must be injected *between* platform-service setup and block registration. A closure shape forces either (a) auto-configuring a crypto service with a blank key that the user later replaces — ugly, order-dependent, easy to get wrong — or (b) expanding `App`'s method surface with `set_crypto_secret`, `configure_block`, `set_block_settings` and more as each new config surface is discovered. Both roads lead to an incrementally rotting API. Explicit composition stays clean indefinitely.
- **Inventory/ctor-style auto-registration.** User writes `#[solobase::register] fn register(app: &mut App)` and a linker-inserted registry calls it at startup. Hidden control flow; relies on linker-level hacks; confusing when debugging.
- **Macro-generated entrypoints (`export_entrypoints!(register);`).** Less hidden than inventory but still macro-generated code in every user crate.
- **CLI-generated wrapper crate.** Clean for users but depends on the CLI existing (not until sub-project 3) and puts generated code in a temp dir, which is confusing during debugging.

## Architecture

### New crate: `crates/solobase-browser/`

```
crates/solobase-browser/
├── Cargo.toml
├── src/
│   ├── lib.rs             — public API: re-exports + db_init(), factory fns,
│   │                        store_wafer(), dispatch_request()
│   ├── runtime.rs         — thread_local RUNTIME, store_wafer(), dispatch_request()
│   ├── assets.rs          — static_assets(), write_to()
│   ├── bridge.rs          — moved from solobase-web
│   ├── database.rs        — moved from solobase-web (defines make_database_service)
│   ├── storage.rs         — moved from solobase-web (defines make_storage_service)
│   ├── network.rs         — moved from solobase-web (defines make_network_service)
│   ├── crypto.rs          — moved from solobase-web (defines make_crypto_service)
│   ├── logger.rs          — moved from solobase-web (defines make_console_logger)
│   ├── asset_loader.rs    — moved from solobase-web (defines make_sw_asset_loader)
│   └── convert.rs         — request_to_message/output_to_response (used by dispatch_request)
├── assets/
│   ├── sw.js.tmpl
│   ├── loader.js
│   ├── bridge.js
│   ├── index.html.tmpl
│   └── vendor/
│       ├── sql-wasm-esm.js       — vendored from sql.js 1.11.0
│       └── sql-wasm.wasm         — vendored from sql.js 1.11.0
├── bin/
│   └── export-assets.rs   — small CLI binary: writes assets to a dir, runs the bundler
└── tools/bundle/          — former crates/solobase-web-bundle moved here (private module)
    └── (content from PR #4)
```

Dependencies: `wafer-run`, `wafer-core`, `wafer-block`, `wafer-block-config`, `wafer-block-crypto`, `wasm-bindgen`, `wasm-bindgen-futures`, `web-sys`, `js-sys`, `serde`, `serde_json`, `serde-wasm-bindgen`, `async-trait`, `chrono`, `hex`, `pbkdf2`, `hkdf`, `sha2`, `hmac`, `base64ct` (the current set from `solobase-web/Cargo.toml`, minus `solobase` and `solobase-core` — neither of which the framework should depend on).

### Refactored crate: `crates/solobase-web/`

After migration:

```
crates/solobase-web/
├── Cargo.toml             — depends on solobase-browser + solobase-core
├── src/
│   ├── lib.rs             — thin wasm-bindgen wrappers + explicit composition
│   ├── config.rs          — app-specific config (kept)
│   └── llm.rs             — BrowserLlmService (kept; see "Out of scope" below)
├── js/
│   ├── ai-bridge.js       — Solobase's local-LLM integration (kept)
│   ├── webllm-engine.js   — WebLLM JS bridge (kept with llm.rs)
│   └── manifest.json      — PWA manifest (kept)
└── Makefile               — invokes solobase-browser's export-assets binary
```

`llm.rs` (BrowserLlmService) and its JS bridge `webllm-engine.js` are *also* browser-platform services in principle, and a future sub-project should likely move them into the framework so gizza-ai can consume them without copy-pasting. For this sub-project we keep them in `solobase-web` to hold the scope tight — the LLM migration is its own concern with its own dep footprint (`futures`, `tokio-util`, WebLLM's JS shape), and extracting it belongs alongside gizza-ai's migration where the real second consumer exists to validate the framework contract.

The app's `src/lib.rs` becomes roughly (preserves the existing 9-step init flow; every step stays, but platform-service construction and SW plumbing go through the framework's factory helpers):

```rust
use std::sync::Arc;
use solobase::builder::{self, SolobaseBuilder};
use wafer_core::interfaces::config::service::ConfigService;
use wasm_bindgen::prelude::*;

mod config; // app-specific: seed_and_load_variables, load_block_settings
mod llm;    // app-specific: BrowserLlmService (WebLLM integration)

#[wasm_bindgen]
pub async fn initialize() -> Result<(), JsValue> {
    if solobase_browser::is_initialized() {
        return Ok(());
    }

    solobase_browser::db_init().await;

    let vars = config::seed_and_load_variables();
    let features = config::load_block_settings();
    let jwt = vars.get("SUPPERS_AI__AUTH__JWT_SECRET").cloned().unwrap_or_default();

    let config_svc = wafer_block_config::service::EnvConfigService::new();
    for (k, v) in &vars { config_svc.set(k, v); }

    let browser_llm: Arc<dyn wafer_core::interfaces::llm::service::LlmService> =
        Arc::new(llm::BrowserLlmService::new());

    let (mut wafer, storage_block) = SolobaseBuilder::new()
        .database(solobase_browser::make_database_service())
        .storage(solobase_browser::make_storage_service())
        .config(Arc::new(config_svc))
        .crypto(solobase_browser::make_crypto_service(jwt))
        .network(solobase_browser::make_network_service())
        .logger(solobase_browser::make_console_logger())
        .llm_service("browser", browser_llm)
        .block_settings(features)
        .block_config("wafer-run/security-headers", serde_json::json!({ "csp": SOLOBASE_CSP }))
        .build()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    wafer.set_asset_loader(solobase_browser::make_sw_asset_loader());
    wafer.start_without_bind().await.map_err(|e| JsValue::from_str(&e.to_string()))?;
    builder::post_start(&wafer, &storage_block);

    solobase_browser::store_wafer(wafer);
    Ok(())
}

#[wasm_bindgen]
pub async fn handle_request(req: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    solobase_browser::dispatch_request(req).await
}

const SOLOBASE_CSP: &str = concat!(/* …existing CSP… */);
```

Everything else that currently lives in `solobase-web/src/*.rs` moves to `solobase-browser`. The `solobase-web-bundle` workspace member is relocated to `solobase-browser/tools/bundle/` and becomes an internal module of the framework; its public `run` function stays, so the bundler remains unit-testable.

### Dependency diagram

```
solobase-browser  (NEW framework: Rust services + vendored JS assets + bundler)
        ▲
        │
solobase-web  (REFACTORED: ~30 lines of explicit composition + solobase's block suite)
        │
        ▼  (SW host delivers solobase_web_bg.wasm + framework assets to the browser)
```

## Public API

The framework exposes a toolbox of functions, not a closure-based entrypoint. Users compose them explicitly in their own `#[wasm_bindgen]` entrypoints.

### Service factories

Each factory returns an `Arc<dyn …>` matching a WAFER service interface. The factories are thin wrappers around the constructors that already live in the service modules.

```rust
use std::sync::Arc;
use wafer_core::interfaces::{
    database::service::DatabaseService,
    storage::service::StorageService,
    network::service::NetworkService,
    crypto::service::CryptoService,
    logger::Logger,
};
use wafer_run::AssetLoader;

pub fn make_database_service() -> Arc<dyn DatabaseService>;
pub fn make_storage_service() -> Arc<dyn StorageService>;
pub fn make_network_service() -> Arc<dyn NetworkService>;
pub fn make_crypto_service(jwt_secret: String) -> Arc<dyn CryptoService>;
pub fn make_console_logger() -> Arc<dyn Logger>;
pub fn make_sw_asset_loader() -> Arc<dyn AssetLoader>;
```

(Exact `dyn` trait bounds match whatever `wafer_core` currently exposes. If any of these is `Send + Sync`-bounded, the browser implementations that are `!Send` on wasm32 keep working because `Arc<dyn …>` on wasm32 doesn't require `Send`/`Sync`. That's unchanged from today.)

The `jwt_secret` parameter on `make_crypto_service` is explicit because it's app-sourced. The framework never looks it up.

### `db_init`

```rust
pub async fn db_init();
```

Loads sql.js WASM + opens (or creates) the OPFS-backed database. Idempotent-safe to call once at startup, before any factory is instantiated. Wraps `bridge::dbInit()`.

### Service-Worker runtime plumbing

```rust
/// Returns true if a Wafer has been stored via store_wafer() in this SW context.
pub fn is_initialized() -> bool;

/// Store a fully-started Wafer in the SW's thread_local RUNTIME. Subsequent
/// dispatch_request calls use this Wafer. Calling twice in one SW lifetime
/// is a programming error and panics in debug, no-ops in release — the SW
/// should guard with is_initialized() at the top of its initialize() fn.
pub fn store_wafer(w: wafer_run::Wafer);

/// Convert a browser Request into a WAFER Message, dispatch through the
/// `site-main` flow on the stored Wafer, convert the output back into a
/// browser Response. Returns 500 on internal errors. Returns 503 if called
/// before store_wafer.
pub async fn dispatch_request(req: web_sys::Request) -> Result<web_sys::Response, JsValue>;
```

The `RUNTIME` thread_local, `request_to_message`, and `output_to_response` helpers (from today's `solobase-web/src/lib.rs` and `convert.rs`) all move to `solobase-browser` and are exercised exclusively via these three functions. Consumers never touch the thread_local directly.

### `solobase_browser::assets`

```rust
pub struct Asset {
    pub path: &'static str,   // e.g. "sw.js.tmpl"
    pub bytes: &'static [u8], // via include_bytes!
}

pub fn static_assets() -> &'static [Asset];

pub fn write_to(dir: &std::path::Path) -> std::io::Result<()>;
```

`static_assets()` returns the full list. `write_to(dir)` creates `dir` (if missing) and writes every asset; used by the framework's `export-assets` bin and by consumer build scripts.

Contents of `static_assets()` (MVP):

- `sw.js.tmpl`
- `loader.js`
- `bridge.js`
- `index.html.tmpl`
- `vendor/sql-wasm-esm.js` (1.11.0, vendored)
- `vendor/sql-wasm.wasm` (1.11.0, vendored)

### User code shape

A complete consumer `src/lib.rs` using the framework looks like the solobase-web example shown earlier in this spec. The essentials:

1. One call to `solobase_browser::db_init().await`
2. App-specific config loading (reads from the DB, env, or both — consumer's choice)
3. Construction of a `Wafer` via whichever builder the consumer prefers (SolobaseBuilder for solobase-web; direct `wafer-run` calls for gizza-ai or a future bare-bones consumer), passing the framework's factory outputs for platform services
4. `wafer.set_asset_loader(solobase_browser::make_sw_asset_loader())`
5. `wafer.start_without_bind().await`
6. Any app-specific post-start work (`builder::post_start` for solobase)
7. `solobase_browser::store_wafer(wafer)`
8. A `handle_request` wasm-bindgen wrapper that delegates to `solobase_browser::dispatch_request`

~25–35 lines of composition, all explicit. The framework has no opinion on step 3's builder choice.

### `solobase_browser::assets`

```rust
pub struct Asset {
    pub path: &'static str,   // e.g. "sw.js.tmpl"
    pub bytes: &'static [u8], // via include_bytes!
}

pub fn static_assets() -> &'static [Asset];

pub fn write_to(dir: &std::path::Path) -> std::io::Result<()>;
```

`static_assets()` returns the full list. `write_to(dir)` creates `dir` (if missing) and writes every asset; used by the framework's `export-assets` bin and by consumer build scripts.

Contents of `static_assets()` (MVP):

- `sw.js.tmpl`
- `loader.js`
- `bridge.js`
- `index.html.tmpl`
- `vendor/sql-wasm-esm.js` (1.11.0, vendored)
- `vendor/sql-wasm.wasm` (1.11.0, vendored)

### `export-assets` bin

Small binary inside the framework crate invoked from consumer Makefiles:

```bash
cargo run -p solobase-browser --bin export-assets -- <pkg-dir> [--dev]
```

It calls `assets::write_to(pkg_dir)` then runs the internal bundler (formerly `solobase-web-bundle`) on the resulting directory. Same content-hashing + template rendering flow as PR #4, just exposed via a framework-provided binary rather than a workspace-member binary.

## Asset Packaging

**All static assets are vendored inside the crate via `include_bytes!`** and exposed through the public `assets` module. Rationale:

- Single source of truth. No separate asset artifact to keep in sync; `cargo publish` ships everything needed.
- No network fetches during consumer builds. Removes the `npm pack sql.js@1.11.0` step from the current Makefile, improving reliability in sandboxed CI and offline builds.
- Cargo caches the crate's assets alongside its Rust code. No filesystem path coupling between repos.

**sql.js is not content-hashed.** It keeps its canonical filenames (`sql-wasm-esm.js`, `sql-wasm.wasm`). Rationale:

- sql.js is version-pinned in the framework; its bytes only change when we explicitly bump the vendored version. That's rare (roughly annual).
- Content-hashing it broke cross-repo filesystem consumers (gizza-ai) for no benefit — see PR #4 integration testing for context.
- When the version does bump, a deploy cycle triggers new `solobase_web_bg.wasm` bytes, which in turn changes `sw.js` hashed imports, triggering SW update. Users get the new sql.js on their next navigation.

The content-hashing set therefore shrinks to: `solobase_web.js`, `solobase_web_bg.wasm`. The bundler's `REWRITES` table drops its sql.js entry; the `rewrite_all` helper remains in the crate in case a future consumer needs it.

**~1.5 MB of sql.js assets are committed to the crate.** This is accepted as the cost of removing the cross-repo fetch dependency. The crate is internal-use (not yet on crates.io); publish size is not a blocker.

## Migration Plan for `crates/solobase-web/`

Four steps, each independently revertable:

1. **Create `crates/solobase-browser/`** with moved Rust modules + vendored assets + service factories + runtime plumbing (`db_init`, `store_wafer`, `dispatch_request`, `is_initialized`) + `export-assets` bin + bundler. Add to workspace members. Old `solobase-web` remains untouched and still builds. Ship.
2. **Refactor `crates/solobase-web/src/lib.rs`** to the ~30-line explicit-composition shape described in the Architecture section. Delete `crates/solobase-web/src/{bridge,database,storage,network,crypto,logger,asset_loader,convert}.rs` and `crates/solobase-web/js/*.{js,tmpl,html}` except `ai-bridge.js`. Update `Cargo.toml` to depend on `solobase-browser` and drop the now-direct dependencies on `wasm-bindgen`, `web-sys`, `js-sys`, `pbkdf2`, etc. that are only needed by the moved modules. Update `Makefile` to invoke `export-assets`. Verify `make build` still produces a working site. Ship.
3. **Add `examples/minimal-browser/` smoke test.** ~30 lines of Rust wiring a `Wafer` with the framework's factories + one no-op block; builds in CI. Catches accidental solobase-core coupling leaking into the framework and validates that `wafer-run`-only consumers (without `SolobaseBuilder`) are supported.
4. **Remove the now-unused `crates/solobase-web-bundle/` workspace member.** Its content lives at `solobase-browser/tools/bundle/`. Ship.

If step 2 surfaces a framework-shape issue (e.g., a factory needs a parameter it doesn't have), iterate on `solobase-browser` before running step 3.

## Testing

- **`solobase-browser` unit tests.** Existing `solobase-web-bundle` tests carry over unchanged (just relocated). New tests:
  - `assets::write_to` writes every asset with correct bytes to a temp dir.
  - `assets::static_assets` returns non-empty and matches the on-disk asset set under `assets/`.
  - Each service factory returns a non-null `Arc<dyn …>` (smoke-level; the underlying services have their own tests in their respective modules).
  - `store_wafer` + `dispatch_request` round-trip: store a `Wafer` wired to a stub block that returns a fixed response; assert `dispatch_request` delegates through it. This exercises `request_to_message` and `output_to_response` end-to-end.
  - `dispatch_request` returns a 503-shaped response when called before `store_wafer`.
- **`solobase-web` integration.** `make build` produces the same `pkg/` shape as today (hashed `solobase_web_*`, `asset-manifest.json`, rendered `sw.js`/`index.html`, no unresolved `__` placeholders). The Playwright E2E scaffold from PR #4 continues to apply.
- **Framework consumer smoke test.** `examples/minimal-browser/` is a real consumer: a trivial cdylib crate that calls `db_init`, builds a `Wafer` with the framework's factories plus one no-op block, calls `store_wafer`, and has a `handle_request` wrapper. Compiles in CI. Its existence forces us to catch any accidental dependency on `solobase-core` in the framework itself and validates that non-SolobaseBuilder consumers work.

## Risks

- **Runtime state in a `thread_local`**: matches current behavior (`crates/solobase-web/src/lib.rs` uses `thread_local! static RUNTIME`). WASM is single-threaded, so this is safe. `store_wafer` called twice in a release build silently overwrites; in debug it panics — consumers are expected to guard with `is_initialized()` at the top of their `initialize()`. If future work needs teardown/replace, add it then.
- **Hidden coupling via `solobase-core`**: the framework must not pull in feature blocks. Enforced by the `examples/minimal-browser/` smoke test — if it fails to build, we've leaked a feature-block dependency.
- **Service `Arc<dyn Send + Sync>` bounds**: some WAFER service traits may require `Send + Sync`. On wasm32, `Arc<dyn Service>` doesn't require `Send`/`Sync` at the `Arc` level, but the trait bound itself still matters. If a factory function can't produce a non-`Send` service because the trait requires `Send`, we need to relax the trait bound in `wafer_core` (out-of-scope) or use a wasm-only shim. Existing code already works in today's `solobase-web`, so the path is known; we just need to preserve the exact Arc-wrapping pattern.
- **Config handling**: the framework has no opinion on config. Each consumer (solobase-web uses `EnvConfigService` + DB-seeded variables; gizza-ai may do something else) seeds its own config before constructing the Wafer. No helper provided in the MVP because there's no good one-size-fits-all shape.

## Summary

Extract browser platform services + JS assets from `solobase-web` into a new `solobase-browser` framework crate. Expose explicit factory functions for each browser-specific service (`make_database_service`, `make_storage_service`, `make_crypto_service(jwt_secret)`, etc.), a `db_init` async helper, and Service-Worker runtime plumbing (`store_wafer`, `dispatch_request`, `is_initialized`). Users compose these in their own `#[wasm_bindgen]` entrypoints using whichever app-level builder they prefer — `SolobaseBuilder` for solobase-web, direct `wafer-run` usage for gizza-ai or minimal consumers. No closure-based `App`; no macros; zero hidden control flow. Vendor sql.js in the crate via `include_bytes!` and drop it from content-hashing. Migrate the existing `solobase-web` app onto the framework as the first validation consumer. Defer CLI, native extraction, and gizza-ai migration to later sub-projects.
