use crate::blocks::helpers::{self, RecordExt};
use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

const COLLECTION: &str = "auth_users";

pub async fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/users") => handle_list(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/admin/users/") => handle_get(ctx, msg).await,
        ("update", _) if path.starts_with("/admin/users/") => handle_update(ctx, msg).await,
        ("delete", _) if path.starts_with("/admin/users/") => handle_delete(ctx, msg).await,
        _ => err_not_found(msg, "not found"),
    }
}

async fn handle_list(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let (page, page_size, _) = msg.pagination_params(20);
    let search = msg.query("search").to_string();

    let mut filters = vec![Filter {
        field: "deleted_at".to_string(),
        operator: FilterOp::IsNull,
        value: serde_json::Value::Null,
    }];

    if !search.is_empty() {
        filters.push(Filter {
            field: "email".to_string(),
            operator: FilterOp::Like,
            value: serde_json::Value::String(format!("%{}%", search)),
        });
    }

    let sort = vec![SortField {
        field: "created_at".to_string(),
        desc: true,
    }];

    match db::paginated_list(
        ctx,
        COLLECTION,
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await
    {
        Ok(mut result) => {
            // Strip password hashes and enrich with roles
            for record in &mut result.records {
                record.data.remove("password_hash");
                let roles_opts = ListOptions {
                    filters: vec![Filter {
                        field: "user_id".to_string(),
                        operator: FilterOp::Equal,
                        value: serde_json::Value::String(record.id.clone()),
                    }],
                    ..Default::default()
                };
                let roles: Vec<String> = match db::list(ctx, "iam_user_roles", &roles_opts).await {
                    Ok(r) => r
                        .records
                        .iter()
                        .map(|rec| rec.str_field("role").to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    Err(_) => Vec::new(),
                };
                record
                    .data
                    .insert("roles".to_string(), serde_json::json!(roles));
            }
            json_respond(msg, &result)
        }
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_get(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let id = msg.var("id").to_string();
    if id.is_empty() {
        // Extract from path
        let path = msg.path().to_string();
        let id = path.strip_prefix("/admin/users/").unwrap_or("").to_string();
        if id.is_empty() {
            return err_bad_request(msg, "Missing user ID");
        }
        return get_user(ctx, msg, &id).await;
    }
    get_user(ctx, msg, &id).await
}

async fn get_user(ctx: &dyn Context, msg: &mut Message, id: &str) -> Result_ {
    match db::get(ctx, COLLECTION, id).await {
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
            let roles: Vec<String> = match db::list(ctx, "iam_user_roles", &roles_opts).await {
                Ok(r) => r
                    .records
                    .iter()
                    .map(|rec| rec.str_field("role").to_string())
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
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "User not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_update(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = msg.var("id");
    let id = if id.is_empty() {
        path.strip_prefix("/admin/users/").unwrap_or("")
    } else {
        id
    };
    if id.is_empty() {
        return err_bad_request(msg, "Missing user ID");
    }

    // Prevent admin from disabling themselves
    let current_user_id = msg.user_id();
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    if id == current_user_id {
        if let Some(disabled) = body.get("disabled") {
            if disabled == &serde_json::Value::Bool(true) || disabled == &serde_json::json!(1) {
                return err_bad_request(msg, "Cannot disable your own account");
            }
        }
    }

    // Only allow safe fields
    let mut data = HashMap::new();
    for key in &["name", "disabled", "avatar_url"] {
        if let Some(val) = body.get(*key) {
            data.insert(key.to_string(), val.clone());
        }
    }
    helpers::stamp_updated(&mut data);

    match db::update(ctx, COLLECTION, id, data).await {
        Ok(mut record) => {
            record.data.remove("password_hash");
            json_respond(msg, &record)
        }
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "User not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_delete(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = msg.var("id");
    let id = if id.is_empty() {
        path.strip_prefix("/admin/users/").unwrap_or("")
    } else {
        id
    };
    if id.is_empty() {
        return err_bad_request(msg, "Missing user ID");
    }

    // Prevent admin from deleting themselves
    if id == msg.user_id() {
        return err_bad_request(msg, "Cannot delete your own account");
    }

    // Soft delete
    match db::soft_delete(ctx, COLLECTION, id).await {
        Ok(_) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "User not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}
