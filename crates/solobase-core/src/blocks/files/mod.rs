mod storage;
mod cloud;
mod quota;
mod share;
pub(crate) mod models;

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use super::rate_limit::{UserRateLimiter, RateLimit, check_rate_limit};

pub struct FilesBlock {
    limiter: UserRateLimiter,
}

impl Default for FilesBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl FilesBlock {
    pub fn new() -> Self {
        Self { limiter: UserRateLimiter::new() }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for FilesBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/files".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "File storage, sharing, quotas, and access logging".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
        }
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();

        // Direct share access (public, no auth, no user rate limit)
        if path.starts_with("/storage/direct/") {
            return share::handle_direct_access(ctx, msg).await;
        }

        // Per-user rate limiting for authenticated endpoints
        let user_id = msg.user_id().to_string();
        if !user_id.is_empty() {
            let action = msg.action().to_string();
            let (default, category) = if action == "create" {
                (RateLimit::UPLOAD, "upload")
            } else if action == "retrieve" {
                (RateLimit::API_READ, "api_read")
            } else {
                (RateLimit::API_WRITE, "api_write")
            };
            if let Some(r) = check_rate_limit(&self.limiter, ctx, msg, &user_id, category, default).await {
                return r;
            }
        }

        // Cloud storage routes
        if path.starts_with("/b/cloudstorage") || path.starts_with("/admin/b/cloudstorage") {
            return cloud::handle(ctx, msg).await;
        }

        // Admin storage routes
        if path.starts_with("/admin/storage") {
            return storage::handle_admin(ctx, msg).await;
        }

        // User storage routes
        if path.starts_with("/storage") {
            return storage::handle(ctx, msg).await;
        }

        err_not_found(msg, "not found")
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
