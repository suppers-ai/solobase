use std::collections::HashMap;

use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};

use crate::wafer::block_world::types::*;
use crate::helpers::*;
use crate::quota;
use crate::share;

const SHARES_COLLECTION: &str = "cloud_shares";

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

pub fn handle(msg: &Message) -> BlockResult {
    let action = msg_get_meta(msg, "req.action");
    let path = msg_get_meta(msg, "req.resource");

    match (action, path) {
        // User-facing cloud storage
        ("retrieve", "/b/cloudstorage/shares") => handle_list_shares(msg),
        ("create", "/b/cloudstorage/shares") => handle_create_share(msg),
        ("delete", p) if p.starts_with("/b/cloudstorage/shares/") => handle_delete_share(msg),
        ("retrieve", "/b/cloudstorage/quota") => handle_get_quota(msg),
        // Admin cloud storage
        ("retrieve", "/admin/b/cloudstorage/shares") => share::handle_admin_list_shares(msg),
        ("retrieve", "/admin/b/cloudstorage/access-logs") => share::handle_access_logs(msg),
        ("retrieve", "/admin/b/cloudstorage/quotas") => handle_admin_quotas(msg),
        ("update", p) if p.starts_with("/admin/b/cloudstorage/quotas/") => handle_update_quota(msg),
        _ => err_not_found(msg, "not found"),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn handle_list_shares(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id");

    let opts = ListOptions {
        filters: vec![Filter {
            field: "created_by".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::json!(user_id),
        }],
        sort: vec![SortField { field: "created_at".to_string(), desc: true }],
        limit: 100,
        ..Default::default()
    };

    match db::list(SHARES_COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::json!(result)),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_create_share(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct Req {
        bucket: String,
        key: String,
        expires_in_hours: Option<i64>,
        max_access_count: Option<i64>,
    }

    let body: Req = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Generate share token
    let token = match share::generate_share_token(&body.bucket, &body.key) {
        Ok(t) => t,
        Err(r) => return r,
    };

    let user_id = msg_get_meta(msg, "auth.user_id");

    let mut data = HashMap::new();
    data.insert("token".to_string(), serde_json::json!(token));
    data.insert("bucket".to_string(), serde_json::json!(body.bucket));
    data.insert("key".to_string(), serde_json::json!(body.key));
    data.insert("created_by".to_string(), serde_json::json!(user_id));
    data.insert("created_at".to_string(), serde_json::json!(now_rfc3339()));
    data.insert("access_count".to_string(), serde_json::json!(0));

    if let Some(hours) = body.expires_in_hours {
        // Store the requested expiry hours; the host/native runtime will
        // compute the actual timestamp from server time.
        data.insert("expires_in_hours".to_string(), serde_json::json!(hours));
    }
    if let Some(max) = body.max_access_count {
        data.insert("max_access_count".to_string(), serde_json::json!(max));
    }

    match db::create(SHARES_COLLECTION, data) {
        Ok(record) => json_respond(msg, &serde_json::json!({
            "id": record.id,
            "token": token,
            "direct_url": format!("/storage/direct/{}", token)
        })),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_delete_share(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let id = path.strip_prefix("/b/cloudstorage/shares/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing share ID");
    }

    // Verify ownership
    let user_id = msg_get_meta(msg, "auth.user_id");
    let user_roles = msg_get_meta(msg, "auth.user_roles");
    if let Ok(existing) = db::get(SHARES_COLLECTION, id) {
        let owner = existing.data.get("created_by").and_then(|v| v.as_str()).unwrap_or("");
        if owner != user_id && !user_roles.split(',').any(|r| r.trim() == "admin") {
            return err_forbidden(msg, "Cannot delete another user's share");
        }
    }

    match db::delete(SHARES_COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == wafer_block::ErrorCode::NOT_FOUND => err_not_found(msg, "Share not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_get_quota(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id");
    let q = quota::get_user_quota(user_id);
    let usage = quota::get_user_usage(user_id);
    json_respond(msg, &serde_json::json!({
        "quota": {
            "max_storage_bytes": q.max_storage_bytes,
            "max_file_size_bytes": q.max_file_size_bytes,
            "max_files_per_bucket": q.max_files_per_bucket,
        },
        "usage": usage
    }))
}

fn handle_admin_quotas(msg: &Message) -> BlockResult {
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db::list("cloud_quotas", &opts) {
        Ok(result) => json_respond(msg, &serde_json::json!(result)),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_update_quota(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let target_user_id = path.strip_prefix("/admin/b/cloudstorage/quotas/").unwrap_or("");
    if target_user_id.is_empty() {
        return err_bad_request(msg, "Missing user ID");
    }

    let body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let mut data = body;
    data.insert("user_id".to_string(), serde_json::json!(target_user_id));
    data.insert("updated_at".to_string(), serde_json::json!(now_rfc3339()));

    match db::upsert("cloud_quotas", "user_id", serde_json::json!(target_user_id), data) {
        Ok(record) => json_respond(msg, &serde_json::json!(record)),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}
