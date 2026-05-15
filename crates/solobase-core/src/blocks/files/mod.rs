mod cloud;
mod migrations;
pub(crate) mod models;
mod pages_admin;
pub(crate) mod pages_user;
mod quota;
mod share;
pub(crate) mod storage;

pub(crate) use quota::TABLE as QUOTAS_TABLE;
pub(crate) use share::{ACCESS_LOGS_TABLE, SHARES_TABLE};
pub(crate) use storage::{BUCKETS_TABLE, OBJECTS_TABLE};

/// Object-view audit table. Has no dedicated owner module — only the
/// schema declaration here and a single insert site in `storage.rs`
/// (`record_view`) — so the constant lives in mod.rs.
pub(crate) const VIEWS_TABLE: &str = "suppers_ai__files__views";

use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use super::rate_limit::{check_rate_limit, RateLimit, RateLimitOutcome, UserRateLimiter};
use crate::blocks::helpers::{self, err_not_found};

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
        Self {
            limiter: UserRateLimiter::new(),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for FilesBlock {
    fn info(&self) -> BlockInfo {
        use wafer_block::types::{ResourceGrant, ResourceType};
        use wafer_run::{types::CollectionSchema, AuthLevel};

        BlockInfo::new("suppers-ai/files", "0.0.1", "http-handler@v1", "File storage, sharing, quotas, and access logging")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into(), "wafer-run/storage".into(), "wafer-run/config".into()])
            // Storage WRAP grant: this block manages user-created buckets at
            // arbitrary names ({bucket}/{key}). Without an explicit grant,
            // `wafer-run/local-storage` denies all storage ops with caller
            // `suppers-ai/files`. Owned by this block — no other block writes
            // through these paths. (Network/Storage grants skip the owner
            // check at startup; see `runtime/lifecycle.rs::collect_wrap_grants`.)
            .grants(vec![
                ResourceGrant::read_write("suppers-ai/files", "*").typed(ResourceType::Storage),
            ])
            .collections(vec![
                CollectionSchema::new(BUCKETS_TABLE)
                    .field("name", "string")
                    .field_default("public", "bool", "false")
                    .field_default("created_by", "string", ""),
                CollectionSchema::new(OBJECTS_TABLE)
                    .field("bucket", "string")
                    .field("key", "string")
                    .field_default("size", "int", "0")
                    .field_default("content_type", "string", "application/octet-stream")
                    .field_default("status", "string", "complete")
                    .field_default("uploaded_by", "string", "")
                    .field_optional("uploaded_at", "datetime")
                    .index(&["bucket"]),
                CollectionSchema::new(VIEWS_TABLE)
                    .field("bucket", "string")
                    .field("key", "string")
                    .field_default("user_id", "string", "")
                    .field_optional("viewed_at", "datetime"),
                CollectionSchema::new(SHARES_TABLE)
                    .field("token", "string")
                    .field("bucket", "string")
                    .field("key", "string")
                    .field_default("created_by", "string", "")
                    .field_optional("expires_at", "datetime")
                    .field_default("access_count", "int", "0")
                    .field_optional("max_access_count", "int")
                    .index(&["token"]),
                CollectionSchema::new(ACCESS_LOGS_TABLE)
                    .field("share_id", "string")
                    .field_optional("accessed_at", "datetime")
                    .field_default("ip_address", "string", "")
                    .field_default("user_agent", "string", "")
                    .index(&["share_id"]),
                CollectionSchema::new(QUOTAS_TABLE)
                    .field_unique("user_id", "string")
                    .field_default("max_storage_bytes", "int64", "1073741824")
                    .field_default("max_file_size_bytes", "int64", "104857600")
                    .field_default("max_files_per_bucket", "int", "10000")
                    .field_default("reset_period_days", "int", "0"),
            ])
            .category(wafer_run::BlockCategory::Feature)
            .description("File storage and management with bucket-based organization. Supports file upload, download, deletion, search, and sharing via public links with expiration and access counting. Includes per-user storage quotas.")
            .endpoints(vec![
                BlockEndpoint::get("/b/storage/").summary("Bucket list (user)").auth(AuthLevel::Authenticated),
                BlockEndpoint::get("/b/storage/{bucket}/").summary("Object list").auth(AuthLevel::Authenticated),
                BlockEndpoint::get("/b/storage/{bucket}/{prefix...}/").summary("Object list (nested)").auth(AuthLevel::Authenticated),
                BlockEndpoint::get("/b/storage/api/buckets").summary("List buckets").auth(AuthLevel::Authenticated),
                BlockEndpoint::post("/b/storage/api/buckets").summary("Create bucket").auth(AuthLevel::Authenticated),
                BlockEndpoint::get("/b/storage/api/buckets/{name}/objects").summary("List objects").auth(AuthLevel::Authenticated),
                BlockEndpoint::post("/b/storage/api/buckets/{name}/objects").summary("Upload file").auth(AuthLevel::Authenticated),
                BlockEndpoint::get("/b/storage/api/buckets/{name}/objects/{key}").summary("Download file").auth(AuthLevel::Authenticated),
                BlockEndpoint::delete("/b/storage/api/buckets/{name}/objects/{key}").summary("Delete file").auth(AuthLevel::Authenticated),
                BlockEndpoint::get("/b/storage/direct/{token}").summary("Access shared file"),
                BlockEndpoint::get("/b/cloudstorage/").summary("Shares + quota page").auth(AuthLevel::Authenticated),
            ])
            .admin_url("/b/storage/admin/")
            .can_disable(true)
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let mut msg = msg;
        let path = msg.path().to_string();

        // Admin SSR pages at /b/storage/admin/...
        if path.starts_with("/b/storage/admin") && msg.action() == "retrieve" {
            let is_admin = helpers::is_admin(&msg);
            if !is_admin {
                return crate::ui::forbidden_response(&msg);
            }
            let sub = path.strip_prefix("/b/storage/admin").unwrap_or("/");
            return match sub {
                "" | "/" => pages_admin::overview(ctx, &msg).await,
                "/buckets" => pages_admin::buckets(ctx, &msg).await,
                "/shares" => pages_admin::shares(ctx, &msg).await,
                "/quotas" => pages_admin::quotas(ctx, &msg).await,
                _ => err_not_found("not found"),
            };
        }

        // Normalize: /b/storage/... → /storage/..., /b/cloudstorage/... stays as-is
        let normalized = if let Some(rest) = path.strip_prefix("/b/storage") {
            format!("/storage{rest}")
        } else {
            path.clone()
        };
        if normalized != path {
            msg.set_meta("req.resource", &normalized);
        }

        // Direct share access (public, no auth required) — still rate-limited
        // per remote IP inside the handler to stop token enumeration / DOS.
        if normalized.starts_with("/storage/direct/") {
            return share::handle_direct_access(ctx, &msg, &self.limiter).await;
        }

        // Require authentication for all non-public endpoints
        let user_id = msg.user_id().to_string();
        if user_id.is_empty() {
            return crate::blocks::helpers::err_unauthorized("Authentication required");
        }

        // Per-user rate limiting
        {
            let action = msg.action().to_string();
            let (default, category) = if action == "create" {
                (RateLimit::UPLOAD, "upload")
            } else if action == "retrieve" {
                (RateLimit::API_READ, "api_read")
            } else {
                (RateLimit::API_WRITE, "api_write")
            };
            // Allowed(headers) is discarded here: attaching X-RateLimit-* to
            // a streaming response would need platform-side middleware to
            // inject the headers after the handler returns its OutputStream.
            // Tracked as a non-blocker — limits still enforced, just not
            // surfaced.
            if let RateLimitOutcome::Limited(r) =
                check_rate_limit(&self.limiter, ctx, &user_id, category, default).await
            {
                return r;
            }
        }

        // User-facing SSR pages.
        if msg.action() == "retrieve" {
            if path == "/b/storage" || path == "/b/storage/" {
                return pages_user::bucket_list_page(ctx, &msg).await;
            }
            // /b/storage/{bucket}/[{prefix...}/]  (must end with `/`).
            // Skip admin/ (handled above before normalize), api/, direct/.
            if let Some(rest) = path.strip_prefix("/b/storage/") {
                if rest.ends_with('/')
                    && !rest.starts_with("admin/")
                    && !rest.starts_with("api/")
                    && !rest.starts_with("direct/")
                {
                    let trimmed = rest.trim_end_matches('/');
                    let mut parts = trimmed.splitn(2, '/');
                    let bucket = parts.next().unwrap_or_default();
                    let prefix = parts.next().unwrap_or_default();
                    let prefix_with_slash = if prefix.is_empty() {
                        String::new()
                    } else {
                        format!("{prefix}/")
                    };
                    if !bucket.is_empty() {
                        return pages_user::object_list_page(ctx, &msg, bucket, &prefix_with_slash)
                            .await;
                    }
                }
            }
        }

        // Cloud storage SSR page — must check before JSON dispatch below.
        if msg.action() == "retrieve" && (path == "/b/cloudstorage" || path == "/b/cloudstorage/") {
            return pages_user::cloudstorage_page(ctx, &msg).await;
        }

        // Cloud storage routes (/b/cloudstorage/...)
        if normalized.starts_with("/b/cloudstorage") {
            return cloud::handle(ctx, msg, input).await;
        }

        // User storage API routes (/b/storage/api/... → /storage/api/...)
        if normalized.starts_with("/storage/api/") || normalized == "/storage/api" {
            // Normalize for sub-module: /storage/api/buckets → /storage/buckets
            let api_path = normalized.replacen("/storage/api", "/storage", 1);
            msg.set_meta("req.resource", &api_path);
            return storage::handle(ctx, msg, input).await;
        }

        err_not_found("not found")
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::admin("/admin/"),
            wafer_run::UiRoute::admin("/admin/buckets"),
            wafer_run::UiRoute::admin("/admin/shares"),
            wafer_run::UiRoute::admin("/admin/quotas"),
            wafer_run::UiRoute::authenticated("/"),
            wafer_run::UiRoute::authenticated("/{bucket}/"),
            wafer_run::UiRoute::authenticated("/{bucket}/{prefix...}/"),
        ]
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            migrations::apply(ctx).await.map_err(|e| {
                WaferError::new(
                    wafer_run::ErrorCode::Internal,
                    format!("files migrations: {e}"),
                )
            })?;
        }
        Ok(())
    }
}

/// Admin storage delegation — called from the Admin block's API section.
/// Expects msg path already normalized to `/admin/storage/...`.
pub async fn handle_admin_storage(
    ctx: &dyn Context,
    msg: Message,
    input: InputStream,
) -> OutputStream {
    storage::handle_admin(ctx, msg, input).await
}

/// Admin cloud storage delegation — called from the Admin block's API section.
/// Expects msg path already normalized to `/admin/b/cloudstorage/...`.
pub async fn handle_admin_cloud(
    ctx: &dyn Context,
    msg: Message,
    input: InputStream,
) -> OutputStream {
    cloud::handle(ctx, msg, input).await
}

#[cfg(not(target_arch = "wasm32"))]
::wafer_run::register_static_block!("suppers-ai/files", FilesBlock);
