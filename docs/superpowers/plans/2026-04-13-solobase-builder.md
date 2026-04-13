# SolobaseBuilder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate duplicated WAFER runtime setup across native, browser WASM, and Cloudflare Worker platforms by introducing a `SolobaseBuilder` in the `solobase` crate, making `solobase` fully platform-agnostic, and moving native-specific code to a new `solobase-native` crate.

**Architecture:** The `SolobaseBuilder` accepts 6 platform-specific service trait objects and performs all common registration (service blocks, middleware, feature blocks, router, flow). Each platform crate shrinks to ~15 lines of service wiring. WRAP grant injection into the storage block is handled via a `post_start()` helper using thread-local storage.

**Tech Stack:** Rust, wafer-run, wafer-core, solobase-core, wasm-bindgen (solobase-web), worker crate (solobase-cloud)

**Spec:** `docs/superpowers/specs/2026-04-13-solobase-builder-design.md`

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `solobase/crates/solobase/src/builder.rs` | `SolobaseBuilder` struct + `post_start()` helper |
| `solobase/crates/solobase-native/Cargo.toml` | Native binary crate — tokio, rusqlite, tracing, native service impls |
| `solobase/crates/solobase-native/src/main.rs` | Binary entry point — uses builder |
| `solobase/crates/solobase-native/src/app_config.rs` | InfraConfig, load_block_settings, load_wrap_grants (moved from solobase) |

### Modified Files

| File | Change |
|------|--------|
| `solobase/crates/solobase/src/lib.rs` | Add `pub mod builder`, remove `pub mod app_config` |
| `solobase/crates/solobase/Cargo.toml` | Remove server feature, tokio, rusqlite, bin section; make middleware blocks non-optional |
| `solobase/Cargo.toml` | Add `solobase-native` to workspace members |
| `solobase/crates/solobase-web/src/lib.rs` | Replace ~150 lines of manual registration with builder call |
| `solobase/crates/solobase-web/Cargo.toml` | Remove middleware block deps (now pulled via `solobase`) |
| `solobase-cloud/crates/solobase-worker/src/lib.rs` | Replace ~150 lines of manual registration with builder call |
| `solobase-cloud/crates/solobase-worker/Cargo.toml` | Remove middleware block deps, add `solobase` dep |

### Deleted Files

| File | Reason |
|------|--------|
| `solobase/crates/solobase/src/main.rs` | Moved to `solobase-native/src/main.rs` |
| `solobase/crates/solobase/src/app_config.rs` | Moved to `solobase-native/src/app_config.rs` |
| `solobase/crates/solobase/src/blocks/router.rs` | Dead code — `NativeBlockFactory` and `SolobaseRouterBlock` live in `solobase_core::blocks::router`. This file was never compiled (lib.rs uses `pub use solobase_core::blocks`, not `pub mod blocks`). |

---

### Task 1: Create `SolobaseBuilder` module

**Files:**
- Create: `solobase/crates/solobase/src/builder.rs`

- [ ] **Step 1: Create `builder.rs` with the `SolobaseBuilder` struct and builder methods**

