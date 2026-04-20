# solobase-native Framework Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the current `solobase-native` binary crate into a new `solobase-native` library (platform-service factories + bootstrap helpers + serve/shutdown loop) plus a `solobase-server` binary (thin composer using `SolobaseBuilder`). Mirrors the browser-side `solobase-browser` / `solobase-web` split exactly.

**Architecture:** The library exposes explicit factory functions per service (`make_sqlite_database_service`, `make_local_storage_service`, `make_jwt_crypto_service`, etc.), bootstrap helpers (`load_dotenv`, `init_tracing`, `InfraConfig`, `collect_app_env_vars`), observability hook registration, and a two-part serve API (`register_http_listener` + `serve_until_shutdown`) that lets the app control the post-start ordering. The binary keeps its solobase-app knowledge (schema-specific SQLite seeding, `SolobaseBuilder` composition chain, per-block config JSON).

**Tech Stack:** Rust library + binary; `wafer-run`, `wafer-core`, `wafer-block-*` services; `tokio`, `tracing`, `tracing-subscriber`, `dotenvy`, `rusqlite`, `getrandom`, `uuid`. No wasm target in this sub-project.

**Spec:** `docs/superpowers/specs/2026-04-20-solobase-native-framework-design.md`

---

## File Structure

### New files (in `crates/solobase-native/` — the new library)

- `crates/solobase-native/Cargo.toml` — library crate manifest
- `crates/solobase-native/src/lib.rs` — public API + re-exports
- `crates/solobase-native/src/database.rs` — `make_sqlite_database_service`, feature-gated `make_postgres_database_service`
- `crates/solobase-native/src/storage.rs` — `make_local_storage_service`, feature-gated `make_s3_storage_service` + `S3Config` re-export
- `crates/solobase-native/src/network.rs` — `make_fetch_network_service`
- `crates/solobase-native/src/crypto.rs` — `make_jwt_crypto_service(secret)`
- `crates/solobase-native/src/logger.rs` — `make_tracing_logger`
- `crates/solobase-native/src/env.rs` — `load_dotenv`, `collect_app_env_vars`, `InfraConfig`
- `crates/solobase-native/src/log_init.rs` — `init_tracing` (renamed from `tracing.rs` to avoid collision with the `tracing` crate)
- `crates/solobase-native/src/hooks.rs` — `register_observability_hooks`
- `crates/solobase-native/src/serve.rs` — `register_http_listener`, `serve_until_shutdown`
- `crates/solobase-native/tests/serve_roundtrip.rs` — integration test

### Renamed files (`crates/solobase-native/` → `crates/solobase-server/`)

The entire contents of today's `crates/solobase-native/` move to `crates/solobase-server/`. Specifically:
- `crates/solobase-server/Cargo.toml` (renamed package)
- `crates/solobase-server/src/main.rs` (will be thinned to ~70 lines)
- `crates/solobase-server/src/config.rs` (moved and renamed from `src/app_config.rs`)

### Modified files

- Root `Cargo.toml` workspace `members` list: replace `"crates/solobase-native"` entry with two entries `"crates/solobase-native"` (new lib) and `"crates/solobase-server"`.
- `crates/solobase-server/src/main.rs` rewrites the composer using the new lib.
- `crates/solobase-server/Cargo.toml` gets a path-dep on `solobase-native = { path = "../solobase-native" }` and sheds direct deps on `wafer-block-sqlite`, `wafer-block-local-storage`, `wafer-block-network`, `wafer-block-crypto`, `wafer-block-logger`, `wafer-block-http-listener`, `dotenvy`, `tracing-subscriber`, `rusqlite` (the lib owns these now).
- Any repo-wide references to the `solobase-native` crate name in `.github/workflows/`, top-level `README.md`, deployment docs → update to `solobase-server`. The produced binary name stays `solobase` (unchanged `[[bin]] name = "solobase"`).

### Behavior adjustments from today

- Today's `collect_app_env_vars()` filters env vars against a known-key list pulled from `solobase-core::blocks::all_block_infos`. That coupling to solobase-core is wrong for the library: a generic WAFER native app doesn't have solobase's block set. **The lib version returns all env vars that are NOT prefixed `SOLOBASE_`.** The solobase-app's prior filter-by-declared-keys behavior moves to `solobase-server/src/config.rs` as `filter_to_declared_keys(env_vars)` and is applied in `main.rs` between `collect_app_env_vars()` and `seed_and_load_variables()`.
- Today `register_observability_hooks` is inline in `main.rs`. The lib exposes it via `solobase_native::register_observability_hooks(&mut wafer)`. No behavior change.
- Today `wafer-run/http-listener` registration + config are two inline lines in `main.rs`. The lib exposes both behind `solobase_native::register_http_listener(&mut wafer, listen_addr)`. No behavior change.
- Today shutdown signal handling is inline in `main.rs` as `shutdown_signal()`. The lib exposes `solobase_native::serve_until_shutdown(&wafer)` which awaits the signal and calls `wafer.shutdown()` itself. Semantically identical to today's two separate calls.

---

## Task 1: Rename existing `solobase-native` → `solobase-server`; scaffold empty `solobase-native` library

This task is one atomic commit covering both the rename and the new library scaffold. `cargo check --workspace` must stay green at the end.

**Files (all in one commit):**
- Move directory: `crates/solobase-native/` → `crates/solobase-server/`
- Modify: `crates/solobase-server/Cargo.toml` (rename `name` field)
- Modify: root `Cargo.toml` workspace members
- Create: `crates/solobase-native/Cargo.toml`
- Create: `crates/solobase-native/src/lib.rs`
- Create: `crates/solobase-native/src/{database,storage,network,crypto,logger,env,log_init,hooks,serve}.rs` (empty module files)

- [ ] **Step 1: Rename the directory with `git mv`**

```bash
git mv crates/solobase-native crates/solobase-server
```

- [ ] **Step 2: Update `crates/solobase-server/Cargo.toml` package name**

