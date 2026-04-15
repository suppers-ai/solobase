//! Solobase storage block wrapper.
//!
//! Wraps the wafer-core `StorageBlock` to add:
//! - Per-block path isolation (each block gets its own storage namespace)
//! - Cross-block access control via WRAP grants (default deny)
//! - Storage access logging
//!
//! ## Isolation model
//!
//! Each block's storage is namespaced under its block name:
//! - `wafer-run/web` calling `store::get(ctx, "public", "key")` → `wafer-run/web/public/key`
//! - `suppers-ai/files` calling `store::put(ctx, "uploads", ...)` → `suppers-ai/files/uploads/...`
//!
//! ## Cross-block access
//!
//! Blocks can request access to another block's namespace by prefixing the
//! folder with `@`:
//! - `store::get(ctx, "@wafer-run/web/public", "key")` → cross-block read of `wafer-run/web/public/key`
//!
//! Cross-block access is **denied by default** and requires a WRAP grant with
//! `resource_type = Storage` matching the target path.

use std::sync::Arc;

use wafer_core::interfaces::storage::service::StorageService;
use wafer_run::block::Block;
use wafer_run::context::Context;
use wafer_run::streams::output::TerminalNotResponse;
use wafer_run::types::*;
use wafer_run::BlockInfo;
use wafer_run::ResourceGrant;
use wafer_run::ResourceType;
use wafer_run::{InputStream, OutputStream};

use wafer_core::clients::database as db;

use super::admin::STORAGE_ACCESS_LOGS_COLLECTION;
use super::helpers::{json_map, now_millis};

/// A storage block that enforces per-block path isolation and WRAP-based
/// cross-block access control.
pub struct SolobaseStorageBlock {
    inner: wafer_core::service_blocks::storage::StorageBlock,
    /// WRAP grants for cross-block storage access checks.
    /// Updated after runtime startup via `update_wrap_grants()`.
    wrap_grants: std::sync::RwLock<Vec<ResourceGrant>>,
    /// The admin block ID (has full storage access).
    wrap_admin_block: Arc<String>,
}

impl SolobaseStorageBlock {
    pub fn new(service: Arc<dyn StorageService>, admin_block: Arc<String>) -> Self {
        Self {
            inner: wafer_core::service_blocks::storage::StorageBlock::new(service),
            wrap_grants: std::sync::RwLock::new(Vec::new()),
            wrap_admin_block: admin_block,
        }
    }

    /// Update the WRAP grants used for cross-block access checks.
    /// Called after runtime startup once grants are collected.
    pub fn update_wrap_grants(&self, grants: &[ResourceGrant]) {
        let mut g = self.wrap_grants.write().unwrap();
        *g = grants.to_vec();
    }
}

/// Validate that a block name is safe for use as a storage path prefix.
/// Only allows `[a-z0-9-_]` per segment, `/` as separator. No `..`, no dots, no empty segments.
fn is_safe_block_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    for segment in name.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return false;
        }
        if !segment
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return false;
        }
    }
    true
}

/// Result of resolving a storage path.
struct ResolvedPath {
    /// The actual storage path after resolution.
    path: String,
    /// Whether this is a cross-block access (folder started with `@`).
    cross_block: bool,
}

/// Resolve a folder name: own-namespace prefixing or cross-block via `@` prefix.
fn resolve_folder(caller: &str, folder: &str) -> ResolvedPath {
    if let Some(absolute) = folder.strip_prefix('@') {
        // Cross-block access: use the path after @ as-is
        ResolvedPath {
            path: absolute.to_string(),
            cross_block: true,
        }
    } else if folder.is_empty() {
        ResolvedPath {
            path: caller.to_string(),
            cross_block: false,
        }
    } else {
        ResolvedPath {
            path: format!("{caller}/{folder}"),
            cross_block: false,
        }
    }
}

/// Determine access type from the storage operation kind.
fn access_type_for_op(kind: &str) -> &'static str {
    match kind {
        "storage.get" | "storage.list" | "storage.list_folders" => "read",
        _ => "write",
    }
}

