use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use super::{PRODUCTS_COLLECTION, GROUPS_COLLECTION, TYPES_COLLECTION, PRICING_COLLECTION};

pub fn handle_admin(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        // Products
        ("retrieve", "/admin/ext/products/products") => handle_list_products(ctx, msg),
        ("retrieve", _) if path.starts_with("/admin/ext/products/products/") => handle_get_product(ctx, msg),
        ("create", "/admin/ext/products/products") => handle_create_product(ctx, msg),
        ("update", _) if path.starts_with("/admin/ext/products/products/") => handle_update_product(ctx, msg),
        ("delete", _) if path.starts_with("/admin/ext/products/products/") => handle_delete_product(ctx, msg),
        // Groups
        ("retrieve", "/admin/ext/products/groups") => handle_list_groups(ctx, msg),
        ("create", "/admin/ext/products/groups") => handle_create_group(ctx, msg),
        ("update", _) if path.starts_with("/admin/ext/products/groups/") => handle_update_group(ctx, msg),
        ("delete", _) if path.starts_with("/admin/ext/products/groups/") => handle_delete_group(ctx, msg),
        // Types
        ("retrieve", "/admin/ext/products/types") => handle_list_types(ctx, msg),
        ("create", "/admin/ext/products/types") => handle_create_type(ctx, msg),
        ("delete", _) if path.starts_with("/admin/ext/products/types/") => handle_delete_type(ctx, msg),
        // Pricing templates
        ("retrieve", "/admin/ext/products/pricing") => handle_list_pricing(ctx, msg),
        ("create", "/admin/ext/products/pricing") => handle_create_pricing(ctx, msg),
        ("update", _) if path.starts_with("/admin/ext/products/pricing/") => handle_update_pricing(ctx, msg),
        ("delete", _) if path.starts_with("/admin/ext/products/pricing/") => handle_delete_pricing(ctx, msg),
        // Variables
        ("retrieve", "/admin/ext/products/variables") => super::variables::handle_list(ctx, msg),
        ("create", "/admin/ext/products/variables") => super::variables::handle_create(ctx, msg),
        ("update", _) if path.starts_with("/admin/ext/products/variables/") => super::variables::handle_update(ctx, msg),
        ("delete", _) if path.starts_with("/admin/ext/products/variables/") => super::variables::handle_delete(ctx, msg),
        // Purchases (admin view)
        ("retrieve", "/admin/ext/products/purchases") => super::purchase::handle_list_admin(ctx, msg),
        ("retrieve", _) if path.starts_with("/admin/ext/products/purchases/") => super::purchase::handle_get(ctx, msg),
        ("update", _) if path.starts_with("/admin/ext/products/purchases/") && path.ends_with("/refund") => {
            super::purchase::handle_refund(ctx, msg)
        }
        // Stats
        ("retrieve", "/admin/ext/products/stats") => handle_stats(ctx, msg),
        _ => err_not_found(msg.clone(), "not found"),
    }
}

pub fn handle_user(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/ext/products/catalog") => handle_catalog(ctx, msg),
        ("retrieve", _) if path.starts_with("/ext/products/catalog/") => handle_get_product_public(ctx, msg),
        ("create", "/ext/products/calculate-price") => super::pricing::handle_calculate(ctx, msg),
        ("create", "/ext/products/purchases") => super::purchase::handle_create(ctx, msg),
        ("retrieve", "/ext/products/purchases") => super::purchase::handle_list_user(ctx, msg),
        ("retrieve", _) if path.starts_with("/ext/products/purchases/") => super::purchase::handle_get(ctx, msg),
        ("create", "/ext/products/checkout") => super::stripe::handle_checkout(ctx, msg),
        _ => err_not_found(msg.clone(), "not found"),
    }
}

// --- Product CRUD ---

