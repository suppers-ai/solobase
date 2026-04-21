//! Glue: construct `AuthServiceImpl`, wrap in a solobase-local block that
//! both fulfils the `auth@v1` service interface (delegating message handling
//! to `wafer_core::interfaces::auth::handler`) and runs migrations + the
//! first-run bootstrap on `Init`.
//!
//! The core `wafer_core::service_blocks::auth::AuthBlock` has a no-op
//! lifecycle on purpose — the service crate has no opinion on schema or
//! seed data. Those concerns are solobase-local, so we layer them here in a
//! thin wrapper around the same `AuthService`.

use std::sync::Arc;

use wafer_core::interfaces::auth::{handler, service::AuthService};
use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::{LifecycleEvent, LifecycleType, Message, WaferError},
    BlockCategory, BlockRegistry, InputStream, OutputStream, RuntimeError,
};

use super::{
    bootstrap, config::AuthConfig, migrations, service::AuthServiceImpl, service::BlockState,
};

/// Solobase-local auth block. Wraps any [`AuthService`] implementation and
/// adds an `Init` hook that applies migrations and runs the bootstrap
/// admin/token flow.
pub struct SolobaseAuthBlock {
    service: Arc<dyn AuthService>,
    config: AuthConfig,
}

impl SolobaseAuthBlock {
    pub fn new(service: Arc<dyn AuthService>, config: AuthConfig) -> Self {
        Self { service, config }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SolobaseAuthBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new(
            "suppers-ai/auth",
            "0.0.1",
            "auth@v1",
            "Identity, sessions, PATs, orgs — see auth-block-design spec",
        )
        .category(BlockCategory::Service)
    }

    async fn handle(&self, _ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let body = input.collect_to_bytes().await;
        handler::handle_message(self.service.as_ref(), &msg, &body).await
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            migrations::apply(ctx).await.map_err(|e| {
                WaferError::new(
                    wafer_run::types::ErrorCode::INTERNAL,
                    format!("auth migrations: {e}"),
                )
            })?;
            bootstrap::run(ctx, &self.config).await?;
        }
        Ok(())
    }
}

/// Register the solobase auth block with an [`AuthServiceImpl`] backed by
/// the given runtime context. The block's `Init` hook runs migrations and
/// bootstrap — so callers must register the `wafer-run/database` and
/// `wafer-run/crypto` service blocks before the lifecycle fires.
pub fn register(
    registry: &mut dyn BlockRegistry,
    ctx: Arc<dyn Context>,
) -> Result<(), RuntimeError> {
    register_with_config(registry, ctx, AuthConfig::from_env_for_test(&[]))
}

/// Register with an explicit [`AuthConfig`]. Tests use this to inject
/// bootstrap env values without touching a real `wafer-run/config` block.
pub fn register_with_config(
    registry: &mut dyn BlockRegistry,
    ctx: Arc<dyn Context>,
    config: AuthConfig,
) -> Result<(), RuntimeError> {
    let svc: Arc<dyn AuthService> = Arc::new(AuthServiceImpl::new(BlockState::new(ctx)));
    registry.register_block(
        "suppers-ai/auth",
        Arc::new(SolobaseAuthBlock::new(svc, config)),
    )
}
