use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use super::{PRODUCTS_COLLECTION, GROUPS_COLLECTION, TYPES_COLLECTION, PRICING_COLLECTION, PURCHASES_COLLECTION};
use crate::blocks::helpers::{RecordExt, field_as_string};

pub async fn handle_admin(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        // Products
        ("retrieve", "/admin/b/products/products") => handle_list_products(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/admin/b/products/products/") => handle_get_product(ctx, msg).await,
        ("create", "/admin/b/products/products") => handle_create_product(ctx, msg).await,
        ("update", _) if path.starts_with("/admin/b/products/products/") => handle_update_product(ctx, msg).await,
        ("delete", _) if path.starts_with("/admin/b/products/products/") => handle_delete_product(ctx, msg).await,
        // Groups
        ("retrieve", "/admin/b/products/groups") => handle_list_groups(ctx, msg).await,
        ("create", "/admin/b/products/groups") => handle_create_group(ctx, msg).await,
        ("update", _) if path.starts_with("/admin/b/products/groups/") => handle_update_group(ctx, msg).await,
        ("delete", _) if path.starts_with("/admin/b/products/groups/") => handle_delete_group(ctx, msg).await,
        // Types
        ("retrieve", "/admin/b/products/types") => handle_list_types(ctx, msg).await,
        ("create", "/admin/b/products/types") => handle_create_type(ctx, msg).await,
        ("delete", _) if path.starts_with("/admin/b/products/types/") => handle_delete_type(ctx, msg).await,
        // Pricing templates
        ("retrieve", "/admin/b/products/pricing") => handle_list_pricing(ctx, msg).await,
        ("create", "/admin/b/products/pricing") => handle_create_pricing(ctx, msg).await,
        ("update", _) if path.starts_with("/admin/b/products/pricing/") => handle_update_pricing(ctx, msg).await,
        ("delete", _) if path.starts_with("/admin/b/products/pricing/") => handle_delete_pricing(ctx, msg).await,
        // Variables
        ("retrieve", "/admin/b/products/variables") => super::variables::handle_list(ctx, msg).await,
        ("create", "/admin/b/products/variables") => super::variables::handle_create(ctx, msg).await,
        ("update", _) if path.starts_with("/admin/b/products/variables/") => super::variables::handle_update(ctx, msg).await,
        ("delete", _) if path.starts_with("/admin/b/products/variables/") => super::variables::handle_delete(ctx, msg).await,
        // Purchases (admin view)
        ("retrieve", "/admin/b/products/purchases") => super::purchase::handle_list_admin(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/admin/b/products/purchases/") => super::purchase::handle_get(ctx, msg).await,
        ("update", _) if path.starts_with("/admin/b/products/purchases/") && path.ends_with("/refund") => {
            super::purchase::handle_refund(ctx, msg).await
        }
        // Stats
        ("retrieve", "/admin/b/products/stats") => handle_stats(ctx, msg).await,
        _ => err_not_found(msg, "not found"),
    }
}

pub async fn handle_user(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        // User's own products
        ("retrieve", "/b/products/products") => handle_user_list_products(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/b/products/products/") => handle_user_get_product(ctx, msg).await,
        ("create", "/b/products/products") => handle_user_create_product(ctx, msg).await,
        ("update", _) if path.starts_with("/b/products/products/") => handle_user_update_product(ctx, msg).await,
        ("delete", _) if path.starts_with("/b/products/products/") => handle_user_delete_product(ctx, msg).await,
        // User's own groups
        ("retrieve", "/b/products/groups") => handle_user_list_groups(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/b/products/groups/") && !path.ends_with("/products") => handle_user_get_group(ctx, msg).await,
        ("create", "/b/products/groups") => handle_user_create_group(ctx, msg).await,
        ("update", _) if path.starts_with("/b/products/groups/") && !path.ends_with("/products") => handle_user_update_group(ctx, msg).await,
        ("delete", _) if path.starts_with("/b/products/groups/") && !path.ends_with("/products") => handle_user_delete_group(ctx, msg).await,
        // Products in a group
        ("retrieve", _) if path.starts_with("/b/products/groups/") && path.ends_with("/products") => handle_user_group_products(ctx, msg).await,
        // Read-only: types and group templates
        ("retrieve", "/b/products/types") => handle_list_types(ctx, msg).await,
        ("retrieve", "/b/products/group-templates") => handle_user_list_group_templates(ctx, msg).await,
        // Catalog (public)
        ("retrieve", "/b/products/catalog") => handle_catalog(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/b/products/catalog/") => handle_get_product_public(ctx, msg).await,
        // Pricing, purchases, checkout
        ("create", "/b/products/calculate-price") => super::pricing::handle_calculate(ctx, msg).await,
        ("create", "/b/products/purchases") => super::purchase::handle_create(ctx, msg).await,
        ("retrieve", "/b/products/purchases") => super::purchase::handle_list_user(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/b/products/purchases/") => super::purchase::handle_get(ctx, msg).await,
        ("create", "/b/products/checkout") => super::stripe::handle_checkout(ctx, msg).await,
        _ => err_not_found(msg, "not found"),
    }
}

// --- Product CRUD ---

async fn handle_list_products(ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
    match db::paginated_list(ctx, PRODUCTS_COLLECTION, page as i64, page_size as i64, filters, sort).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_get_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }
    match db::get(ctx, PRODUCTS_COLLECTION, id).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Product not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_create_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    let mut data = body;
    let now = chrono::Utc::now().to_rfc3339();
    data.entry("status".to_string()).or_insert(serde_json::Value::String("draft".to_string()));
    data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now));
    data.insert("created_by".to_string(), serde_json::Value::String(msg.user_id().to_string()));

    match db::create(ctx, PRODUCTS_COLLECTION, data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_update_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }

    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    body.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    match db::update(ctx, PRODUCTS_COLLECTION, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Product not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_delete_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }
    match db::delete(ctx, PRODUCTS_COLLECTION, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Product not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Groups ---

async fn handle_list_groups(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions {
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, GROUPS_COLLECTION, &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_create_group(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    body.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    body.entry("user_id".to_string()).or_insert(serde_json::Value::String(msg.user_id().to_string()));
    match db::create(ctx, GROUPS_COLLECTION, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_update_group(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/b/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing group ID"); }
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    match db::update(ctx, GROUPS_COLLECTION, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Group not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_delete_group(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/b/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing group ID"); }
    match db::delete(ctx, GROUPS_COLLECTION, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Group not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Types ---

async fn handle_list_types(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db::list(ctx, TYPES_COLLECTION, &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_create_type(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    match db::create(ctx, TYPES_COLLECTION, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_delete_type(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/b/products/types/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing type ID"); }
    match db::delete(ctx, TYPES_COLLECTION, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Type not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Pricing Templates ---

async fn handle_list_pricing(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions {
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, PRICING_COLLECTION, &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_create_pricing(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    body.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    match db::create(ctx, PRICING_COLLECTION, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_update_pricing(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/b/products/pricing/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing pricing template ID"); }
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    match db::update(ctx, PRICING_COLLECTION, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Pricing template not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_delete_pricing(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/b/products/pricing/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing pricing template ID"); }
    match db::delete(ctx, PRICING_COLLECTION, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Pricing template not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Public catalog ---

async fn handle_catalog(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let (page, page_size, _) = msg.pagination_params(20);
    let filters = vec![Filter {
        field: "status".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String("active".to_string()),
    }];
    let sort = vec![SortField { field: "name".to_string(), desc: false }];
    match db::paginated_list(ctx, PRODUCTS_COLLECTION, page as i64, page_size as i64, filters, sort).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_get_product_public(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/b/products/catalog/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }

    match db::get(ctx, PRODUCTS_COLLECTION, id).await {
        Ok(record) => {
            let status = record.str_field("status");
            if status != "active" {
                return err_not_found(msg, "Product not found");
            }
            json_respond(msg, &record)
        }
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Product not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- User's own products ---

async fn handle_user_list_products(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() { return err_unauthorized(msg, "Not authenticated"); }

    let (page, page_size, _) = msg.pagination_params(20);
    let mut filters = vec![Filter {
        field: "created_by".to_string(), operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id),
    }];
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
    match db::paginated_list(ctx, PRODUCTS_COLLECTION, page as i64, page_size as i64, filters, sort).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_user_get_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    let path = msg.path();
    let id = path.strip_prefix("/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }

    match db::get(ctx, PRODUCTS_COLLECTION, id).await {
        Ok(record) => {
            if field_as_string(&record, "created_by") != user_id {
                return err_not_found(msg, "Product not found");
            }
            json_respond(msg, &record)
        }
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Product not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_user_create_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() { return err_unauthorized(msg, "Not authenticated"); }

    let mut data: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    // Verify user owns the group (if provided)
    let group_id_str = data.get("group_id").and_then(|v| v.as_str().map(|s| s.to_string()))
        .or_else(|| data.get("group_id").and_then(|v| v.as_i64().map(|n| n.to_string())))
        .unwrap_or_default();
    if !group_id_str.is_empty() {
        match db::get(ctx, GROUPS_COLLECTION, &group_id_str).await {
            Ok(group) => {
                if field_as_string(&group, "user_id") != user_id {
                    return err_bad_request(msg, "You don't own this group");
                }
            }
            Err(_) => return err_bad_request(msg, "Group not found"),
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    data.entry("status".to_string()).or_insert(serde_json::Value::String("draft".to_string()));
    data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now));
    data.insert("created_by".to_string(), serde_json::Value::String(user_id));
    // Default product_template_id to the seeded template (id=1) if not provided
    data.entry("product_template_id".to_string()).or_insert(serde_json::json!(1));

    match db::create(ctx, PRODUCTS_COLLECTION, data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_user_update_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    let path = msg.path();
    let id = path.strip_prefix("/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }

    // Verify ownership
    match db::get(ctx, PRODUCTS_COLLECTION, id).await {
        Ok(record) => {
            if field_as_string(&record, "created_by") != user_id {
                return err_not_found(msg, "Product not found");
            }
        }
        Err(e) if e.code == ErrorCode::NotFound => return err_not_found(msg, "Product not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    }

    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    body.remove("created_by"); // prevent ownership change
    body.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    match db::update(ctx, PRODUCTS_COLLECTION, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_user_delete_product(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    let path = msg.path();
    let id = path.strip_prefix("/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }

    // Verify ownership
    match db::get(ctx, PRODUCTS_COLLECTION, id).await {
        Ok(record) => {
            if field_as_string(&record, "created_by") != user_id {
                return err_not_found(msg, "Product not found");
            }
        }
        Err(e) if e.code == ErrorCode::NotFound => return err_not_found(msg, "Product not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    }

    match db::delete(ctx, PRODUCTS_COLLECTION, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- User's own groups ---

async fn handle_user_list_groups(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() { return err_unauthorized(msg, "Not authenticated"); }

    let opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".to_string(), operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        }],
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, GROUPS_COLLECTION, &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_user_get_group(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    let path = msg.path();
    let id = path.strip_prefix("/b/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing group ID"); }

    match db::get(ctx, GROUPS_COLLECTION, id).await {
        Ok(record) => {
            if field_as_string(&record, "user_id") != user_id {
                return err_not_found(msg, "Group not found");
            }
            json_respond(msg, &record)
        }
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Group not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_user_create_group(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() { return err_unauthorized(msg, "Not authenticated"); }

    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    body.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    body.insert("user_id".to_string(), serde_json::Value::String(user_id));
    // Default group_template_id to the seeded template (id=1) if not provided
    body.entry("group_template_id".to_string()).or_insert(serde_json::json!(1));

    match db::create(ctx, GROUPS_COLLECTION, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_user_update_group(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    let path = msg.path();
    let id = path.strip_prefix("/b/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing group ID"); }

    // Verify ownership
    match db::get(ctx, GROUPS_COLLECTION, id).await {
        Ok(record) => {
            if field_as_string(&record, "user_id") != user_id {
                return err_not_found(msg, "Group not found");
            }
        }
        Err(e) if e.code == ErrorCode::NotFound => return err_not_found(msg, "Group not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    }

    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    body.remove("user_id"); // prevent ownership change

    match db::update(ctx, GROUPS_COLLECTION, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_user_delete_group(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    let path = msg.path();
    let id = path.strip_prefix("/b/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing group ID"); }

    // Verify ownership
    match db::get(ctx, GROUPS_COLLECTION, id).await {
        Ok(record) => {
            if field_as_string(&record, "user_id") != user_id {
                return err_not_found(msg, "Group not found");
            }
        }
        Err(e) if e.code == ErrorCode::NotFound => return err_not_found(msg, "Group not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    }

    match db::delete(ctx, GROUPS_COLLECTION, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// Products in a user's group
async fn handle_user_group_products(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    let path = msg.path();
    // Path: /b/products/groups/{id}/products
    let rest = path.strip_prefix("/b/products/groups/").unwrap_or("");
    let group_id = rest.strip_suffix("/products").unwrap_or("");
    if group_id.is_empty() { return err_bad_request(msg, "Missing group ID"); }

    // Verify group ownership
    match db::get(ctx, GROUPS_COLLECTION, group_id).await {
        Ok(record) => {
            if field_as_string(&record, "user_id") != user_id {
                return err_not_found(msg, "Group not found");
            }
        }
        Err(_) => return err_not_found(msg, "Group not found"),
    }

    let (page, page_size, _) = msg.pagination_params(20);
    let filters = vec![Filter {
        field: "group_id".to_string(), operator: FilterOp::Equal,
        value: serde_json::Value::String(group_id.to_string()),
    }];
    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];
    match db::paginated_list(ctx, PRODUCTS_COLLECTION, page as i64, page_size as i64, filters, sort).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// User-accessible group templates (read-only)
async fn handle_user_list_group_templates(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db::list(ctx, super::GROUP_TEMPLATES_COLLECTION, &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Stats ---

async fn handle_stats(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let total_products = db::count(ctx, PRODUCTS_COLLECTION, &[]).await.unwrap_or(0);
    let active_products = db::count(ctx, PRODUCTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("active".to_string()),
    }]).await.unwrap_or(0);
    let total_purchases = db::count(ctx, PURCHASES_COLLECTION, &[]).await.unwrap_or(0);
    let total_revenue = db::sum(ctx, PURCHASES_COLLECTION, "total_cents", &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("completed".to_string()),
    }]).await.unwrap_or(0.0);
    let total_groups = db::count(ctx, GROUPS_COLLECTION, &[]).await.unwrap_or(0);

    json_respond(msg, &serde_json::json!({
        "total_products": total_products,
        "active_products": active_products,
        "total_purchases": total_purchases,
        "total_revenue": total_revenue,
        "total_groups": total_groups
    }))
}