```rust
//! SolobaseBuilder — unified WAFER runtime setup for all platforms.
//!
//! Each platform (native, browser WASM, Cloudflare Workers) provides its own
//! service implementations and calls the builder. The builder handles all
//! common registration: service blocks, middleware, feature blocks, router, flow.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use solobase_core::blocks::storage::SolobaseStorageBlock;
use solobase_core::features::{BlockSettings, FeatureConfig};
use solobase_core::routing::BlockId;
use wafer_core::interfaces::config::service::ConfigService;
use wafer_core::interfaces::crypto::service::CryptoService;
use wafer_core::interfaces::database::service::DatabaseService;
use wafer_core::interfaces::logger::service::LoggerService;
use wafer_core::interfaces::network::service::NetworkService;
use wafer_core::interfaces::storage::service::StorageService;
use wafer_run::block::Block;
use wafer_run::Wafer;

// NativeBlockFactory and SolobaseRouterBlock live in solobase_core::blocks::router.
// Accessible via crate::blocks::router thanks to `pub use solobase_core::blocks` in lib.rs.
use solobase_core::blocks::router::{NativeBlockFactory, SolobaseRouterBlock};

// Thread-local storage for the storage block reference.
// Used by `post_start()` to inject WRAP grants after the runtime starts.
// Safe on all platforms: native (single-threaded at setup), CF (single-threaded per request),
// browser (single-threaded).
thread_local! {
    static STORAGE_BLOCK_REF: RefCell<Option<Arc<SolobaseStorageBlock>>> = const { RefCell::new(None) };
}

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
    pub fn new() -> Self {
        Self {
            database: None,
            storage: None,
            config: None,
            crypto: None,
            network: None,
            logger: None,
            block_settings: BlockSettings::from_map(HashMap::new()),
            extra_blocks: Vec::new(),
        }
    }

    pub fn database(mut self, svc: Arc<dyn DatabaseService>) -> Self {
        self.database = Some(svc);
        self
    }

    pub fn storage(mut self, svc: Arc<dyn StorageService>) -> Self {
        self.storage = Some(svc);
        self
    }

    pub fn config(mut self, svc: Arc<dyn ConfigService>) -> Self {
        self.config = Some(svc);
        self
    }

    pub fn crypto(mut self, svc: Arc<dyn CryptoService>) -> Self {
        self.crypto = Some(svc);
        self
    }

    pub fn network(mut self, svc: Arc<dyn NetworkService>) -> Self {
        self.network = Some(svc);
        self
    }

    pub fn logger(mut self, svc: Arc<dyn LoggerService>) -> Self {
        self.logger = Some(svc);
        self
    }

    pub fn block_settings(mut self, settings: BlockSettings) -> Self {
        self.block_settings = settings;
        self
    }

    pub fn extra_block(mut self, name: &str, block: Arc<dyn Block>) -> Self {
        self.extra_blocks.push((name.to_string(), block));
        self
    }

    pub fn build(self) -> Result<Wafer, String> {
        // 1. Validate required services
        let database = self.database.ok_or("database service required")?;
        let storage = self.storage.ok_or("storage service required")?;
        let config = self.config.ok_or("config service required")?;
        let crypto = self.crypto.ok_or("crypto service required")?;
        let network = self.network.ok_or("network service required")?;
        let logger = self.logger.ok_or("logger service required")?;

        // 2. Read JWT secret before registering config block
        let jwt_secret = config.get("SUPPERS_AI__AUTH__JWT_SECRET").unwrap_or_default();

        // 3. Create runtime
        let mut wafer = Wafer::new();
        wafer.set_admin_block("suppers-ai/admin");

        // 4. Register service blocks
        wafer_core::service_blocks::database::register_with(&mut wafer, database)?;
        wafer.add_alias("db", "wafer-run/database");

        let admin_block_id = Arc::new("suppers-ai/admin".to_string());
        let storage_block = solobase_core::blocks::storage::create(storage, admin_block_id);
        STORAGE_BLOCK_REF.with(|cell| *cell.borrow_mut() = Some(storage_block.clone()));
        wafer.register_block("wafer-run/storage", storage_block)?;
        wafer.add_alias("storage", "wafer-run/storage");

        wafer_core::service_blocks::config::register_with(&mut wafer, config)?;
        wafer_core::service_blocks::crypto::register_with(&mut wafer, crypto)?;

        let network_block = solobase_core::blocks::network::create(network);
        wafer.register_block("wafer-run/network", network_block)?;

        wafer_core::service_blocks::logger::register_with(&mut wafer, logger)?;

        // 5. Register ALL middleware blocks
        wafer_block_auth_validator::register(&mut wafer)?;
        wafer_block_cors::register(&mut wafer)?;
        wafer_block_iam_guard::register(&mut wafer)?;
        wafer_block_inspector::register(&mut wafer)?;
        wafer.add_block_config(
            "wafer-run/inspector",
            serde_json::json!({ "allow_anonymous": false }),
        );
        wafer_block_readonly_guard::register(&mut wafer)?;
        wafer_block_router::register(&mut wafer)?;
        wafer_block_security_headers::register(&mut wafer)?;
        wafer_block_web::register(&mut wafer)?;

        // 6. Create and register feature blocks
        let shared_blocks = solobase_core::blocks::create_blocks(|name| {
            self.block_settings.is_enabled(name)
        });
        solobase_core::blocks::register_shared_blocks(&mut wafer, &shared_blocks);

        // 7. Email block (always on, not feature-gated)
        wafer.register_block(
            "suppers-ai/email",
            Arc::new(solobase_core::blocks::email::EmailBlock),
        )?;

        // 8. Extra platform-specific blocks
        for (name, block) in self.extra_blocks {
            wafer.register_block(&name, block)?;
        }

        // 9. Build and register the solobase router
        let feature_config: Arc<dyn FeatureConfig> = Arc::new(self.block_settings);
        let factory = NativeBlockFactory::new(shared_blocks);
        let router = SolobaseRouterBlock::new(jwt_secret, feature_config, factory);
        wafer.register_block("suppers-ai/router", Arc::new(router))?;
        wafer.add_block_config("suppers-ai/router", solobase_core::routing::routes_config());

        // 10. Register site-main flow
        crate::flows::register_site_main(&mut wafer)?;

        Ok(wafer)
    }
}

/// Call after `wafer.start()` or `wafer.start_without_bind()` to inject
/// collected WRAP grants into the storage block for cross-block access control.
pub fn post_start(wafer: &Wafer) {
    STORAGE_BLOCK_REF.with(|cell| {
        if let Some(ref storage) = *cell.borrow() {
            storage.update_wrap_grants(wafer.wrap_grants());
        }
    });
}
```

- [ ] **Step 2: Verify the file compiles in isolation**

Check that the imports resolve. This step will fail until we update `lib.rs` and `Cargo.toml` in later tasks, but confirms the code is syntactically valid.

- [ ] **Step 3: Commit**

```bash
git add solobase/crates/solobase/src/builder.rs
git commit -m "feat: add SolobaseBuilder for unified runtime setup"
```

---

### Task 2: Update `solobase` crate to be platform-agnostic

**Files:**
- Modify: `solobase/crates/solobase/src/lib.rs`
- Modify: `solobase/crates/solobase/Cargo.toml`

- [ ] **Step 1: Update `lib.rs` — add builder module, remove app_config**

Replace the entire contents of `solobase/crates/solobase/src/lib.rs`:

```rust
//! Solobase — self-hosted backend platform powered by the WAFER runtime.
//!
//! This library is platform-agnostic. It provides the `SolobaseBuilder` for
//! unified runtime setup, flow definitions, and the router block. Platform-specific
//! code (native binary, browser WASM, Cloudflare Workers) lives in separate crates
//! that depend on this one.

pub mod builder;
pub use solobase_core::blocks;
pub mod flows;
```

- [ ] **Step 2: Update `Cargo.toml` — remove server feature, make middleware blocks required**

Replace the entire `[features]` section and update `[dependencies]`:

Remove:
- The `[[bin]]` section entirely
- The `default = ["server"]` and `server` features
- The `otel` feature (moves to solobase-native)
- The `storage-s3` and `postgres` features (move to solobase-native)
- Optional deps: `tokio`, `tracing-subscriber`, `dotenvy`, `rusqlite`
- Optional deps: `wafer-block-sqlite`, `wafer-block-local-storage`, `wafer-block-http-listener`
- Optional deps: `wafer-block-s3`, `wafer-block-postgres`
- Optional deps: `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `tracing-opentelemetry`

Make non-optional (remove `optional = true`):
- `wafer-block-auth-validator`
- `wafer-block-cors`
- `wafer-block-iam-guard`
- `wafer-block-inspector`
- `wafer-block-readonly-guard`
- `wafer-block-router`
- `wafer-block-security-headers`
- `wafer-block-web`
- `wafer-block-config`
- `wafer-block-logger`
- `wafer-block-crypto`
- `wafer-block-network`

Remove the `[features]` section entirely (no features needed).

Change `wafer-run` to `default-features = false` (no `full` feature needed).

Change `solobase-core` to `default-features = false` (no sqlite/storage-local needed at library level).

The resulting `Cargo.toml`:

```toml
[package]
name = "solobase"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Solobase — self-hosted backend platform powered by the WAFER runtime"

[lib]
name = "solobase"
path = "src/lib.rs"

[dependencies]
# WAFER runtime (minimal — types + clients only)
wafer-run = { path = "../../../wafer-run/crates/wafer-run", default-features = false }
wafer-core = { path = "../../../wafer-run/crates/wafer-core" }

# Middleware blocks (always registered by the builder)
wafer-block-auth-validator = { path = "../../../wafer-run/crates/wafer-block-auth-validator" }
wafer-block-cors = { path = "../../../wafer-run/crates/wafer-block-cors" }
wafer-block-iam-guard = { path = "../../../wafer-run/crates/wafer-block-iam-guard" }
wafer-block-inspector = { path = "../../../wafer-run/crates/wafer-block-inspector" }
wafer-block-readonly-guard = { path = "../../../wafer-run/crates/wafer-block-readonly-guard" }
wafer-block-router = { path = "../../../wafer-run/crates/wafer-block-router" }
wafer-block-security-headers = { path = "../../../wafer-run/crates/wafer-block-security-headers" }
wafer-block-web = { path = "../../../wafer-run/crates/wafer-block-web" }

# Service block config (needed by builder for ConfigService trait)
wafer-block-config = { path = "../../../wafer-run/crates/wafer-block-config" }
wafer-block-logger = { path = "../../../wafer-run/crates/wafer-block-logger" }
wafer-block-crypto = { path = "../../../wafer-run/crates/wafer-block-crypto" }
wafer-block-network = { path = "../../../wafer-run/crates/wafer-block-network" }

# Solobase shared core (routing, crypto, features, pipeline, backend blocks)
solobase-core = { path = "../solobase-core", default-features = false }

# Async
async-trait = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Observability
tracing = { workspace = true }

# Crypto
sha2 = { workspace = true }
hmac = "0.12"
getrandom = "0.2"

# Time
chrono = { workspace = true }

# Utils
uuid = { workspace = true }
```

- [ ] **Step 3: Delete `solobase/crates/solobase/src/main.rs`**

This file moves to `solobase-native` in Task 3. Delete it now.

- [ ] **Step 4: Delete `solobase/crates/solobase/src/app_config.rs`**

This file moves to `solobase-native` in Task 3. Delete it now.

- [ ] **Step 5: Delete `solobase/crates/solobase/src/blocks/router.rs`**

This is dead code — `NativeBlockFactory` and `SolobaseRouterBlock` live in `solobase_core::blocks::router`. The `lib.rs` uses `pub use solobase_core::blocks` (not `pub mod blocks`), so this local file was never compiled. Delete the entire `src/blocks/` directory.

- [ ] **Step 6: Verify `solobase` crate compiles as lib-only**

Run: `cargo check -p solobase`

Expected: Compiles successfully (may have warnings about unused deps — fine for now).

- [ ] **Step 7: Commit**

```bash
git add solobase/crates/solobase/
git commit -m "refactor: make solobase crate platform-agnostic (lib-only, no tokio)"
```

---

### Task 3: Create `solobase-native` crate

**Files:**
- Create: `solobase/crates/solobase-native/Cargo.toml`
- Create: `solobase/crates/solobase-native/src/main.rs` (moved from `solobase/crates/solobase/src/main.rs`)
- Create: `solobase/crates/solobase-native/src/app_config.rs` (moved from `solobase/crates/solobase/src/app_config.rs`)
- Modify: `solobase/Cargo.toml` (workspace)

- [ ] **Step 1: Create `solobase-native/Cargo.toml`**

```toml
[package]
name = "solobase-native"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Solobase native binary — standalone server with tokio, SQLite, local storage"

[[bin]]
name = "solobase"
path = "src/main.rs"

