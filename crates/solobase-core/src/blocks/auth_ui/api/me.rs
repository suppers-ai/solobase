//! GET / UPDATE /b/auth/api/me — relocated from auth/login.rs in Task 5.

use std::collections::HashMap;

use wafer_core::clients::database as db;
use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

use crate::blocks::{
    auth::{helpers::get_user_roles, USERS_TABLE},
    errors::{error_response, ErrorCode},
    helpers::{err_bad_request, err_internal, err_not_found, ok_json, RecordExt},
};

pub async fn handle_get(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id();
    if user_id.is_empty() {
        return error_response(ErrorCode::NotAuthenticated, "Not authenticated");
    }
    let user = match db::get(ctx, USERS_TABLE, user_id).await {
        Ok(u) => u,
        Err(_) => return err_not_found("User not found"),
    };
    let roles = get_user_roles(ctx, user_id).await;
    ok_json(&serde_json::json!({
        "user": {
            "id": user.id,
            "email": user.str_field("email"),
            "name": user.str_field("name"),
            "roles": roles,
            "created_at": user.str_field("created_at"),
            "avatar_url": user.str_field("avatar_url")
        }
    }))
}

pub async fn handle_update(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let user_id = msg.user_id();
    if user_id.is_empty() {
        return error_response(ErrorCode::NotAuthenticated, "Not authenticated");
    }

    let raw = input.collect_to_bytes().await;
    let body: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Only allow updating certain fields
    let mut data = HashMap::new();
    for key in &["name", "avatar_url"] {
        if let Some(val) = body.get(*key) {
            data.insert(key.to_string(), val.clone());
        }
    }
    crate::blocks::helpers::stamp_updated(&mut data);

    match db::update(ctx, USERS_TABLE, user_id, data).await {
        Ok(user) => {
            let roles = get_user_roles(ctx, user_id).await;
            ok_json(&serde_json::json!({
                "id": user.id,
                "email": user.str_field("email"),
                "name": user.str_field("name"),
                "roles": roles
            }))
        }
        Err(e) => err_internal(&format!("Update failed: {e}")),
    }
}