Change:
```toml
[package]
name = "solobase-native"
```
to:
```toml
[package]
name = "solobase-server"
```

No other field changes. In particular the `[[bin]]` block stays:
```toml
[[bin]]
name = "solobase"
path = "src/main.rs"
```

- [ ] **Step 3: Update the root workspace `Cargo.toml` members**

Open `Cargo.toml` at the repo root. Replace:
```toml
members = [
    "crates/solobase",
    "crates/solobase-browser",
    "crates/solobase-core",
    "crates/solobase-native",
    "crates/solobase-web",
    "examples/minimal-browser",
]
```
with:
```toml
members = [
    "crates/solobase",
    "crates/solobase-browser",
    "crates/solobase-core",
    "crates/solobase-native",
    "crates/solobase-server",
    "crates/solobase-web",
    "examples/minimal-browser",
]
```

(Keep alphabetical order.)

- [ ] **Step 4: Create the new `solobase-native` library Cargo.toml**

Path: `crates/solobase-native/Cargo.toml`

```toml
[package]
name = "solobase-native"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Native platform services + bootstrap helpers for WAFER-based server apps"

[features]
default = []
postgres = ["dep:wafer-block-postgres"]
s3 = ["dep:wafer-block-s3"]
otel = [
    "dep:opentelemetry",
    "dep:opentelemetry_sdk",
    "dep:opentelemetry-otlp",
    "dep:tracing-opentelemetry",
]

[dependencies]
anyhow = "1"

wafer-run = { workspace = true, features = ["full"] }
wafer-core = { workspace = true }

wafer-block-sqlite = { workspace = true }
wafer-block-local-storage = { workspace = true }
wafer-block-network = { workspace = true }
wafer-block-crypto = { workspace = true }
wafer-block-logger = { workspace = true }
wafer-block-http-listener = { workspace = true }

wafer-block-postgres = { workspace = true, optional = true }
wafer-block-s3 = { workspace = true, optional = true }

tokio = { workspace = true, features = ["full"] }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
opentelemetry = { workspace = true, optional = true }
opentelemetry_sdk = { workspace = true, optional = true }
opentelemetry-otlp = { workspace = true, optional = true }
tracing-opentelemetry = { workspace = true, optional = true }

dotenvy = { workspace = true }

serde_json = { workspace = true }
```

Note: this intentionally does NOT depend on `solobase`, `solobase-core`, `rusqlite`, `uuid`, `getrandom`, `sha2`, `hmac`, `chrono`. Those stay in `solobase-server` where they're used for solobase-app-specific schema work.

- [ ] **Step 5: Create the library root `src/lib.rs`**

```rust
//! Native platform services and bootstrap helpers for WAFER-based server
//! apps. Sibling to `solobase-browser`; provides the same shape (factory
//! functions per service, lightweight bootstrap helpers, a runtime /
//! serve layer) so a consumer's entrypoint looks structurally identical
//! across both targets.
//!
//! The library contains zero solobase-app-specific knowledge — app-level
//! schema work (reading/seeding `variables` tables, per-block config
//! JSON, SolobaseBuilder composition) lives in the consumer's binary.

pub mod crypto;
pub mod database;
pub mod env;
pub mod hooks;
pub mod log_init;
pub mod logger;
pub mod network;
pub mod serve;
pub mod storage;

pub use crypto::make_jwt_crypto_service;
pub use database::make_sqlite_database_service;
#[cfg(feature = "postgres")]
pub use database::make_postgres_database_service;
pub use env::{collect_app_env_vars, load_dotenv, InfraConfig};
pub use hooks::register_observability_hooks;
pub use log_init::init_tracing;
pub use logger::make_tracing_logger;
pub use network::make_fetch_network_service;
pub use serve::{register_http_listener, serve_until_shutdown};
pub use storage::make_local_storage_service;
#[cfg(feature = "s3")]
pub use storage::{make_s3_storage_service, S3Config};
```

- [ ] **Step 6: Create empty module files**

For each of `database.rs`, `storage.rs`, `network.rs`, `crypto.rs`, `logger.rs`, `env.rs`, `log_init.rs`, `hooks.rs`, `serve.rs` in `crates/solobase-native/src/`: create the file with only a single-line doc comment matching the module name, e.g.:

```rust
//! Database platform-service factories.
```

This lets `lib.rs`'s `pub mod database;` resolve while each module stays empty for Task 2 to fill.

- [ ] **Step 7: Verify the workspace still builds**

Run: `cargo check --workspace`
Expected: builds clean; emits unused-import warnings on the new library's `pub use` statements referencing empty modules (these will resolve in Task 2).

If `cargo check` fails because the `pub use` statements reference names that don't exist yet, temporarily comment out the `pub use` block in `lib.rs` for Step 7 and uncomment it in Task 2. The module declarations alone (`pub mod …;`) are enough for this task's invariant.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/solobase-native crates/solobase-server
git commit -m "refactor: rename solobase-native (bin) → solobase-server; scaffold solobase-native library

Prepares for sub-project 2 of the framework refactor. The existing
binary crate is renamed and its contents moved verbatim to
crates/solobase-server/. A new empty library crate takes its place
at crates/solobase-native/ with module skeletons for later tasks to
fill. The produced binary name (solobase) is unchanged.

No behavior change; cargo check --workspace stays green."
```

---

## Task 2: Move platform-service factories into the library (5 sub-commits)

Each sub-task is its own atomic commit: the source file moves to the library and the consumer (`solobase-server/src/main.rs`) updates its call site to import from the library. Do one service at a time so any regression bisects cleanly.

### Task 2a: Move SQLite database service factory

**Files:**
- Modify: `crates/solobase-native/src/database.rs`
- Modify: `crates/solobase-server/src/main.rs`

- [ ] **Step 1: Fill in `crates/solobase-native/src/database.rs`**

```rust
//! Database platform-service factories for native targets.
//!
//! - `make_sqlite_database_service(path)` — wraps `wafer-block-sqlite`
//!   `SQLiteDatabaseService`.
//! - `make_postgres_database_service(url)` — feature-gated on `postgres`.

