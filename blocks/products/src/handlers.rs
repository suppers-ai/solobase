use std::collections::HashMap;
use crate::wafer::block_world::types::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use crate::helpers::*;
use crate::{
    PRODUCTS_COLLECTION, GROUPS_COLLECTION, TYPES_COLLECTION,
    PRICING_COLLECTION, PURCHASES_COLLECTION, GROUP_TEMPLATES_COLLECTION,
};

pub fn handle_admin(msg: &Message) -> BlockResult {
    let action = msg_action(msg).to_string();
    let path = msg_path(msg).to_string();

    match (action.as_str(), path.as_str()) {
        // Products
        ("retrieve", "/admin/b/products/products") => handle_list_products(msg),
        ("retrieve", p) if p.starts_with("/admin/b/products/products/") => handle_get_product(msg),
        ("create", "/admin/b/products/products") => handle_create_product(msg),
        ("update", p) if p.starts_with("/admin/b/products/products/") => handle_update_product(msg),
        ("delete", p) if p.starts_with("/admin/b/products/products/") => handle_delete_product(msg),
        // Groups
        ("retrieve", "/admin/b/products/groups") => handle_list_groups(msg),
        ("create", "/admin/b/products/groups") => handle_create_group(msg),
        ("update", p) if p.starts_with("/admin/b/products/groups/") => handle_update_group(msg),
        ("delete", p) if p.starts_with("/admin/b/products/groups/") => handle_delete_group(msg),
        // Types
        ("retrieve", "/admin/b/products/types") => handle_list_types(msg),
        ("create", "/admin/b/products/types") => handle_create_type(msg),
        ("delete", p) if p.starts_with("/admin/b/products/types/") => handle_delete_type(msg),
        // Pricing templates
        ("retrieve", "/admin/b/products/pricing") => handle_list_pricing(msg),
        ("create", "/admin/b/products/pricing") => handle_create_pricing(msg),
        ("update", p) if p.starts_with("/admin/b/products/pricing/") => handle_update_pricing(msg),
        ("delete", p) if p.starts_with("/admin/b/products/pricing/") => handle_delete_pricing(msg),
        // Variables
        ("retrieve", "/admin/b/products/variables") => crate::variables::handle_list(msg),
        ("create", "/admin/b/products/variables") => crate::variables::handle_create(msg),
        ("update", p) if p.starts_with("/admin/b/products/variables/") => crate::variables::handle_update(msg),
        ("delete", p) if p.starts_with("/admin/b/products/variables/") => crate::variables::handle_delete(msg),
        // Purchases (admin view)
        ("retrieve", "/admin/b/products/purchases") => crate::purchase::handle_list_admin(msg),
        ("retrieve", p) if p.starts_with("/admin/b/products/purchases/") => crate::purchase::handle_get(msg),
        ("update", p) if p.starts_with("/admin/b/products/purchases/") && p.ends_with("/refund") => {
            crate::purchase::handle_refund(msg)
        }
        // Stats
        ("retrieve", "/admin/b/products/stats") => handle_stats(msg),
        _ => err_not_found(msg, "not found"),
    }
}

