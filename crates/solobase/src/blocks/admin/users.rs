use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{self, Filter, FilterOp, ListOptions, SortField};
use super::get_db;

const COLLECTION: &str = "auth_users";

pub fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/users") => handle_list(ctx, msg),
        ("retrieve", _) if path.starts_with("/admin/users/") => handle_get(ctx, msg),
        ("update", _) if path.starts_with("/admin/users/") => handle_update(ctx, msg),
        ("delete", _) if path.starts_with("/admin/users/") => handle_delete(ctx, msg),
        _ => err_not_found(msg.clone(), "not found"),
    }
}

fn handle_list(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) {
        Ok(db) => db,
        Err(r) => return r,
    };

    let (page, page_size, _) = msg.pagination_params(20);
    let search = msg.query("search").to_string();

    let mut filters = vec![
        Filter {
            field: "deleted_at".to_string(),
            operator: FilterOp::IsNull,
            value: serde_json::Value::Null,
        },
    ];

    if !search.is_empty() {
        filters.push(Filter {
            field: "email".to_string(),
            operator: FilterOp::Like,
            value: serde_json::Value::String(format!("%{}%", search)),
        });
    }

    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];

    match database::paginated_list(db.as_ref(), COLLECTION, page as i64, page_size as i64, filters, sort) {
        Ok(mut result) => {
            // Strip password hashes from response
            for record in &mut result.records {
                record.data.remove("password_hash");
            }
            json_respond(msg.clone(), 200, &result)
        }
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_get(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) {
        Ok(db) => db,
        Err(r) => return r,
    };
    let id = msg.var("id").to_string();
    if id.is_empty() {
        // Extract from path
        let path = msg.path().to_string();
        let id = path.strip_prefix("/admin/users/").unwrap_or("").to_string();
        if id.is_empty() {
            return err_bad_request(msg.clone(), "Missing user ID");
        }
        return get_user(db.as_ref(), msg, &id);
    }
    get_user(db.as_ref(), msg, &id)
}

fn get_user(db: &dyn database::DatabaseService, msg: &mut Message, id: &str) -> Result_ {
    match db.get(COLLECTION, id) {
        Ok(mut record) => {
            record.data.remove("password_hash");
            // Get roles
            let roles_opts = ListOptions {
                filters: vec![Filter {
                    field: "user_id".to_string(),
                    operator: FilterOp::Equal,
                    value: serde_json::Value::String(id.to_string()),
                }],
                ..Default::default()
            };
            let roles: Vec<String> = match db.list("iam_user_roles", &roles_opts) {
                Ok(r) => r.records.iter()
                    .filter_map(|rec| rec.data.get("role").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .collect(),
                Err(_) => Vec::new(),
            };
            let mut resp = serde_json::to_value(&record).unwrap_or_default();
            if let Some(obj) = resp.as_object_mut() {
                obj.insert("roles".to_string(), serde_json::json!(roles));
            }
            json_respond(msg.clone(), 200, &resp)
        }
        Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "User not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_update(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) {
        Ok(db) => db,
        Err(r) => return r,
    };
    let path = msg.path();
    let id = msg.var("id");
    let id = if id.is_empty() { path.strip_prefix("/admin/users/").unwrap_or("") } else { id };
    if id.is_empty() {
        return err_bad_request(msg.clone(), "Missing user ID");
    }

    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };

    // Only allow safe fields
    let mut data = HashMap::new();
    for key in &["name", "disabled", "avatar_url"] {
        if let Some(val) = body.get(*key) {
            data.insert(key.to_string(), val.clone());
        }
    }
    data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    match db.update(COLLECTION, id, data) {
        Ok(mut record) => {
            record.data.remove("password_hash");
            json_respond(msg.clone(), 200, &record)
        }
        Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "User not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_delete(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) {
        Ok(db) => db,
        Err(r) => return r,
    };
    let path = msg.path();
    let id = msg.var("id");
    let id = if id.is_empty() { path.strip_prefix("/admin/users/").unwrap_or("") } else { id };
    if id.is_empty() {
        return err_bad_request(msg.clone(), "Missing user ID");
    }

    // Soft delete
    match database::soft_delete(db.as_ref(), COLLECTION, id) {
        Ok(_) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
        Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "User not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}
