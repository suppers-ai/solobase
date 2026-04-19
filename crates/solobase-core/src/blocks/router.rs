//! Solobase router block — delegates to the shared `solobase_core` pipeline.
//!
//! This block replaces the individual per-feature flow definitions (auth, admin,
//! files, etc.) by routing all API requests through `crate::handle_request()`.
//! The WAFER flow engine still provides middleware (CORS, security headers) via
//! `wafer-run/infra`, but routing, feature gates, admin checks, and JWT validation
//! are handled by the shared pipeline.

use std::{collections::HashMap, sync::Arc};

use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use crate::{
    features::FeatureConfig,
    routing::{BlockId, ExtraRoute},
};

/// Block factory that returns shared block instances (same Arc across requests).
///
/// Unlike the CF adapter which creates fresh instances per request, the native
/// factory keeps shared instances so that stateful blocks (e.g. AuthBlock with
/// its in-memory rate limiter) maintain state across requests.
pub struct NativeBlockFactory {
    blocks: HashMap<BlockId, Arc<dyn Block>>,
}

impl NativeBlockFactory {
    pub fn new(blocks: HashMap<BlockId, Arc<dyn Block>>) -> Self {
        Self { blocks }
    }
}

impl crate::BlockFactory for NativeBlockFactory {
    fn create(&self, block_id: BlockId) -> Option<Arc<dyn Block>> {
        self.blocks.get(&block_id).cloned()
    }

    fn all_block_infos(&self) -> Vec<wafer_run::BlockInfo> {
        self.blocks.values().map(|b| b.info()).collect()
    }
}

/// The solobase router block — dispatches all API requests via the shared
/// `crate::handle_request()` pipeline.
pub struct SolobaseRouterBlock {
    jwt_secret: String,
    features: Arc<dyn FeatureConfig>,
    factory: NativeBlockFactory,
    /// Runtime-added routes from downstream projects (see `SolobaseBuilder::add_route`).
    /// Built-in `ROUTES` take priority — see `routing::route_to_block`.
    extra_routes: Arc<Vec<ExtraRoute>>,
}

impl SolobaseRouterBlock {
    /// Construct a router with no extra routes (backward-compatible).
    pub fn new(
        jwt_secret: String,
        features: Arc<dyn FeatureConfig>,
        factory: NativeBlockFactory,
    ) -> Self {
        Self::with_extra_routes(jwt_secret, features, factory, Vec::new())
    }

    /// Construct a router with project-registered extra routes appended after
    /// the built-in `ROUTES` table.
    pub fn with_extra_routes(
        jwt_secret: String,
        features: Arc<dyn FeatureConfig>,
        factory: NativeBlockFactory,
        extra_routes: Vec<ExtraRoute>,
    ) -> Self {
        Self {
            jwt_secret,
            features,
            factory,
            extra_routes: Arc::new(extra_routes),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SolobaseRouterBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new(
            "suppers-ai/router",
            "0.0.1",
            "http-handler@v1",
            "Solobase shared router — delegates to solobase-core pipeline",
        )
        .instance_mode(InstanceMode::Singleton)
        .category(wafer_run::BlockCategory::Infrastructure)
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> Result<(), WaferError> {
        Ok(()) // No-op — individual blocks handle their own lifecycle
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        // Resolve auth token from Authorization header or auth_token cookie.
        let auth_header = msg.header("authorization");
        let auth_value = if !auth_header.is_empty() {
            Some(auth_header.to_string())
        } else {
            let cookie_token = msg.cookie("auth_token");
            if !cookie_token.is_empty() {
                Some(format!("Bearer {}", cookie_token))
            } else {
                None
            }
        };

        crate::handle_request(
            ctx,
            msg,
            input,
            auth_value.as_deref(),
            &self.jwt_secret,
            self.features.as_ref(),
            &self.factory,
            &self.extra_routes,
        )
        .await
    }
}
