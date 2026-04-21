//! Glue: construct `AuthServiceImpl`, wrap in a solobase-local block that
//! both fulfils the `auth@v1` service interface (delegating message handling
//! to `wafer_core::interfaces::auth::handler`) and runs migrations + the
//! first-run bootstrap on `Init`.
//!
//! The core `wafer_core::service_blocks::auth::AuthBlock` has a no-op
//! lifecycle on purpose — the service crate has no opinion on schema or
//! seed data. Those concerns are solobase-local, so we layer them here in a
//! thin wrapper around the same `AuthService`.
//!
//! Plan A2 Cluster B also mounts the `/auth/*` HTTP handlers on this block
//! via [`SolobaseAuthBlock::handle`]: the block inspects `msg.action()` +
//! `msg.path()` and routes to `handlers::login`, `handlers::me`, or
//! `handlers::tokens`. Non-HTTP messages (service-op dispatches like
//! `auth.require_user`) fall through to the wafer-core handler.

use std::sync::Arc;

use wafer_core::interfaces::auth::{handler, service::AuthService};
use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::{LifecycleEvent, LifecycleType, Message, WaferError},
    BlockCategory, BlockRegistry, InputStream, OutputStream, RuntimeError,
};

use super::{
    bootstrap,
    config::AuthConfig,
    handlers::{self, HttpReply},
    migrations,
    service::{AuthServiceImpl, BlockState},
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

    /// List of `{METHOD} {PATH}` strings this block responds to. Test-only
    /// introspection for Layer-2 "routes-mounted" checks.
    pub fn mounted_routes() -> &'static [&'static str] {
        &[
            "POST /auth/login",
            "POST /auth/logout",
            "GET /auth/me",
            "GET /auth/tokens",
            "POST /auth/tokens",
            "DELETE /auth/tokens/{id}",
        ]
    }
}

/// Map an HTTP method name to the wafer-run action verb. Accepts both the
/// raw HTTP verb the adapter emits (e.g. `"GET"`) and the action-verb form
/// (`"retrieve"`) so tests don't need to care which shape they mock.
fn normalise_action(a: &str) -> &'static str {
    match a.to_ascii_uppercase().as_str() {
        "GET" | "RETRIEVE" => "retrieve",
        "POST" | "CREATE" => "create",
        "PUT" | "UPDATE" => "update",
        "DELETE" => "delete",
        _ => "",
    }
}

/// Extract the `{id}` segment from `/auth/tokens/{id}`.
fn tokens_id(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("/auth/tokens/")?;
    if rest.is_empty() || rest.contains('/') {
        None
    } else {
        Some(rest)
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

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let action = normalise_action(msg.action());
        let path = msg.path().to_string();

        // Collect body once — `InputStream::collect_to_bytes` consumes self.
        let body = input.collect_to_bytes().await;

        // HTTP endpoints — Plan A2 routes.
        let http_reply: Option<Result<HttpReply, WaferError>> = match (action, path.as_str()) {
            ("create", "/auth/login") => {
                Some(handlers::login::post_login(ctx, &self.config, &body).await)
            }
            ("create", "/auth/logout") => Some(handlers::login::post_logout(ctx, &msg).await),
            ("retrieve", "/auth/me") => {
                Some(Ok(handlers::me::get_me(self.service.as_ref(), &msg).await))
            }
            ("retrieve", "/auth/tokens") => {
                Some(handlers::tokens::list_tokens(ctx, self.service.as_ref(), &msg).await)
            }
            ("create", "/auth/tokens") => {
                Some(handlers::tokens::create_token(ctx, self.service.as_ref(), &msg, &body).await)
            }
            ("delete", p) if tokens_id(p).is_some() => {
                let id = tokens_id(p).expect("guarded by match arm").to_string();
                Some(handlers::tokens::delete_token(ctx, self.service.as_ref(), &msg, &id).await)
            }
            _ => None,
        };

        if let Some(reply) = http_reply {
            return match reply {
                Ok(r) => r.into(),
                Err(e) => OutputStream::error(e),
            };
        }

        // Non-HTTP messages (service-op dispatch: auth.require_user, …).
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
