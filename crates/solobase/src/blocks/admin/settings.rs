use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{self, DatabaseService, ListOptions};
use super::get_db;

const COLLECTION: &str = "settings";

pub fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/settings") | ("retrieve", "/settings") => handle_list(ctx, msg),
        ("retrieve", _) if path.starts_with("/admin/settings/") || path.starts_with("/settings/") => handle_get(ctx, msg),
        ("update", _) if path.starts_with("/admin/settings/") => handle_set(ctx, msg),
        ("create", "/admin/settings") => handle_set_batch(ctx, msg),
        _ => err_not_found(msg.clone(), "not found"),
    }
}

fn handle_list(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let opts = ListOptions { limit: 1000, ..Default::default() };
    match db.list(COLLECTION, &opts) {
        Ok(result) => {
            // Convert to key-value map
            let mut settings = HashMap::new();
            for record in &result.records {
                let key = record.data.get("key").and_then(|v| v.as_str()).unwrap_or("");
                let value = record.data.get("value").cloned().unwrap_or(serde_json::Value::Null);
                if !key.is_empty() {
                    settings.insert(key.to_string(), value);
                }
            }
            json_respond(msg.clone(), 200, &settings)
        }
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_get(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let path = msg.path();
    let key = path.strip_prefix("/admin/settings/")
        .or_else(|| path.strip_prefix("/settings/"))
        .unwrap_or("");
    if key.is_empty() { return err_bad_request(msg.clone(), "Missing setting key"); }

    match database::get_by_field(db.as_ref(), COLLECTION, "key", serde_json::Value::String(key.to_string())) {
        Ok(record) => json_respond(msg.clone(), 200, &record),
        Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "Setting not found"),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_set(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let path = msg.path();
    let key = path.strip_prefix("/admin/settings/").unwrap_or("");
    if key.is_empty() { return err_bad_request(msg.clone(), "Missing setting key"); }

    #[derive(serde::Deserialize)]
    struct Req { value: serde_json::Value }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };

    let mut data = HashMap::new();
    data.insert("key".to_string(), serde_json::Value::String(key.to_string()));
    data.insert("value".to_string(), body.value);
    data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    data.insert("updated_by".to_string(), serde_json::Value::String(msg.user_id().to_string()));

    match database::upsert(db.as_ref(), COLLECTION, "key", serde_json::Value::String(key.to_string()), data) {
        Ok(record) => json_respond(msg.clone(), 200, &record),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}

fn handle_set_batch(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };

    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };

    let now = chrono::Utc::now().to_rfc3339();
    let user_id = msg.user_id().to_string();

    for (key, value) in &body {
        let mut data = HashMap::new();
        data.insert("key".to_string(), serde_json::Value::String(key.clone()));
        data.insert("value".to_string(), value.clone());
        data.insert("updated_at".to_string(), serde_json::Value::String(now.clone()));
        data.insert("updated_by".to_string(), serde_json::Value::String(user_id.clone()));
        let _ = database::upsert(db.as_ref(), COLLECTION, "key", serde_json::Value::String(key.clone()), data);
    }

    json_respond(msg.clone(), 200, &serde_json::json!({"updated": body.len()}))
}

pub fn seed_defaults(db: &dyn DatabaseService) {
    let count = db.count(COLLECTION, &[]).unwrap_or(0);
    if count > 0 { return; }

    let defaults = vec![
        ("APP_NAME", serde_json::json!("Solobase")),
        ("ALLOW_SIGNUP", serde_json::json!(true)),
        ("ENABLE_OAUTH", serde_json::json!(false)),
        ("PRIMARY_COLOR", serde_json::json!("#6366f1")),
    ];

    for (key, value) in defaults {
        let mut data = HashMap::new();
        data.insert("key".to_string(), serde_json::Value::String(key.to_string()));
        data.insert("value".to_string(), value);
        data.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        if let Err(e) = db.create(COLLECTION, data) {
            tracing::warn!("Failed to seed default setting '{key}': {e}");
        }
    }
}
