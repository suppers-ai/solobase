//! Solobase storage block wrapper.
//!
//! Wraps the wafer-core `StorageBlock` to add:
//! - Per-block path isolation (each block gets its own storage namespace)
//! - Storage rule enforcement (cross-block access requires explicit rules)
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
//! Cross-block access is **denied by default** and requires an explicit storage
//! rule granting access.

use std::sync::Arc;

use wafer_core::interfaces::storage::service::StorageService;
use wafer_run::block::Block;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::BlockInfo;

use wafer_core::clients::database as db;

use super::helpers::{json_map, now_millis};

/// A storage block that enforces per-block path isolation and storage rules.
pub struct SolobaseStorageBlock {
    inner: wafer_core::service_blocks::storage::StorageBlock,
}

impl SolobaseStorageBlock {
    pub fn new(service: Arc<dyn StorageService>) -> Self {
        Self {
            inner: wafer_core::service_blocks::storage::StorageBlock::new(service),
        }
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

/// Check storage rules. Returns `None` if allowed, or `Some(reason)` if blocked.
///
/// Evaluation: block rules are checked first (any match = deny).
/// Then allow rules: if any exist, the source+target must match at least one.
/// No rules = deny cross-block access by default.
async fn check_storage_rules(
    ctx: &dyn Context,
    source_block: &str,
    target_path: &str,
    access: &str,
) -> Option<String> {
    let rules = match db::list(
        ctx,
        "suppers_ai__admin__storage_rules",
        &db::ListOptions {
            sort: vec![db::SortField {
                field: "priority".into(),
                desc: true,
            }],
            limit: 10_000,
            ..Default::default()
        },
    )
    .await
    {
        Ok(result) => result.records,
        Err(e) => {
            tracing::debug!("storage rules query failed (table may not exist yet): {e}");
            // No rules table = deny cross-block by default
            return Some("cross-block storage access denied (no rules configured)".into());
        }
    };

    if rules.is_empty() {
        return Some("cross-block storage access denied (no rules configured)".into());
    }

    let mut has_allow_rules = false;
    let mut explicitly_allowed = false;

    for rule in &rules {
        let rule_type = rule
            .data
            .get("rule_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let rule_source = rule
            .data
            .get("source_block")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let rule_target = rule
            .data
            .get("target_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let rule_access = rule
            .data
            .get("access")
            .and_then(|v| v.as_str())
            .unwrap_or("readwrite");

        // Check if rule applies to this source block
        if !rule_source.is_empty()
            && rule_source != "*"
            && !pattern_matches(rule_source, source_block)
        {
            continue;
        }

        // Check if rule access type matches
        if rule_access != "readwrite" && rule_access != access {
            continue;
        }

        let target_matches = pattern_matches(rule_target, target_path);

        if rule_type == "block" && target_matches {
            return Some(format!("blocked by storage rule: {rule_target}"));
        }
        if rule_type == "allow" {
            has_allow_rules = true;
            if target_matches {
                explicitly_allowed = true;
            }
        }
    }

    if has_allow_rules && explicitly_allowed {
        return None; // allowed
    }

    Some("cross-block storage access denied".into())
}

/// Simple glob-style pattern matching (same as network rules).
fn pattern_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return value == pattern;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if let Some(found) = value[pos..].find(part) {
            if i == 0 && found != 0 {
                return false;
            }
            pos += found + part.len();
        } else {
            return false;
        }
    }

    if !pattern.ends_with('*') {
        return pos == value.len();
    }

    true
}

/// Determine access type from the storage operation kind.
fn access_type_for_op(kind: &str) -> &'static str {
    match kind {
        "storage.get" | "storage.list" | "storage.list_folders" => "read",
        _ => "write",
    }
}

/// Rewrite the folder/name field in the message data.
///
/// Returns the resolved path and whether it's a cross-block access.
fn rewrite_message_path(msg: &mut Message, caller: &str) -> Result<ResolvedPath, WaferError> {
    let mut v: serde_json::Value = serde_json::from_slice(&msg.data).map_err(|e| {
        WaferError::new(
            ErrorCode::INVALID_ARGUMENT,
            format!("invalid storage request: {e}"),
        )
    })?;

    // For folder-level ops, rewrite the "name" field
    if msg.kind == "storage.create_folder" || msg.kind == "storage.delete_folder" {
        let name = v
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let resolved = resolve_folder(caller, &name);
        v["name"] = serde_json::Value::String(resolved.path.clone());
        msg.data = serde_json::to_vec(&v).unwrap_or_default();
        return Ok(resolved);
    }

    // For list_folders, no folder field to rewrite — handled by filtering results
    if msg.kind == "storage.list_folders" {
        return Ok(ResolvedPath {
            path: caller.to_string(),
            cross_block: false,
        });
    }

    // For object-level ops, rewrite the "folder" field
    let folder = v
        .get("folder")
        .and_then(|f| f.as_str())
        .unwrap_or("")
        .to_string();
    let resolved = resolve_folder(caller, &folder);
    v["folder"] = serde_json::Value::String(resolved.path.clone());
    msg.data = serde_json::to_vec(&v).unwrap_or_default();
    Ok(resolved)
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SolobaseStorageBlock {
    fn info(&self) -> BlockInfo {
        self.inner.info()
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let caller = ctx.caller_id().unwrap_or("unknown").to_string();

        // Validate caller name is safe for storage paths
        if caller != "unknown" && !is_safe_block_name(&caller) {
            return Result_::error(WaferError::new(
                ErrorCode::PERMISSION_DENIED,
                format!("block name '{}' is not safe for storage paths", caller),
            ));
        }

        let access = access_type_for_op(&msg.kind);

        // Rewrite folder/name in the message, resolving own vs cross-block
        let resolved = match rewrite_message_path(msg, &caller) {
            Ok(r) => r,
            Err(e) => return Result_::error(e),
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
            return Result_::error(WaferError::new(
                ErrorCode::PERMISSION_DENIED,
                "storage path traversal not allowed",
            ));
        }

        // Cross-block access requires explicit rules
        if resolved.cross_block {
            if let Some(reason) = check_storage_rules(ctx, &caller, &resolved.path, access).await {
                let _ = log_storage_access(
                    ctx,
                    &caller,
                    &msg.kind,
                    &resolved.path,
                    &format!("BLOCKED: {reason}"),
                )
                .await;
                return Result_::error(WaferError::new(
                    ErrorCode::PERMISSION_DENIED,
                    format!("storage access blocked: {reason}"),
                ));
            }
        }

        // Execute the actual storage operation
        let start = now_millis();
        let result = self.inner.handle(ctx, msg).await;
        let duration_ms = (now_millis() - start) as i64;

        // Log the access (best-effort)
        let status = match &result.action {
            Action::Error => result
                .error
                .as_ref()
                .map(|e| format!("ERROR: {}", e.message))
                .unwrap_or_else(|| "ERROR".into()),
            _ => format!("OK ({duration_ms}ms)"),
        };
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
        "suppers_ai__admin__storage_access_logs",
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
pub fn create(service: Arc<dyn StorageService>) -> Arc<SolobaseStorageBlock> {
    Arc::new(SolobaseStorageBlock::new(service))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicU64;
    use std::sync::Mutex;
    use wafer_core::clients::database::Record;
    use wafer_core::interfaces::storage::service::{
        FolderInfo, ListOptions, ObjectInfo, ObjectList, StorageError,
    };

    // -----------------------------------------------------------------------
    // Unit tests for pure functions
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
    fn test_pattern_matches() {
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("wafer-run/web/*", "wafer-run/web/public"));
        assert!(!pattern_matches("wafer-run/web/*", "suppers-ai/auth/data"));
        assert!(pattern_matches(
            "suppers-ai/solobase/*",
            "suppers-ai/solobase/files/uploads"
        ));
        assert!(pattern_matches(
            "wafer-run/web/public",
            "wafer-run/web/public"
        ));
        assert!(!pattern_matches(
            "wafer-run/web/public",
            "wafer-run/web/private"
        ));
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
    fn test_rewrite_message_path_put() {
        let mut msg = Message::new(
            "storage.put",
            serde_json::to_vec(&serde_json::json!({
                "folder": "uploads",
                "key": "photo.jpg",
                "data": [],
                "content_type": "image/jpeg"
            }))
            .unwrap(),
        );
        let resolved = rewrite_message_path(&mut msg, "suppers-ai/files").unwrap();
        assert_eq!(resolved.path, "suppers-ai/files/uploads");
        assert!(!resolved.cross_block);

        // Verify the message was rewritten
        let v: serde_json::Value = serde_json::from_slice(&msg.data).unwrap();
        assert_eq!(v["folder"], "suppers-ai/files/uploads");
    }

    #[test]
    fn test_rewrite_message_path_cross_block() {
        let mut msg = Message::new(
            "storage.get",
            serde_json::to_vec(&serde_json::json!({
                "folder": "@wafer-run/web/public",
                "key": "index.html"
            }))
            .unwrap(),
        );
        let resolved = rewrite_message_path(&mut msg, "suppers-ai/files").unwrap();
        assert_eq!(resolved.path, "wafer-run/web/public");
        assert!(resolved.cross_block);

        let v: serde_json::Value = serde_json::from_slice(&msg.data).unwrap();
        assert_eq!(v["folder"], "wafer-run/web/public");
    }

    #[test]
    fn test_rewrite_message_path_create_folder() {
        let mut msg = Message::new(
            "storage.create_folder",
            serde_json::to_vec(&serde_json::json!({
                "name": "uploads",
                "public": false
            }))
            .unwrap(),
        );
        let resolved = rewrite_message_path(&mut msg, "suppers-ai/files").unwrap();
        assert_eq!(resolved.path, "suppers-ai/files/uploads");
        assert!(!resolved.cross_block);

        let v: serde_json::Value = serde_json::from_slice(&msg.data).unwrap();
        assert_eq!(v["name"], "suppers-ai/files/uploads");
    }

    // -----------------------------------------------------------------------
    // In-memory storage service for integration tests
    // -----------------------------------------------------------------------

    struct MemoryStorageService {
        /// folder → (key → (data, content_type))
        objects: Mutex<HashMap<String, HashMap<String, (Vec<u8>, String)>>>,
    }

    impl MemoryStorageService {
        fn new() -> Self {
            Self {
                objects: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl StorageService for MemoryStorageService {
        async fn put(
            &self,
            folder: &str,
            key: &str,
            data: &[u8],
            content_type: &str,
        ) -> Result<(), StorageError> {
            let mut store = self.objects.lock().unwrap();
            store
                .entry(folder.to_string())
                .or_default()
                .insert(key.to_string(), (data.to_vec(), content_type.to_string()));
            Ok(())
        }

        async fn get(
            &self,
            folder: &str,
            key: &str,
        ) -> Result<(Vec<u8>, ObjectInfo), StorageError> {
            let store = self.objects.lock().unwrap();
            let folder_map = store.get(folder).ok_or(StorageError::NotFound)?;
            let (data, ct) = folder_map.get(key).ok_or(StorageError::NotFound)?;
            Ok((
                data.clone(),
                ObjectInfo {
                    key: key.to_string(),
                    size: data.len() as i64,
                    content_type: ct.clone(),
                    last_modified: chrono::Utc::now(),
                },
            ))
        }

        async fn delete(&self, folder: &str, key: &str) -> Result<(), StorageError> {
            let mut store = self.objects.lock().unwrap();
            if let Some(f) = store.get_mut(folder) {
                f.remove(key);
            }
            Ok(())
        }

        async fn list(
            &self,
            folder: &str,
            _opts: &ListOptions,
        ) -> Result<ObjectList, StorageError> {
            let store = self.objects.lock().unwrap();
            let objects: Vec<ObjectInfo> = store
                .get(folder)
                .map(|f| {
                    f.iter()
                        .map(|(k, (data, ct))| ObjectInfo {
                            key: k.clone(),
                            size: data.len() as i64,
                            content_type: ct.clone(),
                            last_modified: chrono::Utc::now(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let total_count = objects.len() as i64;
            Ok(ObjectList {
                objects: objects,
                total_count,
            })
        }

        async fn create_folder(&self, name: &str, _public: bool) -> Result<(), StorageError> {
            let mut store = self.objects.lock().unwrap();
            store.entry(name.to_string()).or_default();
            Ok(())
        }

        async fn delete_folder(&self, name: &str) -> Result<(), StorageError> {
            let mut store = self.objects.lock().unwrap();
            store.remove(name);
            Ok(())
        }

        async fn list_folders(&self) -> Result<Vec<FolderInfo>, StorageError> {
            let store = self.objects.lock().unwrap();
            Ok(store
                .keys()
                .map(|k| FolderInfo {
                    name: k.clone(),
                    public: false,
                    created_at: chrono::Utc::now(),
                })
                .collect())
        }
    }

    // -----------------------------------------------------------------------
    // Mock context that handles db + storage calls
    // -----------------------------------------------------------------------

    struct TestContext {
        caller: Option<String>,
        storage_block: SolobaseStorageBlock,
        db: Mutex<HashMap<String, Vec<Record>>>,
        next_id: AtomicU64,
    }

    impl TestContext {
        fn new(caller: Option<&str>, storage: Arc<MemoryStorageService>) -> Self {
            Self {
                caller: caller.map(|s| s.to_string()),
                storage_block: SolobaseStorageBlock::new(storage),
                db: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(1),
            }
        }

        fn seed_rule(
            &self,
            rule_type: &str,
            source_block: &str,
            target_path: &str,
            access: &str,
            priority: i64,
        ) {
            let id = self
                .next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                .to_string();
            let mut data = HashMap::new();
            data.insert("rule_type".into(), serde_json::json!(rule_type));
            data.insert("source_block".into(), serde_json::json!(source_block));
            data.insert("target_path".into(), serde_json::json!(target_path));
            data.insert("access".into(), serde_json::json!(access));
            data.insert("priority".into(), serde_json::json!(priority));
            let record = Record { id, data };
            let mut db = self.db.lock().unwrap();
            db.entry("suppers_ai__admin__storage_rules".to_string())
                .or_default()
                .push(record);
        }

        fn handle_db_call(&self, kind: &str, data: &[u8]) -> Result<Vec<u8>, WaferError> {
            match kind {
                "database.create" => {
                    #[derive(serde::Deserialize)]
                    struct Req {
                        collection: String,
                        data: HashMap<String, serde_json::Value>,
                    }
                    let req: Req = serde_json::from_slice(data)
                        .map_err(|e| WaferError::new("internal", e.to_string()))?;
                    let id = self
                        .next_id
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                        .to_string();
                    let record = Record { id, data: req.data };
                    let mut db = self.db.lock().unwrap();
                    db.entry(req.collection).or_default().push(record.clone());
                    Ok(serde_json::to_vec(&record).unwrap())
                }
                "database.list" => {
                    #[derive(serde::Deserialize)]
                    struct Req {
                        collection: String,
                    }
                    let req: Req = serde_json::from_slice(data)
                        .map_err(|e| WaferError::new("internal", e.to_string()))?;
                    let db = self.db.lock().unwrap();
                    let records = db.get(&req.collection).cloned().unwrap_or_default();
                    let total_count = records.len() as i64;
                    let result = db::RecordList { records, total_count, page: 1, page_size: 10_000 };
                    Ok(serde_json::to_vec(&result).unwrap())
                }
                _ => Err(WaferError::new(
                    "not_implemented",
                    format!("unhandled db op: {kind}"),
                )),
            }
        }
    }

    #[async_trait::async_trait]
    impl Context for TestContext {
        async fn call_block(&self, block_name: &str, msg: &mut Message) -> Result_ {
            let kind = msg.kind.clone();
            let data = msg.data.clone();

            match block_name {
                "wafer-run/database" => {
                    let result = self.handle_db_call(&kind, &data);
                    match result {
                        Ok(response_data) => Result_ {
                            action: Action::Respond,
                            response: Some(Response {
                                data: response_data,
                                meta: Vec::new(),
                            }),
                            error: None,
                            message: None,
                        },
                        Err(e) => Result_ {
                            action: Action::Error,
                            response: None,
                            error: Some(e),
                            message: None,
                        },
                    }
                }
                _ => Result_ {
                    action: Action::Error,
                    response: None,
                    error: Some(WaferError::new(
                        "not_found",
                        format!("block '{}' not found", block_name),
                    )),
                    message: None,
                },
            }
        }

        fn is_cancelled(&self) -> bool {
            false
        }

        fn config_get(&self, _key: &str) -> Option<&str> {
            None
        }

        fn caller_id(&self) -> Option<&str> {
            self.caller.as_deref()
        }
    }

    // -----------------------------------------------------------------------
    // Helper: run a storage op through the block
    // -----------------------------------------------------------------------

    async fn do_put(
        block: &SolobaseStorageBlock,
        ctx: &TestContext,
        folder: &str,
        key: &str,
        data: &[u8],
    ) -> Result_ {
        let mut msg = Message::new(
            "storage.put",
            serde_json::to_vec(&serde_json::json!({
                "folder": folder,
                "key": key,
                "data": data,
                "content_type": "text/plain"
            }))
            .unwrap(),
        );
        block.handle(ctx, &mut msg).await
    }

    async fn do_get(
        block: &SolobaseStorageBlock,
        ctx: &TestContext,
        folder: &str,
        key: &str,
    ) -> Result_ {
        let mut msg = Message::new(
            "storage.get",
            serde_json::to_vec(&serde_json::json!({
                "folder": folder,
                "key": key
            }))
            .unwrap(),
        );
        block.handle(ctx, &mut msg).await
    }

    // -----------------------------------------------------------------------
    // Integration tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_own_namespace_isolation_put_and_get() {
        let storage = Arc::new(MemoryStorageService::new());
        let ctx = TestContext::new(Some("wafer-run/web"), storage.clone());
        let block = &ctx.storage_block;

        // Put a file — should land in wafer-run/web/public
        let result = do_put(block, &ctx, "public", "index.html", b"<html>hello</html>").await;
        assert!(
            matches!(result.action, Action::Respond),
            "put should succeed, got: {:?}",
            result.error
        );

        // Verify it was stored under the prefixed path
        let store = storage.objects.lock().unwrap();
        assert!(
            store
                .get("wafer-run/web/public")
                .and_then(|f| f.get("index.html"))
                .is_some(),
            "file should be stored at wafer-run/web/public/index.html, got: {:?}",
            store.keys().collect::<Vec<_>>()
        );
        // NOT stored at bare "public"
        assert!(store.get("public").is_none());
        drop(store);

        // Get it back
        let result = do_get(block, &ctx, "public", "index.html").await;
        assert!(matches!(result.action, Action::Respond));
    }

    #[tokio::test]
    async fn test_different_blocks_are_isolated() {
        let storage = Arc::new(MemoryStorageService::new());

        // Block A writes
        let ctx_a = TestContext::new(Some("suppers-ai/files"), storage.clone());
        let result = do_put(
            &ctx_a.storage_block,
            &ctx_a,
            "uploads",
            "secret.txt",
            b"secret data",
        )
        .await;
        assert!(matches!(result.action, Action::Respond));

        // Block B cannot read Block A's file (it reads from its own namespace)
        let ctx_b = TestContext::new(Some("suppers-ai/auth"), storage.clone());
        let result = do_get(&ctx_b.storage_block, &ctx_b, "uploads", "secret.txt").await;
        // Should fail — the file is at suppers-ai/files/uploads, not suppers-ai/auth/uploads
        assert!(
            matches!(result.action, Action::Error),
            "block B should NOT see block A's files"
        );
    }

    #[tokio::test]
    async fn test_cross_block_denied_by_default() {
        let storage = Arc::new(MemoryStorageService::new());

        // Block A writes a file
        let ctx_a = TestContext::new(Some("wafer-run/web"), storage.clone());
        do_put(
            &ctx_a.storage_block,
            &ctx_a,
            "public",
            "index.html",
            b"<html>",
        )
        .await;

        // Block B tries cross-block read via @ prefix — denied (no rules)
        let ctx_b = TestContext::new(Some("suppers-ai/files"), storage.clone());
        let result = do_get(
            &ctx_b.storage_block,
            &ctx_b,
            "@wafer-run/web/public",
            "index.html",
        )
        .await;
        assert!(
            matches!(result.action, Action::Error),
            "cross-block access should be denied by default"
        );
        let err_msg = result
            .error
            .as_ref()
            .map(|e| e.message.as_str())
            .unwrap_or("");
        assert!(
            err_msg.contains("denied"),
            "error should mention denied: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_cross_block_allowed_by_rule() {
        let storage = Arc::new(MemoryStorageService::new());

        // Block A writes a file
        let ctx_a = TestContext::new(Some("wafer-run/web"), storage.clone());
        do_put(
            &ctx_a.storage_block,
            &ctx_a,
            "public",
            "index.html",
            b"<html>hello</html>",
        )
        .await;

        // Set up a rule allowing suppers-ai/files to read wafer-run/web/*
        let ctx_b = TestContext::new(Some("suppers-ai/files"), storage.clone());
        ctx_b.seed_rule("allow", "suppers-ai/files", "wafer-run/web/*", "read", 0);

        // Block B reads via @ prefix — should succeed now
        let result = do_get(
            &ctx_b.storage_block,
            &ctx_b,
            "@wafer-run/web/public",
            "index.html",
        )
        .await;
        assert!(
            matches!(result.action, Action::Respond),
            "cross-block read should be allowed by rule, got: {:?}",
            result.error
        );
    }

    #[tokio::test]
    async fn test_cross_block_rule_wrong_access_type() {
        let storage = Arc::new(MemoryStorageService::new());

        let ctx_a = TestContext::new(Some("wafer-run/web"), storage.clone());
        do_put(&ctx_a.storage_block, &ctx_a, "public", "test.txt", b"data").await;

        // Rule allows only read, but we try write
        let ctx_b = TestContext::new(Some("suppers-ai/files"), storage.clone());
        ctx_b.seed_rule("allow", "suppers-ai/files", "wafer-run/web/*", "read", 0);

        let result = do_put(
            &ctx_b.storage_block,
            &ctx_b,
            "@wafer-run/web/public",
            "evil.txt",
            b"hacked",
        )
        .await;
        assert!(
            matches!(result.action, Action::Error),
            "write should be denied when rule only allows read"
        );
    }

    #[tokio::test]
    async fn test_cross_block_block_rule() {
        let storage = Arc::new(MemoryStorageService::new());

        let ctx_a = TestContext::new(Some("wafer-run/web"), storage.clone());
        do_put(&ctx_a.storage_block, &ctx_a, "public", "test.txt", b"data").await;

        // Allow rule + block rule: block rule should take precedence
        let ctx_b = TestContext::new(Some("suppers-ai/files"), storage.clone());
        ctx_b.seed_rule("allow", "*", "wafer-run/web/*", "readwrite", 0);
        ctx_b.seed_rule(
            "block",
            "suppers-ai/files",
            "wafer-run/web/*",
            "readwrite",
            10,
        );

        let result = do_get(
            &ctx_b.storage_block,
            &ctx_b,
            "@wafer-run/web/public",
            "test.txt",
        )
        .await;
        assert!(
            matches!(result.action, Action::Error),
            "block rule should deny even with allow rule present"
        );
    }

    #[tokio::test]
    async fn test_path_traversal_blocked() {
        let storage = Arc::new(MemoryStorageService::new());
        let ctx = TestContext::new(Some("wafer-run/web"), storage.clone());

        let result = do_get(&ctx.storage_block, &ctx, "../../etc", "passwd").await;
        assert!(
            matches!(result.action, Action::Error),
            "path traversal should be blocked"
        );
        let err_msg = result
            .error
            .as_ref()
            .map(|e| e.message.as_str())
            .unwrap_or("");
        assert!(
            err_msg.contains("traversal"),
            "error should mention traversal: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_own_namespace_unaffected_by_rules() {
        let storage = Arc::new(MemoryStorageService::new());

        // Seed an allow rule for a different block — should NOT affect own namespace
        let ctx = TestContext::new(Some("wafer-run/web"), storage.clone());
        ctx.seed_rule("allow", "suppers-ai/files", "wafer-run/web/*", "read", 0);

        // Own namespace put should still work fine
        let result = do_put(&ctx.storage_block, &ctx, "public", "test.txt", b"hello").await;
        assert!(
            matches!(result.action, Action::Respond),
            "own namespace access should not be affected by cross-block rules, got: {:?}",
            result.error
        );
    }
}
