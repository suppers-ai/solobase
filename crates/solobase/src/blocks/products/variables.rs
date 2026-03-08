use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};

const COLLECTION: &str = "ext_products_variables";

pub async fn handle_list(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let mut filters = Vec::new();
    let scope = msg.query("scope").to_string();
    if !scope.is_empty() {
        filters.push(Filter { field: "scope".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(scope) });
    }
    let product_id = msg.query("product_id").to_string();
    if !product_id.is_empty() {
        filters.push(Filter { field: "product_id".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(product_id) });
    }

    let opts = ListOptions {
        filters,
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };

    match db::list(ctx, COLLECTION, &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub async fn handle_create(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    #[derive(serde::Deserialize)]
    struct Req {
        name: String,
        var_type: Option<String>,
        default_value: Option<serde_json::Value>,
        scope: Option<String>,
        product_id: Option<String>,
    }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
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
    data.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    match db::create(ctx, COLLECTION, data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub async fn handle_update(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/ext/products/variables/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing variable ID"); }

    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    body.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    match db::update(ctx, COLLECTION, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == "not_found" => err_not_found(msg, "Variable not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub async fn handle_delete(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/ext/products/variables/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing variable ID"); }
    match db::delete(ctx, COLLECTION, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == "not_found" => err_not_found(msg, "Variable not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}
