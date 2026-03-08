mod handlers;

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use super::rate_limit::{UserRateLimiter, RateLimit, set_rate_limit_headers, rate_limited_response};

pub(crate) const DEPLOYMENTS_COLLECTION: &str = "ext_deployments";

pub struct DeploymentsBlock {
    limiter: UserRateLimiter,
}

impl DeploymentsBlock {
    pub fn new() -> Self {
        Self { limiter: UserRateLimiter::new() }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for DeploymentsBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "@solobase/deployments".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Deployment management for users and admins".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
        }
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();

        // Per-user rate limiting for authenticated endpoints
        let user_id = msg.user_id().to_string();
        if !user_id.is_empty() {
            let action = msg.action().to_string();
            let (default, name) = if action == "retrieve" {
                (RateLimit::API_READ, "api_read")
            } else {
                (RateLimit::API_WRITE, "api_write")
            };
            if let Some(limit) = default.resolve(ctx, name).await {
                let key = UserRateLimiter::key(&user_id, "deployments");
                match self.limiter.check(&key, limit) {
                    Ok(remaining) => set_rate_limit_headers(msg, limit.max_requests, remaining),
                    Err(retry_after) => return rate_limited_response(msg, retry_after),
                }
            }
        }

        // Admin routes
        if path.starts_with("/admin/ext/deployments") {
            return handlers::handle_admin(ctx, msg).await;
        }

        // User-facing routes
        if path.starts_with("/ext/deployments") {
            return handlers::handle_user(ctx, msg).await;
        }

        err_not_found(msg, "not found")
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
