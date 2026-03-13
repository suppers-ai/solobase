use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use crate::blocks::helpers::{self, json_map, RecordExt};

const ROLES_COLLECTION: &str = "iam_roles";
const PERMISSIONS_COLLECTION: &str = "iam_permissions";
const USER_ROLES_COLLECTION: &str = "iam_user_roles";

pub async fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        // Roles
        ("retrieve", "/admin/iam/roles") => handle_list_roles(ctx, msg).await,
        ("create", "/admin/iam/roles") => handle_create_role(ctx, msg).await,
        ("update", _) if path.starts_with("/admin/iam/roles/") => handle_update_role(ctx, msg).await,
        ("delete", _) if path.starts_with("/admin/iam/roles/") => handle_delete_role(ctx, msg).await,
        // Permissions
        ("retrieve", "/admin/iam/permissions") => handle_list_permissions(ctx, msg).await,
        ("create", "/admin/iam/permissions") => handle_create_permission(ctx, msg).await,
        ("delete", _) if path.starts_with("/admin/iam/permissions/") => handle_delete_permission(ctx, msg).await,
        // User-role assignments
        ("retrieve", "/admin/iam/user-roles") => handle_list_user_roles(ctx, msg).await,
        ("create", "/admin/iam/user-roles") => handle_assign_role(ctx, msg).await,
        ("delete", _) if path.starts_with("/admin/iam/user-roles/") => handle_remove_role(ctx, msg).await,
        _ => err_not_found(msg, "not found"),
    }
}

async fn handle_list_roles(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions {
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, ROLES_COLLECTION, &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_create_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    #[derive(serde::Deserialize)]
    struct Req { name: String, description: Option<String>, permissions: Option<Vec<String>> }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    let mut data = json_map(serde_json::json!({
        "name": body.name,
        "description": body.description.unwrap_or_default(),
        "permissions": body.permissions.unwrap_or_default(),
        "is_system": false
    }));
    helpers::stamp_created(&mut data);
    match db::create(ctx, ROLES_COLLECTION, data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_update_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/roles/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing role ID"); }

    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    let mut data = HashMap::new();
    for key in &["name", "description", "permissions"] {
        if let Some(val) = body.get(*key) {
            data.insert(key.to_string(), val.clone());
        }
    }
    helpers::stamp_updated(&mut data);
    match db::update(ctx, ROLES_COLLECTION, id, data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Role not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_delete_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/roles/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing role ID"); }

    // Check if system role
    if let Ok(role) = db::get(ctx, ROLES_COLLECTION, id).await {
        if role.bool_field("is_system") {
            return err_forbidden(msg, "Cannot delete system role");
        }
    }

    match db::delete(ctx, ROLES_COLLECTION, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Role not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_list_permissions(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db::list(ctx, PERMISSIONS_COLLECTION, &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_create_permission(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    #[derive(serde::Deserialize)]
    struct Req { name: String, resource: String, actions: Vec<String> }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    let mut data = json_map(serde_json::json!({
        "name": body.name,
        "resource": body.resource,
        "actions": body.actions
    }));
    helpers::stamp_created(&mut data);
    match db::create(ctx, PERMISSIONS_COLLECTION, data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_delete_permission(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/permissions/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing permission ID"); }
    match db::delete(ctx, PERMISSIONS_COLLECTION, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Permission not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_list_user_roles(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.query("user_id").to_string();
    let mut filters = Vec::new();
    if !user_id.is_empty() {
        filters.push(Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        });
    }
    let opts = ListOptions { filters, limit: 1000, ..Default::default() };
    match db::list(ctx, USER_ROLES_COLLECTION, &opts).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_assign_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    #[derive(serde::Deserialize)]
    struct Req { user_id: String, role: String }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    // Check if already assigned
    let existing = db::list_all(ctx, USER_ROLES_COLLECTION, vec![
        Filter { field: "user_id".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(body.user_id.clone()) },
        Filter { field: "role".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(body.role.clone()) },
    ]).await;
    if let Ok(records) = existing {
        if !records.is_empty() {
            return err_conflict(msg, "Role already assigned to user");
        }
    }

    let data = json_map(serde_json::json!({
        "user_id": body.user_id,
        "role": body.role,
        "assigned_at": helpers::now_rfc3339(),
        "assigned_by": msg.user_id()
    }));
    match db::create(ctx, USER_ROLES_COLLECTION, data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_remove_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/user-roles/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing user-role ID"); }
    match db::delete(ctx, USER_ROLES_COLLECTION, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "User-role assignment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub async fn seed_defaults(ctx: &dyn Context) {
    let count = db::count(ctx, ROLES_COLLECTION, &[]).await.unwrap_or(0);
    if count > 0 { return; }

    let now = helpers::now_rfc3339();
    for (name, desc) in &[("admin", "Full access to all resources"), ("user", "Standard user access")] {
        let data = json_map(serde_json::json!({
            "name": name,
            "description": desc,
            "is_system": true,
            "created_at": now,
            "permissions": []
        }));
        if let Err(e) = db::create(ctx, ROLES_COLLECTION, data).await {
            tracing::warn!("Failed to seed default role '{name}': {e}");
        }
    }
}