use std::sync::Arc;

use wafer_core::interfaces::database::service::DatabaseService;

/// Open a SQLite database at `path` and wrap it in `Arc<dyn DatabaseService>`.
///
/// Panics if the file cannot be opened or created — same failure mode as
/// today's inline call. Consumers who want fallible construction can call
/// `wafer_block_sqlite::service::SQLiteDatabaseService::open(path)` directly.
pub fn make_sqlite_database_service(path: &str) -> Arc<dyn DatabaseService> {
    let svc = wafer_block_sqlite::service::SQLiteDatabaseService::open(path)
        .unwrap_or_else(|e| panic!("failed to open SQLite database at {path}: {e}"));
    Arc::new(svc)
}

/// Open a PostgreSQL connection via `url` and wrap it in
/// `Arc<dyn DatabaseService>`. Feature-gated.
#[cfg(feature = "postgres")]
pub fn make_postgres_database_service(
    url: &str,
) -> Arc<dyn DatabaseService> {
    let svc = wafer_block_postgres::service::PostgresDatabaseService::connect(url)
        .unwrap_or_else(|e| panic!("failed to connect to Postgres at {url}: {e}"));
    Arc::new(svc)
}
```

- [ ] **Step 2: Update `solobase-server/src/main.rs` to use the factory**

Locate:
```rust
.database(Arc::new(
    wafer_block_sqlite::service::SQLiteDatabaseService::open(&infra.db_path)
        .expect("failed to open SQLite database"),
))
```
Replace with:
```rust
.database(solobase_native::make_sqlite_database_service(&infra.db_path))
```

- [ ] **Step 3: Add `solobase-native` path-dep to `solobase-server/Cargo.toml`**

Add under `[dependencies]`:
```toml
solobase-native = { path = "../solobase-native" }
```

Do NOT yet remove `wafer-block-sqlite` from `solobase-server`'s dep list — later sub-tasks still reference it inline. Removal happens in Task 2f's cleanup pass.

- [ ] **Step 4: Build check**

Run: `cargo check -p solobase-server --no-default-features --features "sqlite,storage-local"`
Expected: clean build.

Run: `cargo check -p solobase-server --features postgres`
Expected: clean build (exercises the `postgres` feature path).

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-native/src/database.rs crates/solobase-server/Cargo.toml crates/solobase-server/src/main.rs
git commit -m "refactor(solobase-native): extract make_sqlite_database_service (+ postgres factory)"
```

### Task 2b: Move local storage service factory

**Files:**
- Modify: `crates/solobase-native/src/storage.rs`
- Modify: `crates/solobase-server/src/main.rs`

- [ ] **Step 1: Fill in `crates/solobase-native/src/storage.rs`**

```rust
//! Storage platform-service factories for native targets.

use std::sync::Arc;

use wafer_core::interfaces::storage::service::StorageService;

/// Initialise a local-filesystem storage service rooted at `root`.
pub fn make_local_storage_service(root: &str) -> Arc<dyn StorageService> {
    let svc = wafer_block_local_storage::service::LocalStorageService::new(root)
        .unwrap_or_else(|e| panic!("failed to init local storage at {root}: {e}"));
    Arc::new(svc)
}

#[cfg(feature = "s3")]
pub use wafer_block_s3::S3Config;

/// Construct an S3-backed storage service. Feature-gated.
#[cfg(feature = "s3")]
pub fn make_s3_storage_service(config: S3Config) -> Arc<dyn StorageService> {
    let svc = wafer_block_s3::service::S3StorageService::new(config)
        .unwrap_or_else(|e| panic!("failed to init S3 storage: {e}"));
    Arc::new(svc)
}
```

If `wafer_block_s3::S3Config` doesn't exist verbatim (grep to confirm before writing), adjust the `pub use` to whichever type the constructor accepts. Do NOT invent a new type — re-export whatever upstream already ships.

- [ ] **Step 2: Update `solobase-server/src/main.rs`**

Locate:
```rust
.storage(Arc::new(
    wafer_block_local_storage::service::LocalStorageService::new(&infra.storage_root)
        .expect("failed to create local storage service"),
))
```
Replace with:
```rust
.storage(solobase_native::make_local_storage_service(&infra.storage_root))
```

- [ ] **Step 3: Build check**

Run: `cargo check -p solobase-server`
Expected: clean.

Run: `cargo check -p solobase-server --features storage-s3`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-native/src/storage.rs crates/solobase-server/src/main.rs
git commit -m "refactor(solobase-native): extract make_local_storage_service (+ s3 factory)"
```

### Task 2c: Move network, crypto, logger factories

These three are identical in shape (no feature gates, single `Arc::new(...)` wrap), so they land together.

**Files:**
- Modify: `crates/solobase-native/src/network.rs`
- Modify: `crates/solobase-native/src/crypto.rs`
- Modify: `crates/solobase-native/src/logger.rs`
- Modify: `crates/solobase-server/src/main.rs`

- [ ] **Step 1: Fill in `crates/solobase-native/src/network.rs`**

```rust
//! Network platform-service factory for native targets.

use std::sync::Arc;

use wafer_core::interfaces::network::service::NetworkService;

/// Construct an HTTP network service backed by `reqwest`.
pub fn make_fetch_network_service() -> Arc<dyn NetworkService> {
    Arc::new(wafer_block_network::service::HttpNetworkService::new())
}
```

- [ ] **Step 2: Fill in `crates/solobase-native/src/crypto.rs`**

```rust
//! Crypto platform-service factory for native targets.

use std::sync::Arc;

use wafer_core::interfaces::crypto::service::CryptoService;

