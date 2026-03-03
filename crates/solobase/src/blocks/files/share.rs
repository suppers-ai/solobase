use std::collections::HashMap;
use std::time::Duration;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::storage as store;
use wafer_core::clients::crypto;

const SHARES_COLLECTION: &str = "cloud_shares";
const ACCESS_LOGS_COLLECTION: &str = "cloud_access_logs";

pub fn generate_share_token(ctx: &dyn Context, bucket: &str, key: &str) -> Result<String, Result_> {
    let mut claims = HashMap::new();
    claims.insert("bucket".to_string(), serde_json::Value::String(bucket.to_string()));
    claims.insert("key".to_string(), serde_json::Value::String(key.to_string()));
    claims.insert("type".to_string(), serde_json::Value::String("share".to_string()));

    crypto::sign(ctx, &claims, Duration::from_secs(365 * 24 * 3600))
        .map_err(|e| Result_::error(WaferError::new("internal", format!("Token generation failed: {e}"))))
}

pub fn handle_direct_access(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let token = path.strip_prefix("/storage/direct/").unwrap_or("");
    if token.is_empty() {
        return err_bad_request(msg.clone(), "Missing share token");
    }

    // Look up share by token
    let share = match db::get_by_field(ctx, SHARES_COLLECTION, "token", serde_json::Value::String(token.to_string())) {
        Ok(s) => s,
        Err(_) => return err_not_found(msg.clone(), "Share not found or expired"),
    };

    // Check expiry
    if let Some(expires) = share.data.get("expires_at").and_then(|v| v.as_str()) {
        if !expires.is_empty() {
            if let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(expires) {
                if exp_time < chrono::Utc::now() {
                    return err_forbidden(msg.clone(), "Share link has expired");
                }
            }
        }
    }

    // Check access count
    let access_count = share.data.get("access_count").and_then(|v| v.as_i64()).unwrap_or(0);
    if let Some(max) = share.data.get("max_access_count").and_then(|v| v.as_i64()) {
        if max > 0 && access_count >= max {
            return err_forbidden(msg.clone(), "Share link access limit reached");
        }
    }

    let bucket = share.data.get("bucket").and_then(|v| v.as_str()).unwrap_or("");
    let key = share.data.get("key").and_then(|v| v.as_str()).unwrap_or("");

    if bucket.is_empty() || key.is_empty() {
        return err_internal(msg.clone(), "Invalid share data");
    }

    // Increment access count
    let mut upd = HashMap::new();
    upd.insert("access_count".to_string(), serde_json::json!(access_count + 1));
    if let Err(e) = db::update(ctx, SHARES_COLLECTION, &share.id, upd) {
        tracing::warn!("Failed to increment share access count: {e}");
    }

    // Log access
    let mut log_data = HashMap::new();
    log_data.insert("share_id".to_string(), serde_json::Value::String(share.id.clone()));
    log_data.insert("accessed_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    log_data.insert("ip_address".to_string(), serde_json::Value::String(msg.remote_addr().to_string()));
    log_data.insert("user_agent".to_string(), serde_json::Value::String(msg.header("User-Agent").to_string()));
    if let Err(e) = db::create(ctx, ACCESS_LOGS_COLLECTION, log_data) {
        tracing::warn!("Failed to log share access: {e}");
    }

    // Serve the file
    match store::get(ctx, bucket, key) {
        Ok((data, info)) => {
            ResponseBuilder::new(msg.clone(), 200)
                .set_header("Content-Disposition", &format!("inline; filename=\"{}\"", key))
                .set_header("Cache-Control", "private, max-age=3600")
                .body(data, &info.content_type)
        }
        Err(e) if e.code == "not_found" => err_not_found(msg.clone(), "File not found"),
        Err(e) => err_internal(msg.clone(), &format!("Storage error: {e}")),
    }
}
