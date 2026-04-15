use crate::blocks::helpers::{
    self, err_bad_request, err_conflict, err_forbidden, err_internal, err_not_found, json_map,
    ok_json, RecordExt,
};
use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};

use super::{PERMISSIONS_COLLECTION, ROLES_COLLECTION, USER_ROLES_COLLECTION};

pub async fn handle(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        // Roles
        ("retrieve", "/admin/iam/roles") => handle_list_roles(ctx).await,
        ("create", "/admin/iam/roles") => handle_create_role(ctx, input).await,
        ("update", _) if path.starts_with("/admin/iam/roles/") => {
            handle_update_role(ctx, msg, input).await
        }
        ("delete", _) if path.starts_with("/admin/iam/roles/") => {
            handle_delete_role(ctx, msg).await
        }
        // Permissions
        ("retrieve", "/admin/iam/permissions") => handle_list_permissions(ctx).await,
        ("create", "/admin/iam/permissions") => handle_create_permission(ctx, input).await,
        ("delete", _) if path.starts_with("/admin/iam/permissions/") => {
            handle_delete_permission(ctx, msg).await
        }
        // User-role assignments
        ("retrieve", "/admin/iam/user-roles") => handle_list_user_roles(ctx, msg).await,
        ("create", "/admin/iam/user-roles") => handle_assign_role(ctx, msg, input).await,
        ("delete", _) if path.starts_with("/admin/iam/user-roles/") => {
            handle_remove_role(ctx, msg).await
        }
        _ => err_not_found("not found"),
    }
}

async fn handle_list_roles(ctx: &dyn Context) -> OutputStream {
    let opts = ListOptions {
        sort: vec![SortField {
            field: "name".to_string(),
            desc: false,
        }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, ROLES_COLLECTION, &opts).await {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_create_role(ctx: &dyn Context, input: InputStream) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct Req {
        name: String,
        description: Option<String>,
        permissions: Option<Vec<String>>,
    }
    let raw = input.collect_to_bytes().await;
    let body: Req = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };
    let mut data = json_map(serde_json::json!({
        "name": body.name,
        "description": body.description.unwrap_or_default(),
        "permissions": body.permissions.unwrap_or_default(),
        "is_system": false
    }));
    helpers::stamp_created(&mut data);
    match db::create(ctx, ROLES_COLLECTION, data).await {
        Ok(record) => ok_json(&record),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_update_role(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/roles/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request("Missing role ID");
    }

    let raw = input.collect_to_bytes().await;
    let body_peek: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Protect system roles from name changes (renaming "admin" would break auth)
    if let Ok(existing) = db::get(ctx, ROLES_COLLECTION, id).await {
        if existing.bool_field("is_system") {
            if body_peek.contains_key("name") {
                return err_forbidden("Cannot rename system roles");
            }
            let mut data = HashMap::new();
            for key in &["description", "permissions"] {
                if let Some(val) = body_peek.get(*key) {
                    data.insert(key.to_string(), val.clone());
                }
            }
            helpers::stamp_updated(&mut data);
            return match db::update(ctx, ROLES_COLLECTION, id, data).await {
                Ok(record) => ok_json(&record),
                Err(e) => err_internal(&format!("Database error: {e}")),
            };
        }
    }

    let mut data = HashMap::new();
    for key in &["name", "description", "permissions"] {
        if let Some(val) = body_peek.get(*key) {
            data.insert(key.to_string(), val.clone());
        }
    }
    helpers::stamp_updated(&mut data);
    match db::update(ctx, ROLES_COLLECTION, id, data).await {
        Ok(record) => ok_json(&record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("Role not found"),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_delete_role(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/roles/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request("Missing role ID");
    }

    // Check if system role
    if let Ok(role) = db::get(ctx, ROLES_COLLECTION, id).await {
        if role.bool_field("is_system") {
            return err_forbidden("Cannot delete system role");
        }
    }

    match db::delete(ctx, ROLES_COLLECTION, id).await {
        Ok(()) => ok_json(&serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("Role not found"),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_list_permissions(ctx: &dyn Context) -> OutputStream {
    let opts = ListOptions {
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, PERMISSIONS_COLLECTION, &opts).await {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_create_permission(ctx: &dyn Context, input: InputStream) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct Req {
        name: String,
        resource: String,
        actions: Vec<String>,
    }
    let raw = input.collect_to_bytes().await;
    let body: Req = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };
    let mut data = json_map(serde_json::json!({
        "name": body.name,
        "resource": body.resource,
        "actions": body.actions
    }));
    helpers::stamp_created(&mut data);
    match db::create(ctx, PERMISSIONS_COLLECTION, data).await {
        Ok(record) => ok_json(&record),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_delete_permission(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/permissions/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request("Missing permission ID");
    }
    match db::delete(ctx, PERMISSIONS_COLLECTION, id).await {
        Ok(()) => ok_json(&serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("Permission not found"),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_list_user_roles(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.query("user_id").to_string();
    let mut filters = Vec::new();
    if !user_id.is_empty() {
        filters.push(Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        });
    }
    let opts = ListOptions {
        filters,
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, USER_ROLES_COLLECTION, &opts).await {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_assign_role(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct Req {
        user_id: String,
        role: String,
    }
    let raw = input.collect_to_bytes().await;
    let body: Req = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Check if already assigned
    let existing = db::list_all(
        ctx,
        USER_ROLES_COLLECTION,
        vec![
            Filter {
                field: "user_id".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(body.user_id.clone()),
            },
            Filter {
                field: "role".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(body.role.clone()),
            },
        ],
    )
    .await;
    if let Ok(records) = existing {
        if !records.is_empty() {
            return err_conflict("Role already assigned to user");
        }
    }

    let data = json_map(serde_json::json!({
        "user_id": body.user_id,
        "role": body.role,
        "assigned_at": helpers::now_rfc3339(),
        "assigned_by": msg.user_id()
    }));
    match db::create(ctx, USER_ROLES_COLLECTION, data).await {
        Ok(record) => ok_json(&record),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_remove_role(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/user-roles/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request("Missing user-role ID");
    }

    // Prevent admins from removing their own admin role (self-lockout)
    match db::get(ctx, USER_ROLES_COLLECTION, id).await {
        Ok(record) => {
            let role_user = record.str_field("user_id");
            let role_name = record.str_field("role");
            if role_user == msg.user_id() && role_name == "admin" {
                return err_bad_request("Cannot remove your own admin role");
            }
        }
        Err(e) if e.code == ErrorCode::NotFound => {
            return err_not_found("User-role assignment not found");
        }
        Err(e) => {
            return err_internal(&format!("Database error: {e}"));
        }
    }

    match db::delete(ctx, USER_ROLES_COLLECTION, id).await {
        Ok(()) => ok_json(&serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found("User-role assignment not found")
        }
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

pub async fn seed_defaults(ctx: &dyn Context) {
    let count = db::count(ctx, ROLES_COLLECTION, &[]).await.unwrap_or(0);
    if count > 0 {
        return;
    }

    let now = helpers::now_rfc3339();
    for (name, desc) in &[
        ("admin", "Full access to all resources"),
        ("user", "Standard user access"),
    ] {
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