/// Construct a CryptoService seeded with `jwt_secret`. Argon2-backed on
/// native (see `wafer_block_crypto::service::Argon2JwtCryptoService`).
pub fn make_jwt_crypto_service(jwt_secret: String) -> Arc<dyn CryptoService> {
    Arc::new(
        wafer_block_crypto::service::Argon2JwtCryptoService::new(jwt_secret),
    )
}
```

- [ ] **Step 3: Fill in `crates/solobase-native/src/logger.rs`**

```rust
//! Logger platform-service factory for native targets.

use std::sync::Arc;

use wafer_core::interfaces::logger::service::LoggerService;

/// Construct a LoggerService that emits via the `tracing` crate. Consumers
/// should call `init_tracing(format)` once at startup to install a
/// tracing subscriber (the logger alone does not install one).
pub fn make_tracing_logger() -> Arc<dyn LoggerService> {
    Arc::new(wafer_block_logger::service::TracingLogger)
}
```

- [ ] **Step 4: Update `solobase-server/src/main.rs`**

Three call-site replacements:

```rust
.network(Arc::new(wafer_block_network::service::HttpNetworkService::new()))
```
→
```rust
.network(solobase_native::make_fetch_network_service())
```

```rust
.crypto(Arc::new(wafer_block_crypto::service::Argon2JwtCryptoService::new(jwt_secret)))
```
→
```rust
.crypto(solobase_native::make_jwt_crypto_service(jwt_secret))
```

```rust
.logger(Arc::new(wafer_block_logger::service::TracingLogger))
```
→
```rust
.logger(solobase_native::make_tracing_logger())
```

- [ ] **Step 5: Build check**

Run: `cargo check -p solobase-server`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/solobase-native/src/{network,crypto,logger}.rs crates/solobase-server/src/main.rs
git commit -m "refactor(solobase-native): extract make_fetch_network_service / make_jwt_crypto_service / make_tracing_logger"
```

---

## Task 3: Move bootstrap helpers into the library

### Task 3a: `load_dotenv` + `collect_app_env_vars` + `InfraConfig`

**Files:**
- Modify: `crates/solobase-native/src/env.rs`
- Modify: `crates/solobase-server/src/main.rs`
- Create: `crates/solobase-server/src/config.rs` (replaces `src/app_config.rs`)
- Delete: `crates/solobase-server/src/app_config.rs`

- [ ] **Step 1: Fill in `crates/solobase-native/src/env.rs`**

```rust
//! Environment-variable bootstrap helpers for native WAFER apps.

use std::collections::HashMap;

/// Load `.env`. Honors `SOLOBASE_ENV_FILE` for an explicit path; otherwise
/// auto-detects `.env` in the current working directory. Failures on the
/// explicit-path form are logged to stderr but do not abort.
pub fn load_dotenv() {
    if let Ok(path) = std::env::var("SOLOBASE_ENV_FILE") {
        match dotenvy::from_filename(&path) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("warning: failed to load env file '{path}': {e}");
            }
        }
        return;
    }
    let _ = dotenvy::dotenv();
}

/// Collect env vars that are NOT prefixed `SOLOBASE_`. These are the
/// app-level config vars the consumer may want to seed into a config
/// service or a `variables` table.
///
/// Consumers who want additional filtering (e.g., only env vars that
/// match declared config var keys) should apply their own filter on top
/// of this result.
pub fn collect_app_env_vars() -> HashMap<String, String> {
    std::env::vars()
        .filter(|(k, _)| !k.starts_with("SOLOBASE_"))
        .collect()
}

/// Infrastructure config read from `SOLOBASE_*` env vars.
pub struct InfraConfig {
    pub listen: String,
    pub db_type: String,
    pub db_path: String,
    pub db_url: Option<String>,
    pub storage_type: String,
    pub storage_root: String,
}

impl InfraConfig {
    pub fn from_env() -> Self {
        Self {
            listen: env_or("SOLOBASE_LISTEN", "0.0.0.0:8090"),
            db_type: env_or("SOLOBASE_DB_TYPE", "sqlite"),
            db_path: env_or("SOLOBASE_DB_PATH", "data/solobase.db"),
            db_url: std::env::var("SOLOBASE_DB_URL").ok(),
            storage_type: env_or("SOLOBASE_STORAGE_TYPE", "local"),
            storage_root: env_or("SOLOBASE_STORAGE_ROOT", "data/storage"),
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
```

Note: the library function returns `HashMap<String, String>`, not `Vec<(String, String)>`. Today's `solobase-server` uses `Vec<(String, String)>` because it filters by declared-keys inside the function (which keeps insertion order). The new shape is simpler and the consumer can convert if needed (`vec.into_iter().collect::<Vec<_>>()`).

- [ ] **Step 2: Create `crates/solobase-server/src/config.rs`**

Copy the contents of today's `crates/solobase-server/src/app_config.rs` into `crates/solobase-server/src/config.rs` verbatim, then PREPEND this new helper at the top of `config.rs`:

```rust
//! App-specific config loading (schema-aware, depends on solobase-core).
//!
//! `filter_to_declared_keys` sits between the library's raw-env-var
//! collection and the SQLite-backed variable seeding, preserving the
//! prior behavior of only persisting env vars that match a declared
//! block/shared config var key.

use std::collections::HashMap;

pub fn filter_to_declared_keys(
    env_vars: HashMap<String, String>,
) -> Vec<(String, String)> {
    let block_infos = solobase_core::blocks::all_block_infos();
    let all_vars = solobase_core::config_vars::collect_all_config_vars(&block_infos);
    let known: std::collections::HashSet<String> =
        all_vars.iter().map(|v| v.key.clone()).collect();
    env_vars
        .into_iter()
        .filter(|(k, _)| known.contains(k))
        .collect()
}

// … existing contents of app_config.rs appended verbatim below …
```

- [ ] **Step 3: Update `solobase-server/src/main.rs`**

Change the `mod app_config;` line to `mod config;`.

Change:
```rust
use app_config::{load_block_settings, InfraConfig};
```
to:
```rust
use config::{filter_to_declared_keys, load_block_settings};
use solobase_native::{collect_app_env_vars, load_dotenv, InfraConfig};
```