/// Rewrite the folder/name field in the request body bytes.
///
/// Returns the rewritten body bytes plus the resolved path info.
fn rewrite_request_body(
    kind: &str,
    body: &[u8],
    caller: &str,
) -> Result<(Vec<u8>, ResolvedPath), WaferError> {
    // For list_folders, no folder field to rewrite — handled by filtering results
    if kind == "storage.list_folders" {
        return Ok((
            body.to_vec(),
            ResolvedPath {
                path: caller.to_string(),
                cross_block: false,
            },
        ));
    }

    let mut v: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
        WaferError::new(
            ErrorCode::INVALID_ARGUMENT,
            format!("invalid storage request: {e}"),
        )
    })?;

    // For folder-level ops, rewrite the "name" field
    if kind == "storage.create_folder" || kind == "storage.delete_folder" {
        let name = v
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let resolved = resolve_folder(caller, &name);
        v["name"] = serde_json::Value::String(resolved.path.clone());
        let bytes = serde_json::to_vec(&v).unwrap_or_default();
        return Ok((bytes, resolved));
    }

    // For object-level ops, rewrite the "folder" field
    let folder = v
        .get("folder")
        .and_then(|f| f.as_str())
        .unwrap_or("")
        .to_string();
    let resolved = resolve_folder(caller, &folder);
    v["folder"] = serde_json::Value::String(resolved.path.clone());
    let bytes = serde_json::to_vec(&v).unwrap_or_default();
    Ok((bytes, resolved))
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SolobaseStorageBlock {
    fn info(&self) -> BlockInfo {
        self.inner.info()
    }

    async fn handle(
        &self,
        ctx: &dyn Context,
        msg: Message,
        input: InputStream,
    ) -> OutputStream {
        let caller = ctx.caller_id().unwrap_or("unknown").to_string();

        // Validate caller name is safe for storage paths
        if caller != "unknown" && !is_safe_block_name(&caller) {
            return OutputStream::error(WaferError::new(
                ErrorCode::PermissionDenied,
                format!("block name '{}' is not safe for storage paths", caller),
            ));
        }

        let access = access_type_for_op(&msg.kind);
        let body = input.collect_to_bytes().await;

        // Rewrite folder/name in the request body, resolving own vs cross-block
        let (rewritten_body, resolved) = match rewrite_request_body(&msg.kind, &body, &caller) {
            Ok(r) => r,
            Err(e) => return OutputStream::error(e),
        };

        // Check for path traversal
        if resolved.path.contains("..") {
            let _ = log_storage_access(
                ctx,
                &caller,
                &msg.kind,
                &resolved.path,
                "BLOCKED: path traversal",
            )
            .await;
            return OutputStream::error(WaferError::new(
                ErrorCode::PermissionDenied,
                "storage path traversal not allowed",
            ));
        }

        // Cross-block access requires a WRAP grant with resource_type = Storage
        if resolved.cross_block {
            let is_write = access == "write";
            let grants = self.wrap_grants.read().unwrap().clone();
            if let Err(e) = wafer_run::wrap::check_access(
                Some(&caller),
                &resolved.path,
                is_write,
                Some(&ResourceType::Storage),
                &grants,
                &self.wrap_admin_block,
            ) {
                let _ = log_storage_access(
                    ctx,
                    &caller,
                    &msg.kind,
                    &resolved.path,
                    &format!("BLOCKED: {}", e.message),
                )
                .await;
                return OutputStream::error(e);
            }
        }

        // Execute the actual storage operation, then collect to log status.
        let start = now_millis();
        let inner_out = self
            .inner
            .handle(ctx, msg.clone(), InputStream::from_bytes(rewritten_body))
            .await;
        let buffered = inner_out.collect_buffered().await;
        let duration_ms = (now_millis() - start) as i64;

        let (status, result) = match buffered {
            Ok(resp) => {
                let mut builder = crate::blocks::helpers::ResponseBuilder::new();
                for entry in &resp.meta {
                    builder = builder.set_header(&entry.key, &entry.value);
                }
                let out = builder.body(resp.body, "");
                (format!("OK ({duration_ms}ms)"), out)
            }
            Err(TerminalNotResponse::Error(e)) => {
                let s = format!("ERROR: {}", e.message);
                (s, OutputStream::error(e))
            }
            Err(TerminalNotResponse::Drop) => ("ERROR: dropped".into(), OutputStream::drop_request()),
            Err(TerminalNotResponse::Continue(m)) => {
                ("ERROR: continue".into(), OutputStream::continue_with(m))
            }
            Err(TerminalNotResponse::Malformed) => (
                "ERROR: malformed".into(),
                OutputStream::error(WaferError::new(
                    ErrorCode::Internal,
                    "malformed inner response",
                )),
            ),
        };

        // Log the access (best-effort)
        let _ = log_storage_access(ctx, &caller, &msg.kind, &resolved.path, &status).await;

        result
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        self.inner.lifecycle(ctx, event).await
    }
}

/// Log a storage access event (best-effort).
async fn log_storage_access(
    ctx: &dyn Context,
    source_block: &str,
    operation: &str,
    path: &str,
    status: &str,
) -> Result<(), WaferError> {
    db::create(
        ctx,
        STORAGE_ACCESS_LOGS_COLLECTION,
        json_map(serde_json::json!({
            "source_block": source_block,
            "operation": operation,
            "path": path,
            "status": status,
        })),
    )
    .await
    .map(|_| ())
}

