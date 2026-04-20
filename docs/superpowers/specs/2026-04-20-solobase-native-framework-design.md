# solobase-native Framework Design

**Date:** 2026-04-20
**Status:** Design approved; pending implementation plan
**Sub-project:** 2 of 4 in the solobase framework refactor

## Problem

Today `crates/solobase-native/` is a binary crate that inlines ~640 lines of platform-service wiring, bootstrap helpers, and app-specific composition in `src/main.rs` + `src/app_config.rs`. By contrast, the browser side is cleanly split:

- `solobase-browser` (library) — browser platform services (factories, SW plumbing, vendored JS assets).
- `solobase-web` (cdylib binary) — the app, ~90 lines of explicit composition over the framework.

The native side has no library counterpart. Any future native WAFER application (or Phase E's CLI code-generator) has nothing to depend on — everything is inlined inside a binary crate. The asymmetry will force Phase E's CLI to either generate duplicated wiring from scratch each time or reach *into* `solobase-native`'s binary crate via `extern crate` tricks. Both are wrong.

## Non-Goals

- Any changes to `wafer-run/crates/wafer-block-*`. Native platform services already live as separate workspace crates there with clean interfaces; this refactor wraps them, doesn't change them.
- Backward compatibility for crate consumers. Nothing outside this workspace depends on the `solobase-native` *crate name*. The produced *binary* (`solobase`) keeps its name, so runtime deployment is unaffected.
- CLI code-generator (Phase E). This spec shapes the library API so the CLI has a clean target; it does not ship the CLI.
- LLM service factories (Phase D). Needs its own spec.
- Solobase-core schema work. Any logic that reads `variables`, `block_settings`, or other solobase-specific tables belongs in the app, not the framework.

## Chosen Approach

**Split `solobase-native` into a library crate (`solobase-native`, reshaped) plus an app binary crate (`solobase-server`, new). The library exposes explicit factory functions for each native platform service, bootstrap helpers, and a `serve()` loop. The binary is a thin (~60-line) composer that uses `SolobaseBuilder` to assemble the solobase app.**

Mirrors the browser-side split (`solobase-browser` library + `solobase-web` cdylib binary) exactly. Consumers (including Phase E's CLI) can generate or hand-write a native entrypoint by composing factory calls, symmetrically to how a browser entrypoint composes `solobase_browser::make_*` calls.

Alternative approaches considered and rejected:

- **Hybrid library+binary crate** (single crate with both `[lib]` and `[[bin]]`). Less disruptive but asymmetric with the browser side. The extra indirection to "which name points to the lib vs the app" adds a taxing conceptual cost for no gain given active development has no external crate consumers to break.
- **Enum-based backend selection** (`make_database_service(DatabaseBackend::Sqlite { path })` instead of `make_sqlite_database_service(path)`). Consolidates the API surface but hides which backends are compiled in behind runtime dispatch. Rejected because Cargo-feature-gated factories make compile-time backend selection explicit; the consumer already knows which backend they want at `cargo build` time.
- **Auto-reading infra from env vars inside factories** (`make_database_service_from_env()`). Magic; mixes config reading with service construction; makes testing harder. Rejected.

## Architecture

### Crate layout

```
crates/
  solobase/              library  — SolobaseBuilder (unchanged)
  solobase-core/         library  — feature blocks, routing, UI (unchanged)
  solobase-browser/      library  — browser platform services + assets (unchanged)
  solobase-native/       library  — NEW: native platform + bootstrap + serve()
  solobase-web/          cdylib   — browser app (unchanged)
  solobase-server/       binary   — native app (renamed from current solobase-native)
```

The produced binary name stays `solobase` (unchanged `[[bin]] name = "solobase"`), so runtime deployment is unaffected. Only the crate name changes.

### Dependency graph (post-refactor)

```
solobase-server (bin)
├── solobase-native (new lib)
│   ├── wafer-run, wafer-core
│   └── wafer-block-sqlite, wafer-block-local-storage, wafer-block-network,
│       wafer-block-crypto, wafer-block-logger, wafer-block-http-listener
│       (+ optional: wafer-block-postgres, wafer-block-s3 via Cargo features)
├── solobase (cross-platform SolobaseBuilder)
└── solobase-core (feature blocks, schema)
```

No circular edges. `solobase-native` does not depend on `solobase` or `solobase-core` — it's a pure platform-layer library. Same purity guarantee as `solobase-browser`.

## Public API of `solobase-native` (library)

### Service factories

```rust
use std::sync::Arc;
use wafer_core::interfaces::{
    database::service::DatabaseService,
    storage::service::StorageService,
    network::service::NetworkService,
    crypto::service::CryptoService,
    logger::service::LoggerService,
};

// Database
pub fn make_sqlite_database_service(path: &str) -> Arc<dyn DatabaseService>;

#[cfg(feature = "postgres")]
pub fn make_postgres_database_service(url: &str) -> Arc<dyn DatabaseService>;

// Storage
pub fn make_local_storage_service(root: &str) -> Arc<dyn StorageService>;

#[cfg(feature = "s3")]
pub fn make_s3_storage_service(config: S3Config) -> Arc<dyn StorageService>;

// Single-implementation services
pub fn make_fetch_network_service() -> Arc<dyn NetworkService>;
pub fn make_jwt_crypto_service(jwt_secret: String) -> Arc<dyn CryptoService>;
pub fn make_tracing_logger() -> Arc<dyn LoggerService>;
```

Each factory is a thin `Arc::new(...)` around the corresponding `wafer-block-*` service constructor. Feature-gated factories live next to their sibling factories in `src/database.rs` / `src/storage.rs`.

`S3Config` is the existing `wafer-block-s3::S3Config` type (or a `pub use` re-export). Same for `JwtCryptoConfig` if `wafer-block-crypto` has one; otherwise the factory's explicit `jwt_secret` param is the only input.

### Bootstrap helpers

```rust
/// Load `.env` file. Honors `SOLOBASE_ENV_FILE` override; falls back to
/// auto-detection of `.env` in the current working dir.
pub fn load_dotenv();

/// Initialize `tracing` / `tracing-subscriber` with `format` ∈ {"text", "json"}.
/// Honors `RUST_LOG` for filter config.
pub fn init_tracing(format: &str);

/// Infrastructure config read from `SOLOBASE_*` env vars. Fields default
/// to the same values the current solobase-native uses.
pub struct InfraConfig {
    pub listen: String,        // SOLOBASE_LISTEN, default "0.0.0.0:8090"
    pub db_type: String,       // SOLOBASE_DB_TYPE, default "sqlite"
    pub db_path: String,       // SOLOBASE_DB_PATH, default "data/solobase.db"
    pub db_url: Option<String>,// SOLOBASE_DB_URL, default None
    pub storage_type: String,  // SOLOBASE_STORAGE_TYPE, default "local"
    pub storage_root: String,  // SOLOBASE_STORAGE_ROOT, default "data/storage"
}

impl InfraConfig {
    pub fn from_env() -> Self;
}

/// Collect env vars NOT prefixed `SOLOBASE_` as a key→value map, for
/// seeding into a config service.
pub fn collect_app_env_vars() -> std::collections::HashMap<String, String>;
```

### HTTP serve loop

```rust
/// Bind an HTTP listener on `addr`, dispatch requests through `wafer`'s
/// `site-main` flow via the `wafer-block-http-listener` block already
/// attached inside `wafer`, and await graceful shutdown on SIGINT / SIGTERM.
pub async fn serve(wafer: wafer_run::Wafer, addr: &str) -> anyhow::Result<()>;
```

Wraps today's inline `wafer.start_with_bind(addr)` + shutdown-signal handling.

### Deliberately NOT in the library

| Concern | Location |
|---|---|
| Reading/seeding the `variables` table | `solobase-server/src/config.rs` (app) |
| Reading `block_settings` table | `solobase-server/src/config.rs` (app) |
| CSP string | `solobase-server/src/main.rs` (app) |
| Per-block config JSON blobs (auth / email / llm / etc.) | `solobase-server/src/main.rs` (app) |
| `SolobaseBuilder::new()` chain | `solobase-server/src/main.rs` (app) |
| `builder::post_start(&wafer, &storage_block)` | `solobase-server/src/main.rs` (app) |

The library contains zero solobase-app knowledge — any native WAFER application can consume it without pulling in solobase's feature blocks.

## Migration Plan

Four atomic phases. Each is a single commit. `cargo check --workspace` stays green after each commit.

### Phase 1 — Scaffold `solobase-native` library crate

- Create `crates/solobase-native/Cargo.toml` (library, not binary).
- Create `crates/solobase-native/src/lib.rs` with a module tree:
  ```rust
  pub mod database;
  pub mod storage;
  pub mod network;
  pub mod crypto;
  pub mod logger;
  pub mod env;     // load_dotenv, collect_app_env_vars, InfraConfig
  pub mod tracing; // init_tracing
  pub mod serve;   // serve() fn
  pub use self::{
      database::make_sqlite_database_service,
      storage::make_local_storage_service,
      network::make_fetch_network_service,
      crypto::make_jwt_crypto_service,
      logger::make_tracing_logger,
      env::{collect_app_env_vars, load_dotenv, InfraConfig},
      tracing::init_tracing,
      serve::serve,
  };
  ```
- Empty module files for now; phase 2 fills them.
- Add to workspace members.
- **Naming collision concern:** a new crate named `solobase-native` conflicts with the existing binary crate at `crates/solobase-native/`. This phase therefore does two things in one atomic commit, in order:
  1. Rename directory `crates/solobase-native/` → `crates/solobase-server/`; update its `Cargo.toml` `name = "solobase-server"`; update the workspace `members` list.
  2. Create the fresh `crates/solobase-native/` (library) at the now-vacant path, with the scaffolded `Cargo.toml` + `src/lib.rs` module tree above.

Neither change is exposed to downstream code in this phase — the binary still has all its inlined wiring, the new library is empty — so the rename is pure bookkeeping and `cargo check --workspace` stays green.

### Phase 2 — Move platform-service factories + bootstrap helpers into the library

For each of {database, storage, network, crypto, logger, env, tracing, serve}: move the corresponding code from `solobase-server/src/main.rs` (+ `src/app_config.rs` for `InfraConfig`) into the matching module in `solobase-native/src/`. Update `solobase-server/src/main.rs` to import from `solobase_native::*` instead of having the logic inlined.

After this commit:
- `solobase-native` lib has all the new factories + helpers.
- `solobase-server` binary imports them; `main.rs` is roughly the same size as before but now reads as composition over library functions.
- Observable behavior: identical. `make -p solobase-server` produces a binary with the same runtime behavior.
- `app_config.rs` keeps `seed_and_load_variables` + `load_block_settings` (renamed to `src/config.rs` for symmetry with solobase-web/src/config.rs).

### Phase 3 — Thin out `solobase-server/src/main.rs`

Simplify `main.rs` to the ~60-line composer shown in Section 3 of the design. All bootstrap calls now flow through `solobase_native::*`. The solobase-specific composition chain (block_config calls, CSP, post_start, etc.) stays in `main.rs`.

This phase is mostly cosmetic — it reorganizes imports and removes now-redundant comments. But it's the commit that validates "the library exposes everything a thin app needs" and proves the boundary.

### Phase 4 — Structural parity check

Read `solobase-server/src/main.rs` next to `solobase-web/src/lib.rs`. Confirm the two are structurally isomorphic — same step ordering (dotenv/tracing → config load → JWT extract → SolobaseBuilder → block registration → start → post_start → serve). Any difference that isn't forced by the browser/native dichotomy is a boundary leak that needs fixing.

No code change expected in this phase if phases 1–3 landed cleanly. It's a design-audit commit: add a `docs/architecture/` note or a comment in both entrypoints cross-referencing each other. If a real mismatch is found, fix it here (a small last pass before this sub-project closes).

## Testing

- **Unit tests on factories.** For each `make_*_service()` function, a smoke test that constructs the service with sensible dummy args and asserts it returns a non-null `Arc<dyn ...>`. Mirror the bundler unit tests in `solobase-browser`. Native-only, so these run in `cargo test --workspace` without any wasm target.
- **Integration test for `serve()`.** A test binary under `crates/solobase-native/tests/serve_roundtrip.rs`:
  1. Constructs a minimal `Wafer` with a stub block serving `/health`.
  2. Calls `serve(wafer, "127.0.0.1:0")` in a spawned task, capturing the bound port via a oneshot channel.
  3. A test client sends `GET /health`.
  4. Asserts the response is what the stub returned.
  5. Sends a shutdown signal and awaits the task.

  This is the native equivalent of the Playwright smoke test. Catches regressions in the bind → dispatch → shutdown chain.
- **`solobase-server` end-to-end smoke.** After migration, `cargo run -p solobase-server` must start, bind the listener, respond to `GET /health`, and shut down cleanly. If CI currently exercises this, keep the job. If not (the existing CI only builds, doesn't run), this sub-project adds a `Server Smoke` CI job that starts the server in the background, curls `/health`, and kills it.

## Risks

- **Rename churn across docs / scripts.** Every mention of the `solobase-native` crate name in `.github/workflows/`, deployment Dockerfiles, Makefiles, and top-level docs needs to change to `solobase-server`. After phase 1's commit, run `grep -rn "solobase-native" .` across the repo and fix every hit in the same commit. The produced binary name is unchanged (`solobase`), so runtime deployment is unaffected; this is purely crate-name churn.
- **Feature-flag propagation.** Today's `solobase-native/Cargo.toml` has features (`sqlite`, `storage-local`, `storage-s3`, `postgres`, `otel`, `native-embedding`) that propagate to `solobase-core` and `solobase`. The new `solobase-native` library keeps the same features; the binary crate `solobase-server` inherits from the lib via `default-features = false` + explicit feature selection. Verify each feature compiles both alone and in combination after phase 2.
- **`S3Config` exposure.** The S3 factory takes a config struct. If `wafer-block-s3` exposes its own config type, we re-export it verbatim (`pub use wafer_block_s3::S3Config`). If it doesn't (unlikely), we define a small pass-through struct in `solobase-native/src/storage.rs`. Decide at implementation time, prefer re-export.
- **`tracing` module name collision.** Rust's `tracing` crate and `solobase-native`'s `pub mod tracing` could confuse readers. Consider renaming the module to `log_init` or `logging` to make the intent clearer. Non-blocking, but worth a second look during implementation.
- **LLM service absence on native.** Today's `solobase-native` has no LLM wiring. Phase D adds it for both targets symmetrically. Not a risk for Phase C; flagged so it doesn't surprise anyone reviewing this spec.

## Summary

Split the current `solobase-native` binary crate into a `solobase-native` library (factories for sqlite / postgres / local-storage / s3 / fetch-network / jwt-crypto / tracing-logger, plus `load_dotenv` / `init_tracing` / `InfraConfig` / `collect_app_env_vars` / `serve`) and a `solobase-server` binary (thin composer using `SolobaseBuilder`). Mirrors the `solobase-browser` / `solobase-web` split exactly. Four atomic migration phases, each keeping `cargo check --workspace` green. The produced `solobase` binary is unchanged; only the crate name changes. Unblocks Phase E's CLI by giving its code-generator a stable symmetric library surface to emit entrypoints against.