Replace the inline `load_dotenv` fn with nothing (gone — imported from lib). Delete the `fn load_dotenv()` definition and its `// --- .env loading ---` section from `main.rs`.

Replace the inline `collect_app_env_vars` fn with nothing. Delete it + its `// --- known-key filter ---` section.

Replace the call site:
```rust
let env_vars = collect_app_env_vars();
```
with:
```rust
let env_vars = filter_to_declared_keys(collect_app_env_vars());
```

- [ ] **Step 4: Delete `crates/solobase-server/src/app_config.rs`**

```bash
git rm crates/solobase-server/src/app_config.rs
```

(Its contents are now in `config.rs`.)

- [ ] **Step 5: Build check**

Run: `cargo check -p solobase-server`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/solobase-native/src/env.rs crates/solobase-server/src/main.rs crates/solobase-server/src/config.rs
git commit -m "refactor(solobase-native): extract load_dotenv / collect_app_env_vars / InfraConfig"
```

### Task 3b: `init_tracing` (including optional OpenTelemetry)

**Files:**
- Modify: `crates/solobase-native/src/log_init.rs`
- Modify: `crates/solobase-server/src/main.rs`

- [ ] **Step 1: Fill in `crates/solobase-native/src/log_init.rs`**

Copy the `init_tracing` fn and its `#[cfg(feature = "otel")]` helper `init_tracing_with_otel` from today's `main.rs` verbatim, into `log_init.rs`, prefixed by:

```rust
//! `tracing` / `tracing-subscriber` initialisation helper.
//!
//! Called once at startup to install a tracing subscriber. Supports
//! `text` and `json` formats. OpenTelemetry OTLP export is enabled by
//! the `otel` feature and auto-activates when
//! `OTEL_EXPORTER_OTLP_ENDPOINT` is set.

use tracing_subscriber::{fmt, EnvFilter};

pub fn init_tracing(log_format: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wafer=debug,solobase=debug"));

    #[cfg(feature = "otel")]
    {
        if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
            init_tracing_with_otel(log_format, filter);
            return;
        }
    }

    if log_format == "json" {
        fmt()
            .json()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(false)
            .init();
    } else {
        fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(false)
            .init();
    }
}

#[cfg(feature = "otel")]
fn init_tracing_with_otel(log_format: &str, filter: EnvFilter) {
    use opentelemetry::trace::TracerProvider;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("failed to create OTLP span exporter");

    let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "solobase".into());
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name),
        ]))
        .build();

    let tracer = provider.tracer("solobase");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let fmt_layer: Box<dyn Layer<_> + Send + Sync> = if log_format == "json" {
        Box::new(fmt::layer().json().with_target(true).with_thread_ids(false))
    } else {
        Box::new(fmt::layer().with_target(true).with_thread_ids(false))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    tracing::info!("OpenTelemetry tracing enabled");
}
```

Note: the `"solobase"` literal in `OTEL_SERVICE_NAME` default + `provider.tracer("solobase")` is arguably solobase-app-specific, but it's only used as a fallback when the env var isn't set. Keep as-is — consumers using this library who care about service-name branding will set `OTEL_SERVICE_NAME` themselves.

- [ ] **Step 2: Update `solobase-server/src/main.rs`**

Change:
```rust
use tracing_subscriber::{fmt, EnvFilter};
```
to:
```rust
use solobase_native::init_tracing;
```

Delete the inline `fn init_tracing(...)` and `fn init_tracing_with_otel(...)` functions + their `// --- Tracing init ---` section from `main.rs`.

Call-site (already calls `init_tracing(&log_format)`) stays unchanged.

- [ ] **Step 3: Build check**

Run: `cargo check -p solobase-server`
Expected: clean.

Run: `cargo check -p solobase-server --features otel`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-native/src/log_init.rs crates/solobase-server/src/main.rs
git commit -m "refactor(solobase-native): extract init_tracing (+ otel path)"
```

### Task 3c: Observability hooks

**Files:**
- Modify: `crates/solobase-native/src/hooks.rs`
- Modify: `crates/solobase-server/src/main.rs`

- [ ] **Step 1: Fill in `crates/solobase-native/src/hooks.rs`**

```rust
//! Observability hooks wired into a `Wafer` instance: logs every flow
//! start / block end / flow end via the `tracing` crate. Call once after
//! `Wafer` construction, before `wafer.start()`.

use wafer_run::Wafer;

pub fn register_observability_hooks(wafer: &mut Wafer) {
    wafer.hooks.on_flow_start(|flow_id, _msg| {
        tracing::info_span!("flow", flow = %flow_id).in_scope(|| {});
    });

    wafer.hooks.on_block_end(|obs_ctx, duration| {
        tracing::debug!(
            flow   = %obs_ctx.flow_id,
            block  = %obs_ctx.block_name,
            trace  = %obs_ctx.trace_id,
            ms     = duration.as_millis() as u64,
            "block executed"
        );
    });

    wafer.hooks.on_flow_end(|flow_id, duration| {
        tracing::info!(
            flow   = %flow_id,
            ms     = duration.as_millis() as u64,
            "flow completed"
        );
    });
}
```

- [ ] **Step 2: Update `solobase-server/src/main.rs`**

Change the call:
```rust
register_observability_hooks(&mut wafer);
```
to:
```rust
solobase_native::register_observability_hooks(&mut wafer);
```

Delete the inline `fn register_observability_hooks(...)` from `main.rs`.

- [ ] **Step 3: Build check**

Run: `cargo check -p solobase-server`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-native/src/hooks.rs crates/solobase-server/src/main.rs
git commit -m "refactor(solobase-native): extract register_observability_hooks"
```

### Task 3d: Serve / shutdown loop

**Files:**
- Modify: `crates/solobase-native/src/serve.rs`
- Modify: `crates/solobase-server/src/main.rs`

- [ ] **Step 1: Fill in `crates/solobase-native/src/serve.rs`**