[features]
default = ["sqlite", "storage-local"]
sqlite = ["solobase-core/sqlite"]
storage-local = ["solobase-core/storage-local"]
otel = [
    "dep:opentelemetry",
    "dep:opentelemetry_sdk",
    "dep:opentelemetry-otlp",
    "dep:tracing-opentelemetry",
]
storage-s3 = ["solobase-core/storage-s3", "dep:wafer-block-s3"]
postgres = ["solobase-core/postgres", "dep:wafer-block-postgres"]

[dependencies]
# Solobase library (platform-agnostic builder + flows + router)
solobase = { path = "../solobase" }
solobase-core = { path = "../solobase-core", default-features = false }

# WAFER runtime
wafer-run = { path = "../../../wafer-run/crates/wafer-run", default-features = false, features = ["full"] }
wafer-core = { path = "../../../wafer-run/crates/wafer-core" }

# Native service implementation crates
wafer-block-sqlite = { path = "../../../wafer-run/crates/wafer-block-sqlite" }
wafer-block-local-storage = { path = "../../../wafer-run/crates/wafer-block-local-storage" }
wafer-block-config = { path = "../../../wafer-run/crates/wafer-block-config" }
wafer-block-crypto = { path = "../../../wafer-run/crates/wafer-block-crypto" }
wafer-block-network = { path = "../../../wafer-run/crates/wafer-block-network" }
wafer-block-logger = { path = "../../../wafer-run/crates/wafer-block-logger" }
wafer-block-http-listener = { path = "../../../wafer-run/crates/wafer-block-http-listener" }

# Optional backends
wafer-block-s3 = { path = "../../../wafer-run/crates/wafer-block-s3", optional = true }
wafer-block-postgres = { path = "../../../wafer-run/crates/wafer-block-postgres", optional = true }

# Async runtime
tokio = { workspace = true }

# Observability
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
opentelemetry = { workspace = true, optional = true }
opentelemetry_sdk = { workspace = true, optional = true }
opentelemetry-otlp = { workspace = true, optional = true }
tracing-opentelemetry = { workspace = true, optional = true }

# Config loading
dotenvy = { workspace = true }
rusqlite = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Crypto
sha2 = { workspace = true }
hmac = "0.12"
getrandom = "0.2"

# Time
chrono = { workspace = true }

# Utils
uuid = { workspace = true }
```

- [ ] **Step 2: Move `main.rs` to `solobase-native/src/main.rs`**

Copy the file from `solobase/crates/solobase/src/main.rs` to `solobase/crates/solobase-native/src/main.rs`.

Update the imports at the top — replace:
```rust
use solobase::app_config::{load_block_settings, InfraConfig};
use solobase::blocks;
use solobase::blocks::router::{NativeBlockFactory, SolobaseRouterBlock};
use solobase::flows;
```

With:
```rust
mod app_config;

use app_config::{load_block_settings, InfraConfig};
```

The rest of `main.rs` will be rewritten in Task 5 to use the builder.

- [ ] **Step 3: Move `app_config.rs` to `solobase-native/src/app_config.rs`**

Copy the file from `solobase/crates/solobase/src/app_config.rs` to `solobase/crates/solobase-native/src/app_config.rs`.

No import changes needed — `app_config.rs` uses `solobase_core::features::BlockSettings` and `rusqlite`, which are now dependencies of `solobase-native`.

- [ ] **Step 4: Add `solobase-native` to workspace members**

In `solobase/Cargo.toml`, update the members list:

```toml
[workspace]
resolver = "2"
members = [
    "crates/solobase",
    "crates/solobase-core",
    "crates/solobase-native",
    "crates/solobase-web",
]
```

- [ ] **Step 5: Verify workspace compiles**

Run: `cd solobase && cargo check -p solobase-native`

Expected: May fail because `main.rs` still has old imports. That's OK — we'll fix it in Task 5.

- [ ] **Step 6: Commit**

```bash
git add solobase/crates/solobase-native/ solobase/Cargo.toml
git commit -m "feat: create solobase-native crate (move binary + native deps)"
```

---

### Task 4: Rewrite `solobase-native/main.rs` to use the builder

**Files:**
- Modify: `solobase/crates/solobase-native/src/main.rs`

- [ ] **Step 1: Rewrite `main()` to use `SolobaseBuilder`**

Replace the WAFER runtime setup section (steps 7-16 in the original main.rs, roughly lines 69-202) with:

```rust
use std::sync::Arc;

use solobase::builder::{self, SolobaseBuilder};

use app_config::{load_block_settings, InfraConfig};

use tracing_subscriber::{fmt, EnvFilter};
use wafer_core::interfaces::config::service::ConfigService;

mod app_config;

