//! Glue: construct `AuthServiceImpl`, wrap in wafer-core's unified
//! `AuthBlock`, and register it under `suppers-ai/auth`.
//!
//! Consumers (the solobase host) call [`register`] during runtime wiring
//! after building the per-project `Context`. The resulting block responds
//! to `auth.require_user`, `auth.require_token`, `auth.require_role`
//! messages — see `wafer_core::interfaces::auth::handler`.

use std::sync::Arc;

use wafer_core::service_blocks::auth as auth_block;
use wafer_run::{context::Context, BlockRegistry, RuntimeError};

use super::service::{AuthServiceImpl, BlockState};

/// Register the `suppers-ai/auth` block with an `AuthServiceImpl` backed
/// by the given runtime context.
pub fn register(
    registry: &mut dyn BlockRegistry,
    ctx: Arc<dyn Context>,
) -> Result<(), RuntimeError> {
    let svc = Arc::new(AuthServiceImpl::new(BlockState::new(ctx)));
    auth_block::register_with(registry, svc)
}
