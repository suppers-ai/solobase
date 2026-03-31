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
        use wafer_run::types::CollectionSchema;

        BlockInfo {
            name: "suppers-ai/files".to_string(),
            version: "0.0.1".to_string(),
            interface: "http-handler@v1".to_string(),
            summary: "File storage, sharing, quotas, and access logging".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: vec!["wafer-run/database".into(), "wafer-run/storage".into(), "wafer-run/config".into()],
            collections: vec![
                CollectionSchema::new("storage_buckets")
                    .field("name", "string")
                    .field_default("public", "bool", "false")
                    .field_default("created_by", "string", ""),
                CollectionSchema::new("storage_objects")
                    .field("bucket", "string")
                    .field("key", "string")
                    .field_default("size", "int", "0")
                    .field_default("content_type", "string", "application/octet-stream")
                    .field_default("uploaded_by", "string", "")
                    .field_optional("uploaded_at", "datetime")
                    .index(&["bucket"]),
                CollectionSchema::new("storage_views")
                    .field("bucket", "string")
                    .field("key", "string")
                    .field_default("user_id", "string", "")
                    .field_optional("viewed_at", "datetime"),
                CollectionSchema::new("cloud_shares")
                    .field("token", "string")
                    .field("bucket", "string")
                    .field("key", "string")
                    .field_default("created_by", "string", "")
                    .field_optional("expires_at", "datetime")
                    .field_default("access_count", "int", "0")
                    .field_optional("max_access_count", "int")
                    .index(&["token"]),
                CollectionSchema::new("cloud_access_logs")
                    .field("share_id", "string")
                    .field_optional("accessed_at", "datetime")
                    .field_default("ip_address", "string", "")
                    .field_default("user_agent", "string", "")
                    .index(&["share_id"]),
                CollectionSchema::new("cloud_quotas")
                    .field_unique("user_id", "string")
                    .field_default("max_storage_bytes", "int64", "1073741824")
                    .field_default("max_file_size_bytes", "int64", "104857600")
                    .field_default("max_files_per_bucket", "int", "10000")
                    .field_default("reset_period_days", "int", "0"),
            ],
            config_schema: None,
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