pub fn handle_user(msg: &Message) -> BlockResult {
    let action = msg_action(msg).to_string();
    let path = msg_path(msg).to_string();

    match (action.as_str(), path.as_str()) {
        // User's own products
        ("retrieve", "/b/products/products") => handle_user_list_products(msg),
        ("retrieve", p) if p.starts_with("/b/products/products/") => handle_user_get_product(msg),
        ("create", "/b/products/products") => handle_user_create_product(msg),
        ("update", p) if p.starts_with("/b/products/products/") => handle_user_update_product(msg),
        ("delete", p) if p.starts_with("/b/products/products/") => handle_user_delete_product(msg),
        // User's own groups
        ("retrieve", "/b/products/groups") => handle_user_list_groups(msg),
        ("retrieve", p) if p.starts_with("/b/products/groups/") && !p.ends_with("/products") => handle_user_get_group(msg),
        ("create", "/b/products/groups") => handle_user_create_group(msg),
        ("update", p) if p.starts_with("/b/products/groups/") && !p.ends_with("/products") => handle_user_update_group(msg),
        ("delete", p) if p.starts_with("/b/products/groups/") && !p.ends_with("/products") => handle_user_delete_group(msg),
        // Products in a group
        ("retrieve", p) if p.starts_with("/b/products/groups/") && p.ends_with("/products") => handle_user_group_products(msg),
        // Read-only: types and group templates
        ("retrieve", "/b/products/types") => handle_list_types(msg),
        ("retrieve", "/b/products/group-templates") => handle_user_list_group_templates(msg),
        // Catalog (public)
        ("retrieve", "/b/products/catalog") => handle_catalog(msg),
        ("retrieve", p) if p.starts_with("/b/products/catalog/") => handle_get_product_public(msg),
        // Pricing, purchases, checkout
        ("create", "/b/products/calculate-price") => crate::pricing::handle_calculate(msg),
        ("create", "/b/products/purchases") => crate::purchase::handle_create(msg),
        ("retrieve", "/b/products/purchases") => crate::purchase::handle_list_user(msg),
        ("retrieve", p) if p.starts_with("/b/products/purchases/") => crate::purchase::handle_get(msg),
        ("create", "/b/products/checkout") => crate::stripe::handle_checkout(msg),
        _ => err_not_found(msg, "not found"),
    }
}

// --- Product CRUD ---