#[tokio::main]
async fn main() {
    // 1. Load .env file
    load_dotenv();

    // 2. Initialize tracing
    let log_format = std::env::var("SOLOBASE_LOG_FORMAT").unwrap_or_else(|_| "text".into());
    init_tracing(&log_format);
    tracing::info!("solobase starting (Rust/WAFER runtime)");

    // 3. Read infrastructure config
    let infra = InfraConfig::from_env();
    tracing::info!(
        listen = %infra.listen,
        db = %infra.db_type,
        db_path = %infra.db_path,
        storage = %infra.storage_type,
        "infrastructure config loaded"
    );

    // 4. Collect app config vars from env
    let env_vars = collect_app_env_vars();

    // 5. Open SQLite, seed variables, read config
    let vars = seed_and_load_variables(&infra.db_path, &env_vars);
    tracing::info!(vars = vars.len(), "variables loaded from database");

    // 6. Extract JWT secret and feature config
    let jwt_secret = vars
        .get("SUPPERS_AI__AUTH__JWT_SECRET")
        .cloned()
        .unwrap_or_default();
    let features = load_block_settings(&infra.db_path);

    // 7. Build WAFER runtime via SolobaseBuilder
    let config_service = wafer_block_config::service::EnvConfigService::new();
    for (key, value) in &vars {
        config_service.set(key, value);
    }

    let mut wafer = SolobaseBuilder::new()
        .database(Arc::new(
            wafer_block_sqlite::service::SQLiteDatabaseService::open(&infra.db_path)
                .expect("failed to open SQLite database"),
        ))
        .storage(Arc::new(
            wafer_block_local_storage::service::LocalStorageService::new(&infra.storage_root)
                .expect("failed to create local storage service"),
        ))
        .config(Arc::new(config_service))
        .crypto(Arc::new(
            wafer_block_crypto::service::Argon2JwtCryptoService::new(jwt_secret),
        ))
        .network(Arc::new(wafer_block_network::service::HttpNetworkService::new()))
        .logger(Arc::new(wafer_block_logger::service::TracingLogger))
        .block_settings(features)
        .build()
        .expect("failed to build solobase runtime");

    // 8. Native-only: register http-listener
    wafer_block_http_listener::register(&mut wafer).expect("register http-listener");
    wafer.add_block_config(
        "wafer-run/http-listener",
        serde_json::json!({ "flow": "site-main", "listen": infra.listen }),
    );

    // 9. Register observability hooks
    register_observability_hooks(&mut wafer);

    // 10. Load custom WRAP grants from DB
    let db_grants = app_config::load_wrap_grants(&infra.db_path);
    if !db_grants.is_empty() {
        tracing::info!(count = db_grants.len(), "loaded custom WRAP grants from database");
        wafer.add_wrap_grants(db_grants);
    }

    // 11. Start runtime
    let wafer = wafer
        .start()
        .await
        .expect("failed to start WAFER runtime");

    // 12. Inject WRAP grants into storage block
    builder::post_start(&wafer);
    tracing::info!("WAFER runtime started — all blocks resolved");

    // 13. Wait for shutdown
    shutdown_signal().await;

    // 14. Graceful shutdown
    wafer.shutdown().await;
    tracing::info!("solobase shutdown complete");
}
```

Keep all the helper functions (`load_dotenv`, `collect_app_env_vars`, `seed_and_load_variables`, `seed_auto_generated`, `init_tracing`, `register_observability_hooks`, `shutdown_signal`) unchanged — just copy them from the original `main.rs`.

Remove the old imports that are no longer needed:
- `use solobase::blocks`
- `use solobase::blocks::router::{NativeBlockFactory, SolobaseRouterBlock}`
- `use solobase::flows`

- [ ] **Step 2: Verify `solobase-native` compiles**

Run: `cd solobase && cargo check -p solobase-native`

Expected: Compiles successfully.

- [ ] **Step 3: Verify `solobase-native` runs**

Run: `cd solobase && cargo run -p solobase-native`

Expected: Server starts on the configured listen address. Ctrl+C to stop. Verify at least the `/health` endpoint responds.

- [ ] **Step 4: Commit**

```bash
git add solobase/crates/solobase-native/src/main.rs
git commit -m "feat: solobase-native uses SolobaseBuilder"
```

---

### Task 5: Migrate `solobase-web` to use the builder

**Files:**
- Modify: `solobase/crates/solobase-web/src/lib.rs`
- Modify: `solobase/crates/solobase-web/Cargo.toml`

- [ ] **Step 1: Update `Cargo.toml` — add `solobase` dep, remove middleware block deps**

Add to `[dependencies]`:
```toml
# Solobase library (builder + flows + router)
solobase = { path = "../solobase", default-features = false }
```

Remove these dependencies (now pulled transitively via `solobase`):
- `wafer-block-auth-validator`
- `wafer-block-cors`
- `wafer-block-iam-guard`
- `wafer-block-inspector`
- `wafer-block-readonly-guard`
- `wafer-block-router`
- `wafer-block-security-headers`
- `wafer-block-web`
- `wafer-block-crypto` (if only used for service trait, kept via solobase)

Keep:
- `wafer-block-config` (needed for `EnvConfigService::new()` and `ConfigService::set()`)
- All browser-specific deps (wasm-bindgen, web-sys, js-sys, pbkdf2, etc.)

- [ ] **Step 2: Rewrite `lib.rs` — replace manual registration with builder**

Replace the `initialize()` function and delete `register_service_blocks()`, `register_middleware_blocks()`, and `register_site_main_flow()`:

```rust
//! Solobase compiled to WASM for running in the browser via Service Worker.

use std::cell::RefCell;
use std::sync::Arc;

use wasm_bindgen::prelude::*;

pub mod bridge;
pub mod config;
pub mod convert;
pub mod crypto;
pub mod database;
pub mod logger;
pub mod network;
pub mod storage;

use solobase::builder::{self, SolobaseBuilder};
use wafer_core::interfaces::config::service::ConfigService;