/// Create a new SolobaseStorageBlock (caller must register it with the runtime).
///
/// After the runtime starts, call `update_wrap_grants()` to inject the
/// collected grants for cross-block access checks.
pub fn create(service: Arc<dyn StorageService>, admin_block: Arc<String>) -> Arc<SolobaseStorageBlock> {
    Arc::new(SolobaseStorageBlock::new(service, admin_block))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Unit tests for pure functions (integration tests would require porting
    // a TestContext to the streaming protocol — left for a future change).
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_safe_block_name() {
        assert!(is_safe_block_name("wafer-run/web"));
        assert!(is_safe_block_name("suppers-ai/solobase/auth"));
        assert!(is_safe_block_name("my_block"));
        assert!(is_safe_block_name("a/b/c"));

        assert!(!is_safe_block_name(""));
        assert!(!is_safe_block_name("../evil"));
        assert!(!is_safe_block_name("wafer-run/.."));
        assert!(!is_safe_block_name("wafer-run/.hidden"));
        assert!(!is_safe_block_name("UPPER/case"));
        assert!(!is_safe_block_name("has spaces/bad"));
        assert!(!is_safe_block_name("/leading-slash"));
        assert!(!is_safe_block_name("trailing/"));
    }

    #[test]
    fn test_resolve_folder_own_namespace() {
        let r = resolve_folder("wafer-run/web", "public");
        assert_eq!(r.path, "wafer-run/web/public");
        assert!(!r.cross_block);

        let r = resolve_folder("wafer-run/web", "");
        assert_eq!(r.path, "wafer-run/web");
        assert!(!r.cross_block);

        let r = resolve_folder("suppers-ai/files", "uploads");
        assert_eq!(r.path, "suppers-ai/files/uploads");
        assert!(!r.cross_block);
    }

    #[test]
    fn test_resolve_folder_cross_block() {
        let r = resolve_folder("suppers-ai/files", "@wafer-run/web/public");
        assert_eq!(r.path, "wafer-run/web/public");
        assert!(r.cross_block);

        let r = resolve_folder("suppers-ai/admin", "@suppers-ai/files/uploads");
        assert_eq!(r.path, "suppers-ai/files/uploads");
        assert!(r.cross_block);
    }

    #[test]
    fn test_access_type_for_op() {
        assert_eq!(access_type_for_op("storage.get"), "read");
        assert_eq!(access_type_for_op("storage.list"), "read");
        assert_eq!(access_type_for_op("storage.list_folders"), "read");
        assert_eq!(access_type_for_op("storage.put"), "write");
        assert_eq!(access_type_for_op("storage.delete"), "write");
        assert_eq!(access_type_for_op("storage.create_folder"), "write");
        assert_eq!(access_type_for_op("storage.delete_folder"), "write");
    }

    #[test]
    fn test_rewrite_request_body_put() {
        let body = serde_json::to_vec(&serde_json::json!({
            "folder": "uploads",
            "key": "photo.jpg",
            "data": [],
            "content_type": "image/jpeg"
        }))
        .unwrap();
        let (rewritten, resolved) =
            rewrite_request_body("storage.put", &body, "suppers-ai/files").unwrap();
        assert_eq!(resolved.path, "suppers-ai/files/uploads");
        assert!(!resolved.cross_block);

        let v: serde_json::Value = serde_json::from_slice(&rewritten).unwrap();
        assert_eq!(v["folder"], "suppers-ai/files/uploads");
    }

    #[test]
    fn test_rewrite_request_body_cross_block() {
        let body = serde_json::to_vec(&serde_json::json!({
            "folder": "@wafer-run/web/public",
            "key": "index.html"
        }))
        .unwrap();
        let (rewritten, resolved) =
            rewrite_request_body("storage.get", &body, "suppers-ai/files").unwrap();
        assert_eq!(resolved.path, "wafer-run/web/public");
        assert!(resolved.cross_block);

        let v: serde_json::Value = serde_json::from_slice(&rewritten).unwrap();
        assert_eq!(v["folder"], "wafer-run/web/public");
    }

    #[test]
    fn test_rewrite_request_body_create_folder() {
        let body = serde_json::to_vec(&serde_json::json!({
            "name": "uploads",
            "public": false
        }))
        .unwrap();
        let (rewritten, resolved) =
            rewrite_request_body("storage.create_folder", &body, "suppers-ai/files").unwrap();
        assert_eq!(resolved.path, "suppers-ai/files/uploads");
        assert!(!resolved.cross_block);

        let v: serde_json::Value = serde_json::from_slice(&rewritten).unwrap();
        assert_eq!(v["name"], "suppers-ai/files/uploads");
    }
}