fn handle_list_products(msg: &Message) -> BlockResult {
    let (page, page_size) = pagination_params(msg);

    let mut filters = Vec::new();
    let group_id = msg_query(msg, "group_id");
    if !group_id.is_empty() {
        filters.push(Filter { field: "group_id".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(group_id.to_string()) });
    }
    let status = msg_query(msg, "status");
    if !status.is_empty() {
        filters.push(Filter { field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(status.to_string()) });
    }
    let search = msg_query(msg, "search");
    if !search.is_empty() {
        filters.push(Filter { field: "name".to_string(), operator: FilterOp::Like, value: serde_json::Value::String(format!("%{}%", search)) });
    }

    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];
    match db::paginated_list(PRODUCTS_COLLECTION, page, page_size, filters, sort) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_get_product(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/admin/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }
    match db::get(PRODUCTS_COLLECTION, id) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Product not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_create_product(msg: &Message) -> BlockResult {
    let mut data: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    data.entry("status".to_string()).or_insert(serde_json::Value::String("draft".to_string()));
    stamp_created(&mut data);
    let user_id = msg_user_id(msg);
    data.insert("created_by".to_string(), serde_json::Value::String(user_id.to_string()));

    match db::create(PRODUCTS_COLLECTION, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_update_product(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/admin/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }

    let mut body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    stamp_updated(&mut body);

    match db::update(PRODUCTS_COLLECTION, id, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Product not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_delete_product(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/admin/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }
    match db::delete(PRODUCTS_COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Product not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Groups ---

fn handle_list_groups(msg: &Message) -> BlockResult {
    let opts = ListOptions {
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(GROUPS_COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_create_group(msg: &Message) -> BlockResult {
    let mut body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    stamp_created(&mut body);
    let user_id = msg_user_id(msg);
    body.entry("user_id".to_string()).or_insert(serde_json::Value::String(user_id.to_string()));
    match db::create(GROUPS_COLLECTION, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_update_group(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/admin/b/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing group ID"); }
    let body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    match db::update(GROUPS_COLLECTION, id, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Group not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_delete_group(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/admin/b/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing group ID"); }
    match db::delete(GROUPS_COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Group not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Types ---

fn handle_list_types(msg: &Message) -> BlockResult {
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db::list(TYPES_COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_create_type(msg: &Message) -> BlockResult {
    let body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    match db::create(TYPES_COLLECTION, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_delete_type(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/admin/b/products/types/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing type ID"); }
    match db::delete(TYPES_COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Type not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Pricing Templates ---

fn handle_list_pricing(msg: &Message) -> BlockResult {
    let opts = ListOptions {
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(PRICING_COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_create_pricing(msg: &Message) -> BlockResult {
    let mut body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    stamp_created(&mut body);
    match db::create(PRICING_COLLECTION, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_update_pricing(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/admin/b/products/pricing/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing pricing template ID"); }
    let body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    match db::update(PRICING_COLLECTION, id, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Pricing template not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_delete_pricing(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/admin/b/products/pricing/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing pricing template ID"); }
    match db::delete(PRICING_COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Pricing template not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Public catalog ---

fn handle_catalog(msg: &Message) -> BlockResult {
    let (page, page_size) = pagination_params(msg);
    let filters = vec![Filter {
        field: "status".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String("active".to_string()),
    }];
    let sort = vec![SortField { field: "name".to_string(), desc: false }];
    match db::paginated_list(PRODUCTS_COLLECTION, page, page_size, filters, sort) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_get_product_public(msg: &Message) -> BlockResult {
    let path = msg_path(msg);
    let id = path.strip_prefix("/b/products/catalog/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }

    match db::get(PRODUCTS_COLLECTION, id) {
        Ok(record) => {
            let status = str_field(&record, "status");
            if status != "active" {
                return err_not_found(msg, "Product not found");
            }
            json_respond(msg, &serde_json::to_value(&record).unwrap_or_default())
        }
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Product not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- User's own products ---

fn handle_user_list_products(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    if user_id.is_empty() { return err_unauthorized(msg, "Not authenticated"); }

    let (page, page_size) = pagination_params(msg);
    let mut filters = vec![Filter {
        field: "created_by".to_string(), operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id.to_string()),
    }];
    let group_id = msg_query(msg, "group_id");
    if !group_id.is_empty() {
        filters.push(Filter { field: "group_id".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(group_id.to_string()) });
    }
    let status = msg_query(msg, "status");
    if !status.is_empty() {
        filters.push(Filter { field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(status.to_string()) });
    }
    let search = msg_query(msg, "search");
    if !search.is_empty() {
        filters.push(Filter { field: "name".to_string(), operator: FilterOp::Like, value: serde_json::Value::String(format!("%{}%", search)) });
    }

    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];
    match db::paginated_list(PRODUCTS_COLLECTION, page, page_size, filters, sort) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_user_get_product(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    let path = msg_path(msg);
    let id = path.strip_prefix("/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }

    match db::get(PRODUCTS_COLLECTION, id) {
        Ok(record) => {
            if field_as_string(&record, "created_by") != user_id {
                return err_not_found(msg, "Product not found");
            }
            json_respond(msg, &serde_json::to_value(&record).unwrap_or_default())
        }
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Product not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_user_create_product(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    if user_id.is_empty() { return err_unauthorized(msg, "Not authenticated"); }

    let mut data: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Verify user owns the group (if provided)
    let group_id_str = data.get("group_id").and_then(|v| v.as_str().map(|s| s.to_string()))
        .or_else(|| data.get("group_id").and_then(|v| v.as_i64().map(|n| n.to_string())))
        .unwrap_or_default();
    if !group_id_str.is_empty() {
        match db::get(GROUPS_COLLECTION, &group_id_str) {
            Ok(group) => {
                if field_as_string(&group, "user_id") != user_id {
                    return err_bad_request(msg, "You don't own this group");
                }
            }
            Err(_) => return err_bad_request(msg, "Group not found"),
        }
    }

    data.entry("status".to_string()).or_insert(serde_json::Value::String("draft".to_string()));
    stamp_created(&mut data);
    data.insert("created_by".to_string(), serde_json::Value::String(user_id.to_string()));
    // Default product_template_id to the seeded template (id=1) if not provided
    data.entry("product_template_id".to_string()).or_insert(serde_json::json!(1));

    match db::create(PRODUCTS_COLLECTION, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_user_update_product(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    let path = msg_path(msg);
    let id = path.strip_prefix("/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }

    // Verify ownership
    match db::get(PRODUCTS_COLLECTION, id) {
        Ok(record) => {
            if field_as_string(&record, "created_by") != user_id {
                return err_not_found(msg, "Product not found");
            }
        }
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => return err_not_found(msg, "Product not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    }

    let mut body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    body.remove("created_by"); // prevent ownership change
    stamp_updated(&mut body);

    match db::update(PRODUCTS_COLLECTION, id, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_user_delete_product(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    let path = msg_path(msg);
    let id = path.strip_prefix("/b/products/products/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing product ID"); }

    // Verify ownership
    match db::get(PRODUCTS_COLLECTION, id) {
        Ok(record) => {
            if field_as_string(&record, "created_by") != user_id {
                return err_not_found(msg, "Product not found");
            }
        }
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => return err_not_found(msg, "Product not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    }

    match db::delete(PRODUCTS_COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- User's own groups ---

fn handle_user_list_groups(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    if user_id.is_empty() { return err_unauthorized(msg, "Not authenticated"); }

    let opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".to_string(), operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        }],
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(GROUPS_COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_user_get_group(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    let path = msg_path(msg);
    let id = path.strip_prefix("/b/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing group ID"); }

    match db::get(GROUPS_COLLECTION, id) {
        Ok(record) => {
            if field_as_string(&record, "user_id") != user_id {
                return err_not_found(msg, "Group not found");
            }
            json_respond(msg, &serde_json::to_value(&record).unwrap_or_default())
        }
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Group not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_user_create_group(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    if user_id.is_empty() { return err_unauthorized(msg, "Not authenticated"); }

    let mut body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    stamp_created(&mut body);
    body.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
    // Default group_template_id to the seeded template (id=1) if not provided
    body.entry("group_template_id".to_string()).or_insert(serde_json::json!(1));

    match db::create(GROUPS_COLLECTION, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_user_update_group(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    let path = msg_path(msg);
    let id = path.strip_prefix("/b/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing group ID"); }

    // Verify ownership
    match db::get(GROUPS_COLLECTION, id) {
        Ok(record) => {
            if field_as_string(&record, "user_id") != user_id {
                return err_not_found(msg, "Group not found");
            }
        }
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => return err_not_found(msg, "Group not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    }

    let mut body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    body.remove("user_id"); // prevent ownership change

    match db::update(GROUPS_COLLECTION, id, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_user_delete_group(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    let path = msg_path(msg);
    let id = path.strip_prefix("/b/products/groups/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing group ID"); }

    // Verify ownership
    match db::get(GROUPS_COLLECTION, id) {
        Ok(record) => {
            if field_as_string(&record, "user_id") != user_id {
                return err_not_found(msg, "Group not found");
            }
        }
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => return err_not_found(msg, "Group not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    }

    match db::delete(GROUPS_COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// Products in a user's group
fn handle_user_group_products(msg: &Message) -> BlockResult {
    let user_id = msg_user_id(msg);
    let path = msg_path(msg);
    // Path: /b/products/groups/{id}/products
    let rest = path.strip_prefix("/b/products/groups/").unwrap_or("");
    let group_id = rest.strip_suffix("/products").unwrap_or("");
    if group_id.is_empty() { return err_bad_request(msg, "Missing group ID"); }

    // Verify group ownership
    match db::get(GROUPS_COLLECTION, group_id) {
        Ok(record) => {
            if field_as_string(&record, "user_id") != user_id {
                return err_not_found(msg, "Group not found");
            }
        }
        Err(_) => return err_not_found(msg, "Group not found"),
    }

    let (page, page_size) = pagination_params(msg);
    let filters = vec![Filter {
        field: "group_id".to_string(), operator: FilterOp::Equal,
        value: serde_json::Value::String(group_id.to_string()),
    }];
    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];
    match db::paginated_list(PRODUCTS_COLLECTION, page, page_size, filters, sort) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// User-accessible group templates (read-only)
fn handle_user_list_group_templates(msg: &Message) -> BlockResult {
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db::list(GROUP_TEMPLATES_COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Stats ---

fn handle_stats(msg: &Message) -> BlockResult {
    let total_products = db::count(PRODUCTS_COLLECTION, &[]).unwrap_or(0);
    let active_products = db::count(PRODUCTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("active".to_string()),
    }]).unwrap_or(0);
    let total_purchases = db::count(PURCHASES_COLLECTION, &[]).unwrap_or(0);
    let total_revenue = db::sum(PURCHASES_COLLECTION, "total_cents", &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("completed".to_string()),
    }]).unwrap_or(0.0);
    let total_groups = db::count(GROUPS_COLLECTION, &[]).unwrap_or(0);

    json_respond(msg, &serde_json::json!({
        "total_products": total_products,
        "active_products": active_products,
        "total_purchases": total_purchases,
        "total_revenue": total_revenue,
        "total_groups": total_groups
    }))
}
