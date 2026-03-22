use std::collections::HashMap;

use crate::wafer::block_world::types::*;
use crate::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::ListOptions;

const COLLECTION: &str = "settings";

pub fn handle(msg: &Message) -> BlockResult {
    let action = msg_get_meta(msg, "req.action");
    let path = msg_get_meta(msg, "req.resource");

    match (action, path) {
        ("retrieve", "/admin/settings") | ("retrieve", "/settings") => handle_list(msg),
        ("retrieve", p) if p.starts_with("/admin/settings/") || p.starts_with("/settings/") => handle_get(msg),
        ("update", p) if p.starts_with("/admin/settings/") => handle_set(msg),
        ("create", "/admin/settings") => handle_set_batch(msg),
        _ => err_not_found(msg, "not found"),
    }
}

fn handle_list(msg: &Message) -> BlockResult {
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db::list(COLLECTION, &opts) {
        Ok(result) => {
            // Convert to key-value map
            let mut settings = HashMap::new();
            for record in &result.records {
                let key = str_field(record, "key");
                let value = record.data.get("value").cloned().unwrap_or(serde_json::Value::Null);
                if !key.is_empty() {
                    settings.insert(key.to_string(), value);
                }
            }
            json_respond(msg, &serde_json::to_value(&settings).unwrap_or_default())
        }
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_get(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let key = path.strip_prefix("/admin/settings/")
        .or_else(|| path.strip_prefix("/settings/"))
        .unwrap_or("");
    if key.is_empty() { return err_bad_request(msg, "Missing setting key"); }

    match db::get_by_field(COLLECTION, "key", serde_json::Value::String(key.to_string())) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => {
            let we = convert_error(e);
            if we.code == ErrorCode::NotFound {
                err_not_found(msg, "Setting not found")
            } else {
                err_internal(msg, &format!("Database error: {}", we.message))
            }
        }
    }
}

fn handle_set(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let key = path.strip_prefix("/admin/settings/").unwrap_or("");
    if key.is_empty() { return err_bad_request(msg, "Missing setting key"); }

    #[derive(serde::Deserialize)]
    struct Req { value: serde_json::Value }
    let body: Req = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let mut data = json_map(serde_json::json!({
        "key": key,
        "value": body.value,
        "updated_by": msg_user_id(msg)
    }));
    stamp_updated(&mut data);

    match db::upsert(COLLECTION, "key", serde_json::Value::String(key.to_string()), data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_set_batch(msg: &Message) -> BlockResult {
    let body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let now = now_rfc3339();
    let user_id = msg_user_id(msg).to_string();

    for (key, value) in &body {
        let data = json_map(serde_json::json!({
            "key": key,
            "value": value,
            "updated_at": now,
            "updated_by": user_id
        }));
        let _ = db::upsert(COLLECTION, "key", serde_json::Value::String(key.clone()), data);
    }

    json_respond(msg, &serde_json::json!({"updated": body.len()}))
}
