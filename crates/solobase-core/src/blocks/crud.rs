//! Generic CRUD helpers for block handlers.
//!
//! These encapsulate the repeated list/get/create/update/delete patterns
//! so each handler reduces to a one-liner for pure-CRUD operations.

use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, SortField};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};

use super::helpers::{err_bad_request, err_internal, err_not_found, ok_json, stamp_created,
    stamp_updated};

// ---------------------------------------------------------------------------
// CRUD helpers
// ---------------------------------------------------------------------------

/// List records with pagination, optional extra filters, and default sort by created_at desc.
pub async fn crud_list(
    ctx: &dyn Context,
    msg: &Message,
    collection: &str,
    extra_filters: Vec<Filter>,
) -> OutputStream {
    let (page, page_size, _) = msg.pagination_params(20);
    let sort = vec![SortField {
        field: "created_at".to_string(),
        desc: true,
    }];
    match db::paginated_list(ctx, collection, page as i64, page_size as i64, extra_filters, sort)
        .await
    {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

/// Get a single record by ID extracted from the path.
pub async fn crud_get(
    ctx: &dyn Context,
    msg: &Message,
    collection: &str,
    path_prefix: &str,
    not_found_label: &str,
) -> OutputStream {
    let path = msg.path();
    let id = path.strip_prefix(path_prefix).unwrap_or("");
    if id.is_empty() {
        return err_bad_request(&format!("Missing {} ID", not_found_label.to_lowercase()));
    }
    match db::get(ctx, collection, id).await {
        Ok(record) => ok_json(&record),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(&format!("{not_found_label} not found"))
        }
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

/// Create a record from the request body, with auto-timestamps and optional defaults.
pub async fn crud_create(
    ctx: &dyn Context,
    _msg: &Message,
    input: InputStream,
    collection: &str,
    defaults: HashMap<String, serde_json::Value>,
) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    let mut data = body;
    stamp_created(&mut data);

    for (key, val) in defaults {
        data.entry(key).or_insert(val);
    }

    match db::create(ctx, collection, data).await {
        Ok(record) => ok_json(&record),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

/// Update a record by ID extracted from the path, with auto-updated_at.
pub async fn crud_update(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
    collection: &str,
    path_prefix: &str,
    not_found_label: &str,
) -> OutputStream {
    let path = msg.path();
    let id = path.strip_prefix(path_prefix).unwrap_or("").to_string();
    if id.is_empty() {
        return err_bad_request(&format!("Missing {} ID", not_found_label.to_lowercase()));
    }

    let raw = input.collect_to_bytes().await;
    let mut body: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };
    stamp_updated(&mut body);

    match db::update(ctx, collection, &id, body).await {
        Ok(record) => ok_json(&record),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(&format!("{not_found_label} not found"))
        }
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

/// Delete a record by ID extracted from the path.
pub async fn crud_delete(
    ctx: &dyn Context,
    msg: &Message,
    collection: &str,
    path_prefix: &str,
    not_found_label: &str,
) -> OutputStream {
    let path = msg.path();
    let id = path.strip_prefix(path_prefix).unwrap_or("").to_string();
    if id.is_empty() {
        return err_bad_request(&format!("Missing {} ID", not_found_label.to_lowercase()));
    }
    match db::delete(ctx, collection, &id).await {
        Ok(()) => ok_json(&serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(&format!("{not_found_label} not found"))
        }
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}
