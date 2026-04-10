//! Generic CRUD helpers for block handlers.
//!
//! These encapsulate the repeated list/get/create/update/delete patterns
//! so each handler reduces to a one-liner for pure-CRUD operations.

use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, SortField};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

use super::helpers::{stamp_created, stamp_updated};

/// List records with pagination, optional extra filters, and default sort by created_at desc.
pub async fn crud_list(
    ctx: &dyn Context,
    msg: &mut Message,
    collection: &str,
    extra_filters: Vec<Filter>,
) -> Result_ {
    let (page, page_size, _) = msg.pagination_params(20);
    let sort = vec![SortField {
        field: "created_at".to_string(),
        desc: true,
    }];
    match db::paginated_list(ctx, collection, page as i64, page_size as i64, extra_filters, sort)
        .await
    {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

/// Get a single record by ID extracted from the path.
pub async fn crud_get(
    ctx: &dyn Context,
    msg: &mut Message,
    collection: &str,
    path_prefix: &str,
    not_found_label: &str,
) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix(path_prefix).unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, &format!("Missing {} ID", not_found_label.to_lowercase()));
    }
    match db::get(ctx, collection, id).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(msg, &format!("{not_found_label} not found"))
        }
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

/// Create a record from the request body, with auto-timestamps and optional defaults.
pub async fn crud_create(
    ctx: &dyn Context,
    msg: &mut Message,
    collection: &str,
    defaults: HashMap<String, serde_json::Value>,
) -> Result_ {
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    let mut data = body;
    stamp_created(&mut data);

    for (key, val) in defaults {
        data.entry(key).or_insert(val);
    }

    match db::create(ctx, collection, data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

/// Update a record by ID extracted from the path, with auto-updated_at.
pub async fn crud_update(
    ctx: &dyn Context,
    msg: &mut Message,
    collection: &str,
    path_prefix: &str,
    not_found_label: &str,
) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix(path_prefix).unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, &format!("Missing {} ID", not_found_label.to_lowercase()));
    }

    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    stamp_updated(&mut body);

    match db::update(ctx, collection, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(msg, &format!("{not_found_label} not found"))
        }
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

/// Delete a record by ID extracted from the path.
pub async fn crud_delete(
    ctx: &dyn Context,
    msg: &mut Message,
    collection: &str,
    path_prefix: &str,
    not_found_label: &str,
) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix(path_prefix).unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, &format!("Missing {} ID", not_found_label.to_lowercase()));
    }
    match db::delete(ctx, collection, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(msg, &format!("{not_found_label} not found"))
        }
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}
