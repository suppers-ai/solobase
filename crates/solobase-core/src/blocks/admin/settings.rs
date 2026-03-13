use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::ListOptions;
use crate::blocks::helpers::{self, json_map, RecordExt};

const COLLECTION: &str = "settings";

pub async fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/settings") | ("retrieve", "/settings") => handle_list(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/admin/settings/") || path.starts_with("/settings/") => handle_get(ctx, msg).await,
        ("update", _) if path.starts_with("/admin/settings/") => handle_set(ctx, msg).await,
        ("create", "/admin/settings") => handle_set_batch(ctx, msg).await,
        _ => err_not_found(msg, "not found"),
    }
}

async fn handle_list(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db::list(ctx, COLLECTION, &opts).await {
        Ok(result) => {
            // Convert to key-value map
            let mut settings = HashMap::new();
            for record in &result.records {
                let key = record.str_field("key");
                let value = record.data.get("value").cloned().unwrap_or(serde_json::Value::Null);
                if !key.is_empty() {
                    settings.insert(key.to_string(), value);
                }
            }
            json_respond(msg, &settings)
        }
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_get(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let key = path.strip_prefix("/admin/settings/")
        .or_else(|| path.strip_prefix("/settings/"))
        .unwrap_or("");
    if key.is_empty() { return err_bad_request(msg, "Missing setting key"); }

    match db::get_by_field(ctx, COLLECTION, "key", serde_json::Value::String(key.to_string())).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Setting not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_set(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let key = path.strip_prefix("/admin/settings/").unwrap_or("");
    if key.is_empty() { return err_bad_request(msg, "Missing setting key"); }

    #[derive(serde::Deserialize)]
    struct Req { value: serde_json::Value }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    let mut data = json_map(serde_json::json!({
        "key": key,
        "value": body.value,
        "updated_by": msg.user_id()
    }));
    helpers::stamp_updated(&mut data);

    match db::upsert(ctx, COLLECTION, "key", serde_json::Value::String(key.to_string()), data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_set_batch(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    let now = helpers::now_rfc3339();
    let user_id = msg.user_id().to_string();

    for (key, value) in &body {
        let data = json_map(serde_json::json!({
            "key": key,
            "value": value,
            "updated_at": now,
            "updated_by": user_id
        }));
        let _ = db::upsert(ctx, COLLECTION, "key", serde_json::Value::String(key.clone()), data).await;
    }

    json_respond(msg, &serde_json::json!({"updated": body.len()}))
}

pub async fn seed_defaults(ctx: &dyn Context) {
    let count = db::count(ctx, COLLECTION, &[]).await.unwrap_or(0);
    if count > 0 { return; }

    let defaults = vec![
        ("APP_NAME", serde_json::json!("Solobase")),
        ("ALLOW_SIGNUP", serde_json::json!(true)),
        ("ENABLE_OAUTH", serde_json::json!(false)),
        ("PRIMARY_COLOR", serde_json::json!("#6366f1")),
    ];

    for (key, value) in defaults {
        let data = json_map(serde_json::json!({
            "key": key,
            "value": value,
            "created_at": helpers::now_rfc3339()
        }));
        if let Err(e) = db::create(ctx, COLLECTION, data).await {
            tracing::warn!("Failed to seed default setting '{key}': {e}");
        }
    }
}
