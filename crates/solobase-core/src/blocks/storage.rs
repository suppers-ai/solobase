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

use futures::StreamExt;
use wafer_block::{codec, stream::StreamEvent, wire::storage as wire};
use wafer_core::{clients::database as db, interfaces::storage::service::StorageService};
use wafer_run::{
    block::Block, context::Context, types::*, BlockInfo, InputStream, OutputStream, ResourceGrant,
    ResourceType,
};

use super::{
    admin::STORAGE_ACCESS_LOGS_TABLE,
    helpers::{json_map, now_millis},
};

/// A storage block that enforces per-block path isolation and WRAP-based
/// cross-block access control.
pub struct SolobaseStorageBlock {
    inner: wafer_core::service_blocks::storage::StorageBlock,
    /// WRAP grants for cross-block storage access checks.
    /// Updated after runtime startup via `update_wrap_grants()`.
    wrap_grants: std::sync::RwLock<Vec<ResourceGrant>>,
    /// The admin block ID (has full storage access).
    wrap_admin_block: Arc<str>,
}

impl SolobaseStorageBlock {
    pub fn new(service: Arc<dyn StorageService>, admin_block: Arc<str>) -> Self {
        Self {
            inner: wafer_core::service_blocks::storage::StorageBlock::new(service),
            wrap_grants: std::sync::RwLock::new(Vec::new()),
            wrap_admin_block: admin_block,
        }
    }

