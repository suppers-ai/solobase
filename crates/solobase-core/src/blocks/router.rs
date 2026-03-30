//! Solobase router block — delegates to the shared `solobase_core` pipeline.
//!
//! This block replaces the individual per-feature flow definitions (auth, admin,
//! files, etc.) by routing all API requests through `crate::handle_request()`.
//! The WAFER flow engine still provides middleware (CORS, security headers) via
//! `wafer-run/infra`, but routing, feature gates, admin checks, and JWT validation
//! are handled by the shared pipeline.

use std::collections::HashMap;
use std::sync::Arc;

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::types::{LifecycleEvent, WaferError};

use crate::features::FeatureConfig;
use crate::routing::BlockId;

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
}

/// The solobase router block — dispatches all API requests via the shared
/// `crate::handle_request()` pipeline.
pub struct SolobaseRouterBlock {
    jwt_secret: String,
    features: Arc<dyn FeatureConfig>,
    factory: NativeBlockFactory,
}

impl SolobaseRouterBlock {
    pub fn new(
        jwt_secret: String,
        features: Arc<dyn FeatureConfig>,
        factory: NativeBlockFactory,
    ) -> Self {
        Self {
            jwt_secret,
            features,
            factory,
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SolobaseRouterBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/router".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Solobase shared router — delegates to solobase-core pipeline".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: BlockRuntime::Native,
            requires: Vec::new(),
            collections: Vec::new(),
        }
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> Result<(), WaferError> {
        Ok(()) // No-op — individual blocks handle their own lifecycle
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
            auth_value.as_deref(),
            &self.jwt_secret,
            self.features.as_ref(),
            &self.factory,
        )
        .await
    }
}
