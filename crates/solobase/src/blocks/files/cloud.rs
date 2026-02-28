use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{self, Filter, FilterOp, ListOptions, SortField};
use super::get_db;

const SHARES_COLLECTION: &str = "cloud_shares";
const ACCESS_LOGS_COLLECTION: &str = "cloud_access_logs";

pub fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        // User-facing cloud storage
        ("retrieve", "/ext/cloudstorage/shares") => handle_list_shares(ctx, msg),
        ("create", "/ext/cloudstorage/shares") => handle_create_share(ctx, msg),
        ("delete", _) if path.starts_with("/ext/cloudstorage/shares/") => handle_delete_share(ctx, msg),
        ("retrieve", "/ext/cloudstorage/quota") => handle_get_quota(ctx, msg),
        // Admin cloud storage
        ("retrieve", "/admin/ext/cloudstorage/shares") => handle_admin_list_shares(ctx, msg),
        ("retrieve", "/admin/ext/cloudstorage/access-logs") => handle_access_logs(ctx, msg),
        ("retrieve", "/admin/ext/cloudstorage/quotas") => handle_admin_quotas(ctx, msg),
        ("update", _) if path.starts_with("/admin/ext/cloudstorage/quotas/") => handle_update_quota(ctx, msg),
        _ => err_not_found(msg.clone(), "not found"),
    }
}

fn handle_list_shares(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let user_id = msg.user_id().to_string();

    let opts = ListOptions {
        filters: vec![Filter {
            field: "created_by".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        }],
        sort: vec![SortField { field: "created_at".to_string(), desc: true }],
        limit: 100,
        ..Default::default()
    };

    match db.list(SHARES_COLLECTION, &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_create_share(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };

    #[derive(serde::Deserialize)]
    struct Req {
        bucket: String,
        key: String,
        expires_in_hours: Option<i64>,
        max_access_count: Option<i64>,
    }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };

    // Generate share token
    let token = super::share::generate_share_token(ctx, &body.bucket, &body.key);
    let token = match token {
        Ok(t) => t,
        Err(r) => return r,
    };

    let now = chrono::Utc::now();
    let expires_at = body.expires_in_hours.map(|h| {
        (now + chrono::Duration::hours(h)).to_rfc3339()
    });

    let mut data = HashMap::new();
    data.insert("token".to_string(), serde_json::Value::String(token.clone()));
    data.insert("bucket".to_string(), serde_json::Value::String(body.bucket));
    data.insert("key".to_string(), serde_json::Value::String(body.key));
    data.insert("created_by".to_string(), serde_json::Value::String(msg.user_id().to_string()));
    data.insert("created_at".to_string(), serde_json::Value::String(now.to_rfc3339()));
    data.insert("access_count".to_string(), serde_json::json!(0));
    if let Some(exp) = &expires_at {
        data.insert("expires_at".to_string(), serde_json::Value::String(exp.clone()));
    }
    if let Some(max) = body.max_access_count {
        data.insert("max_access_count".to_string(), serde_json::json!(max));
    }

    match db.create(SHARES_COLLECTION, data) {
        Ok(record) => json_respond(msg.clone(), 201, &serde_json::json!({
            "id": record.id,
            "token": token,
            "direct_url": format!("/storage/direct/{}", token)
        })),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_delete_share(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let path = msg.path();
    let id = path.strip_prefix("/ext/cloudstorage/shares/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing share ID"); }

    // Verify ownership
    if let Ok(share) = db.get(SHARES_COLLECTION, id) {
        let owner = share.data.get("created_by").and_then(|v| v.as_str()).unwrap_or("");
        if owner != msg.user_id() && !msg.is_admin() {
            return err_forbidden(msg.clone(), "Cannot delete another user's share");
        }
    }

    match db.delete(SHARES_COLLECTION, id) {
        Ok(()) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
        Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "Share not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_get_quota(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let quota = super::quota::get_user_quota(ctx, msg.user_id());
    let usage = super::quota::get_user_usage(ctx, msg.user_id());
    json_respond(msg.clone(), 200, &serde_json::json!({
        "quota": quota,
        "usage": usage
    }))
}

fn handle_admin_list_shares(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let (page, page_size, _) = msg.pagination_params(20);
    let opts = ListOptions {
        sort: vec![SortField { field: "created_at".to_string(), desc: true }],
        limit: page_size as i64,
        offset: ((page - 1) * page_size) as i64,
        ..Default::default()
    };
    match db.list(SHARES_COLLECTION, &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_access_logs(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let (page, page_size, _) = msg.pagination_params(50);

    let mut filters = Vec::new();
    let share_id = msg.query("share_id").to_string();
    if !share_id.is_empty() {
        filters.push(Filter {
            field: "share_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(share_id),
        });
    }

    let opts = ListOptions {
        filters,
        sort: vec![SortField { field: "accessed_at".to_string(), desc: true }],
        limit: page_size as i64,
        offset: ((page - 1) * page_size) as i64,
    };

    match db.list(ACCESS_LOGS_COLLECTION, &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_admin_quotas(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db.list("cloud_quotas", &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_update_quota(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let path = msg.path();
    let user_id = path.strip_prefix("/admin/ext/cloudstorage/quotas/").unwrap_or("");
    if user_id.is_empty() { return err_bad_request(msg.clone(), "Missing user ID"); }

    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };

    let mut data = body;
    data.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
    data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    match database::upsert(db.as_ref(), "cloud_quotas", "user_id", serde_json::Value::String(user_id.to_string()), data) {
        Ok(record) => json_respond(msg.clone(), 200, &record),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}
