use std::collections::HashMap;
use crate::wafer::block_world::types::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use crate::helpers::*;

const COLLECTION: &str = "block_products_variables";

pub fn handle_list(msg: &Message) -> BlockResult {
    let mut filters = Vec::new();
    let scope = msg_query(msg, "scope");
    if !scope.is_empty() {
        filters.push(Filter { field: "scope".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(scope.to_string()) });
    }
    let product_id = msg_query(msg, "product_id");
    if !product_id.is_empty() {
        filters.push(Filter { field: "product_id".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(product_id.to_string()) });
    }

    let opts = ListOptions {
        filters,
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };

    match db::list(COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub fn handle_create(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct Req {
        name: String,
        var_type: Option<String>,
        default_value: Option<serde_json::Value>,
        scope: Option<String>,
        product_id: Option<String>,
    }
    let body: Req = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let mut data = HashMap::new();
    data.insert("name".to_string(), serde_json::Value::String(body.name));
    data.insert("var_type".to_string(), serde_json::Value::String(body.var_type.unwrap_or_else(|| "number".to_string())));
    data.insert("scope".to_string(), serde_json::Value::String(body.scope.unwrap_or_else(|| "system".to_string())));
    if let Some(default) = body.default_value {
        data.insert("default_value".to_string(), default);
    }
    if let Some(pid) = body.product_id {
        data.insert("product_id".to_string(), serde_json::Value::String(pid));
    }
    stamp_created(&mut data);

    match db::create(COLLECTION, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub fn handle_update(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/admin/b/products/variables/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing variable ID"); }

    let mut body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    stamp_updated(&mut body);

    match db::update(COLLECTION, id, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Variable not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub fn handle_delete(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/admin/b/products/variables/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing variable ID"); }
    match db::delete(COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Variable not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}
