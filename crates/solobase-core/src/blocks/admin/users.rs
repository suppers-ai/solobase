use std::collections::HashMap;

use wafer_core::clients::{
    database as db,
    database::{Filter, FilterOp, ListOptions, SortField},
};
use wafer_run::{context::Context, types::*, InputStream, OutputStream};

use super::USER_ROLES_COLLECTION;
use crate::blocks::{
    auth::USERS_COLLECTION as COLLECTION,
    helpers::{self, err_bad_request, err_internal, err_not_found, ok_json, RecordExt},
};

pub async fn handle(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/users") => handle_list(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/admin/users/") => handle_get(ctx, msg).await,
        ("update", _) if path.starts_with("/admin/users/") => handle_update(ctx, msg, input).await,
        ("delete", _) if path.starts_with("/admin/users/") => handle_delete(ctx, msg).await,
        _ => err_not_found("not found"),
    }
}

async fn handle_list(ctx: &dyn Context, msg: &Message) -> OutputStream {
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
                let roles: Vec<String> =
                    match db::list(ctx, USER_ROLES_COLLECTION, &roles_opts).await {
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
            ok_json(&result)
        }
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_get(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let id = msg.var("id").to_string();
    if id.is_empty() {
        // Extract from path
        let path = msg.path().to_string();
        let id = path.strip_prefix("/admin/users/").unwrap_or("").to_string();
        if id.is_empty() {
            return err_bad_request("Missing user ID");
        }
        return get_user(ctx, &id).await;
    }
    get_user(ctx, &id).await
}

async fn get_user(ctx: &dyn Context, id: &str) -> OutputStream {
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
            let roles: Vec<String> = match db::list(ctx, USER_ROLES_COLLECTION, &roles_opts).await {
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
            ok_json(&resp)
        }
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("User not found"),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_update(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let path = msg.path();
    let id = msg.var("id");
    let id = if id.is_empty() {
        path.strip_prefix("/admin/users/").unwrap_or("")
    } else {
        id
    };
    if id.is_empty() {
        return err_bad_request("Missing user ID");
    }

    // Prevent admin from disabling themselves
    let current_user_id = msg.user_id();
    let raw = input.collect_to_bytes().await;
    let body: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };
    if id == current_user_id {
        if let Some(disabled) = body.get("disabled") {
            if disabled == &serde_json::Value::Bool(true) || disabled == &serde_json::json!(1) {
                return err_bad_request("Cannot disable your own account");
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
            ok_json(&record)
        }
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("User not found"),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_delete(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let id = msg.var("id");
    let id = if id.is_empty() {
        path.strip_prefix("/admin/users/").unwrap_or("")
    } else {
        id
    };
    if id.is_empty() {
        return err_bad_request("Missing user ID");
    }

    // Prevent admin from deleting themselves
    if id == msg.user_id() {
        return err_bad_request("Cannot delete your own account");
    }

    // Soft delete
    match db::soft_delete(ctx, COLLECTION, id).await {
        Ok(_) => ok_json(&serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("User not found"),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}
