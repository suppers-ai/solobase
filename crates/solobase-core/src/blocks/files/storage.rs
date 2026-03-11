use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_core::clients::storage as store;

const OBJECTS_META_COLLECTION: &str = "storage_objects";
const BUCKETS_COLLECTION: &str = "storage_buckets";

pub async fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/storage/buckets") => handle_list_buckets(ctx, msg).await,
        ("create", "/storage/buckets") => handle_create_bucket(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/storage/buckets/") && path.contains("/objects") => {
            if path.contains("/objects/") {
                handle_get_object(ctx, msg).await
            } else {
                handle_list_objects(ctx, msg).await
            }
        }
        ("create", _) if path.starts_with("/storage/buckets/") && path.contains("/objects") => {
            handle_upload_object(ctx, msg).await
        }
        ("delete", _) if path.starts_with("/storage/buckets/") && path.contains("/objects/") => {
            handle_delete_object(ctx, msg).await
        }
        ("delete", _) if path.starts_with("/storage/buckets/") => handle_delete_bucket(ctx, msg).await,
        ("retrieve", "/storage/search") => handle_search(ctx, msg).await,
        ("retrieve", "/storage/recent") => handle_recent(ctx, msg).await,
        _ => err_not_found(msg, "not found"),
    }
}

pub async fn handle_admin(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/storage/buckets") => handle_list_buckets(ctx, msg).await,
        ("retrieve", "/admin/storage/stats") => handle_stats(ctx, msg).await,
        _ => err_not_found(msg, "not found"),
    }
}

fn extract_bucket_name(path: &str) -> &str {
    let rest = path.strip_prefix("/storage/buckets/")
        .or_else(|| path.strip_prefix("/admin/storage/buckets/"))
        .unwrap_or("");
    if let Some(idx) = rest.find('/') { &rest[..idx] } else { rest }
}

fn extract_object_key(path: &str) -> &str {
    if let Some(idx) = path.find("/objects/") {
        &path[idx + 9..]
    } else {
        ""
    }
}

/// Validate a storage key for path traversal attacks.
/// Rejects keys containing `..`, absolute paths, or null bytes.
/// Validate a storage key for path traversal attacks.
/// Rejects keys containing `..`, absolute paths, or null bytes.
fn is_valid_storage_key(key: &str) -> bool {
    !key.is_empty()
        && !key.contains("..")
        && !key.starts_with('/')
        && !key.contains('\0')
}

/// Validate a bucket name. Must be non-empty, no path traversal, no slashes.
fn is_valid_bucket_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\0')
}

async fn handle_list_buckets(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    match store::list_folders(ctx).await {
        Ok(folders) => json_respond(msg, &serde_json::json!({"buckets": folders})),
        Err(e) => err_internal(msg, &format!("Storage error: {e}")),
    }
}

async fn handle_create_bucket(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    #[derive(serde::Deserialize)]
    struct Req { name: String, #[serde(default)] public: bool }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    if body.name.is_empty() {
        return err_bad_request(msg, "Bucket name is required");
    }
    if !is_valid_bucket_name(&body.name) {
        return err_bad_request(msg, "Invalid bucket name");
    }

    match store::create_folder(ctx, &body.name, body.public).await {
        Ok(()) => {
            // Track in DB
            let mut data = HashMap::new();
            data.insert("name".to_string(), serde_json::Value::String(body.name.clone()));
            data.insert("public".to_string(), serde_json::Value::Bool(body.public));
            data.insert("created_by".to_string(), serde_json::Value::String(msg.user_id().to_string()));
            data.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            if let Err(e) = db::create(ctx, BUCKETS_COLLECTION, data).await {
                tracing::warn!("Failed to track bucket creation in database: {e}");
            }
            json_respond(msg, &serde_json::json!({"name": body.name, "created": true}))
        }
        Err(e) => err_internal(msg, &format!("Failed to create bucket: {e}")),
    }
}