    /// Update the WRAP grants used for cross-block access checks.
    /// Called after runtime startup once grants are collected.
    pub fn update_wrap_grants(&self, grants: &[ResourceGrant]) {
        // Recover from poison: the data inside is still valid — the only
        // reason this lock would be poisoned is a panic in a previous
        // writer, and we're replacing the whole `Vec` anyway.
        let mut g = self.wrap_grants.write().unwrap_or_else(|e| e.into_inner());
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
    /// The value the downstream wafer-core handler's SEC-003
    /// cross-validation expects in `wrap.resource` meta. Equals `path`
    /// for folder ops (list, create_folder, delete_folder); equals
    /// `format!("{path}/{key}")` for object ops (put, get, delete).
    wrap_resource: String,
}

/// Resolve a folder name: own-namespace prefixing or cross-block via `@` prefix.
/// Sets `wrap_resource = path` by default; callers handling object ops
/// (put/get/delete) post-process to include the object key.
fn resolve_folder(caller: &str, folder: &str) -> ResolvedPath {
    let (path, cross_block) = if let Some(absolute) = folder.strip_prefix('@') {
        (absolute.to_string(), true)
    } else if folder.is_empty() {
        (caller.to_string(), false)
    } else {
        (format!("{caller}/{folder}"), false)
    };
    ResolvedPath {
        wrap_resource: path.clone(),
        path,
        cross_block,
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
///
/// Bodies are MessagePack-encoded `wire::storage` request types (matching
/// the binary transport overhaul). We decode the relevant typed request,
/// rewrite the path field, and re-encode — no `serde_json::Value`
/// round-trip, because that would lose the byte-fidelity guarantees
/// needed for the schema-locked wire types (and silently strip
/// non-string-encodable fields like `PutRequest.data`).
fn rewrite_request_body(
    kind: &str,
    body: &[u8],
    caller: &str,
) -> Result<(Vec<u8>, ResolvedPath), WaferError> {
    let invalid = |e: wafer_block::WaferError| {
        WaferError::new(
            ErrorCode::INVALID_ARGUMENT,
            format!("invalid storage request: {}", e.message),
        )
    };
    let encode_err = |e: wafer_block::WaferError| {
        WaferError::new(
            ErrorCode::INTERNAL,
            format!("encoding storage request: {}", e.message),
        )
    };

    match kind {
        // No folder field to rewrite — handled by filtering results.
        // list_folders has no `wrap.resource` cross-check in the wafer-core
        // handler, so wrap_resource is unused; populate it for consistency.
        "storage.list_folders" => Ok((
            body.to_vec(),
            ResolvedPath {
                wrap_resource: caller.to_string(),
                path: caller.to_string(),
                cross_block: false,
            },
        )),
        "storage.create_folder" => {
            let mut req: wire::CreateFolderRequest = codec::decode(body).map_err(invalid)?;
            let resolved = resolve_folder(caller, &req.name);
            req.name = resolved.path.clone();
            let bytes = codec::encode(&req).map_err(encode_err)?;
            Ok((bytes, resolved))
        }
        "storage.delete_folder" => {
            let mut req: wire::DeleteFolderRequest = codec::decode(body).map_err(invalid)?;
            let resolved = resolve_folder(caller, &req.name);
            req.name = resolved.path.clone();
            let bytes = codec::encode(&req).map_err(encode_err)?;
            Ok((bytes, resolved))
        }
        "storage.put" => {
            let mut req: wire::PutRequest = codec::decode(body).map_err(invalid)?;
            let mut resolved = resolve_folder(caller, &req.folder);
            req.folder = resolved.path.clone();
            resolved.wrap_resource = format!("{}/{}", resolved.path, req.key);
            let bytes = codec::encode(&req).map_err(encode_err)?;
            Ok((bytes, resolved))
        }
        "storage.get" => {
            let mut req: wire::GetRequest = codec::decode(body).map_err(invalid)?;
            let mut resolved = resolve_folder(caller, &req.folder);
            req.folder = resolved.path.clone();
            resolved.wrap_resource = format!("{}/{}", resolved.path, req.key);
            let bytes = codec::encode(&req).map_err(encode_err)?;
            Ok((bytes, resolved))
        }
        "storage.delete" => {
            let mut req: wire::DeleteRequest = codec::decode(body).map_err(invalid)?;
            let mut resolved = resolve_folder(caller, &req.folder);
            req.folder = resolved.path.clone();
            resolved.wrap_resource = format!("{}/{}", resolved.path, req.key);
            let bytes = codec::encode(&req).map_err(encode_err)?;
            Ok((bytes, resolved))
        }
        "storage.list" => {
            let mut req: wire::ListRequest = codec::decode(body).map_err(invalid)?;
            let resolved = resolve_folder(caller, &req.folder);
            req.folder = resolved.path.clone();
            let bytes = codec::encode(&req).map_err(encode_err)?;
            Ok((bytes, resolved))
        }
        other => Err(WaferError::new(
            ErrorCode::INVALID_ARGUMENT,
            format!("unknown storage op: {other}"),
        )),
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SolobaseStorageBlock {
    fn info(&self) -> BlockInfo {
        self.inner.info()
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
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

        // SEC-003: keep wrap.resource meta in sync with the namespacing
        // rewrite so the downstream wafer-core handler's cross-validation
        // passes. The expected value depends on the op (folder vs
        // folder/key composite); rewrite_request_body computes the right
        // value per-op as `resolved.wrap_resource`. The WRAP grant check
        // at the call_block boundary has already validated the caller's
        // original wrap.resource against their grants; this is a
        // payload-meta sync, not a grant bypass.
        let mut msg = msg;
        msg.set_meta(
            wafer_block::meta::META_WRAP_RESOURCE,
            &resolved.wrap_resource,
        );

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
            let grants = self
                .wrap_grants
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
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

        // Execute the actual storage operation. Forward events to the caller
        // as they arrive — previously we drained the whole stream into a
        // `Vec<StreamEvent>` first, which defeated streaming and buffered
        // the entire body in memory. Frame boundaries are preserved because
        // we forward each event individually rather than `collect_buffered`
        // (which would concatenate header + body into a single chunk and
        // break downstream `buffered_header_and_body` decoding).
        let start = now_millis();
        let inner_out = self
            .inner
            .handle(ctx, msg.clone(), InputStream::from_bytes(rewritten_body))
            .await;
        let caller_log = caller.clone();
        let path_log = resolved.path.clone();
        let kind_log = msg.kind.clone();
        let ctx_arc = ctx.clone_arc();

        OutputStream::from_producer(move |sink, _cancel| async move {
            let mut inner = inner_out;
            let mut error_status: Option<String> = None;
            while let Some(evt) = inner.next().await {
                match evt {
                    StreamEvent::Chunk(bytes) => {
                        if sink.send_chunk(bytes).await.is_err() {
                            return;
                        }
                    }
                    StreamEvent::Meta(entry) => {
                        let _ = sink.send_meta(entry).await;
                    }
                    StreamEvent::Complete { meta } => {
                        let duration_ms = (now_millis() - start) as i64;
                        let status = error_status
                            .clone()
                            .unwrap_or_else(|| format!("OK ({duration_ms}ms)"));
                        let _ = log_storage_access(
                            ctx_arc.as_ref(),
                            &caller_log,
                            &kind_log,
                            &path_log,
                            &status,
                        )
                        .await;
                        let _ = sink.complete(meta).await;
                        return;
                    }
                    StreamEvent::Error(e) => {
                        error_status = Some(format!("ERROR: {}", e.message));
                        let _ = log_storage_access(
                            ctx_arc.as_ref(),
                            &caller_log,
                            &kind_log,
                            &path_log,
                            error_status.as_deref().unwrap_or("ERROR"),
                        )
                        .await;
                        let _ = sink.error(*e).await;
                        return;
                    }
                    StreamEvent::Drop => {
                        let _ = sink.drop_request().await;
                        return;
                    }
                    StreamEvent::Continue(m) => {
                        let _ = sink.continue_with(m).await;
                        return;
                    }
                    StreamEvent::Halt { body, meta } => {
                        let duration_ms = (now_millis() - start) as i64;
                        let status = error_status
                            .clone()
                            .unwrap_or_else(|| format!("OK ({duration_ms}ms)"));
                        let _ = log_storage_access(
                            ctx_arc.as_ref(),
                            &caller_log,
                            &kind_log,
                            &path_log,
                            &status,
                        )
                        .await;
                        let _ = sink.halt(body, meta).await;
                        return;
                    }
                }
            }
            // Stream ended without a terminal event — best-effort log.
            let duration_ms = (now_millis() - start) as i64;
            let _ = log_storage_access(
                ctx_arc.as_ref(),
                &caller_log,
                &kind_log,
                &path_log,
                &format!("OK ({duration_ms}ms)"),
            )
            .await;
        })
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
        STORAGE_ACCESS_LOGS_TABLE,
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
pub fn create(
    service: Arc<dyn StorageService>,
    admin_block: Arc<str>,
) -> Arc<SolobaseStorageBlock> {
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
        let body = codec::encode(&wire::PutRequest {
            folder: "uploads".into(),
            key: "photo.jpg".into(),
            data: vec![],
            content_type: "image/jpeg".into(),
        })
        .unwrap();
        let (rewritten, resolved) =
            rewrite_request_body("storage.put", &body, "suppers-ai/files").unwrap();
        assert_eq!(resolved.path, "suppers-ai/files/uploads");
        assert!(!resolved.cross_block);

        let req: wire::PutRequest = codec::decode(&rewritten).unwrap();
        assert_eq!(req.folder, "suppers-ai/files/uploads");
    }

    #[test]
    fn test_rewrite_request_body_cross_block() {
        let body = codec::encode(&wire::GetRequest {
            folder: "@wafer-run/web/public".into(),
            key: "index.html".into(),
        })
        .unwrap();
        let (rewritten, resolved) =
            rewrite_request_body("storage.get", &body, "suppers-ai/files").unwrap();
        assert_eq!(resolved.path, "wafer-run/web/public");
        assert!(resolved.cross_block);

        let req: wire::GetRequest = codec::decode(&rewritten).unwrap();
        assert_eq!(req.folder, "wafer-run/web/public");
    }

    #[test]
    fn test_rewrite_request_body_create_folder() {
        let body = codec::encode(&wire::CreateFolderRequest {
            name: "uploads".into(),
            public: false,
        })
        .unwrap();
        let (rewritten, resolved) =
            rewrite_request_body("storage.create_folder", &body, "suppers-ai/files").unwrap();
        assert_eq!(resolved.path, "suppers-ai/files/uploads");
        assert!(!resolved.cross_block);

        let req: wire::CreateFolderRequest = codec::decode(&rewritten).unwrap();
        assert_eq!(req.name, "suppers-ai/files/uploads");
    }

    /// SEC-003 regression — folder ops set `wrap_resource = path`;
    /// object ops (put/get/delete) set `wrap_resource = format!("{path}/{key}")`
    /// to match what the wafer-core storage handler's check_wrap_resource
    /// will compare the meta against.
    #[test]
    fn test_rewrite_request_body_wrap_resource_per_op() {
        // Folder op — wrap_resource == path
        let body = codec::encode(&wire::CreateFolderRequest {
            name: "smoke".into(),
            public: false,
        })
        .unwrap();
        let (_, resolved) =
            rewrite_request_body("storage.create_folder", &body, "suppers-ai/files").unwrap();
        assert_eq!(resolved.wrap_resource, "suppers-ai/files/smoke");

        // Object op — wrap_resource == path + "/" + key
        let body = codec::encode(&wire::PutRequest {
            folder: "smoke".into(),
            key: "a.png".into(),
            data: vec![],
            content_type: "image/png".into(),
        })
        .unwrap();
        let (_, resolved) = rewrite_request_body("storage.put", &body, "suppers-ai/files").unwrap();
        assert_eq!(resolved.wrap_resource, "suppers-ai/files/smoke/a.png");

        // Cross-block object op — wrap_resource uses the post-resolution path
        let body = codec::encode(&wire::GetRequest {
            folder: "@wafer-run/web/public".into(),
            key: "index.html".into(),
        })
        .unwrap();
        let (_, resolved) = rewrite_request_body("storage.get", &body, "suppers-ai/files").unwrap();
        assert_eq!(resolved.wrap_resource, "wafer-run/web/public/index.html");
        assert!(resolved.cross_block);

        // list_folders — no folder field; wrap_resource == caller
        let (_, resolved) =
            rewrite_request_body("storage.list_folders", &[], "suppers-ai/files").unwrap();
        assert_eq!(resolved.wrap_resource, "suppers-ai/files");
    }
}
