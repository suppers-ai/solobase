//! SolobaseBuilder — unified WAFER runtime setup for all platforms.
//!
//! Each platform (native, browser WASM, Cloudflare Workers) provides its own
//! service implementations and calls the builder. The builder handles all
//! common registration: service blocks, middleware, feature blocks, router, flow.

use std::{collections::HashMap, sync::Arc};

// Force linker inclusion of wafer-block-* crates so their linkme
// distributed-slice entries land in the binary. Without these `use as _`
// anchors the linker excludes the crate's .o file entirely and the
// register_static_block! entries never appear in STATIC_BLOCK_REGISTRATIONS.
use wafer_block_cors as _;
use wafer_block_inspector as _;
use wafer_block_readonly_guard as _;
use wafer_block_router as _;
use wafer_block_security_headers as _;
use wafer_block_web as _;
use wafer_core::interfaces::{
    config::service::ConfigService, crypto::service::CryptoService,
    database::service::DatabaseService, image::service::ImageService, llm::service::LlmService,
    logger::service::LoggerService, network::service::NetworkService,
    storage::service::StorageService,
};
use wafer_run::{block::Block, RuntimeError, Wafer};

use crate::{
    blocks::{router::SolobaseRouterBlock, storage::SolobaseStorageBlock},
    features::{BlockSettings, FeatureConfig},
    ExtraRoute, RouteAccess,
};

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
    /// Additional `ImageService` backends to register on the
    /// `MultiBackendImageService` router backing `wafer-run/image`. Same
    /// shape and order semantics as `extra_llm_services`. No built-in
    /// provider on native — the prototype's only backend is
    /// `BrowserImageService` from `solobase-web`.
    extra_image_services: Vec<(String, Arc<dyn ImageService>)>,
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
    /// Browser-side `VectorService` + `EmbeddingService`. When both are
    /// `Some`, `build()` registers `wafer-run/vector` (with the pair) and
    /// `suppers-ai/transformers-embed` (with the embedding service). The
    /// native `register_vector_block` path is gated behind the
    /// `native-embedding` feature and remains unaffected.
    extra_vector_service: Option<Arc<dyn wafer_core::interfaces::vector::service::VectorService>>,
    extra_embedding_service:
        Option<Arc<dyn wafer_core::interfaces::vector::service::EmbeddingService>>,
    /// Per-block env-config source consulted on first init. Defaults to an
    /// empty [`wafer_run::StaticConfigSource`] if unset — sufficient for
    /// blocks that declare no required config or that read their config from
    /// `RuntimeContext::block_configs` (composite/uses). Native consumers
    /// should pass `EnvConfigSource`; cloudflare consumers pass
    /// `D1ConfigSource`.
    config_source: Option<Arc<dyn wafer_run::ConfigSource>>,
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
            extra_image_services: Vec::new(),
            extra_routes: Vec::new(),
            sqlite_db_path: None,
            extra_vector_service: None,
            extra_embedding_service: None,
            config_source: None,
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

    pub fn extra_block(mut self, name: impl Into<String>, block: Arc<dyn Block>) -> Self {
        self.extra_blocks.push((name.into(), block));
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

    /// Register an additional `ImageService` backend on the router backing
    /// `wafer-run/image`. Mirrors `llm_service` — `label` is for tracing,
    /// dispatch is by `claims_backend`. Order semantics: first
    /// `claims_backend` match wins.
    pub fn image_service(
        mut self,
        label: impl Into<String>,
        service: Arc<dyn ImageService>,
    ) -> Self {
        self.extra_image_services.push((label.into(), service));
        self
    }

    /// Inject a browser-side `VectorService` (e.g. `BrowserVectorService` from
    /// `solobase-browser`). When both `vector_service` and `embedding_service`
    /// are provided, `build()` registers `wafer-run/vector` with the pair and
    /// `suppers-ai/transformers-embed` with the embedding half. Mutually
    /// exclusive with the `native-embedding` feature path — both produce
    /// `wafer-run/vector` and would conflict on register.
    pub fn vector_service(
        mut self,
        svc: Arc<dyn wafer_core::interfaces::vector::service::VectorService>,
    ) -> Self {
        self.extra_vector_service = Some(svc);
        self
    }

    /// Inject a browser-side `EmbeddingService` (e.g. `BrowserEmbeddingService`
    /// from `solobase-browser`). See `vector_service` for full semantics.
    pub fn embedding_service(
        mut self,
        svc: Arc<dyn wafer_core::interfaces::vector::service::EmbeddingService>,
    ) -> Self {
        self.extra_embedding_service = Some(svc);
        self
    }

    /// Supply the runtime's [`wafer_run::ConfigSource`] for lazy per-block
    /// env-config loading. If not provided, defaults to an empty
    /// [`wafer_run::StaticConfigSource`] — sufficient for blocks that declare
    /// no required config or that read their config from
    /// `RuntimeContext::block_configs` (composite/uses). Native consumers
    /// should pass `EnvConfigSource`; cloudflare consumers pass
    /// `D1ConfigSource`.
    pub fn config_source(mut self, source: Arc<dyn wafer_run::ConfigSource>) -> Self {
        self.config_source = Some(source);
        self
    }

    pub fn block_config(mut self, name: impl Into<String>, config: serde_json::Value) -> Self {
        self.block_configs.push((name.into(), config));
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
            .get(crate::blocks::auth::JWT_SECRET_KEY)
            .unwrap_or_default();

        // 3. Create runtime
        let config_source = self
            .config_source
            .clone()
            .unwrap_or_else(|| Arc::new(wafer_run::StaticConfigSource::default()));
        let mut wafer = Wafer::new(config_source)?;
        wafer.set_admin_block("suppers-ai/admin");

        // 4. Register service blocks
        wafer_core::service_blocks::database::register_with(&mut wafer, database)?;
        wafer.add_alias("db", "wafer-run/database");

        // `Arc::from(&'static str)` allocates the inline buffer once; no
        // `String::to_string` round-trip needed for a literal identifier.
        let admin_block_id: Arc<str> = Arc::from("suppers-ai/admin");
        let storage_block = crate::blocks::storage::create(storage, admin_block_id);
        wafer.register_block("wafer-run/storage", storage_block.clone())?;
        wafer.add_alias("storage", "wafer-run/storage");

        let config_ref = config.clone();
        wafer_core::service_blocks::config::register_with(&mut wafer, config)?;
        wafer_core::service_blocks::crypto::register_with(&mut wafer, crypto)?;

        let network_block = crate::blocks::network::create(network);
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
            let svc = Arc::new(crate::blocks::llm::providers::ProviderLlmService::new());
            llm_router.register("provider", svc.clone());
            svc
        };

        for (label, svc) in self.extra_llm_services {
            llm_router.register(label, svc);
        }

        wafer_core::service_blocks::llm::register_with(&mut wafer, Arc::new(llm_router))?;

        // 4a-bis. Build the image router and register the service block
        // backing `wafer-run/image`. Mirrors the LLM path above — no built-in
        // native provider for the prototype; backends are populated entirely
        // from `.image_service(...)` entries (typically a `BrowserImageService`
        // from `solobase-web`).
        let mut image_router =
            wafer_core::interfaces::image::router::MultiBackendImageService::new();
        for (label, svc) in self.extra_image_services {
            image_router.register(label, svc);
        }
        wafer_core::service_blocks::image::register_with(&mut wafer, Arc::new(image_router))?;

        // 4b. Register the `wafer-run/vector` runtime block when the
        // `native-embedding` feature is on. `suppers-ai/vector` declares
        // `requires=["wafer-run/vector"]`, so without this registration
        // dependency resolution fails at startup.
        #[cfg(feature = "native-embedding")]
        register_vector_block(&mut wafer, self.sqlite_db_path.as_deref())?;

        // Browser path: when callers (typically `solobase-web`) inject vector
        // + embedding services, register the runtime block + transformers
        // embed feature block. Mutually exclusive with `native-embedding` —
        // both producing `wafer-run/vector` would conflict on register.
        if let (Some(vec_svc), Some(emb_svc)) =
            (self.extra_vector_service, self.extra_embedding_service)
        {
            wafer_core::service_blocks::vector::register_with(
                &mut wafer,
                vec_svc,
                emb_svc.clone(),
            )?;
            #[cfg(target_arch = "wasm32")]
            {
                wafer.register_block(
                    "suppers-ai/transformers-embed".to_string(),
                    Arc::new(
                        crate::blocks::transformers_embed::TransformersEmbedBlock::new(emb_svc),
                    ),
                )?;
            }
        }

        // 5. All middleware blocks (cors, inspector, readonly-guard, router,
        // security-headers, web) self-register via register_static_block! in
        // their respective wafer-block-* crates. The `use wafer_block_xxx as _`
        // anchors at the top of this file ensure the linker includes those crate
        // .o files so the linkme distributed-slice entries land in the binary.
        //
        // linkme's distributed_slice does not work on wasm32 (its link-section
        // attributes only target ELF/Mach-O/PE — see linkme-impl/src/declaration.rs
        // for the target_os match), so on wasm32 the auto-registration is a no-op.
        // Register the six middleware blocks explicitly when targeting wasm32.
        #[cfg(target_arch = "wasm32")]
        {
            wafer.register_block(
                "wafer-run/cors",
                Arc::new(wafer_block_cors::CorsBlock::new()),
            )?;
            wafer.register_block(
                "wafer-run/inspector",
                Arc::new(wafer_block_inspector::InspectorBlock::new()),
            )?;
            wafer.register_block(
                "wafer-run/readonly-guard",
                Arc::new(wafer_block_readonly_guard::ReadonlyGuardBlock::new()),
            )?;
            wafer.register_block(
                "wafer-run/router",
                Arc::new(wafer_block_router::RouterBlock::new()),
            )?;
            wafer.register_block(
                "wafer-run/security-headers",
                Arc::new(wafer_block_security_headers::SecurityHeadersBlock::new()),
            )?;
            wafer.register_block("wafer-run/web", Arc::new(wafer_block_web::WebBlock::new()))?;

            // Solobase feature blocks (suppers-ai/*) self-register via
            // `register_static_block!` on native, but linkme's distributed_slice
            // doesn't emit on wasm32 — see `crate::blocks::register_all_static_blocks`
            // for the full reasoning. Without this call the wasm runtime has only
            // wafer-run/* middleware and the SolobaseRouter resolves every feature
            // route to a `block 'suppers-ai/<name>' not found` error.
            crate::blocks::register_all_static_blocks(&mut wafer)?;
        }

        wafer.add_block_config(
            "wafer-run/inspector",
            serde_json::json!({ "allow_anonymous": false }),
        );

        // 5b. Apply platform-specific block configs
        for (name, config) in self.block_configs {
            wafer.add_block_config(&name, config);
        }

        // 6. Register the framework AuthBlock — it can't self-register via
        //    register_static_block! because its constructor takes
        //    Arc<dyn AuthService>. The wrapped AuthServiceImpl picks up its
        //    Context handle when the runtime fires the block's
        //    lifecycle(Init) event.
        crate::blocks::register_auth(&mut wafer)?;

        // 6b. Register LlmBlock — it can't self-register via register_static_block!
        //     because its constructor takes Arc<ProviderLlmService>. All other solobase
        //     blocks self-register via register_static_block! at link time.
        #[cfg(feature = "llm")]
        crate::blocks::register_llm(&mut wafer, provider_llm_svc.clone())?;

        // 7. Extra platform-specific blocks
        for (name, block) in self.extra_blocks {
            wafer.register_block(&name, block)?;
        }

        // 9. Inject feature block configs from the ConfigService.
        //
        // The WAFER runtime validates required config vars at start() time by
        // checking `block_configs` (serde_json objects), not the ConfigService
        // directly. Feature blocks don't have explicit block_configs — they read
        // from the config service at lifecycle(Init) time. We bridge this gap by
        // reading each declared config var from the ConfigService and injecting
        // the non-empty values into the wafer block_configs so that the validator
        // sees them as provided.
        {
            let block_infos = crate::blocks::all_block_infos();
            for info in &block_infos {
                let mut obj = serde_json::Map::new();
                for cv in &info.config_keys {
                    if let Some(val) = config_ref.get(&cv.key) {
                        if !val.is_empty() {
                            obj.insert(cv.key.clone(), serde_json::Value::String(val));
                        }
                    }
                }
                if !obj.is_empty() {
                    wafer.add_block_config(&info.name, serde_json::Value::Object(obj));
                }
            }
        }

        // 10. Build and register the solobase router.
        //     Collect BlockInfo from the registry AFTER all blocks are registered
        //     so that the discovery endpoints (/openapi.json, /.well-known/agent.json)
        //     see the full set. Wafer is the single source of truth — no parallel
        //     HashMap needed.
        let feature_config: Arc<dyn FeatureConfig> = Arc::new(self.block_settings);
        let block_infos = wafer.block_infos();
        let router = SolobaseRouterBlock::with_extra_routes(
            jwt_secret,
            feature_config,
            block_infos,
            self.extra_routes,
        );
        wafer.register_block("suppers-ai/router", Arc::new(router))?;
        wafer.add_block_config("suppers-ai/router", crate::routing::routes_config());

        // 11. Auto-discover WASM blocks from cwd/blocks/**/target/block.wasm
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

        // 12. Register site-main flow
        crate::flows::register_site_main(&mut wafer)?;

        Ok((wafer, storage_block))
    }
}

/// Call after `wafer.start()` or `wafer.seal()` to inject
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
