use std::collections::HashMap;
use std::time::Duration;

use crate::wafer::block_world::types::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, Record};
use wafer_core::clients::crypto;

use super::{TOKENS_COLLECTION, USER_ROLES_COLLECTION};

// ---------------------------------------------------------------------------
// Type conversion: wafer_block types → local WIT types
// ---------------------------------------------------------------------------

fn convert_error(e: wafer_block::WaferError) -> WaferError {
    WaferError {
        code: convert_error_code(e.code),
        message: e.message,
        meta: e.meta.into_iter().map(|m| MetaEntry { key: m.key, value: m.value }).collect(),
    }
}

fn convert_error_code(c: wafer_block::ErrorCode) -> ErrorCode {
    match c {
        wafer_block::ErrorCode::Ok => ErrorCode::Ok,
        wafer_block::ErrorCode::Cancelled => ErrorCode::Cancelled,
        wafer_block::ErrorCode::Unknown => ErrorCode::Unknown,
        wafer_block::ErrorCode::InvalidArgument => ErrorCode::InvalidArgument,
        wafer_block::ErrorCode::DeadlineExceeded => ErrorCode::DeadlineExceeded,
        wafer_block::ErrorCode::NotFound => ErrorCode::NotFound,
        wafer_block::ErrorCode::AlreadyExists => ErrorCode::AlreadyExists,
        wafer_block::ErrorCode::PermissionDenied => ErrorCode::PermissionDenied,
        wafer_block::ErrorCode::ResourceExhausted => ErrorCode::ResourceExhausted,
        wafer_block::ErrorCode::FailedPrecondition => ErrorCode::FailedPrecondition,
        wafer_block::ErrorCode::Aborted => ErrorCode::Aborted,
        wafer_block::ErrorCode::OutOfRange => ErrorCode::OutOfRange,
        wafer_block::ErrorCode::Unimplemented => ErrorCode::Unimplemented,
        wafer_block::ErrorCode::Internal => ErrorCode::Internal,
        wafer_block::ErrorCode::Unavailable => ErrorCode::Unavailable,
        wafer_block::ErrorCode::DataLoss => ErrorCode::DataLoss,
        wafer_block::ErrorCode::Unauthenticated => ErrorCode::Unauthenticated,
    }
}

// ---------------------------------------------------------------------------
// Message helpers
// ---------------------------------------------------------------------------

pub fn msg_get_meta<'a>(msg: &'a Message, key: &str) -> &'a str {
    msg.meta
        .iter()
        .find(|e| e.key == key)
        .map(|e| e.value.as_str())
        .unwrap_or("")
}

pub fn decode_body<T: serde::de::DeserializeOwned>(msg: &Message) -> Result<T, BlockResult> {
    serde_json::from_slice(&msg.data).map_err(|e| {
        error_response(msg, ErrorCode::InvalidArgument, &format!("Invalid body: {e}"))
    })
}

// ---------------------------------------------------------------------------
// Record field access
// ---------------------------------------------------------------------------

pub fn str_field<'a>(record: &'a Record, key: &str) -> &'a str {
    record
        .data
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
}

pub fn bool_field(record: &Record, key: &str) -> bool {
    record
        .data
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Data helpers
// ---------------------------------------------------------------------------

pub fn json_map(val: serde_json::Value) -> HashMap<String, serde_json::Value> {
    match val {
        serde_json::Value::Object(map) => map.into_iter().collect(),
        _ => HashMap::new(),
    }
}

pub fn now_rfc3339() -> String {
    // In WASM Components, there's no system clock. Timestamps are set by
    // the database layer or the host runtime. Return empty string as a
    // sentinel; the database block fills in server-side timestamps.
    String::new()
}

pub fn stamp_created(data: &mut HashMap<String, serde_json::Value>) {
    let now = now_rfc3339();
    data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now));
}

pub fn stamp_updated(data: &mut HashMap<String, serde_json::Value>) {
    data.insert("updated_at".to_string(), serde_json::Value::String(now_rfc3339()));
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------------
// Auth-specific helpers
// ---------------------------------------------------------------------------

pub fn get_user_roles(user_id: &str) -> Vec<String> {
    let opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        }],
        ..Default::default()
    };
    match db::list(USER_ROLES_COLLECTION, &opts) {
        Ok(r) => r.records.iter()
            .filter_map(|rec| rec.data.get("role").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect(),
        Err(_) => Vec::new(),
    }
}

