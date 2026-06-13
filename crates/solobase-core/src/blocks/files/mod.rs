mod cloud;
pub(crate) mod migrations;
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
    context::Context, Block, BlockEndpoint, BlockInfo, InputStream, InstanceMode, LifecycleEvent,
    LifecycleType, Message, OutputStream, WaferError,
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
        use wafer_run::{AuthLevel, CollectionSchema};

        BlockInfo::new("suppers-ai/files", "0.0.1", "http-handler@v1", "File storage, sharing, quotas, and access logging")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into(), "wafer-run/storage".into(), "wafer-run/config".into()])
            // No explicit Storage grant needed. Wave 26 (c18) made WRAP
            // namespace-aware for Storage; this block self-admits its
            // own `suppers-ai/files/*` namespace via Rule 3.
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
                    .field_default("max_storage_bytes", "int64", &models::QuotaConfig::DEFAULT_MAX_STORAGE_BYTES.to_string())
                    .field_default("max_file_size_bytes", "int64", &models::QuotaConfig::DEFAULT_MAX_FILE_SIZE_BYTES.to_string())
                    .field_default("max_files_per_bucket", "int", &models::QuotaConfig::DEFAULT_MAX_FILES_PER_BUCKET.to_string())
                    .field_default("reset_period_days", "int", &models::QuotaConfig::DEFAULT_RESET_PERIOD_DAYS.to_string()),
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

        // Admin-block delegation: when the Admin block routes a request for
        // `/admin/storage/...` or `/admin/b/cloudstorage/...` through
        // `ctx.call_block("suppers-ai/files", ...)`, the messages already
        // carry the normalized admin path. Authorization (admin role check)
        // is enforced by the Admin block before delegation; we accept the
        // calls here without re-checking so the admin path stays a thin
        // pass-through to the sub-module handlers that own the SQL.
        if path.starts_with("/admin/storage") {
            return storage::handle_admin(ctx, msg, input).await;
        }
        if path.starts_with("/admin/b/cloudstorage") {
            return cloud::handle(ctx, msg, input).await;
        }

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

#[cfg(not(target_arch = "wasm32"))]
::wafer_block::register_static_block!("suppers-ai/files", FilesBlock);

#[cfg(test)]
mod schema_tests {
    use wafer_run::Block;

    use super::{models::QuotaConfig, FilesBlock, QUOTAS_TABLE};

    /// The quota defaults advertised through the collection schema must
    /// match the `QuotaConfig` consts (single in-code source). The
    /// migration SQL files carry the same values DB-side; if you change
    /// the consts, change `migrations/001_initial_schema.*.sql` too (and
    /// remember `SOLOBASE_RUN_MIGRATIONS=1`).
    #[test]
    fn quota_schema_defaults_match_quota_config_consts() {
        let info = FilesBlock::new().info();
        let quotas = info
            .collections
            .iter()
            .find(|c| c.name == QUOTAS_TABLE)
            .expect("quotas collection declared");

        let default_of = |field: &str| -> String {
            quotas
                .fields
                .iter()
                .find(|f| f.name == field)
                .unwrap_or_else(|| panic!("field {field} declared"))
                .default_value
                .clone()
        };

        assert_eq!(
            default_of("max_storage_bytes"),
            QuotaConfig::DEFAULT_MAX_STORAGE_BYTES.to_string()
        );
        assert_eq!(
            default_of("max_file_size_bytes"),
            QuotaConfig::DEFAULT_MAX_FILE_SIZE_BYTES.to_string()
        );
        assert_eq!(
            default_of("max_files_per_bucket"),
            QuotaConfig::DEFAULT_MAX_FILES_PER_BUCKET.to_string()
        );
        assert_eq!(
            default_of("reset_period_days"),
            QuotaConfig::DEFAULT_RESET_PERIOD_DAYS.to_string()
        );
    }
}

#[cfg(test)]
mod grant_tests {
    use wafer_run::{Block, ResourceType};

    use super::FilesBlock;

    #[test]
    fn files_block_does_not_declare_typed_storage() {
        // Wave 26 (c18): files block doesn't need a typed Storage grant
        // for its own namespace because Rule 3 self-admit covers it. A
        // grant here would also be redundant for *cross-block* access:
        // any block that wants to expose its storage to files declares
        // the grant from its own side.
        let files = FilesBlock::new();
        let grants = files.info().grants;

        let typed_storage = grants
            .iter()
            .find(|g| g.resource_type == Some(ResourceType::Storage));

        assert!(
            typed_storage.is_none(),
            "files block must not declare a typed Storage grant — own-namespace \
             access is covered by WRAP Rule 3 self-admit (Wave 26 / c18). \
             Cross-block grants belong on the owning block's side. (got: {typed_storage:?})",
        );
    }
}