```rust
//! HTTP listener registration + graceful-shutdown helpers for native
//! WAFER apps.
//!
//! The shape is deliberately two-phase: `register_http_listener` attaches
//! the `wafer-run/http-listener` block before start; `serve_until_shutdown`
//! awaits a ctrl-c / SIGTERM signal and shuts the runtime down. Splitting
//! them lets the consumer run post-start hooks (e.g., WRAP grant
//! injection) between `wafer.start()` and the shutdown wait.

use wafer_run::Wafer;

/// Register the `wafer-run/http-listener` block on `wafer` and configure
/// it to bind `listen_addr` and dispatch through the `site-main` flow.
/// Must be called before `wafer.start()`.
pub fn register_http_listener(wafer: &mut Wafer, listen_addr: &str) {
    wafer_block_http_listener::register(wafer)
        .expect("register http-listener block");
    wafer.add_block_config(
        "wafer-run/http-listener",
        serde_json::json!({ "flow": "site-main", "listen": listen_addr }),
    );
}

/// Await a graceful-shutdown signal (ctrl-c or SIGTERM on Unix), then call
/// `wafer.shutdown().await`. Returns after the shutdown completes.
pub async fn serve_until_shutdown(wafer: &wafer_run::StartedWafer) {
    shutdown_signal().await;
    wafer.shutdown().await;
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received Ctrl+C — shutting down"),
        _ = terminate => tracing::info!("received SIGTERM — shutting down"),
    }
}
```

Note the type `wafer_run::StartedWafer` in `serve_until_shutdown`'s signature is the return of `wafer.start().await`. Grep `wafer-run` for the exact type name (it may be `Wafer`, `StartedWafer`, `RunningWafer`, etc.) and use the one that matches. Today's `main.rs` does `let wafer = wafer.start().await.expect(...)` then `wafer.shutdown().await`, so the type is whatever `.start()` returns. Prefer type-inference if the name is awkward: take the argument as `impl AsyncShutdown` or just use the concrete type found via grep.

- [ ] **Step 2: Update `solobase-server/src/main.rs`**

Three changes:

Replace:
```rust
wafer_block_http_listener::register(&mut wafer).expect("register http-listener");
wafer.add_block_config(
    "wafer-run/http-listener",
    serde_json::json!({ "flow": "site-main", "listen": infra.listen }),
);
```
with:
```rust
solobase_native::register_http_listener(&mut wafer, &infra.listen);
```

Replace:
```rust
shutdown_signal().await;

// 14. Graceful shutdown
wafer.shutdown().await;
```
with:
```rust
solobase_native::serve_until_shutdown(&wafer).await;
```

Delete the inline `async fn shutdown_signal()` from `main.rs`.

- [ ] **Step 3: Build check**

Run: `cargo check -p solobase-server`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-native/src/serve.rs crates/solobase-server/src/main.rs
git commit -m "refactor(solobase-native): extract register_http_listener + serve_until_shutdown"
```

---

## Task 4: Prune `solobase-server/Cargo.toml` of now-indirect deps

**Files:**
- Modify: `crates/solobase-server/Cargo.toml`

- [ ] **Step 1: Remove deps that are now transitive via `solobase-native`**

Delete these entries from `[dependencies]`:

- `wafer-block-sqlite`
- `wafer-block-local-storage`
- `wafer-block-network`
- `wafer-block-crypto`
- `wafer-block-logger`
- `wafer-block-http-listener`
- `wafer-block-s3` (under `[dependencies]` with `optional = true`) — now gated from the lib's `s3` feature
- `wafer-block-postgres` — same pattern
- `dotenvy`
- `tracing-subscriber`
- `opentelemetry` / `opentelemetry_sdk` / `opentelemetry-otlp` / `tracing-opentelemetry` — all lib-side now

Keep:
- `solobase` — app uses `SolobaseBuilder`
- `solobase-core` — `filter_to_declared_keys` + `load_block_settings` + `load_wrap_grants` need it
- `solobase-native` — new framework lib
- `wafer-run` — `Wafer` type
- `wafer-core` — `ConfigService` trait
- `wafer-block-config` — `EnvConfigService` constructor
- `tokio` — `#[tokio::main]`
- `tracing` — macro calls (`tracing::info!`, etc.) in `main.rs`
- `rusqlite` — direct SQLite access in `config.rs`
- `serde` / `serde_json` — JSON block configs
- `sha2` / `hmac` — if still used by `config.rs` (grep before removing)
- `getrandom` — used by `seed_auto_generated` in `config.rs`
- `chrono` — check usages in `config.rs`
- `uuid` — used by `seed_and_load_variables` for var IDs

- [ ] **Step 2: Propagate feature flags**

`solobase-server`'s `[features]` block currently has:
```toml
default = ["sqlite", "storage-local"]
sqlite = ["solobase-core/sqlite"]
storage-local = ["solobase-core/storage-local"]
otel = [ ... opentelemetry deps ... ]
storage-s3 = ["solobase-core/storage-s3", "dep:wafer-block-s3"]
postgres = ["solobase-core/postgres", "dep:wafer-block-postgres"]
native-embedding = ["solobase/native-embedding"]
```

Replace with:
```toml
default = ["sqlite", "storage-local"]
sqlite = ["solobase-core/sqlite"]
storage-local = ["solobase-core/storage-local"]
otel = ["solobase-native/otel"]
storage-s3 = ["solobase-core/storage-s3", "solobase-native/s3"]
postgres = ["solobase-core/postgres", "solobase-native/postgres"]
native-embedding = ["solobase/native-embedding"]
```

