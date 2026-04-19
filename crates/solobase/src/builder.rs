//! SolobaseBuilder — unified WAFER runtime setup for all platforms.
//!
//! Each platform (native, browser WASM, Cloudflare Workers) provides its own
//! service implementations and calls the builder. The builder handles all
//! common registration: service blocks, middleware, feature blocks, router, flow.

use std::{collections::HashMap, sync::Arc};

use solobase_core::{
    blocks::{
        router::{NativeBlockFactory, SolobaseRouterBlock},
        storage::SolobaseStorageBlock,
    },
    features::{BlockSettings, FeatureConfig},
    ExtraRoute, RouteAccess,
};
use wafer_core::interfaces::{
    config::service::ConfigService, crypto::service::CryptoService,
    database::service::DatabaseService, llm::service::LlmService, logger::service::LoggerService,
    network::service::NetworkService, storage::service::StorageService,
};
use wafer_run::{block::Block, RuntimeError, Wafer};

pub struct SolobaseBuilder {
    database: Option<Arc<dyn DatabaseService>>,
    storage: Option<Arc<dyn StorageService>>,
    config: Option<Arc<dyn ConfigService>>,
    crypto: Option<Arc<dyn CryptoService>>,
    network: Option<Arc<dyn NetworkService>>,
    logger: Option<Arc<dyn LoggerService>>,
    block_settings: BlockSettings,
    block_configs: Vec<(String, serde_json::Value)>,
    extra_blocks: Vec<(String, Arc<dyn Block>)>,
    /// Additional LLM backends to register on the `MultiBackendLlmService`
    /// router backing `wafer-run/llm`. Each entry is `(label, service)` and
    /// follows the same order semantics as `MultiBackendLlmService::register`:
    /// first match on `claims_backend` wins. On native builds with the `llm`
    /// feature enabled, `"provider"` is auto-registered first, so HTTP
    /// providers (OpenAI/Anthropic/etc.) take precedence over any backend
    /// added here for overlapping `backend_id`s.
    extra_llm_services: Vec<(String, Arc<dyn LlmService>)>,
    /// Routes registered by downstream projects via `add_route`. Checked
    /// after built-in `ROUTES` — built-ins always win on prefix collision.
    extra_routes: Vec<ExtraRoute>,
    /// Filesystem path to the SQLite database.
    ///
    /// Only used by the `native-embedding` feature to open a dedicated
    /// `rusqlite::Connection` for `SqliteVecService`. Kept as `Option<String>`
    /// (rather than feature-gated) so platforms can always pass it; the
    /// field is simply ignored when the feature is off.
    sqlite_db_path: Option<String>,
}

impl Default for SolobaseBuilder {
    fn default() -> Self {
        Self::new()
    }
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
            block_configs: Vec::new(),
            extra_blocks: Vec::new(),
            extra_llm_services: Vec::new(),
            extra_routes: Vec::new(),
            sqlite_db_path: None,
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

    /// Register an additional `LlmService` backend on the router backing
    /// `wafer-run/llm`. The `label` is used in log/tracing output and must be
    /// unique across registrations (collision is not enforced — later
    /// registrations simply lose to earlier ones on overlapping
    /// `backend_id`s). The backend itself decides which `backend_id`s it
    /// claims via `claims_backend`.
    ///
    /// On native builds with the `llm` feature enabled, the built-in
    /// `"provider"` backend is registered first (in `build()`) and therefore
    /// takes precedence over services added via this method for overlapping
    /// `backend_id`s. This is the expected ordering: HTTP providers win over
    /// browser-only backends on native.
    ///
    /// On wasm32 builds (where the `llm` feature is off), the router is still
    /// created and the `wafer-run/llm` service block is still registered —
    /// it just contains only the backends passed in via this setter.
    pub fn llm_service(mut self, label: impl Into<String>, service: Arc<dyn LlmService>) -> Self {
        self.extra_llm_services.push((label.into(), service));
        self
    }

    pub fn block_config(mut self, name: &str, config: serde_json::Value) -> Self {
        self.block_configs.push((name.to_string(), config));
        self
    }

    /// Register a downstream-project route that dispatches to a custom block.
    ///
    /// Built-in solobase routes take priority — an extra route with the same
    /// prefix as a built-in (e.g. `/b/auth/`) is ignored. To disable a
    /// built-in route, turn off its feature flag.
    ///
    /// `access` declares the auth tier:
    /// - [`RouteAccess::Public`] — no auth check.
    /// - [`RouteAccess::Authenticated`] — rejects empty user_id with 403.
    /// - [`RouteAccess::Admin`] — requires the `admin` role or 403.
    pub fn add_route(
        mut self,
        prefix: impl Into<String>,
        block_name: impl Into<String>,
        access: RouteAccess,
    ) -> Self {
        self.extra_routes.push(ExtraRoute {
            prefix: prefix.into(),
            block_name: block_name.into(),
            access,
        });
        self
    }

