use crate::blocks::helpers::{self, json_map, RecordExt};
use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::database::ListOptions;
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

const COLLECTION: &str = "variables";
const MASKED_VALUE: &str = "********";

pub async fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/settings/all") => handle_list_full(ctx, msg).await,
        ("retrieve", "/admin/settings") | ("retrieve", "/settings") => handle_list(ctx, msg).await,
        ("retrieve", _)
            if path.starts_with("/admin/settings/") || path.starts_with("/settings/") =>
        {
            handle_get(ctx, msg).await
        }
        ("update", _) if path.starts_with("/admin/settings/") => handle_set(ctx, msg).await,
        ("create", "/admin/settings") => handle_create(ctx, msg).await,
        ("delete", _) if path.starts_with("/admin/settings/") => handle_delete(ctx, msg).await,
        _ => err_not_found(msg, "not found"),
    }
}

/// System variable keys that cannot be deleted.
const SYSTEM_KEYS: &[&str] = &[
    "JWT_SECRET",
    "APP_NAME",
    "ALLOW_SIGNUP",
    "ENABLE_OAUTH",
    "PRIMARY_COLOR",
    "POST_LOGIN_REDIRECT",
];

/// Security-critical keys that cannot be set to empty via API (prevents lockout).
const PROTECTED_KEYS: &[&str] = &["JWT_SECRET"];