fn handle_list_products(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let (page, page_size, _) = msg.pagination_params(20);

    let mut filters = Vec::new();
    let group_id = msg.query("group_id").to_string();
    if !group_id.is_empty() {
        filters.push(Filter { field: "group_id".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(group_id) });
    }
    let status = msg.query("status").to_string();
    if !status.is_empty() {
        filters.push(Filter { field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(status) });
    }
    let search = msg.query("search").to_string();
    if !search.is_empty() {
        filters.push(Filter { field: "name".to_string(), operator: FilterOp::Like, value: serde_json::Value::String(format!("%{}%", search)) });
    }

    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];
    match db::paginated_list(ctx, PRODUCTS_COLLECTION, page as i64, page_size as i64, filters, sort) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_get_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/ext/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing product ID"); }
    match db::get(ctx, PRODUCTS_COLLECTION, id) {
        Ok(record) => json_respond(msg.clone(), 200, &record),
        Err(e) if e.code == "not_found" => err_not_found(msg.clone(), "Product not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_create_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };

    let mut data = body;
    let now = chrono::Utc::now().to_rfc3339();
    data.entry("status".to_string()).or_insert(serde_json::Value::String("draft".to_string()));
    data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now));
    data.insert("created_by".to_string(), serde_json::Value::String(msg.user_id().to_string()));

    match db::create(ctx, PRODUCTS_COLLECTION, data) {
        Ok(record) => json_respond(msg.clone(), 201, &record),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_update_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/ext/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing product ID"); }

    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };
    body.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    match db::update(ctx, PRODUCTS_COLLECTION, id, body) {
        Ok(record) => json_respond(msg.clone(), 200, &record),
        Err(e) if e.code == "not_found" => err_not_found(msg.clone(), "Product not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_delete_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/ext/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing product ID"); }
    match db::delete(ctx, PRODUCTS_COLLECTION, id) {
        Ok(()) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == "not_found" => err_not_found(msg.clone(), "Product not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

// --- Groups ---

fn handle_list_groups(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions {
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, GROUPS_COLLECTION, &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_create_group(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };
    body.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    match db::create(ctx, GROUPS_COLLECTION, body) {
        Ok(record) => json_respond(msg.clone(), 201, &record),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_update_group(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/ext/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing group ID"); }
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };
    match db::update(ctx, GROUPS_COLLECTION, id, body) {
        Ok(record) => json_respond(msg.clone(), 200, &record),
        Err(e) if e.code == "not_found" => err_not_found(msg.clone(), "Group not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_delete_group(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/ext/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing group ID"); }
    match db::delete(ctx, GROUPS_COLLECTION, id) {
        Ok(()) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == "not_found" => err_not_found(msg.clone(), "Group not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

// --- Types ---

fn handle_list_types(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db::list(ctx, TYPES_COLLECTION, &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_create_type(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };
    match db::create(ctx, TYPES_COLLECTION, body) {
        Ok(record) => json_respond(msg.clone(), 201, &record),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_delete_type(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/ext/products/types/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing type ID"); }
    match db::delete(ctx, TYPES_COLLECTION, id) {
        Ok(()) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == "not_found" => err_not_found(msg.clone(), "Type not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

// --- Pricing Templates ---

fn handle_list_pricing(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions {
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, PRICING_COLLECTION, &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_create_pricing(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };
    body.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    match db::create(ctx, PRICING_COLLECTION, body) {
        Ok(record) => json_respond(msg.clone(), 201, &record),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_update_pricing(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/ext/products/pricing/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing pricing template ID"); }
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };
    match db::update(ctx, PRICING_COLLECTION, id, body) {
        Ok(record) => json_respond(msg.clone(), 200, &record),
        Err(e) if e.code == "not_found" => err_not_found(msg.clone(), "Pricing template not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_delete_pricing(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/ext/products/pricing/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing pricing template ID"); }
    match db::delete(ctx, PRICING_COLLECTION, id) {
        Ok(()) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == "not_found" => err_not_found(msg.clone(), "Pricing template not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

// --- Public catalog ---

fn handle_catalog(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let (page, page_size, _) = msg.pagination_params(20);
    let filters = vec![Filter {
        field: "status".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String("active".to_string()),
    }];
    let sort = vec![SortField { field: "name".to_string(), desc: false }];
    match db::paginated_list(ctx, PRODUCTS_COLLECTION, page as i64, page_size as i64, filters, sort) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_get_product_public(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/ext/products/catalog/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing product ID"); }

    match db::get(ctx, PRODUCTS_COLLECTION, id) {
        Ok(record) => {
            let status = record.data.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if status != "active" {
                return err_not_found(msg.clone(), "Product not found");
            }
            json_respond(msg.clone(), 200, &record)
        }
        Err(e) if e.code == "not_found" => err_not_found(msg.clone(), "Product not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

// --- Stats ---

fn handle_stats(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let total_products = db::count(ctx, PRODUCTS_COLLECTION, &[]).unwrap_or(0);
    let active_products = db::count(ctx, PRODUCTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("active".to_string()),
    }]).unwrap_or(0);
    let total_purchases = db::count(ctx, "ext_products_purchases", &[]).unwrap_or(0);
    let total_revenue = db::sum(ctx, "ext_products_purchases", "total_amount", &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("completed".to_string()),
    }]).unwrap_or(0.0);
    let total_groups = db::count(ctx, GROUPS_COLLECTION, &[]).unwrap_or(0);

    json_respond(msg.clone(), 200, &serde_json::json!({
        "total_products": total_products,
        "active_products": active_products,
        "total_purchases": total_purchases,
        "total_revenue": total_revenue,
        "total_groups": total_groups
    }))
}