    /// Set the filesystem path to the SQLite database file.
    ///
    /// Only consumed by the `native-embedding` feature to open a second
    /// `rusqlite::Connection` for the `SqliteVecService` backing
    /// `wafer-run/vector`. SQLite supports multi-connection access in WAL
    /// mode, so sharing the underlying file is safe. Without this path,
    /// `native-embedding` cannot register the vector runtime block — the
    /// `build()` call will return an error.
    pub fn sqlite_db_path(mut self, path: impl Into<String>) -> Self {
        self.sqlite_db_path = Some(path.into());
        self
    }

    pub fn build(self) -> Result<(Wafer, Arc<SolobaseStorageBlock>), RuntimeError> {
        // 1. Validate required services
        let database = self.database.ok_or("database service required")?;
        let storage = self.storage.ok_or("storage service required")?;
        let config = self.config.ok_or("config service required")?;
        let crypto = self.crypto.ok_or("crypto service required")?;
        let network = self.network.ok_or("network service required")?;
        let logger = self.logger.ok_or("logger service required")?;

        // 2. Read JWT secret before registering config block
        let jwt_secret = config
            .get("SUPPERS_AI__AUTH__JWT_SECRET")
            .unwrap_or_default();

        // 3. Create runtime
        let mut wafer = Wafer::new();
        wafer.set_admin_block("suppers-ai/admin");

        // 4. Register service blocks
        wafer_core::service_blocks::database::register_with(&mut wafer, database)?;
        wafer.add_alias("db", "wafer-run/database");

        let admin_block_id = Arc::new("suppers-ai/admin".to_string());
        let storage_block = solobase_core::blocks::storage::create(storage, admin_block_id);
        wafer.register_block("wafer-run/storage", storage_block.clone())?;
        wafer.add_alias("storage", "wafer-run/storage");

        wafer_core::service_blocks::config::register_with(&mut wafer, config)?;
        wafer_core::service_blocks::crypto::register_with(&mut wafer, crypto)?;

        let network_block = solobase_core::blocks::network::create(network);
        wafer.register_block("wafer-run/network", network_block)?;

        wafer_core::service_blocks::logger::register_with(&mut wafer, logger)?;

        // 4c. Construct the LLM service + router and register `wafer-run/llm`.
        //     The feature block `suppers-ai/llm` receives `provider_llm_svc`
        //     via its constructor for admin CRUD and `lifecycle(Init)`
        //     configure. Chat/model-listing requests from the feature block
        //     go through `ctx.call_block("wafer-run/llm", ...)`, which hits
        //     the `MultiBackendLlmService` router registered here.
        //
        //     On native (`llm` feature on) the HTTP `ProviderLlmService` is
        //     auto-registered under `"provider"` first — reqwest-based
        //     providers aren't Send-safe on wasm32, so the `llm` feature
        //     gates them. Additional backends passed via
        //     `.llm_service(label, svc)` are registered after `"provider"`
        //     and lose to it on overlapping `backend_id`s.
        //
        //     On wasm32 (`llm` feature off) the router is built empty and
        //     populated purely from `.llm_service(...)` entries (typically a
        //     `BrowserLlmService` from `solobase-web`). If no backends are
        //     registered, the router is still installed — its
        //     `claims_backend` returns false for all ids and produces clean
        //     `unknown backend_id` errors via the standard router dispatch.
        let mut llm_router = wafer_core::interfaces::llm::router::MultiBackendLlmService::new();

        #[cfg(feature = "llm")]
        let provider_llm_svc = {
            let svc = Arc::new(solobase_core::blocks::llm::providers::ProviderLlmService::new());
            llm_router.register("provider", svc.clone());
            svc
        };

        for (label, svc) in self.extra_llm_services {
            llm_router.register(label, svc);
        }

        wafer_core::service_blocks::llm::register_with(&mut wafer, Arc::new(llm_router))?;

        // 4b. Register the `wafer-run/vector` runtime block when the
        // `native-embedding` feature is on. `suppers-ai/vector` declares
        // `requires=["wafer-run/vector"]`, so without this registration
        // dependency resolution fails at startup.
        #[cfg(feature = "native-embedding")]
        register_vector_block(&mut wafer, self.sqlite_db_path.as_deref())?;

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

        // 5b. Apply platform-specific block configs
        for (name, config) in self.block_configs {
            wafer.add_block_config(&name, config);
        }

        // 6. Create and register feature blocks. `LlmBlock` receives the
        //    `provider_llm_svc` Arc we built in step 4c.
        #[cfg(feature = "llm")]
        let shared_blocks = solobase_core::blocks::create_blocks(
            |name| self.block_settings.is_enabled(name),
            &provider_llm_svc,
        );
        #[cfg(not(feature = "llm"))]
        let shared_blocks =
            solobase_core::blocks::create_blocks(|name| self.block_settings.is_enabled(name));
        solobase_core::blocks::register_shared_blocks(&mut wafer, &shared_blocks)?;

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
        let router = SolobaseRouterBlock::with_extra_routes(
            jwt_secret,
            feature_config,
            factory,
            self.extra_routes,
        );
        wafer.register_block("suppers-ai/router", Arc::new(router))?;
        wafer.add_block_config("suppers-ai/router", solobase_core::routing::routes_config());

        // 10. Auto-discover WASM blocks from cwd/blocks/**/target/block.wasm
        //     and flow JSON files from cwd/flows/**/*.json.
        //     Only available when compiled with the `wasm` feature (wasmi interpreter).
        #[cfg(feature = "wasm")]
        {
            use std::sync::Arc;

            use wafer_run::{
                discovery::{discover_flows, discover_wasm_blocks},
                wasm::WasmiBlock,
            };

            let cwd = std::env::current_dir()
                .map_err(|e| format!("failed to get current directory: {e}"))?;

            // Discover and load WASM blocks.
            let wasm_paths = discover_wasm_blocks(&cwd.join("blocks"));
            for wasm_path in &wasm_paths {
                let bytes = match std::fs::read(wasm_path) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!(path = %wasm_path.display(), error = %e, "failed to read WASM block — skipping");
                        continue;
                    }
                };
                let block = match WasmiBlock::load_from_bytes(&bytes) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!(path = %wasm_path.display(), error = %e, "failed to load WASM block — skipping");
                        continue;
                    }
                };
                let name = block.info().name.clone();
                tracing::info!(name = %name, path = %wasm_path.display(), "discovered WASM block");
                wafer
                    .register_block(&name, Arc::new(block))
                    .map_err(|e| format!("auto-discovered block '{name}': {e}"))?;
            }

            // Discover and load flow JSON files.
            let flow_paths = discover_flows(&cwd.join("flows"));
            for flow_path in &flow_paths {
                let json = match std::fs::read_to_string(flow_path) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(path = %flow_path.display(), error = %e, "failed to read flow JSON — skipping");
                        continue;
                    }
                };
                match wafer.add_flow_json(&json) {
                    Ok(()) => {
                        tracing::info!(path = %flow_path.display(), "discovered flow");
                    }
                    Err(e) => {
                        tracing::warn!(path = %flow_path.display(), error = %e, "failed to load flow JSON — skipping");
                    }
                }
            }
        }

        // 11. Register site-main flow
        crate::flows::register_site_main(&mut wafer)?;

        Ok((wafer, storage_block))
    }
}

