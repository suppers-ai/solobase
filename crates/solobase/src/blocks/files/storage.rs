use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{self, Filter, FilterOp, ListOptions, SortField};
use wafer_run::services::storage;
use super::{get_db, get_storage};

const OBJECTS_META_COLLECTION: &str = "storage_objects";
const BUCKETS_COLLECTION: &str = "storage_buckets";

pub fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/storage/buckets") => handle_list_buckets(ctx, msg),
        ("create", "/storage/buckets") => handle_create_bucket(ctx, msg),
        ("retrieve", _) if path.starts_with("/storage/buckets/") && path.contains("/objects") => {
            if path.contains("/objects/") {
                handle_get_object(ctx, msg)
            } else {
                handle_list_objects(ctx, msg)
            }
        }
        ("create", _) if path.starts_with("/storage/buckets/") && path.contains("/objects") => {
            handle_upload_object(ctx, msg)
        }
        ("delete", _) if path.starts_with("/storage/buckets/") && path.contains("/objects/") => {
            handle_delete_object(ctx, msg)
        }
        ("delete", _) if path.starts_with("/storage/buckets/") => handle_delete_bucket(ctx, msg),
        ("retrieve", "/storage/search") => handle_search(ctx, msg),
        ("retrieve", "/storage/recent") => handle_recent(ctx, msg),
        _ => err_not_found(msg.clone(), "not found"),
    }
}

