use std::collections::HashMap;

use crate::wafer::block_world::types::*;
use wafer_core::clients::database::Record;

// ---------------------------------------------------------------------------
// Type conversion: wafer_block types -> local WIT types
// ---------------------------------------------------------------------------

pub fn convert_error(e: wafer_block::WaferError) -> WaferError {
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

pub fn respond_html(msg: &Message, html: Vec<u8>) -> BlockResult {
    BlockResult {
        action: Action::Respond,
        response: Some(Response {
            data: html,
            meta: vec![MetaEntry {
                key: "resp.content_type".to_string(),
                value: "text/html; charset=utf-8".to_string(),
            }],
        }),
        error: None,
        message: Some(msg.clone()),
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

pub fn err_bad_request(msg: &Message, message: &str) -> BlockResult {
    error_response(msg, ErrorCode::InvalidArgument, message)
}