async fn handle_delete_bucket(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() { return err_bad_request(msg, "Missing bucket name"); }

    match store::delete_folder(ctx, bucket).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => err_internal(msg, &format!("Failed to delete bucket: {e}")),
    }
}

async fn handle_list_objects(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() { return err_bad_request(msg, "Missing bucket name"); }

    let prefix = msg.query("prefix").to_string();
    let (_, page_size, offset) = msg.pagination_params(50);

    let opts = store::ListOptions {
        prefix,
        limit: page_size as i64,
        offset: offset as i64,
    };

    match store::list(ctx, bucket, &opts).await {
        Ok(list) => json_respond(msg, &list),
        Err(e) => err_internal(msg, &format!("Storage error: {e}")),
    }
}

async fn handle_get_object(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    let key = extract_object_key(path);
    if bucket.is_empty() || key.is_empty() {
        return err_bad_request(msg, "Missing bucket name or object key");
    }
    if !is_valid_storage_key(key) {
        return err_bad_request(msg, "Invalid object key");
    }

    // Track view in DB
    let mut data = HashMap::new();
    data.insert("bucket".to_string(), serde_json::Value::String(bucket.to_string()));
    data.insert("key".to_string(), serde_json::Value::String(key.to_string()));
    data.insert("user_id".to_string(), serde_json::Value::String(msg.user_id().to_string()));
    data.insert("viewed_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    if let Err(e) = db::create(ctx, "storage_views", data).await {
        tracing::warn!("Failed to track storage object view: {e}");
    }

    match store::get(ctx, bucket, key).await {
        Ok((data, info)) => respond(msg, data, &info.content_type),
        Err(e) if e.code == "not_found" => err_not_found(msg, "Object not found"),
        Err(e) => err_internal(msg, &format!("Storage error: {e}")),
    }
}

async fn handle_upload_object(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() { return err_bad_request(msg, "Missing bucket name"); }

    let key = msg.query("key").to_string();
    if key.is_empty() {
        return err_bad_request(msg, "Missing object key (pass as ?key=filename)");
    }
    if !is_valid_storage_key(&key) {
        return err_bad_request(msg, "Invalid object key");
    }

    let content_type = if msg.content_type().is_empty() { "application/octet-stream" } else { msg.content_type() };

    // Check quota
    if let Err(r) = super::quota::check_quota(ctx, msg.user_id(), msg.body().len() as i64).await {
        return r;
    }

    match store::put(ctx, bucket, &key, msg.body(), content_type).await {
        Ok(()) => {
            // Track metadata
            let mut data = HashMap::new();
            data.insert("bucket".to_string(), serde_json::Value::String(bucket.to_string()));
            data.insert("key".to_string(), serde_json::Value::String(key.clone()));
            data.insert("size".to_string(), serde_json::json!(msg.body().len()));
            data.insert("content_type".to_string(), serde_json::Value::String(content_type.to_string()));
            data.insert("uploaded_by".to_string(), serde_json::Value::String(msg.user_id().to_string()));
            data.insert("uploaded_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            if let Err(e) = db::create(ctx, OBJECTS_META_COLLECTION, data).await {
                tracing::warn!("Failed to store object metadata: {e}");
            }
            json_respond(msg, &serde_json::json!({"bucket": bucket, "key": key, "uploaded": true}))
        }
        Err(e) => err_internal(msg, &format!("Upload failed: {e}")),
    }
}

async fn handle_delete_object(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    let key = extract_object_key(path);
    if bucket.is_empty() || key.is_empty() {
        return err_bad_request(msg, "Missing bucket name or object key");
    }
    if !is_valid_storage_key(key) {
        return err_bad_request(msg, "Invalid object key");
    }

    match store::delete(ctx, bucket, key).await {
        Ok(()) => {
            // Clean up metadata
            db::delete_by_filters(ctx, OBJECTS_META_COLLECTION, vec![
                Filter { field: "bucket".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(bucket.to_string()) },
                Filter { field: "key".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(key.to_string()) },
            ]).await.ok();
            json_respond(msg, &serde_json::json!({"deleted": true}))
        }
        Err(e) if e.code == "not_found" => err_not_found(msg, "Object not found"),
        Err(e) => err_internal(msg, &format!("Delete failed: {e}")),
    }
}

