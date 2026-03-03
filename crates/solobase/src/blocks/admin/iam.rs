use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{self, DatabaseService, Filter, FilterOp, ListOptions, SortField};
use super::get_db;

const ROLES_COLLECTION: &str = "iam_roles";
const PERMISSIONS_COLLECTION: &str = "iam_permissions";
const USER_ROLES_COLLECTION: &str = "iam_user_roles";

pub fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        // Roles
        ("retrieve", "/admin/iam/roles") => handle_list_roles(ctx, msg),
        ("create", "/admin/iam/roles") => handle_create_role(ctx, msg),
        ("update", _) if path.starts_with("/admin/iam/roles/") => handle_update_role(ctx, msg),
        ("delete", _) if path.starts_with("/admin/iam/roles/") => handle_delete_role(ctx, msg),
        // Permissions
        ("retrieve", "/admin/iam/permissions") => handle_list_permissions(ctx, msg),
        ("create", "/admin/iam/permissions") => handle_create_permission(ctx, msg),
        ("delete", _) if path.starts_with("/admin/iam/permissions/") => handle_delete_permission(ctx, msg),
        // User-role assignments
        ("retrieve", "/admin/iam/user-roles") => handle_list_user_roles(ctx, msg),
        ("create", "/admin/iam/user-roles") => handle_assign_role(ctx, msg),
        ("delete", _) if path.starts_with("/admin/iam/user-roles/") => handle_remove_role(ctx, msg),
        _ => err_not_found(msg.clone(), "not found"),
    }
}

fn handle_list_roles(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let opts = ListOptions {
        sort: vec![SortField { field: "name".to_string(), desc: false }],
        limit: 1000,
        ..Default::default()
    };
    match db.list(ROLES_COLLECTION, &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_create_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    #[derive(serde::Deserialize)]
    struct Req { name: String, description: Option<String>, permissions: Option<Vec<String>> }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };
    let now = chrono::Utc::now().to_rfc3339();
    let mut data = HashMap::new();
    data.insert("name".to_string(), serde_json::Value::String(body.name));
    data.insert("description".to_string(), serde_json::Value::String(body.description.unwrap_or_default()));
    data.insert("permissions".to_string(), serde_json::json!(body.permissions.unwrap_or_default()));
    data.insert("created_at".to_string(), serde_json::Value::String(now));
    data.insert("is_system".to_string(), serde_json::Value::Bool(false));
    match db.create(ROLES_COLLECTION, data) {
        Ok(record) => json_respond(msg.clone(), 201, &record),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_update_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/roles/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing role ID"); }

    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };
    let mut data = HashMap::new();
    for key in &["name", "description", "permissions"] {
        if let Some(val) = body.get(*key) {
            data.insert(key.to_string(), val.clone());
        }
    }
    data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    match db.update(ROLES_COLLECTION, id, data) {
        Ok(record) => json_respond(msg.clone(), 200, &record),
        Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "Role not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_delete_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/roles/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing role ID"); }

    // Check if system role
    if let Ok(role) = db.get(ROLES_COLLECTION, id) {
        if role.data.get("is_system").and_then(|v| v.as_bool()).unwrap_or(false) {
            return err_forbidden(msg.clone(), "Cannot delete system role");
        }
    }

    match db.delete(ROLES_COLLECTION, id) {
        Ok(()) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
        Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "Role not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_list_permissions(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db.list(PERMISSIONS_COLLECTION, &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_create_permission(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    #[derive(serde::Deserialize)]
    struct Req { name: String, resource: String, actions: Vec<String> }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };
    let mut data = HashMap::new();
    data.insert("name".to_string(), serde_json::Value::String(body.name));
    data.insert("resource".to_string(), serde_json::Value::String(body.resource));
    data.insert("actions".to_string(), serde_json::json!(body.actions));
    data.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    match db.create(PERMISSIONS_COLLECTION, data) {
        Ok(record) => json_respond(msg.clone(), 201, &record),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_delete_permission(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/permissions/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing permission ID"); }
    match db.delete(PERMISSIONS_COLLECTION, id) {
        Ok(()) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
        Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "Permission not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_list_user_roles(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
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
    match db.list(USER_ROLES_COLLECTION, &opts) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_assign_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    #[derive(serde::Deserialize)]
    struct Req { user_id: String, role: String }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };

    // Check if already assigned
    let existing = database::list_all(db.as_ref(), USER_ROLES_COLLECTION, vec![
        Filter { field: "user_id".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(body.user_id.clone()) },
        Filter { field: "role".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(body.role.clone()) },
    ]);
    if let Ok(records) = existing {
        if !records.is_empty() {
            return err_conflict(msg.clone(), "Role already assigned to user");
        }
    }

    let mut data = HashMap::new();
    data.insert("user_id".to_string(), serde_json::Value::String(body.user_id));
    data.insert("role".to_string(), serde_json::Value::String(body.role));
    data.insert("assigned_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    data.insert("assigned_by".to_string(), serde_json::Value::String(msg.user_id().to_string()));
    match db.create(USER_ROLES_COLLECTION, data) {
        Ok(record) => json_respond(msg.clone(), 201, &record),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_remove_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let path = msg.path();
    let id = path.strip_prefix("/admin/iam/user-roles/").unwrap_or("");
    if id.is_empty() { return err_bad_request(msg.clone(), "Missing user-role ID"); }
    match db.delete(USER_ROLES_COLLECTION, id) {
        Ok(()) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
        Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "User-role assignment not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

pub fn seed_defaults(db: &dyn DatabaseService) {
    let count = db.count(ROLES_COLLECTION, &[]).unwrap_or(0);
    if count > 0 { return; }

    for (name, desc) in &[("admin", "Full access to all resources"), ("user", "Standard user access")] {
        let mut data = HashMap::new();
        data.insert("name".to_string(), serde_json::Value::String(name.to_string()));
        data.insert("description".to_string(), serde_json::Value::String(desc.to_string()));
        data.insert("is_system".to_string(), serde_json::Value::Bool(true));
        data.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        data.insert("permissions".to_string(), serde_json::json!([]));
        if let Err(e) = db.create(ROLES_COLLECTION, data) {
            tracing::warn!("Failed to seed default role '{name}': {e}");
        }
    }
}