(`sqlite` and `storage-local` are solobase-core-side feature toggles that don't have direct lib-side analogues — the lib always compiles SQLite and local-storage factories regardless of features. If that's undesirable because of binary size, move those to feature-gated factories too in a later pass. For now they're unconditional.)

- [ ] **Step 3: Build check**

```bash
cargo check -p solobase-server --no-default-features --features "sqlite,storage-local"
cargo check -p solobase-server --all-features
cargo check -p solobase-server --features postgres
cargo check -p solobase-server --features storage-s3
```
All four should pass.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-server/Cargo.toml
git commit -m "refactor(solobase-server): drop now-indirect deps; propagate features through solobase-native"
```

---

## Task 5: Library unit-test smokes

**Files:**
- Create: `crates/solobase-native/tests/factory_smoke.rs`

- [ ] **Step 1: Write the integration test**

```rust
//! Smoke tests: each `make_*_service` factory returns a non-null
//! `Arc<dyn ...>`. These don't exercise the underlying service in
//! depth — they just catch compile-time type-signature regressions
//! and confirm the factory hands back something usable.

use std::path::PathBuf;

#[test]
fn sqlite_factory_returns_service() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("smoke.db");
    let _svc = solobase_native::make_sqlite_database_service(path.to_str().unwrap());
    // If we got here, the factory worked.
}

#[test]
fn local_storage_factory_returns_service() {
    let tmp = tempfile::tempdir().unwrap();
    let _svc = solobase_native::make_local_storage_service(tmp.path().to_str().unwrap());
}

#[test]
fn network_factory_returns_service() {
    let _svc = solobase_native::make_fetch_network_service();
}

#[test]
fn crypto_factory_returns_service() {
    let _svc = solobase_native::make_jwt_crypto_service("smoke-test-secret".to_string());
}

#[test]
fn logger_factory_returns_service() {
    let _svc = solobase_native::make_tracing_logger();
}

#[test]
fn infra_config_reads_defaults_when_env_unset() {
    // Don't mutate env globally; just assert the struct builds.
    let _cfg = solobase_native::InfraConfig::from_env();
}

#[test]
fn collect_app_env_vars_excludes_solobase_prefix() {
    // SAFETY: this test runs in its own test binary; we're only
    // setting vars for the duration of the test. Still, keep it
    // serial-safe by using a unique prefix unlikely to clash.
    std::env::set_var("SOLOBASE_NATIVE_TEST_INFRA", "1");
    std::env::set_var("SOLOBASE_NATIVE_TEST_APP", "2");
    std::env::set_var("SOLOBASE_NATIVE_TEST_OTHER__VAR", "3");

    let vars = solobase_native::collect_app_env_vars();
    assert!(!vars.contains_key("SOLOBASE_NATIVE_TEST_INFRA"));
    assert!(!vars.contains_key("SOLOBASE_NATIVE_TEST_APP"));
    assert!(!vars.contains_key("SOLOBASE_NATIVE_TEST_OTHER__VAR"));

    std::env::remove_var("SOLOBASE_NATIVE_TEST_INFRA");
    std::env::remove_var("SOLOBASE_NATIVE_TEST_APP");
    std::env::remove_var("SOLOBASE_NATIVE_TEST_OTHER__VAR");
}
```

- [ ] **Step 2: Add `tempfile` as a dev-dep on solobase-native**

In `crates/solobase-native/Cargo.toml`:
```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p solobase-native --test factory_smoke`
Expected: all 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-native/tests/factory_smoke.rs crates/solobase-native/Cargo.toml
git commit -m "test(solobase-native): factory smoke tests + collect_app_env_vars filter"
```

---

## Task 6: `serve()` round-trip integration test

**Files:**
- Create: `crates/solobase-native/tests/serve_roundtrip.rs`

- [ ] **Step 1: Write the integration test**

```rust
//! End-to-end test: register_http_listener + serve_until_shutdown flow.
//!
//! We build a minimal Wafer with a stub block that returns a fixed
//! response for `/health`, call register_http_listener on an ephemeral
//! port, start the runtime, GET /health, then send a shutdown signal
//! and await completion.

use std::{sync::Arc, time::Duration};

use tokio::time::timeout;

// SAFETY: this test uses a port that's assigned by the OS (':0') so it
// doesn't conflict with a parallel test run on the same host.

#[tokio::test]
async fn health_endpoint_responds_then_shutdown_completes() {
    // Wafer construction will need updating once we know the exact
    // minimal API. For now this test exercises `register_http_listener`
    // + `serve_until_shutdown` structurally.
    //
    // If the integration is non-trivial because of wafer-run's builder
    // surface, land this test as a `#[ignore]` marker first and
    // un-ignore once serve.rs stabilises. See Task 6's discussion in
    // the plan for the shape of the minimal wafer.

    let mut wafer = wafer_run::Wafer::new();

    // Insert a stub flow that responds to /health. The exact mechanism
    // depends on wafer-run's public API — pick whichever of these
    // works:
    //  a) wafer.add_flow_json(r#"{"id":"site-main","steps":[{"block":"wafer-run/static","config":{"body":"ok"}}]}"#);
    //  b) wafer.register_block("test/health", Arc::new(HealthBlock));
    //     + route registration via add_block_config for a router block
    //
    // The goal is: serve returns an HTTP 200 with body "ok" for GET /health.

    solobase_native::register_http_listener(&mut wafer, "127.0.0.1:0");

    // Start runtime
    let wafer = wafer.start().await.expect("start");

    // Spawn the serve loop; it will await a shutdown signal.
    let wafer_for_serve = wafer.clone(); // whatever cloning is available
    let serve_handle = tokio::spawn(async move {
        solobase_native::serve_until_shutdown(&wafer_for_serve).await;
    });

    // The exact port is discoverable via wafer's http-listener block
    // introspection. If there's no API for that yet, bind an explicit
    // port like "127.0.0.1:18090" instead of ":0" and document the
    // risk in a comment.

    // GET /health
    let port = /* derive from wafer introspection, or fall back to known port */ 18090;
    let url = format!("http://127.0.0.1:{port}/health");
    let resp = timeout(Duration::from_secs(3), reqwest::get(&url))
        .await
        .expect("health check timed out")
        .expect("failed to GET /health");
    assert!(resp.status().is_success(), "/health returned {}", resp.status());

    // Trigger shutdown by sending SIGTERM to our own process or by
    // calling wafer.shutdown() directly in this test instead of
    // waiting on a signal. For test ergonomics, a manual shutdown
    // call is preferable to signal injection.
    wafer.shutdown().await;

    serve_handle.await.ok();
}
```

**Honesty note**: this test is sketchier than the rest of the plan because the exact shape depends on `wafer-run`'s public API for building a trivial `Wafer` with a test block. If that API isn't ergonomic, the test may need a more involved setup (a custom block crate, or using an existing block like `wafer-run/static` if one exists). During implementation:

- Run `cargo doc -p wafer-run --open` and identify the minimal-viable path to register a block that serves a fixed response.
- Prefer reusing a test-block pattern from `wafer-run`'s own integration tests if one exists (grep `wafer-run/crates/wafer-run/tests/`).
- If the round-trip shape is too involved for a single test, split: test (a) `register_http_listener` attaches the block + config (no server start); test (b) `serve_until_shutdown` returns after the runtime shuts down (no HTTP involved, just the wait-and-shutdown path).

If the integration test turns into yak-shaving, **mark it `#[ignore]` with a comment pointing back to this plan** and proceed; the unit smokes in Task 5 + the end-to-end `make run` check in Task 7 are sufficient validation. Don't block the refactor on a perfect test harness.