async fn handle_search(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let query = msg.query("q").to_string();
    if query.is_empty() { return err_bad_request(msg, "Missing search query"); }

    let (_, page_size, offset) = msg.pagination_params(20);
    let opts = ListOptions {
        filters: vec![Filter {
            field: "key".to_string(),
            operator: FilterOp::Like,
            value: serde_json::Value::String(format!("%{}%", query)),
        }],
        sort: vec![SortField { field: "uploaded_at".to_string(), desc: true }],
        limit: page_size as i64,
        offset: offset as i64,
    };

    match db::list(ctx, OBJECTS_META_COLLECTION, &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Search failed: {e}")),
    }
}

async fn handle_recent(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();

    let opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        }],
        sort: vec![SortField { field: "viewed_at".to_string(), desc: true }],
        limit: 20,
        ..Default::default()
    };

    match db::list(ctx, "storage_views", &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bucket_name() {
        assert_eq!(extract_bucket_name("/storage/buckets/my-bucket"), "my-bucket");
        assert_eq!(extract_bucket_name("/storage/buckets/my-bucket/objects"), "my-bucket");
        assert_eq!(extract_bucket_name("/storage/buckets/my-bucket/objects/file.txt"), "my-bucket");
        assert_eq!(extract_bucket_name("/admin/storage/buckets/admin-bucket"), "admin-bucket");
        assert_eq!(extract_bucket_name("/other/path"), "");
    }

    #[test]
    fn test_extract_object_key() {
        assert_eq!(extract_object_key("/storage/buckets/b/objects/file.txt"), "file.txt");
        assert_eq!(extract_object_key("/storage/buckets/b/objects/dir/file.txt"), "dir/file.txt");
        assert_eq!(extract_object_key("/storage/buckets/b"), "");
        assert_eq!(extract_object_key("/other/path"), "");
    }

    #[test]
    fn test_is_valid_storage_key() {
        // Valid keys
        assert!(is_valid_storage_key("file.txt"));
        assert!(is_valid_storage_key("dir/file.txt"));
        assert!(is_valid_storage_key("a/b/c/file.txt"));
        assert!(is_valid_storage_key("file-name_123.txt"));

        // Invalid keys
        assert!(!is_valid_storage_key(""));
        assert!(!is_valid_storage_key("../etc/passwd"));
        assert!(!is_valid_storage_key("dir/../secret"));
        assert!(!is_valid_storage_key("/absolute/path"));
        assert!(!is_valid_storage_key("file\0name"));
        assert!(!is_valid_storage_key(".."));
    }

    #[test]
    fn test_is_valid_bucket_name() {
        // Valid bucket names
        assert!(is_valid_bucket_name("my-bucket"));
        assert!(is_valid_bucket_name("bucket123"));
        assert!(is_valid_bucket_name("uploads"));

        // Invalid bucket names
        assert!(!is_valid_bucket_name(""));
        assert!(!is_valid_bucket_name("../other"));
        assert!(!is_valid_bucket_name("bucket/subdir"));
        assert!(!is_valid_bucket_name("bucket\0name"));
        assert!(!is_valid_bucket_name(".."));
    }
}

async fn handle_stats(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let total_objects = db::count(ctx, OBJECTS_META_COLLECTION, &[]).await.unwrap_or(0);
    let total_size = db::sum(ctx, OBJECTS_META_COLLECTION, "size", &[]).await.unwrap_or(0.0);
    let bucket_count = store::list_folders(ctx).await.map(|f| f.len()).unwrap_or(0);

    json_respond(msg, &serde_json::json!({
        "total_objects": total_objects,
        "total_size_bytes": total_size as i64,
        "bucket_count": bucket_count
    }))
}
