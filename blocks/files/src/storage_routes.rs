use std::collections::HashMap;

use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_core::clients::storage;

use crate::wafer::block_world::types::*;
use crate::helpers::*;
use crate::quota;

const OBJECTS_META_COLLECTION: &str = "storage_objects";
const BUCKETS_COLLECTION: &str = "storage_buckets";

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn extract_bucket_name(path: &str) -> &str {
    let rest = path
        .strip_prefix("/storage/buckets/")
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

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn is_valid_storage_key(key: &str) -> bool {
    !key.is_empty()
        && !key.contains("..")
        && !key.starts_with('/')
        && !key.contains('\0')
}

fn is_valid_bucket_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\0')
}

// ---------------------------------------------------------------------------
// User storage routes
// ---------------------------------------------------------------------------

pub fn handle(msg: &Message) -> BlockResult {
    let action = msg_get_meta(msg, "req.action");
    let path = msg_get_meta(msg, "req.resource");

    match (action, path) {
        ("retrieve", "/storage/buckets") => handle_list_buckets(msg),
        ("create", "/storage/buckets") => handle_create_bucket(msg),
        ("retrieve", p) if p.starts_with("/storage/buckets/") && p.contains("/objects") => {
            if p.contains("/objects/") {
                handle_get_object(msg)
            } else {
                handle_list_objects(msg)
            }
        }
        ("create", p) if p.starts_with("/storage/buckets/") && p.contains("/objects") => {
            handle_upload_object(msg)
        }
        ("delete", p) if p.starts_with("/storage/buckets/") && p.contains("/objects/") => {
            handle_delete_object(msg)
        }
        ("delete", p) if p.starts_with("/storage/buckets/") => handle_delete_bucket(msg),
        ("retrieve", "/storage/search") => handle_search(msg),
        ("retrieve", "/storage/recent") => handle_recent(msg),
        _ => err_not_found(msg, "not found"),
    }
}

// ---------------------------------------------------------------------------
// Admin storage routes
// ---------------------------------------------------------------------------