- [ ] **Step 2: Add `reqwest` as a dev-dep on solobase-native**

```toml
[dev-dependencies]
tempfile = "3"
reqwest = { version = "0.12", features = ["blocking"], default-features = false }
```

- [ ] **Step 3: Attempt to run the test**

Run: `cargo test -p solobase-native --test serve_roundtrip`
Expected: one of (a) pass, (b) `#[ignore]`d with a comment pointing at wafer-run API limitations, (c) the split fallback described in the honesty note.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-native/tests/serve_roundtrip.rs crates/solobase-native/Cargo.toml
git commit -m "test(solobase-native): serve round-trip integration test"
```

---

## Task 7: Rename-hunt sweep + final verification

**Files:**
- Modify: any file outside the crate tree that references the old `solobase-native` crate name

- [ ] **Step 1: Grep the repo for stale crate-name references**

```bash
grep -rn "solobase-native" \
  --include="*.toml" \
  --include="*.md" \
  --include="*.yml" \
  --include="*.yaml" \
  --include="Makefile" \
  --include="Dockerfile*" \
  .github docs README.md 2>/dev/null \
  | grep -v "docs/superpowers" \
  | grep -v "crates/solobase-native"
```

Expected: any hits that are neither inside the new lib crate nor inside documented design specs. Each hit is a location that should reference `solobase-server` instead (assuming it meant the binary). If a hit explicitly means the new library, keep it.

- [ ] **Step 2: Fix each reference**

For each match, replace `solobase-native` with `solobase-server` if the context refers to the binary crate. Leave library references as-is.

Particular files to inspect:
- `.github/workflows/ci.yml` (if it mentions `-p solobase-native` or `cargo run -p solobase-native`)
- Root `README.md` (if it has build / run commands)
- Any `Dockerfile` or deployment script
- Any `Makefile` target

- [ ] **Step 3: Full workspace verification**

Run:
```bash
cargo check --workspace
cargo test --workspace --exclude solobase-web
cargo +nightly fmt --all -- --check
```
All three should pass.

- [ ] **Step 4: End-to-end smoke**

```bash
# Build + start the server against a throwaway DB
cargo build -p solobase-server --release
SOLOBASE_DB_PATH=/tmp/solobase-smoke.db \
SOLOBASE_STORAGE_ROOT=/tmp/solobase-smoke-storage \
SOLOBASE_LISTEN=127.0.0.1:18090 \
./target/release/solobase &
SOLOBASE_PID=$!
sleep 2
curl -sI http://127.0.0.1:18090/health | head -1  # expect HTTP/1.1 200 (or similar success line)
kill -TERM $SOLOBASE_PID
wait $SOLOBASE_PID || true
rm -rf /tmp/solobase-smoke.db /tmp/solobase-smoke-storage
```

Expected: curl returns success line within a couple seconds; the binary exits cleanly after SIGTERM.

- [ ] **Step 5: Commit (if any renames happened)**

```bash
git add .
git commit -m "chore: sweep stale solobase-native → solobase-server crate-name references"
```

If there are no changes, skip the commit.

---

## Self-Review Checklist

- [ ] **Spec coverage:**
  - Crate split (new `solobase-native` lib + renamed `solobase-server` bin) → Task 1
  - Service factories (`make_sqlite_database_service` etc.) → Tasks 2a-c
  - Bootstrap helpers (`load_dotenv`, `collect_app_env_vars`, `InfraConfig`, `init_tracing`) → Tasks 3a, 3b
  - Observability hooks → Task 3c
  - `register_http_listener` + `serve_until_shutdown` → Task 3d
  - Dep prune on `solobase-server/Cargo.toml` → Task 4
  - Factory smoke tests → Task 5
  - `serve` round-trip integration test → Task 6
  - Rename sweep + final verification → Task 7
  - Feature-flag propagation (postgres / s3 / otel) → Task 4 Step 2
- [ ] **Placeholder scan:** no "TBD"; every code step has full code. Task 6 is honestly flagged as possibly needing `#[ignore]` with a documented fallback, not a TBD.
- [ ] **Type consistency:** `InfraConfig`, `make_sqlite_database_service(path: &str)`, etc. are used with identical signatures between the lib definitions (Tasks 2-3) and the call sites (Tasks 2-3) and the smoke tests (Task 5).
- [ ] **`wafer_run::StartedWafer`:** the exact post-start type name is flagged in Task 3d for grep-confirmation. Not a blocker; implementer adjusts on contact.
- [ ] **`wafer_block_s3::S3Config`:** flagged in Task 2b for grep-confirmation. Same handling.
- [ ] **Atomicity:** each task ends with a green `cargo check`. Bisectable.