async fn handle_list_full(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions {
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, COLLECTION, &opts).await {
        Ok(result) => {
            let vars: Vec<_> = result
                .records
                .iter()
                .map(|record| {
                    let key = record.str_field("key").to_string();
                    let is_sensitive = record.i64_field("sensitive") == 1;
                    let is_system = SYSTEM_KEYS.contains(&key.as_str());
                    // Mask sensitive values even in the "full" listing
                    let value = if is_sensitive {
                        MASKED_VALUE.to_string()
                    } else {
                        record.str_field("value").to_string()
                    };
                    serde_json::json!({
                        "key": key,
                        "name": record.str_field("name"),
                        "description": record.str_field("description"),
                        "value": value,
                        "warning": record.str_field("warning"),
                        "sensitive": is_sensitive,
                        "system": is_system,
                        "updated_at": record.str_field("updated_at"),
                    })
                })
                .collect();
            json_respond(msg, &vars)
        }
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_list(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let opts = ListOptions {
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, COLLECTION, &opts).await {
        Ok(result) => {
            // Convert to key-value map, masking sensitive values
            let mut settings = HashMap::new();
            for record in &result.records {
                let key = record.str_field("key");
                let is_sensitive = record.i64_field("sensitive") == 1;
                let value = if is_sensitive {
                    serde_json::Value::String(MASKED_VALUE.to_string())
                } else {
                    record
                        .data
                        .get("value")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                };
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
    let key = path
        .strip_prefix("/admin/settings/")
        .or_else(|| path.strip_prefix("/settings/"))
        .unwrap_or("");
    if key.is_empty() {
        return err_bad_request(msg, "Missing setting key");
    }

    match db::get_by_field(
        ctx,
        COLLECTION,
        "key",
        serde_json::Value::String(key.to_string()),
    )
    .await
    {
        Ok(mut record) => {
            let is_sensitive = record.i64_field("sensitive") == 1;
            if is_sensitive {
                record.data.insert(
                    "value".to_string(),
                    serde_json::Value::String(MASKED_VALUE.to_string()),
                );
            }
            json_respond(msg, &record)
        }
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Setting not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_set(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let key = path.strip_prefix("/admin/settings/").unwrap_or("");
    if key.is_empty() {
        return err_bad_request(msg, "Missing setting key");
    }

    #[derive(serde::Deserialize)]
    struct Req {
        value: serde_json::Value,
    }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    // Prevent setting protected keys to empty (would break auth)
    if PROTECTED_KEYS.contains(&key) {
        let val_str = body.value.as_str().unwrap_or("");
        if val_str.is_empty() {
            return err_bad_request(msg, &format!("Cannot set {} to an empty value", key));
        }
    }

    let mut data = json_map(serde_json::json!({
        "key": key,
        "value": body.value,
        "updated_by": msg.user_id()
    }));
    helpers::stamp_updated(&mut data);

    match db::upsert(
        ctx,
        COLLECTION,
        "key",
        serde_json::Value::String(key.to_string()),
        data,
    )
    .await
    {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_create(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    #[derive(serde::Deserialize)]
    struct Req {
        key: String,
        value: Option<String>,
        name: Option<String>,
        description: Option<String>,
        sensitive: Option<bool>,
    }
    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    if body.key.is_empty() {
        return err_bad_request(msg, "key is required");
    }

    let data = json_map(serde_json::json!({
        "key": body.key,
        "value": body.value.unwrap_or_default(),
        "name": body.name.unwrap_or_else(|| body.key.clone()),
        "description": body.description.unwrap_or_default(),
        "sensitive": if body.sensitive.unwrap_or(false) { 1 } else { 0 },
        "updated_by": msg.user_id(),
        "created_at": helpers::now_rfc3339()
    }));
    match db::create(ctx, COLLECTION, data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_delete(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let key = path.strip_prefix("/admin/settings/").unwrap_or("");
    if key.is_empty() {
        return err_bad_request(msg, "Missing setting key");
    }

    if SYSTEM_KEYS.contains(&key) {
        return err_bad_request(msg, "Cannot delete system variable");
    }

    match db::get_by_field(
        ctx,
        COLLECTION,
        "key",
        serde_json::Value::String(key.to_string()),
    )
    .await
    {
        Ok(record) => match db::delete(ctx, COLLECTION, &record.id).await {
            Ok(_) => json_respond(msg, &serde_json::json!({"deleted": key})),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        },
        Err(_) => err_not_found(msg, "Setting not found"),
    }
}

pub async fn seed_defaults(ctx: &dyn Context) {
    // (key, name, description, default_value, warning, sensitive)
    let defaults: &[(&str, &str, &str, &str, &str, i32)] = &[
        (
            "APP_NAME",
            "App Name",
            "Display name shown in the UI and emails",
            "Solobase",
            "",
            0,
        ),
        (
            "ALLOW_SIGNUP",
            "Allow Signup",
            "Allow new users to register",
            "true",
            "",
            0,
        ),
        (
            "ENABLE_OAUTH",
            "Enable OAuth",
            "Enable third-party OAuth login",
            "false",
            "",
            0,
        ),
        (
            "PRIMARY_COLOR",
            "Primary Color",
            "Brand color used in the UI",
            "#6366f1",
            "",
            0,
        ),
        (
            "POST_LOGIN_REDIRECT",
            "Post-Login Redirect",
            "URL to redirect to after login",
            "/b/admin/",
            "",
            0,
        ),
        (
            "JWT_SECRET",
            "JWT Secret",
            "Secret key used to sign authentication tokens",
            "",
            "Changing this will invalidate all existing user sessions",
            1,
        ),
    ];

    for &(key, name, description, default_value, warning, sensitive) in defaults {
        // Check if key exists already
        let exists = db::get_by_field(
            ctx,
            COLLECTION,
            "key",
            serde_json::Value::String(key.to_string()),
        )
        .await
        .is_ok();
        if exists {
            // Update metadata (name, description, warning) but keep existing value
            let data = json_map(serde_json::json!({
                "name": name,
                "description": description,
                "warning": warning,
                "sensitive": sensitive,
            }));
            let _ = db::upsert(
                ctx,
                COLLECTION,
                "key",
                serde_json::Value::String(key.to_string()),
                data,
            )
            .await;
        } else if !default_value.is_empty() {
            let data = json_map(serde_json::json!({
                "key": key,
                "name": name,
                "description": description,
                "value": default_value,
                "warning": warning,
                "sensitive": sensitive,
                "created_at": helpers::now_rfc3339()
            }));
            let _ = db::create(ctx, COLLECTION, data).await;
        }
    }
}
