use std::collections::HashMap;

use wafer_core::clients::{database as db, database::ListOptions};
use wafer_run::{
    context::Context,
    types::{self, *},
    InputStream, OutputStream,
};

use super::VARIABLES_COLLECTION as COLLECTION;
use crate::blocks::helpers::{
    self, err_bad_request, err_internal, err_not_found, json_map, ok_json, RecordExt,
};
const MASKED_VALUE: &str = "********";

pub async fn handle(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/settings/all") => handle_list_full(ctx).await,
        ("retrieve", "/admin/settings") | ("retrieve", "/settings") => handle_list(ctx).await,
        ("retrieve", _)
            if path.starts_with("/admin/settings/") || path.starts_with("/settings/") =>
        {
            handle_get(ctx, msg).await
        }
        ("update", _) if path.starts_with("/admin/settings/") => handle_set(ctx, msg, input).await,
        ("create", "/admin/settings") => handle_create(ctx, msg, input).await,
        ("delete", _) if path.starts_with("/admin/settings/") => handle_delete(ctx, msg).await,
        _ => err_not_found("not found"),
    }
}

/// Validate a URL-type config value against SSRF attacks.
/// Empty values are allowed (clears the setting).
/// Relative paths starting with `/` (but not `//`) are allowed.
/// Otherwise must be HTTPS, or http://localhost for local development.
/// Private/internal IP ranges are blocked to prevent SSRF.
fn validate_url_value(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Ok(());
    }
    // Allow relative paths
    if value.starts_with('/') && !value.starts_with("//") {
        return Ok(());
    }
    // Block newlines (header injection)
    if value.contains('\n') || value.contains('\r') {
        return Err("URL must not contain newlines".to_string());
    }
    // Must be https:// or http://localhost for dev
    let is_localhost = value.starts_with("http://localhost");
    if !value.starts_with("https://") && !is_localhost {
        return Err("URL must use HTTPS (or http://localhost for development)".to_string());
    }
    // Extract hostname and check for private/internal IPs
    let host = value
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .and_then(|authority| {
            // Handle [IPv6]:port
            if authority.starts_with('[') {
                authority.strip_prefix('[')?.split(']').next()
            } else {
                authority.split(':').next()
            }
        })
        .unwrap_or("");

    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4) => {
                let is_blocked = v4.is_private()       // 10.x, 172.16-31.x, 192.168.x
                    || v4.is_loopback()                // 127.x
                    || v4.is_link_local()              // 169.254.x
                    || v4.octets()[0] == 0; // 0.0.0.0/8
                if is_blocked && !is_localhost {
                    return Err("URL must not point to private/internal IP addresses".to_string());
                }
            }
            std::net::IpAddr::V6(v6) => {
                if v6.is_loopback() {
                    return Err("URL must not point to loopback address".to_string());
                }
                // Block IPv4-mapped IPv6 addresses (::ffff:10.x.x.x etc.)
                if let Some(v4) = v6.to_ipv4_mapped() {
                    if v4.is_private() || v4.is_loopback() || v4.is_link_local() {
                        return Err(
                            "URL must not point to private/internal IP addresses".to_string()
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_list_full(ctx: &dyn Context) -> OutputStream {
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
                    let is_sensitive = record.i64_field("sensitive") == 1
                        || key.ends_with("_SECRET")
                        || key.ends_with("_KEY");
                    let is_system = key.starts_with("SOLOBASE_SHARED__");
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
            ok_json(&vars)
        }
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_list(ctx: &dyn Context) -> OutputStream {
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
            ok_json(&settings)
        }
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_get(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let key = path
        .strip_prefix("/admin/settings/")
        .or_else(|| path.strip_prefix("/settings/"))
        .unwrap_or("");
    if key.is_empty() {
        return err_bad_request("Missing setting key");
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
            ok_json(&record)
        }
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("Setting not found"),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_set(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let path = msg.path();
    let key = path.strip_prefix("/admin/settings/").unwrap_or("");
    if key.is_empty() {
        return err_bad_request("Missing setting key");
    }

    #[derive(serde::Deserialize)]
    struct Req {
        value: serde_json::Value,
    }
    let raw = input.collect_to_bytes().await;
    let body: Req = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Prevent setting sensitive keys (secrets/keys) to empty (would break auth)
    if key.ends_with("_SECRET") || key.ends_with("_KEY") {
        let val_str = body.value.as_str().unwrap_or("");
        if val_str.is_empty() {
            return err_bad_request(&format!("Cannot set {} to an empty value", key));
        }
    }

    // Validate URL-type keys
    if key.ends_with("_URL") {
        let val_str = body.value.as_str().unwrap_or("");
        if let Err(e) = validate_url_value(val_str) {
            return err_bad_request(&format!("Invalid value for {}: {}", key, e));
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
        Ok(record) => ok_json(&record),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_create(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct Req {
        key: String,
        value: Option<String>,
        name: Option<String>,
        description: Option<String>,
        sensitive: Option<bool>,
    }
    let raw = input.collect_to_bytes().await;
    let body: Req = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };
    if body.key.is_empty() {
        return err_bad_request("key is required");
    }

    // Validate URL-type keys
    if body.key.ends_with("_URL") {
        let val_str = body.value.as_deref().unwrap_or("");
        if let Err(e) = validate_url_value(val_str) {
            return err_bad_request(&format!("Invalid value for {}: {}", body.key, e));
        }
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
        Ok(record) => ok_json(&record),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_delete(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let key = path.strip_prefix("/admin/settings/").unwrap_or("");
    if key.is_empty() {
        return err_bad_request("Missing setting key");
    }

    if key.starts_with("SOLOBASE_SHARED__") {
        return err_bad_request(&format!("Cannot delete shared system variable: {}", key));
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
            Ok(_) => ok_json(&serde_json::json!({"deleted": key})),
            Err(e) => err_internal(&format!("Database error: {e}")),
        },
        Err(_) => err_not_found("Setting not found"),
    }
}

pub async fn seed_defaults(ctx: &dyn Context) {
    let vars = crate::config_vars::shared_config_vars();

    for var in &vars {
        let sensitive: i32 = if var.input_type == types::InputType::Password {
            1
        } else {
            0
        };
        let name = if var.name.is_empty() {
            &var.key
        } else {
            &var.name
        };

        // Check if key exists already
        let exists = db::get_by_field(
            ctx,
            COLLECTION,
            "key",
            serde_json::Value::String(var.key.clone()),
        )
        .await
        .is_ok();
        if exists {
            // Update metadata (name, description, warning) but keep existing value
            let data = json_map(serde_json::json!({
                "name": name,
                "description": var.description,
                "warning": var.warning,
                "sensitive": sensitive,
            }));
            let _ = db::upsert(
                ctx,
                COLLECTION,
                "key",
                serde_json::Value::String(var.key.clone()),
                data,
            )
            .await;
        } else if !var.default.is_empty() {
            let data = json_map(serde_json::json!({
                "key": var.key,
                "name": name,
                "description": var.description,
                "value": var.default,
                "warning": var.warning,
                "sensitive": sensitive,
                "created_at": helpers::now_rfc3339()
            }));
            let _ = db::create(ctx, COLLECTION, data).await;
        }
    }
}