thread_local! {
    static RUNTIME: RefCell<Option<wafer_run::Wafer>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
pub async fn initialize() -> Result<(), JsValue> {
    // 1. Load sql.js WASM + open/create the OPFS database.
    bridge::dbInit().await;

    // 2. Seed variables and load config.
    let vars = config::seed_and_load_variables();
    web_sys::console::log_1(
        &format!("solobase: {} variables loaded from database", vars.len()).into(),
    );

    // 3. Load feature flag settings.
    let features = config::load_block_settings();

    // 4. Extract JWT secret.
    let jwt_secret = vars
        .get("SUPPERS_AI__AUTH__JWT_SECRET")
        .cloned()
        .unwrap_or_default();

    // 5. Build config service.
    let config_svc = wafer_block_config::service::EnvConfigService::new();
    for (key, value) in &vars {
        config_svc.set(key, value);
    }

    // 6. Build WAFER runtime via SolobaseBuilder.
    let mut wafer = SolobaseBuilder::new()
        .database(Arc::new(database::BrowserDatabaseService))
        .storage(Arc::new(storage::BrowserStorageService))
        .config(Arc::new(config_svc))
        .crypto(Arc::new(crypto::BrowserCryptoService::new(jwt_secret)))
        .network(Arc::new(network::BrowserNetworkService))
        .logger(Arc::new(logger::ConsoleLogger))
        .block_settings(features)
        .build()
        .map_err(|e| JsValue::from_str(&e))?;

    // 7. Start runtime.
    wafer
        .start_without_bind()
        .await
        .map_err(|e| JsValue::from_str(&e))?;

    // 8. Inject WRAP grants.
    builder::post_start(&wafer);

    web_sys::console::log_1(&"solobase: WAFER runtime started".into());

    // 9. Store in global.
    RUNTIME.with(|r| {
        *r.borrow_mut() = Some(wafer);
    });

    Ok(())
}

#[wasm_bindgen]
pub async fn handle_request(request: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    let mut msg = convert::request_to_message(&request).await?;

    let wafer_ptr = RUNTIME.with(|r| {
        let borrow = r.borrow();
        match borrow.as_ref() {
            Some(w) => Ok(w as *const wafer_run::Wafer),
            None => Err(JsValue::from_str(
                "solobase: runtime not initialized — call initialize() first",
            )),
        }
    })?;
    let wafer = unsafe { &*wafer_ptr };
    let result = wafer.run("site-main", &mut msg).await;

    convert::result_to_response(result)
}
```

- [ ] **Step 3: Verify solobase-web compiles for wasm32**

Run: `cd solobase && cargo check -p solobase-web --target wasm32-unknown-unknown`

Expected: Compiles successfully. Note: you may need to install the target first with `rustup target add wasm32-unknown-unknown`.

- [ ] **Step 4: Commit**

```bash
git add solobase/crates/solobase-web/
git commit -m "refactor: solobase-web uses SolobaseBuilder (removes duplicated setup)"
```

---

### Task 6: Migrate `solobase-cloud/solobase-worker` to use the builder

**Files:**
- Modify: `solobase-cloud/crates/solobase-worker/src/lib.rs`
- Modify: `solobase-cloud/crates/solobase-worker/Cargo.toml`

- [ ] **Step 1: Update `Cargo.toml` — remove redundant deps, ensure `solobase` is included**

The worker already depends on `solobase` (with `default-features = false`). Keep that.

Remove these dependencies (now pulled via `solobase`):
- `wafer-block-web`
- `wafer-block-router`
- `wafer-block-security-headers`
- `wafer-block-cors`
- `wafer-block-readonly-guard`

Keep:
- `solobase` (already there)
- `solobase-core` (used for `BlockSettings`, `BlockId`)
- `wafer-core` (used for service traits)
- `wafer-run` (used for `Block` trait, `Wafer`)
- `wafer-block` (used for `BlockInfo`)
- `worker` (CF Worker bindings)
- All other deps (serde, chrono, uuid, etc.)
- `wasmi` (optional, for custom blocks)

- [ ] **Step 2: Rewrite `handle_request()` in `lib.rs` to use the builder**

Replace the `handle_request` function (lines 292-518 in the original). Keep:
- `main()` (the `#[event(fetch)]` handler) — unchanged
- `handle_migrate()` — unchanged
- `seed_auto_generated()` — unchanged
- `seed_defaults()` — unchanged
- `ensure_admin_role()` — unchanged
- `handle_install_block()` / `handle_delete_block()` — unchanged
- `load_d1_map()` — unchanged
- `get_env_str()` — unchanged

Delete:
- The `register_blocks!` macro (lines 44-55)

Rewrite `handle_request()`:

```rust
async fn handle_request(req: &Request, env: &Env) -> Result<Response> {
    use std::sync::Arc;
    use solobase::builder::{self, SolobaseBuilder};

    let db = env
        .d1("DB")
        .map_err(|e| Error::RustError(format!("D1: {e}")))?;
    let bucket = env
        .bucket("STORAGE")
        .map_err(|e| Error::RustError(format!("R2: {e}")))?;

    // Load config from D1 variables table
    let mut env_vars = load_d1_map(&db, "SELECT key, value FROM suppers_ai__admin__variables")
        .await
        .unwrap_or_default();

    // Merge protected worker env bindings
    for key in WORKER_BINDING_KEYS {
        let val = get_env_str(env, key);
        if !val.is_empty() {
            env_vars.insert(key.to_string(), val);
        }
    }

    // Dispatcher service binding
    let has_dispatcher = if let Ok(_fetcher) = env.service("DISPATCHER") {
        env_vars.insert("SOLOBASE_SHARED__HAS_DISPATCHER_BINDING".to_string(), "true".to_string());
        true
    } else {
        false
    };

    let jwt_secret = env_vars.get("SUPPERS_AI__AUTH__JWT_SECRET").cloned().unwrap_or_default();

    // Load block settings from D1
    let features = {
        let mut map = std::collections::HashMap::new();
        if let Ok(stmt) = db
            .prepare("SELECT block_name, enabled FROM suppers_ai__admin__block_settings")
            .bind(&[])
        {
            if let Ok(result) = stmt.all().await {
                for row in result.results::<serde_json::Value>().unwrap_or_default() {
                    if let (Some(name), Some(enabled)) = (
                        row.get("block_name").and_then(|v| v.as_str()),
                        row.get("enabled").and_then(|v| v.as_i64()),
                    ) {
                        map.insert(name.to_string(), enabled != 0);
                    }
                }
            }
        }
        solobase_core::features::BlockSettings::from_map(map)
    };

    // Build the runtime
    let mut builder = SolobaseBuilder::new()
        .database(Arc::new(database::D1DatabaseService::new(db)))
        .storage(Arc::new(storage::R2StorageService::new(bucket)))
        .config(Arc::new(config_service::HashMapConfigService::new(env_vars)))
        .crypto(Arc::new(crypto_service::SolobaseCryptoService::new(jwt_secret)))
        .network(Arc::new(network_service::WorkerFetchService))
        .logger(Arc::new(logger_service::ConsoleLoggerService))
        .block_settings(features);

    // CF-specific: dispatcher service binding
    if has_dispatcher {
        if let Ok(fetcher) = env.service("DISPATCHER") {
            builder = builder.extra_block(
                "solobase/dispatcher",
                Arc::new(dispatcher::DispatcherBlock::new(fetcher)),
            );
        }
    }

    let mut wafer = builder
        .build()
        .map_err(|e| Error::RustError(e))?;

    // Load and register custom WASM blocks from R2
    #[cfg(feature = "custom-blocks")]
    {
        let custom_db = env
            .d1("DB")
            .map_err(|e| Error::RustError(format!("D1 for custom blocks: {e}")))?;
        let custom_bucket = env
            .bucket("STORAGE")
            .map_err(|e| Error::RustError(format!("R2 for custom blocks: {e}")))?;
        if let Ok(entries) = custom_blocks::load_enabled_custom_blocks(&custom_db).await {
            for entry in entries {
                match custom_blocks::fetch_and_load_block(&custom_bucket, &entry).await {
                    Ok(block) => {
                        if let Err(e) = wafer.register_block(&entry.name, block) {
                            console_log!(
                                "warning: failed to register custom block '{}': {}",
                                entry.name,
                                e
                            );
                        }
                    }
                    Err(e) => {
                        console_log!(
                            "warning: failed to load custom block '{}': {}",
                            entry.name,
                            e
                        );
                    }
                }
            }
        }
    }

    // Start runtime
    wafer
        .start_without_bind()
        .await
        .map_err(|e| Error::RustError(e))?;

    // Inject WRAP grants
    builder::post_start(&wafer);

    // Convert HTTP request to WAFER Message
    let auth_header = req.headers().get("authorization")?;
    let mut msg = convert::worker_request_to_message(req).await?;

    if let Some(ref auth) = auth_header {
        msg.set_meta("http.header.authorization", auth);
    }

    // Execute flow
    let result = wafer.run("site-main", &mut msg).await;
    convert::wafer_result_to_worker_response(result)
}
```

Also update the module imports at the top of the file. Remove:
```rust
use solobase::blocks;
use solobase_core::features::BlockSettings;
use solobase_core::routing::BlockId;
```

These are no longer needed since the builder handles block creation internally.

Keep the `mod` declarations but remove `mod schema` only if it was used exclusively for the old block setup (check if it's still needed for migrations). The `schema` module is likely still needed for `run_migrations()`.

- [ ] **Step 3: Verify solobase-worker compiles for wasm32**

Run: `cd solobase-cloud && cargo check -p solobase-worker --target wasm32-unknown-unknown`

Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add solobase-cloud/crates/solobase-worker/
git commit -m "refactor: solobase-worker uses SolobaseBuilder (removes duplicated setup)"
```

---

### Task 7: End-to-end verification

**Files:** None (verification only)

- [ ] **Step 1: Run full workspace check for solobase**

Run: `cd solobase && cargo check --workspace`

Expected: All crates compile (solobase, solobase-core, solobase-native, solobase-web).

- [ ] **Step 2: Run full workspace check for solobase-cloud**

Run: `cd solobase-cloud && cargo check --workspace`

Expected: Both solobase-cloudflare and solobase-worker compile.

- [ ] **Step 3: Run solobase-native and verify health endpoint**

Run: `cd solobase && cargo run -p solobase-native`

Then in another terminal:
```bash
curl http://localhost:8090/health
```

Expected: `{"status":"ok"}`

- [ ] **Step 4: Run solobase-web wasm-pack build**

Run: `cd solobase/crates/solobase-web && wasm-pack build --target web`

Expected: Produces `pkg/` directory with WASM binary.

- [ ] **Step 5: Commit any remaining fixes**

If any compilation or runtime issues were found and fixed in steps 1-4:

```bash
git add -A
git commit -m "fix: resolve compilation issues from builder migration"
```

---

### Task 8: npm package scaffolding for solobase-web

**Files:**
- Create: `solobase/packages/solobase-web/package.json`
- Create: `solobase/packages/solobase-web/src/index.ts`
- Create: `solobase/packages/solobase-web/src/worker.ts`
- Create: `solobase/packages/solobase-web/tsconfig.json`

- [ ] **Step 1: Create `package.json`**

```json
{
  "name": "solobase-web",
  "version": "0.1.0",
  "description": "Solobase backend running in the browser via Service Worker + WASM",
  "type": "module",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "exports": {
    ".": {
      "import": "./dist/index.js",
      "types": "./dist/index.d.ts"
    },
    "./worker": {
      "import": "./dist/worker.js",
      "types": "./dist/worker.d.ts"
    }
  },
  "files": [
    "dist/"
  ],
  "scripts": {
    "build:wasm": "cd ../../crates/solobase-web && wasm-pack build --target web --out-dir ../../packages/solobase-web/dist/wasm",
    "build:ts": "tsc",
    "build": "npm run build:wasm && npm run build:ts"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  },
  "keywords": ["solobase", "wasm", "service-worker", "backend"],
  "license": "MIT"
}
```

- [ ] **Step 2: Create `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "declaration": true,
    "outDir": "dist",
    "rootDir": "src",
    "strict": true,
    "lib": ["ES2020", "DOM", "WebWorker"]
  },
  "include": ["src"]
}
```

- [ ] **Step 3: Create `src/index.ts` — batteries-included mode**

```typescript
export interface SolobaseConfig {
  /** URL paths to intercept (default: ['/b/**', '/health']) */
  routes?: string[];
  /** Service Worker scope (default: '/') */
  scope?: string;
}

