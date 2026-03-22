use std::collections::HashMap;

use crate::wafer::block_world::types::*;
use crate::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};

const COLLECTION: &str = "auth_users";

pub fn handle(msg: &Message) -> BlockResult {
    let action = msg_get_meta(msg, "req.action");
    let path = msg_get_meta(msg, "req.resource");

    match (action, path) {
        ("retrieve", "/admin/users") => handle_list(msg),
        ("retrieve", p) if p.starts_with("/admin/users/") => handle_get(msg),
        ("update", p) if p.starts_with("/admin/users/") => handle_update(msg),
        ("delete", p) if p.starts_with("/admin/users/") => handle_delete(msg),
        _ => err_not_found(msg, "not found"),
    }
}

fn handle_list(msg: &Message) -> BlockResult {
    let (page, page_size, _) = pagination_params(msg, 20);
    let search = msg_query(msg, "search");

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

    match db::paginated_list(COLLECTION, page, page_size, filters, sort) {
        Ok(mut result) => {
            // Strip password hashes from response
            for record in &mut result.records {
                record.data.remove("password_hash");
            }
            json_respond(msg, &serde_json::to_value(&result).unwrap_or_default())
        }
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_get(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let id = path.strip_prefix("/admin/users/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing user ID");
    }

    match db::get(COLLECTION, id) {
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
            let roles: Vec<String> = match db::list("iam_user_roles", &roles_opts) {
                Ok(r) => r.records.iter()
                    .map(|rec| str_field(rec, "role").to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                Err(_) => Vec::new(),
            };
            let mut resp = serde_json::to_value(&record).unwrap_or_default();
            if let Some(obj) = resp.as_object_mut() {
                obj.insert("roles".to_string(), serde_json::json!(roles));
            }
            json_respond(msg, &resp)
        }
        Err(e) => {
            let we = convert_error(e);
            if we.code == ErrorCode::NotFound {
                err_not_found(msg, "User not found")
            } else {
                err_internal(msg, &format!("Database error: {}", we.message))
            }
        }
    }
}

fn handle_update(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let id = path.strip_prefix("/admin/users/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing user ID");
    }

    let body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Only allow safe fields
    let mut data = HashMap::new();
    for key in &["name", "disabled", "avatar_url"] {
        if let Some(val) = body.get(*key) {
            data.insert(key.to_string(), val.clone());
        }
    }
    stamp_updated(&mut data);

    match db::update(COLLECTION, id, data) {
        Ok(mut record) => {
            record.data.remove("password_hash");
            json_respond(msg, &serde_json::to_value(&record).unwrap_or_default())
        }
        Err(e) => {
            let we = convert_error(e);
            if we.code == ErrorCode::NotFound {
                err_not_found(msg, "User not found")
            } else {
                err_internal(msg, &format!("Database error: {}", we.message))
            }
        }
    }
}

fn handle_delete(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let id = path.strip_prefix("/admin/users/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing user ID");
    }

    // Soft delete
    match db::soft_delete(COLLECTION, id) {
        Ok(_) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => {
            let we = convert_error(e);
            if we.code == ErrorCode::NotFound {
                err_not_found(msg, "User not found")
            } else {
                err_internal(msg, &format!("Database error: {}", we.message))
            }
        }
    }
}
