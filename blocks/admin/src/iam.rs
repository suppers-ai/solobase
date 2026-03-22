use std::collections::HashMap;

use crate::wafer::block_world::types::*;
use crate::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};

const ROLES_COLLECTION: &str = "iam_roles";
const PERMISSIONS_COLLECTION: &str = "iam_permissions";
const USER_ROLES_COLLECTION: &str = "iam_user_roles";

pub fn handle(msg: &Message) -> BlockResult {
    let action = msg_get_meta(msg, "req.action");
    let path = msg_get_meta(msg, "req.resource");

    match (action, path) {
        // Roles
        ("retrieve", "/admin/iam/roles") => handle_list_roles(msg),
        ("create", "/admin/iam/roles") => handle_create_role(msg),
        ("update", p) if p.starts_with("/admin/iam/roles/") => handle_update_role(msg),
        ("delete", p) if p.starts_with("/admin/iam/roles/") => handle_delete_role(msg),
        // Permissions
        ("retrieve", "/admin/iam/permissions") => handle_list_permissions(msg),
        ("create", "/admin/iam/permissions") => handle_create_permission(msg),
        ("delete", p) if p.starts_with("/admin/iam/permissions/") => handle_delete_permission(msg),
        // User-role assignments
        ("retrieve", "/admin/iam/user-roles") => handle_list_user_roles(msg),
        ("create", "/admin/iam/user-roles") => handle_assign_role(msg),
        ("delete", p) if p.starts_with("/admin/iam/user-roles/") => handle_remove_role(msg),
        _ => err_not_found(msg, "not found"),
    }
}

fn handle_list_roles(msg: &Message) -> BlockResult {
    let opts = ListOptions {
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db::list(ROLES_COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_create_role(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct Req { name: String, description: Option<String>, permissions: Option<Vec<String>> }

    let body: Req = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let mut data = json_map(serde_json::json!({
        "name": body.name,
        "description": body.description.unwrap_or_default(),
        "permissions": body.permissions.unwrap_or_default(),
        "is_system": false
    }));
    stamp_created(&mut data);

    match db::create(ROLES_COLLECTION, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_update_role(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let id = path.strip_prefix("/admin/iam/roles/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing role ID"); }

    let body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let mut data = HashMap::new();
    for key in &["name", "description", "permissions"] {
        if let Some(val) = body.get(*key) {
            data.insert(key.to_string(), val.clone());
        }
    }
    stamp_updated(&mut data);

    match db::update(ROLES_COLLECTION, id, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => {
            let we = convert_error(e);
            if we.code == ErrorCode::NotFound {
                err_not_found(msg, "Role not found")
            } else {
                err_internal(msg, &format!("Database error: {}", we.message))
            }
        }
    }
}

fn handle_delete_role(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let id = path.strip_prefix("/admin/iam/roles/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing role ID"); }

    // Check if system role
    if let Ok(role) = db::get(ROLES_COLLECTION, id) {
        if bool_field(&role, "is_system") {
            return err_forbidden(msg, "Cannot delete system role");
        }
    }

    match db::delete(ROLES_COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => {
            let we = convert_error(e);
            if we.code == ErrorCode::NotFound {
                err_not_found(msg, "Role not found")
            } else {
                err_internal(msg, &format!("Database error: {}", we.message))
            }
        }
    }
}

fn handle_list_permissions(msg: &Message) -> BlockResult {
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db::list(PERMISSIONS_COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_create_permission(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct Req { name: String, resource: String, actions: Vec<String> }

    let body: Req = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let mut data = json_map(serde_json::json!({
        "name": body.name,
        "resource": body.resource,
        "actions": body.actions
    }));
    stamp_created(&mut data);

    match db::create(PERMISSIONS_COLLECTION, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_delete_permission(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let id = path.strip_prefix("/admin/iam/permissions/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing permission ID"); }

    match db::delete(PERMISSIONS_COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => {
            let we = convert_error(e);
            if we.code == ErrorCode::NotFound {
                err_not_found(msg, "Permission not found")
            } else {
                err_internal(msg, &format!("Database error: {}", we.message))
            }
        }
    }
}

fn handle_list_user_roles(msg: &Message) -> BlockResult {
    let user_id = msg_query(msg, "user_id");
    let mut filters = Vec::new();
    if !user_id.is_empty() {
        filters.push(Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        });
    }
    let opts = ListOptions { filters, limit: 1000, ..Default::default() };
    match db::list(USER_ROLES_COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_assign_role(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct Req { user_id: String, role: String }

    let body: Req = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Check if already assigned
    let existing = db::list_all(USER_ROLES_COLLECTION, vec![
        Filter { field: "user_id".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(body.user_id.clone()) },
        Filter { field: "role".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(body.role.clone()) },
    ]);
    if let Ok(records) = existing {
        if !records.is_empty() {
            return err_conflict(msg, "Role already assigned to user");
        }
    }

    let data = json_map(serde_json::json!({
        "user_id": body.user_id,
        "role": body.role,
        "assigned_at": now_rfc3339(),
        "assigned_by": msg_user_id(msg)
    }));
    match db::create(USER_ROLES_COLLECTION, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_remove_role(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let id = path.strip_prefix("/admin/iam/user-roles/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg, "Missing user-role ID"); }

    match db::delete(USER_ROLES_COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => {
            let we = convert_error(e);
            if we.code == ErrorCode::NotFound {
                err_not_found(msg, "User-role assignment not found")
            } else {
                err_internal(msg, &format!("Database error: {}", we.message))
            }
        }
    }
}