const DEFAULT_ROUTES = ['/b/', '/health', '/openapi.json', '/.well-known/agent.json'];

/**
 * Register a Service Worker that runs the Solobase WASM backend.
 * All matching requests are intercepted and handled by the WASM runtime.
 */
export async function setupSolobase(config?: SolobaseConfig): Promise<void> {
  if (!('serviceWorker' in navigator)) {
    throw new Error('Service Workers are not supported in this browser');
  }

  const scope = config?.scope ?? '/';
  const routes = config?.routes ?? DEFAULT_ROUTES;

  const registration = await navigator.serviceWorker.register(
    new URL('./worker.js', import.meta.url),
    { scope, type: 'module' }
  );

  // Wait for the SW to be active
  const sw = registration.installing || registration.waiting || registration.active;
  if (sw && sw.state !== 'activated') {
    await new Promise<void>((resolve) => {
      sw.addEventListener('statechange', () => {
        if (sw.state === 'activated') resolve();
      });
    });
  }

  // Send route config to the SW
  registration.active?.postMessage({ type: 'solobase:config', routes });
}
```

- [ ] **Step 4: Create `src/worker.ts` — composable mode + SW entry point**

```typescript
// Re-export the WASM module's initialize and handleRequest for composable mode.
// Developers who have an existing SW can import these directly.