/// Call after `wafer.start()` or `wafer.start_without_bind()` to inject
/// collected WRAP grants into the storage block for cross-block access control.
pub fn post_start(wafer: &Wafer, storage_block: &SolobaseStorageBlock) {
    storage_block.update_wrap_grants(wafer.wrap_grants());
}

/// Register the `wafer-run/vector` runtime block backed by native
/// `SqliteVecService` + `FastembedService`.
///
/// - Opens a dedicated `rusqlite::Connection` at `db_path`. SQLite supports
///   multi-connection access with WAL, so sharing the DB file with the
///   platform's `DatabaseService` connection is safe.
/// - `FastembedService::default_model()` triggers an ONNX model download on
///   first run. Failure is logged but does not abort startup — the vector
///   runtime block simply won't be registered, and any attempt to use it
///   will fail via the normal dependency-resolution path.
///
/// This function is only compiled when the `native-embedding` feature is on;
/// the `suppers-ai/vector` feature block registration in `solobase-core` is
/// gated by the same feature so the two stay in sync.
#[cfg(feature = "native-embedding")]
fn register_vector_block(wafer: &mut Wafer, db_path: Option<&str>) -> Result<(), RuntimeError> {
    use wafer_block_fastembed::FastembedService;
    use wafer_block_sqlite::vector::SqliteVecService;
    use wafer_core::interfaces::vector::service::{EmbeddingService, VectorService};

    let Some(db_path) = db_path else {
        return Err(RuntimeError::from(
            "native-embedding feature is enabled but no sqlite_db_path was \
             provided to SolobaseBuilder — call .sqlite_db_path(...) before \
             .build()"
                .to_string(),
        ));
    };

    // Dedicated connection for the vector service — see module docs on
    // `sqlite_db_path` for why a second connection is fine.
    let vec_conn = rusqlite::Connection::open(db_path).map_err(|e| {
        RuntimeError::from(format!(
            "failed to open SQLite connection at '{db_path}' for vector service: {e}"
        ))
    })?;
    let vec_svc: Arc<dyn VectorService> = Arc::new(SqliteVecService::new(vec_conn));

    let emb_svc: Arc<dyn EmbeddingService> = match FastembedService::default_model() {
        Ok(svc) => Arc::new(svc),
        Err(e) => {
            // Model download can fail offline or on first-run with restricted
            // egress. Log and skip registration so the rest of the runtime
            // boots; `suppers-ai/vector` registration will fail dep resolution
            // with a clearer error than a half-wired block would.
            tracing::warn!(
                error = ?e,
                "fastembed model unavailable — skipping wafer-run/vector registration"
            );
            return Ok(());
        }
    };

    wafer_core::service_blocks::vector::register_with(wafer, vec_svc, emb_svc)
}
