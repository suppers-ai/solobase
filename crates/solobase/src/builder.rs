//! SolobaseBuilder — unified WAFER runtime setup for all platforms.
//!
//! Each platform (native, browser WASM, Cloudflare Workers) provides its own
//! service implementations and calls the builder. The builder handles all
//! common registration: service blocks, middleware, feature blocks, router, flow.

use std::collections::HashMap;
use std::sync::Arc;

use solobase_core::blocks::storage::SolobaseStorageBlock;
use solobase_core::features::{BlockSettings, FeatureConfig};
use wafer_core::interfaces::config::service::ConfigService;
use wafer_core::interfaces::crypto::service::CryptoService;
use wafer_core::interfaces::database::service::DatabaseService;
use wafer_core::interfaces::logger::service::LoggerService;
use wafer_core::interfaces::network::service::NetworkService;
use wafer_core::interfaces::storage::service::StorageService;
use wafer_run::block::Block;
use wafer_run::Wafer;

use solobase_core::blocks::router::{NativeBlockFactory, SolobaseRouterBlock};

/// Result of `SolobaseBuilder::build()`.
///
/// Contains the configured `Wafer` runtime and references needed for
/// post-start setup. Call `inject_wrap_grants()` after `wafer.start()`
/// or `wafer.start_without_bind()` to propagate WRAP grants into the
/// storage block for cross-block access control.
pub struct BuildResult {
    pub wafer: Wafer,
    storage_block: Arc<SolobaseStorageBlock>,
}

impl BuildResult {
    /// Inject collected WRAP grants into the storage block.
    /// Must be called after `wafer.start()` / `wafer.start_without_bind()`.
    pub fn inject_wrap_grants(&self, wafer: &Wafer) {
        self.storage_block.update_wrap_grants(wafer.wrap_grants());
    }
}

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

    pub fn block_config(mut self, name: &str, config: serde_json::Value) -> Self {
        self.block_configs.push((name.to_string(), config));
        self
    }

    pub fn build(self) -> Result<BuildResult, String> {
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
        wafer.register_block("wafer-run/storage", storage_block.clone())?;
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

        // 5b. Apply platform-specific block configs
        for (name, config) in self.block_configs {
            wafer.add_block_config(&name, config);
        }

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

        Ok(BuildResult {
            wafer,
            storage_block,
        })
    }
}