import init, { initialize as wasmInitialize, handle_request as wasmHandleRequest } from './wasm/solobase_web.js';

let initialized = false;
let routes: string[] = ['/b/', '/health', '/openapi.json', '/.well-known/agent.json'];

/**
 * Initialize the Solobase WASM runtime.
 * Call once before handling requests.
 */
export async function initialize(): Promise<void> {
  if (initialized) return;
  await init();
  await wasmInitialize();
  initialized = true;
}

/**
 * Handle an incoming fetch request through the Solobase WASM runtime.
 */
export async function handleRequest(request: Request): Promise<Response> {
  if (!initialized) {
    return new Response('Solobase not initialized', { status: 503 });
  }
  return await wasmHandleRequest(request);
}

/**
 * Check if a URL path should be handled by Solobase.
 */
function shouldIntercept(pathname: string): boolean {
  return routes.some((route) => pathname.startsWith(route));
}

// --- Batteries-included SW entry point ---
// When this file is loaded as a Service Worker directly, it auto-initializes
// and intercepts matching fetch events.

declare const self: ServiceWorkerGlobalScope;

if (typeof ServiceWorkerGlobalScope !== 'undefined') {
  self.addEventListener('install', (event) => {
    event.waitUntil(initialize().then(() => self.skipWaiting()));
  });

  self.addEventListener('activate', (event) => {
    event.waitUntil(self.clients.claim());
  });

  self.addEventListener('message', (event) => {
    if (event.data?.type === 'solobase:config' && Array.isArray(event.data.routes)) {
      routes = event.data.routes;
    }
  });

  self.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);
    if (shouldIntercept(url.pathname)) {
      event.respondWith(handleRequest(event.request));
    }
  });
}
```

- [ ] **Step 5: Verify the TS compiles**

Run: `cd solobase/packages/solobase-web && npx tsc --noEmit`

Expected: No type errors. (The WASM imports will fail until the WASM is built, but the types should be structurally valid.)

- [ ] **Step 6: Commit**

```bash
git add solobase/packages/solobase-web/
git commit -m "feat: add solobase-web npm package (batteries-included + composable SW)"
```