pub fn handle_admin(msg: &Message) -> BlockResult {
    let action = msg_get_meta(msg, "req.action");
    let path = msg_get_meta(msg, "req.resource");

    match (action, path) {
        ("retrieve", "/admin/storage/buckets") => handle_list_buckets(msg),
        ("retrieve", "/admin/storage/stats") => handle_stats(msg),
        _ => err_not_found(msg, "not found"),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn handle_list_buckets(msg: &Message) -> BlockResult {
    match storage::list_folders() {
        Ok(folders) => json_respond(msg, &serde_json::json!({"buckets": folders})),
        Err(e) => err_internal(msg, &format!("Storage error: {e}")),
    }
}

fn handle_create_bucket(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct Req { name: String, #[serde(default)] public: bool }

    let body: Req = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    if body.name.is_empty() {
        return err_bad_request(msg, "Bucket name is required");
    }
    if !is_valid_bucket_name(&body.name) {
        return err_bad_request(msg, "Invalid bucket name");
    }

    match storage::create_folder(&body.name, body.public) {
        Ok(()) => {
            // Track in DB
            let user_id = msg_get_meta(msg, "auth.user_id");
            let mut data = HashMap::new();
            data.insert("name".to_string(), serde_json::json!(body.name));
            data.insert("public".to_string(), serde_json::json!(body.public));
            data.insert("created_by".to_string(), serde_json::json!(user_id));
            data.insert("created_at".to_string(), serde_json::json!(now_rfc3339()));
            let _ = db::create(BUCKETS_COLLECTION, data);
            json_respond(msg, &serde_json::json!({"name": body.name, "created": true}))
        }
        Err(e) => err_internal(msg, &format!("Failed to create bucket: {e}")),
    }
}

fn handle_delete_bucket(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() {
        return err_bad_request(msg, "Missing bucket name");
    }

    match storage::delete_folder(bucket) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => err_internal(msg, &format!("Failed to delete bucket: {e}")),
    }
}

fn handle_list_objects(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() {
        return err_bad_request(msg, "Missing bucket name");
    }

    let prefix = msg_query(msg, "prefix").to_string();
    let (_, page_size, offset) = pagination_params(msg, 50);

    let opts = storage::ListOptions {
        prefix,
        limit: page_size,
        offset,
    };

    match storage::list(bucket, &opts) {
        Ok(list) => json_respond(msg, &serde_json::json!(list)),
        Err(e) => err_internal(msg, &format!("Storage error: {e}")),
    }
}

fn handle_get_object(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let bucket = extract_bucket_name(path);
    let key = extract_object_key(path);
    if bucket.is_empty() || key.is_empty() {
        return err_bad_request(msg, "Missing bucket name or object key");
    }
    if !is_valid_storage_key(key) {
        return err_bad_request(msg, "Invalid object key");
    }

    // Track view in DB
    let user_id = msg_get_meta(msg, "auth.user_id");
    let mut view_data = HashMap::new();
    view_data.insert("bucket".to_string(), serde_json::json!(bucket));
    view_data.insert("key".to_string(), serde_json::json!(key));
    view_data.insert("user_id".to_string(), serde_json::json!(user_id));
    view_data.insert("viewed_at".to_string(), serde_json::json!(now_rfc3339()));
    let _ = db::create("storage_views", view_data);

    match storage::get(bucket, key) {
        Ok((data, info)) => respond_binary(msg, data, &info.content_type, &[]),
        Err(e) if e.code == wafer_block::ErrorCode::NOT_FOUND => err_not_found(msg, "Object not found"),
        Err(e) => err_internal(msg, &format!("Storage error: {e}")),
    }
}

fn handle_upload_object(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() {
        return err_bad_request(msg, "Missing bucket name");
    }

    let key = msg_query(msg, "key").to_string();
    if key.is_empty() {
        return err_bad_request(msg, "Missing object key (pass as ?key=filename)");
    }
    if !is_valid_storage_key(&key) {
        return err_bad_request(msg, "Invalid object key");
    }

    let content_type = {
        let ct = msg_get_meta(msg, "req.content_type");
        if ct.is_empty() { "application/octet-stream" } else { ct }
    };

    // Check quota
    let user_id = msg_get_meta(msg, "auth.user_id");
    if let Err(r) = quota::check_quota(msg, user_id, msg.data.len() as i64) {
        return r;
    }

    match storage::put(bucket, &key, &msg.data, content_type) {
        Ok(()) => {
            // Track metadata
            let mut data = HashMap::new();
            data.insert("bucket".to_string(), serde_json::json!(bucket));
            data.insert("key".to_string(), serde_json::json!(key));
            data.insert("size".to_string(), serde_json::json!(msg.data.len()));
            data.insert("content_type".to_string(), serde_json::json!(content_type));
            data.insert("uploaded_by".to_string(), serde_json::json!(user_id));
            data.insert("uploaded_at".to_string(), serde_json::json!(now_rfc3339()));
            let _ = db::create(OBJECTS_META_COLLECTION, data);
            json_respond(msg, &serde_json::json!({"bucket": bucket, "key": key, "uploaded": true}))
        }
        Err(e) => err_internal(msg, &format!("Upload failed: {e}")),
    }
}

fn handle_delete_object(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let bucket = extract_bucket_name(path);
    let key = extract_object_key(path);
    if bucket.is_empty() || key.is_empty() {
        return err_bad_request(msg, "Missing bucket name or object key");
    }
    if !is_valid_storage_key(key) {
        return err_bad_request(msg, "Invalid object key");
    }

    match storage::delete(bucket, key) {
        Ok(()) => {
            // Clean up metadata
            let _ = db::delete_by_filters(OBJECTS_META_COLLECTION, vec![
                Filter { field: "bucket".to_string(), operator: FilterOp::Equal, value: serde_json::json!(bucket) },
                Filter { field: "key".to_string(), operator: FilterOp::Equal, value: serde_json::json!(key) },
            ]);
            json_respond(msg, &serde_json::json!({"deleted": true}))
        }
        Err(e) if e.code == wafer_block::ErrorCode::NOT_FOUND => err_not_found(msg, "Object not found"),
        Err(e) => err_internal(msg, &format!("Delete failed: {e}")),
    }
}

fn handle_search(msg: &Message) -> BlockResult {
    let query = msg_query(msg, "q");
    if query.is_empty() {
        return err_bad_request(msg, "Missing search query");
    }

    let (_, page_size, offset) = pagination_params(msg, 20);
    let opts = ListOptions {
        filters: vec![Filter {
            field: "key".to_string(),
            operator: FilterOp::Like,
            value: serde_json::json!(format!("%{}%", query)),
        }],
        sort: vec![SortField { field: "uploaded_at".to_string(), desc: true }],
        limit: page_size,
        offset,
    };

    match db::list(OBJECTS_META_COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::json!(result)),
        Err(e) => err_internal(msg, &format!("Search failed: {e}")),
    }
}

fn handle_recent(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id");

    let opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::json!(user_id),
        }],
        sort: vec![SortField { field: "viewed_at".to_string(), desc: true }],
        limit: 20,
        ..Default::default()
    };

    match db::list("storage_views", &opts) {
        Ok(result) => json_respond(msg, &serde_json::json!(result)),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_stats(msg: &Message) -> BlockResult {
    let total_objects = db::count(OBJECTS_META_COLLECTION, &[]).unwrap_or(0);
    let total_size = db::sum(OBJECTS_META_COLLECTION, "size", &[]).unwrap_or(0.0);
    let bucket_count = storage::list_folders().map(|f| f.len()).unwrap_or(0);

    json_respond(msg, &serde_json::json!({
        "total_objects": total_objects,
        "total_size_bytes": total_size as i64,
        "bucket_count": bucket_count
    }))
}