pub fn handle_admin(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/storage/buckets") => handle_list_buckets(ctx, msg),
        ("retrieve", "/admin/storage/stats") => handle_stats(ctx, msg),
        _ => err_not_found(msg.clone(), "not found"),
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

fn handle_list_buckets(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let store = match get_storage(ctx) { Ok(s) => s, Err(r) => return r };
    match store.list_folders() {
        Ok(folders) => json_respond(msg.clone(), 200, &serde_json::json!({"buckets": folders})),
        Err(e) => err_internal(msg.clone(), &format!("Storage error: {e}")),
    }
}

fn handle_create_bucket(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let store = match get_storage(ctx) { Ok(s) => s, Err(r) => return r };
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };

    #[derive(serde::Deserialize)]
    struct Req { name: String, #[serde(default)] public: bool }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };

    if body.name.is_empty() {
        return err_bad_request(msg.clone(), "Bucket name is required");
    }

    match store.create_folder(&body.name, body.public) {
        Ok(()) => {
            // Track in DB
            let mut data = HashMap::new();
            data.insert("name".to_string(), serde_json::Value::String(body.name.clone()));
            data.insert("public".to_string(), serde_json::Value::Bool(body.public));
            data.insert("created_by".to_string(), serde_json::Value::String(msg.user_id().to_string()));
            data.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            if let Err(e) = db.create(BUCKETS_COLLECTION, data) {
                tracing::warn!("Failed to track bucket creation in database: {e}");
            }
            json_respond(msg.clone(), 201, &serde_json::json!({"name": body.name, "created": true}))
        }
        Err(e) => err_internal(msg.clone(), &format!("Failed to create bucket: {e}")),
    }
}

fn handle_delete_bucket(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let store = match get_storage(ctx) { Ok(s) => s, Err(r) => return r };
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() { return err_bad_request(msg.clone(), "Missing bucket name"); }

    match store.delete_folder(bucket) {
        Ok(()) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
        Err(e) => err_internal(msg.clone(), &format!("Failed to delete bucket: {e}")),
    }
}

fn handle_list_objects(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let store = match get_storage(ctx) { Ok(s) => s, Err(r) => return r };
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() { return err_bad_request(msg.clone(), "Missing bucket name"); }

    let prefix = msg.query("prefix").to_string();
    let (_, page_size, offset) = msg.pagination_params(50);

    let opts = storage::ListOptions {
        prefix,
        limit: page_size as i64,
        offset: offset as i64,
    };

    match store.list(bucket, &opts) {
        Ok(list) => json_respond(msg.clone(), 200, &list),
        Err(e) => err_internal(msg.clone(), &format!("Storage error: {e}")),
    }
}

fn handle_get_object(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let store = match get_storage(ctx) { Ok(s) => s, Err(r) => return r };
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    let key = extract_object_key(path);
    if bucket.is_empty() || key.is_empty() {
        return err_bad_request(msg.clone(), "Missing bucket name or object key");
    }

    // Track view in DB
    if let Ok(db) = get_db(ctx) {
        let mut data = HashMap::new();
        data.insert("bucket".to_string(), serde_json::Value::String(bucket.to_string()));
        data.insert("key".to_string(), serde_json::Value::String(key.to_string()));
        data.insert("user_id".to_string(), serde_json::Value::String(msg.user_id().to_string()));
        data.insert("viewed_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        if let Err(e) = db.create("storage_views", data) {
            tracing::warn!("Failed to track storage object view: {e}");
        }
    }

    match store.get(bucket, key) {
        Ok((data, info)) => respond(msg.clone(), 200, data, &info.content_type),
        Err(storage::StorageError::NotFound) => err_not_found(msg.clone(), "Object not found"),
        Err(e) => err_internal(msg.clone(), &format!("Storage error: {e}")),
    }
}

fn handle_upload_object(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let store = match get_storage(ctx) { Ok(s) => s, Err(r) => return r };
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    if bucket.is_empty() { return err_bad_request(msg.clone(), "Missing bucket name"); }

    let key = msg.query("key").to_string();
    if key.is_empty() {
        return err_bad_request(msg.clone(), "Missing object key (pass as ?key=filename)");
    }

    let content_type = if msg.content_type().is_empty() { "application/octet-stream" } else { msg.content_type() };

    // Check quota
    if let Err(r) = super::quota::check_quota(ctx, msg.user_id(), bucket, msg.body().len() as i64) {
        return r;
    }

    match store.put(bucket, &key, msg.body(), content_type) {
        Ok(()) => {
            // Track metadata
            let mut data = HashMap::new();
            data.insert("bucket".to_string(), serde_json::Value::String(bucket.to_string()));
            data.insert("key".to_string(), serde_json::Value::String(key.clone()));
            data.insert("size".to_string(), serde_json::json!(msg.body().len()));
            data.insert("content_type".to_string(), serde_json::Value::String(content_type.to_string()));
            data.insert("uploaded_by".to_string(), serde_json::Value::String(msg.user_id().to_string()));
            data.insert("uploaded_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            if let Err(e) = db.create(OBJECTS_META_COLLECTION, data) {
                tracing::warn!("Failed to store object metadata: {e}");
            }
            json_respond(msg.clone(), 201, &serde_json::json!({"bucket": bucket, "key": key, "uploaded": true}))
        }
        Err(e) => err_internal(msg.clone(), &format!("Upload failed: {e}")),
    }
}

fn handle_delete_object(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let store = match get_storage(ctx) { Ok(s) => s, Err(r) => return r };
    let path = msg.path();
    let bucket = extract_bucket_name(path);
    let key = extract_object_key(path);
    if bucket.is_empty() || key.is_empty() {
        return err_bad_request(msg.clone(), "Missing bucket name or object key");
    }

    match store.delete(bucket, key) {
        Ok(()) => {
            // Clean up metadata
            if let Ok(db) = get_db(ctx) {
                database::delete_by_filters(db.as_ref(), OBJECTS_META_COLLECTION, vec![
                    Filter { field: "bucket".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(bucket.to_string()) },
                    Filter { field: "key".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(key.to_string()) },
                ]).ok();
            }
            json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true}))
        }
        Err(storage::StorageError::NotFound) => err_not_found(msg.clone(), "Object not found"),
        Err(e) => err_internal(msg.clone(), &format!("Delete failed: {e}")),
    }
}

fn handle_search(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let query = msg.query("q").to_string();
    if query.is_empty() { return err_bad_request(msg.clone(), "Missing search query"); }

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

    match db.list(OBJECTS_META_COLLECTION, &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Search failed: {e}")),
    }
}

fn handle_recent(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
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

    match db.list("storage_views", &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_stats(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let store = match get_storage(ctx) { Ok(s) => s, Err(r) => return r };

    let total_objects = db.count(OBJECTS_META_COLLECTION, &[]).unwrap_or(0);
    let total_size = db.sum(OBJECTS_META_COLLECTION, "size", &[]).unwrap_or(0.0);
    let bucket_count = store.list_folders().map(|f| f.len()).unwrap_or(0);

    json_respond(msg.clone(), 200, &serde_json::json!({
        "total_objects": total_objects,
        "total_size_bytes": total_size as i64,
        "bucket_count": bucket_count
    }))
}
