mod handlers;

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use super::rate_limit::{UserRateLimiter, RateLimit, check_rate_limit};

pub(crate) const PROJECTS_COLLECTION: &str = "block_deployments";

pub struct ProjectsBlock {
    limiter: UserRateLimiter,
}

impl Default for ProjectsBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectsBlock {
    pub fn new() -> Self {
        Self { limiter: UserRateLimiter::new() }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for ProjectsBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::types::CollectionSchema;

        BlockInfo {
            name: "suppers-ai/projects".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Project management for users and admins".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
            collections: vec![
                CollectionSchema::new("block_deployments")
                    .field_ref("user_id", "string", "auth_users.id")
                    .field("name", "string")
                    .field_default("slug", "string", "")
                    .field_default("status", "string", "pending")
                    .field_default("config", "json", "{}")
                    .field_default("plan_id", "string", "")
                    .field_default("purchase_id", "string", "")
                    .field_default("tenant_id", "string", "")
                    .field_default("subdomain", "string", "")
                    .field_optional("provision_error", "string")
                    .field_optional("deprovision_error", "string")
                    .field_optional("grace_period_end", "datetime")
                    .field_optional("deleted_at", "datetime")
                    .index(&["user_id"])
                    .index(&["status"]),
            ],
        }
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();

        // Per-user rate limiting for authenticated endpoints
        let user_id = msg.user_id().to_string();
        if !user_id.is_empty() {
            let action = msg.action().to_string();
            let (default, category) = if action == "retrieve" {
                (RateLimit::API_READ, "api_read")
            } else {
                (RateLimit::API_WRITE, "api_write")
            };
            if let Some(r) = check_rate_limit(&self.limiter, ctx, msg, &user_id, category, default).await {
                return r;
            }
        }

        // Admin routes
        if path.starts_with("/admin/b/projects") {
            return handlers::handle_admin(ctx, msg).await;
        }

        // User-facing routes
        if path.starts_with("/b/projects") {
            return handlers::handle_user(ctx, msg).await;
        }

        err_not_found(msg, "not found")
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