pub fn generate_tokens(
    user_id: &str,
    email: &str,
    roles: &[String],
) -> Result<(String, String), BlockResult> {
    let family = match crypto::random_bytes(16) {
        Ok(bytes) => hex_encode(&bytes),
        Err(e) => return Err(BlockResult {
            action: Action::Error,
            error: Some(convert_error(e)),
            response: None,
            message: None,
        }),
    };

    let mut access_claims = HashMap::new();
    access_claims.insert("user_id".to_string(), serde_json::json!(user_id));
    access_claims.insert("sub".to_string(), serde_json::json!(user_id));
    access_claims.insert("email".to_string(), serde_json::json!(email));
    access_claims.insert("roles".to_string(), serde_json::json!(roles));
    access_claims.insert("type".to_string(), serde_json::json!("access"));

    let access_token = crypto::sign(&access_claims, Duration::from_secs(86400))
        .map_err(|e| BlockResult { action: Action::Error, error: Some(convert_error(e)), response: None, message: None })?;

    let mut refresh_claims = HashMap::new();
    refresh_claims.insert("user_id".to_string(), serde_json::json!(user_id));
    refresh_claims.insert("sub".to_string(), serde_json::json!(user_id));
    refresh_claims.insert("type".to_string(), serde_json::json!("refresh"));
    refresh_claims.insert("family".to_string(), serde_json::json!(family));

    let refresh_token = crypto::sign(&refresh_claims, Duration::from_secs(604800))
        .map_err(|e| BlockResult { action: Action::Error, error: Some(convert_error(e)), response: None, message: None })?;

    Ok((access_token, refresh_token))
}

pub fn store_refresh_token(user_id: &str, token: &str) {
    let data = json_map(serde_json::json!({
        "user_id": user_id,
        "token": token,
        "created_at": now_rfc3339()
    }));
    let _ = db::create(TOKENS_COLLECTION, data);
}

pub fn build_auth_cookie(token: &str, max_age: u64) -> String {
    let env = wafer_core::clients::config::get_default("ENVIRONMENT", "development");
    let secure = env == "production";
    format!(
        "auth_token={}; HttpOnly; Path=/; SameSite=Lax; Max-Age={}{}",
        token, max_age, if secure { "; Secure" } else { "" }
    )
}

// ---------------------------------------------------------------------------
// Response builders (using local WIT types)
// ---------------------------------------------------------------------------

pub fn json_respond(msg: &Message, data: &serde_json::Value) -> BlockResult {
    match serde_json::to_vec(data) {
        Ok(body) => BlockResult {
            action: Action::Respond,
            response: Some(Response {
                data: body,
                meta: vec![MetaEntry {
                    key: "resp.content_type".to_string(),
                    value: "application/json".to_string(),
                }],
            }),
            error: None,
            message: Some(msg.clone()),
        },
        Err(e) => err_internal(msg, &e.to_string()),
    }
}

pub fn respond_with_cookie(msg: &Message, cookie: &str, data: &serde_json::Value) -> BlockResult {
    match serde_json::to_vec(data) {
        Ok(body) => BlockResult {
            action: Action::Respond,
            response: Some(Response {
                data: body,
                meta: vec![
                    MetaEntry { key: "resp.content_type".to_string(), value: "application/json".to_string() },
                    MetaEntry { key: "resp.cookie.0".to_string(), value: cookie.to_string() },
                ],
            }),
            error: None,
            message: Some(msg.clone()),
        },
        Err(e) => err_internal(msg, &e.to_string()),
    }
}

pub fn respond_with_status_and_cookie(msg: &Message, status: u16, cookie: &str, data: &serde_json::Value) -> BlockResult {
    match serde_json::to_vec(data) {
        Ok(body) => BlockResult {
            action: Action::Respond,
            response: Some(Response {
                data: body,
                meta: vec![
                    MetaEntry { key: "resp.content_type".to_string(), value: "application/json".to_string() },
                    MetaEntry { key: "resp.status".to_string(), value: status.to_string() },
                    MetaEntry { key: "resp.cookie.0".to_string(), value: cookie.to_string() },
                ],
            }),
            error: None,
            message: Some(msg.clone()),
        },
        Err(e) => err_internal(msg, &e.to_string()),
    }
}

pub fn error_response(msg: &Message, code: ErrorCode, message: &str) -> BlockResult {
    BlockResult {
        action: Action::Error,
        error: Some(WaferError { code, message: message.to_string(), meta: Vec::new() }),
        response: None,
        message: Some(msg.clone()),
    }
}

pub fn err_not_found(msg: &Message, message: &str) -> BlockResult {
    error_response(msg, ErrorCode::NotFound, message)
}

pub fn err_internal(msg: &Message, message: &str) -> BlockResult {
    error_response(msg, ErrorCode::Internal, message)
}

pub fn respond_with_status_and_cookie_and_header(
    msg: &Message, status: u16, cookie: &str, header_key: &str, header_val: &str, data: &serde_json::Value,
) -> BlockResult {
    match serde_json::to_vec(data) {
        Ok(body) => BlockResult {
            action: Action::Respond,
            response: Some(Response {
                data: body,
                meta: vec![
                    MetaEntry { key: "resp.content_type".to_string(), value: "application/json".to_string() },
                    MetaEntry { key: "resp.status".to_string(), value: status.to_string() },
                    MetaEntry { key: "resp.cookie.0".to_string(), value: cookie.to_string() },
                    MetaEntry { key: format!("resp.header.{}", header_key), value: header_val.to_string() },
                ],
            }),
            error: None,
            message: Some(msg.clone()),
        },
        Err(e) => err_internal(msg, &e.to_string()),
    }
}

pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(data);
    hex_encode(&hash)
}

pub fn urlencode(s: &str) -> String {
    s.as_bytes().iter().map(|&b| match b {
        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
            String::from(b as char)
        }
        _ => format!("%{:02X}", b),
    }).collect()
}
