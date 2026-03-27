use std::collections::HashMap;

use crate::wafer::block_world::types::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, SortField, Record};

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

pub fn msg_get_query<'a>(msg: &'a Message, name: &str) -> &'a str {
    let key = format!("req.query.{}", name);
    msg.meta
        .iter()
        .find(|e| e.key == key)
        .map(|e| e.value.as_str())
        .unwrap_or("")
}

pub fn pagination_params(msg: &Message, default_page_size: usize) -> (usize, usize) {
    let page: usize = msg_get_query(msg, "page").parse().unwrap_or(1).max(1);
    let page_size: usize = msg_get_query(msg, "page_size")
        .parse()
        .unwrap_or(default_page_size)
        .min(100);
    (page, page_size)
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

pub fn i64_field(record: &Record, key: &str) -> i64 {
    record
        .data
        .get(key)
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
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

// ---------------------------------------------------------------------------
// Paginated list helper
// ---------------------------------------------------------------------------

pub fn paginated_list(
    collection: &str,
    page: i64,
    page_size: i64,
    filters: Vec<Filter>,
    sort: Vec<SortField>,
) -> Result<db::RecordList, wafer_block::WaferError> {
    db::paginated_list(collection, page, page_size, filters, sort)
}

// ---------------------------------------------------------------------------
// Plan / activation helpers
// ---------------------------------------------------------------------------

const SUBSCRIPTIONS_COLLECTION: &str = "subscriptions";

/// Map a plan name to the maximum number of active deployments allowed.
fn plan_max_active(plan: &str) -> i64 {
    match plan {
        "starter" => 2,
        "pro" | "platform" => i64::MAX,
        _ => 0, // "free" or unknown
    }
}

/// Look up the user's active subscription and return the plan name.
pub fn get_user_plan(user_id: &str) -> String {
    let filters = vec![
        Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        },
        Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("active".to_string()),
        },
    ];
    match db::list_all(SUBSCRIPTIONS_COLLECTION, filters) {
        Ok(records) if !records.is_empty() => {
            str_field(&records[0], "plan").to_string()
        }
        _ => "free".to_string(),
    }
}

/// Count the user's active deployments in the given collection.
pub fn count_active_deployments(collection: &str, user_id: &str) -> i64 {
    let filters = vec![
        Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        },
        Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("active".to_string()),
        },
    ];
    db::count(collection, &filters).unwrap_or(0)
}

/// Activation capacity for a user: plan name, max allowed, current active count.
pub struct ActivationCapacity {
    pub plan: String,
    pub max_active: i64,
    pub active_count: i64,
}

/// Compute how many more deployments the user can activate under their current plan.
pub fn get_activation_capacity(collection: &str, user_id: &str) -> ActivationCapacity {
    let plan = get_user_plan(user_id);
    let max_active = plan_max_active(&plan);
    let active_count = count_active_deployments(collection, user_id);
    ActivationCapacity { plan, max_active, active_count }
}

/// After creating a deployment, auto-activate it if the user's plan allows.
/// Returns true if the deployment was activated.
pub fn activate_if_allowed(collection: &str, user_id: &str, record_id: &str) -> bool {
    let cap = get_activation_capacity(collection, user_id);
    if cap.active_count < cap.max_active {
        let mut data = std::collections::HashMap::new();
        data.insert("status".to_string(), serde_json::Value::String("active".to_string()));
        data.insert("updated_at".to_string(), serde_json::Value::String(now_rfc3339()));
        let _ = db::update(collection, record_id, data);
        true
    } else {
        false
    }
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

pub fn err_forbidden(msg: &Message, message: &str) -> BlockResult {
    error_response(msg, ErrorCode::PermissionDenied, message)
}

pub fn err_conflict(msg: &Message, message: &str) -> BlockResult {
    error_response(msg, ErrorCode::AlreadyExists, message)
}
