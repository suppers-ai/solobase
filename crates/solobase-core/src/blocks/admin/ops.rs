//! Shared admin-mutation domain layer.
//!
//! Both admin surfaces — the JSON API (`users.rs` / `iam.rs` / `settings.rs`)
//! and the SSR htmx pages (`pages/users.rs` / `pages/variables.rs`) — drive the
//! same handful of privileged operations: disable / enable / delete a user,
//! create / delete a role, create / update a config variable. Historically each
//! surface hand-copied the business rules, and the copies drifted on exactly the
//! things that matter most:
//!
//! * the JSON path issued **zero audit-log rows** for any of these mutations;
//! * the SSR variable create/update path skipped **URL/SSRF validation**;
//! * the SSR variable read path masked secrets on the `sensitive` flag only,
//!   ignoring the SEC-060 `_SECRET` / `_KEY` suffix rule.
//!
//! This module is the single owner of those rules so the surfaces can't diverge
//! again. Every function here performs the guard checks, the validation, the
//! audit-log write, and the database mutation; the callers keep only their
//! response shape (a JSON record vs. an htmx fragment + toast). Guard/validation
//! failures are returned as a ready-to-emit [`OutputStream`] error via the
//! shared `helpers::err_*` constructors, so both surfaces report failures
//! identically.

use std::collections::HashMap;

use wafer_block::db::{Filter, FilterOp};
use wafer_core::clients::database as db;
use wafer_run::{context::Context, ErrorCode, Message, OutputStream};

use super::logs::audit_log;
use super::settings::VARIABLES_TABLE;
use super::{ROLES_TABLE, USER_ROLES_TABLE};
use crate::blocks::auth::USERS_TABLE;
use crate::blocks::helpers::{
    self, err_bad_request, err_forbidden, err_internal, err_not_found, RecordExt,
};

/// Masked placeholder shown in place of a sensitive value.
pub(super) const MASKED_VALUE: &str = "********";

/// SEC-060: a config value is sensitive when the row's `sensitive` flag is set
/// **or** the key follows the `_SECRET` / `_KEY` suffix convention. Both
/// surfaces (JSON `handle_list*` and the SSR variable tables) must agree on
/// this — masking on the flag alone leaked a `*_SECRET` value whenever an admin
/// forgot to flip the flag. This is the single source of truth for that rule.
pub(super) fn is_sensitive_key(key: &str, sensitive_flag: i64) -> bool {
    sensitive_flag == 1 || key.ends_with("_SECRET") || key.ends_with("_KEY")
}

/// Validate a URL-type config value against SSRF attacks.
///
/// Empty values are allowed (clears the setting). Relative paths starting with
/// a single `/` are allowed. Otherwise the value must be HTTPS (or
/// `http://localhost` for local development), must not contain newlines (header
/// injection), and must not resolve to a private/internal/loopback IP.
///
/// Used by every variable create/update path — JSON **and** SSR — so a value an
/// admin can't set through the API can't be smuggled in through the page form.
pub(super) fn validate_url_value(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Ok(());
    }
    // Allow relative paths.
    if value.starts_with('/') && !value.starts_with("//") {
        return Ok(());
    }
    // Block newlines (header injection).
    if value.contains('\n') || value.contains('\r') {
        return Err("URL must not contain newlines".to_string());
    }
    // Must be https:// or http://localhost for dev.
    let is_localhost = value.starts_with("http://localhost");
    if !value.starts_with("https://") && !is_localhost {
        return Err("URL must use HTTPS (or http://localhost for development)".to_string());
    }
    // Extract hostname and check for private/internal IPs.
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

/// Bulk-fetch the roles assigned to each of `user_ids` in a single `In`-filter
/// query, bucketed back into a `user_id -> [role]` map.
///
/// Replaces the per-user `list_all(USER_ROLES_TABLE, …)` loop that both the
/// JSON `users::handle_list` / `get_user` paths and the SSR
/// `pages/users.rs::user_row_fragment` re-implemented. The single-row lookup is
/// the `user_ids = [one]` case, so this is the only roles-fetch helper.
///
/// On query failure every requested user maps to an empty role list (the prior
/// per-surface code swallowed the error the same way).
pub(super) async fn fetch_roles(
    ctx: &dyn Context,
    user_ids: &[&str],
) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    if user_ids.is_empty() {
        return out;
    }
    let values: Vec<serde_json::Value> = user_ids
        .iter()
        .map(|id| serde_json::Value::String((*id).to_string()))
        .collect();
    let filters = vec![Filter {
        field: "user_id".to_string(),
        operator: FilterOp::In,
        value: serde_json::Value::Array(values),
    }];
    if let Ok(rows) = db::list_all(ctx, USER_ROLES_TABLE, filters).await {
        for rec in &rows {
            let uid = rec.str_field("user_id").to_string();
            let role = rec.str_field("role").to_string();
            if !uid.is_empty() && !role.is_empty() {
                out.entry(uid).or_default().push(role);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// User mutations
// ---------------------------------------------------------------------------

/// Set the `disabled` flag on a user (true = disable, false = enable), writing
/// an audit-log row. The self-disable guard only applies when disabling.
///
/// Returns the updated [`db::Record`] (password hash stripped) on success.
pub(super) async fn set_user_disabled(
    ctx: &dyn Context,
    msg: &Message,
    user_id: &str,
    disabled: bool,
) -> Result<db::Record, OutputStream> {
    let admin_id = msg.user_id().to_string();
    // Prevent admins from disabling themselves (lockout). Enabling yourself is
    // harmless, so the guard is disable-only.
    if disabled && admin_id == user_id {
        return Err(err_bad_request("Cannot disable your own account"));
    }

    let mut data = HashMap::new();
    data.insert("disabled".to_string(), serde_json::json!(disabled));
    helpers::stamp_updated(&mut data);

    let record = match db::update(ctx, USERS_TABLE, user_id, data).await {
        Ok(mut record) => {
            record.data.remove("password_hash");
            record
        }
        Err(e) if e.code == ErrorCode::NotFound => return Err(err_not_found("User not found")),
        Err(e) => return Err(err_internal("Database error", e)),
    };

    let action = if disabled { "user.disable" } else { "user.enable" };
    audit_log(
        ctx,
        &admin_id,
        action,
        &format!("users/{user_id}"),
        msg.remote_addr(),
    )
    .await;
    Ok(record)
}

/// Soft-delete a user, writing an audit-log row. Rejects self-deletion.
pub(super) async fn delete_user(
    ctx: &dyn Context,
    msg: &Message,
    user_id: &str,
) -> Result<(), OutputStream> {
    let admin_id = msg.user_id().to_string();
    if admin_id == user_id {
        return Err(err_bad_request("Cannot delete your own account"));
    }

    match db::soft_delete(ctx, USERS_TABLE, user_id).await {
        Ok(_) => {}
        Err(e) if e.code == ErrorCode::NotFound => return Err(err_not_found("User not found")),
        Err(e) => return Err(err_internal("Database error", e)),
    }

    audit_log(
        ctx,
        &admin_id,
        "user.delete",
        &format!("users/{user_id}"),
        msg.remote_addr(),
    )
    .await;
    Ok(())
}

/// Apply the whitelisted user-profile fields (`name`, `disabled`,
/// `avatar_url`) from `body`, writing an audit-log row. Enforces the
/// self-disable guard, mirroring [`set_user_disabled`].
///
/// Returns the updated record (password hash stripped).
pub(super) async fn update_user_fields(
    ctx: &dyn Context,
    msg: &Message,
    user_id: &str,
    body: &HashMap<String, serde_json::Value>,
) -> Result<db::Record, OutputStream> {
    let admin_id = msg.user_id().to_string();
    if admin_id == user_id {
        if let Some(disabled) = body.get("disabled") {
            if disabled == &serde_json::Value::Bool(true) || disabled == &serde_json::json!(1) {
                return Err(err_bad_request("Cannot disable your own account"));
            }
        }
    }

    let mut data = HashMap::new();
    for key in &["name", "disabled", "avatar_url"] {
        if let Some(val) = body.get(*key) {
            data.insert(key.to_string(), val.clone());
        }
    }
    helpers::stamp_updated(&mut data);

    let record = match db::update(ctx, USERS_TABLE, user_id, data).await {
        Ok(mut record) => {
            record.data.remove("password_hash");
            record
        }
        Err(e) if e.code == ErrorCode::NotFound => return Err(err_not_found("User not found")),
        Err(e) => return Err(err_internal("Database error", e)),
    };

    audit_log(
        ctx,
        &admin_id,
        "user.update",
        &format!("users/{user_id}"),
        msg.remote_addr(),
    )
    .await;
    Ok(record)
}

// ---------------------------------------------------------------------------
// Role mutations
// ---------------------------------------------------------------------------

/// Create a role with the given name and optional description, writing an
/// audit-log row. `description` / `permissions` default to empty.
pub(super) async fn create_role(
    ctx: &dyn Context,
    msg: &Message,
    name: &str,
    description: Option<&str>,
    permissions: Option<Vec<String>>,
) -> Result<db::Record, OutputStream> {
    if name.is_empty() {
        return Err(err_bad_request("Role name is required"));
    }
    let admin_id = msg.user_id().to_string();

    let mut data = helpers::json_map(serde_json::json!({
        "name": name,
        "description": description.unwrap_or_default(),
        "permissions": permissions.unwrap_or_default(),
        "is_system": false,
    }));
    helpers::stamp_created(&mut data);

    let record = match db::create(ctx, ROLES_TABLE, data).await {
        Ok(record) => record,
        Err(e) => return Err(err_internal("Database error", e)),
    };

    audit_log(
        ctx,
        &admin_id,
        "role.create",
        &format!("roles/{name}"),
        msg.remote_addr(),
    )
    .await;
    Ok(record)
}

/// Delete a role, writing an audit-log row. Rejects deletion of system roles
/// (the `is_system` flag), which would break auth.
pub(super) async fn delete_role(
    ctx: &dyn Context,
    msg: &Message,
    role_id: &str,
) -> Result<(), OutputStream> {
    if role_id.is_empty() {
        return Err(err_bad_request("Missing role ID"));
    }
    let admin_id = msg.user_id().to_string();

    // Protect system roles. A missing row falls through to db::delete, which
    // returns NotFound below.
    if let Ok(role) = db::get(ctx, ROLES_TABLE, role_id).await {
        if role.bool_field("is_system") {
            return Err(err_forbidden("Cannot delete system role"));
        }
    }

    match db::delete(ctx, ROLES_TABLE, role_id).await {
        Ok(()) => {}
        Err(e) if e.code == ErrorCode::NotFound => return Err(err_not_found("Role not found")),
        Err(e) => return Err(err_internal("Database error", e)),
    }

    audit_log(
        ctx,
        &admin_id,
        "role.delete",
        &format!("roles/{role_id}"),
        msg.remote_addr(),
    )
    .await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Variable mutations
// ---------------------------------------------------------------------------

/// Create a config variable, writing an audit-log row. Validates `_URL` keys
/// against [`validate_url_value`] (SSRF). `key` must be non-empty.
pub(super) async fn create_variable(
    ctx: &dyn Context,
    msg: &Message,
    key: &str,
    value: &str,
    name: Option<&str>,
    description: Option<&str>,
    sensitive: bool,
) -> Result<db::Record, OutputStream> {
    if key.is_empty() {
        return Err(err_bad_request("Key is required"));
    }
    let admin_id = msg.user_id().to_string();

    // Validate URL-type keys (SSRF) on both surfaces.
    if key.ends_with("_URL") {
        if let Err(e) = validate_url_value(value) {
            return Err(err_bad_request(&format!("Invalid value for {key}: {e}")));
        }
    }

    let mut data = helpers::json_map(serde_json::json!({
        "key": key,
        "value": value,
        "name": name.filter(|n| !n.is_empty()).unwrap_or(key),
        "description": description.unwrap_or_default(),
        "sensitive": if sensitive { 1 } else { 0 },
        "updated_by": msg.user_id(),
    }));
    helpers::stamp_created(&mut data);

    let record = match db::create(ctx, VARIABLES_TABLE, data).await {
        Ok(record) => record,
        Err(e) => return Err(err_internal("Database error", e)),
    };

    audit_log(
        ctx,
        &admin_id,
        "variable.create",
        &format!("variables/{key}"),
        msg.remote_addr(),
    )
    .await;
    Ok(record)
}

/// Fields a variable update may change. `None` leaves the existing column
/// untouched.
#[derive(Default)]
pub(super) struct VariableUpdate<'a> {
    pub value: Option<&'a str>,
    pub description: Option<&'a str>,
}

/// Update a config variable identified by `key` (upsert on the `key` column),
/// writing an audit-log row. Enforces the sensitive-empty guard (a `_SECRET` /
/// `_KEY` value can't be cleared) and the `_URL` SSRF validation on both
/// surfaces.
///
/// Returns the upserted record.
pub(super) async fn update_variable(
    ctx: &dyn Context,
    msg: &Message,
    key: &str,
    update: VariableUpdate<'_>,
) -> Result<db::Record, OutputStream> {
    if key.is_empty() {
        return Err(err_bad_request("Missing setting key"));
    }
    let admin_id = msg.user_id().to_string();

    if let Some(value) = update.value {
        // Prevent clearing a secret/key (would break auth).
        if (key.ends_with("_SECRET") || key.ends_with("_KEY")) && value.is_empty() {
            return Err(err_bad_request(&format!(
                "Cannot set {key} to an empty value"
            )));
        }
        // Validate URL-type keys (SSRF) on both surfaces.
        if key.ends_with("_URL") {
            if let Err(e) = validate_url_value(value) {
                return Err(err_bad_request(&format!("Invalid value for {key}: {e}")));
            }
        }
    }

    let mut data = HashMap::new();
    if let Some(value) = update.value {
        data.insert("value".to_string(), serde_json::json!(value));
    }
    if let Some(description) = update.description {
        data.insert("description".to_string(), serde_json::json!(description));
    }
    data.insert("updated_by".to_string(), serde_json::json!(msg.user_id()));
    helpers::stamp_updated(&mut data);

    let record = match db::upsert(
        ctx,
        VARIABLES_TABLE,
        "key",
        serde_json::Value::String(key.to_string()),
        data,
    )
    .await
    {
        Ok(record) => record,
        Err(e) => return Err(err_internal("Database error", e)),
    };

    audit_log(
        ctx,
        &admin_id,
        "variable.update",
        &format!("variables/{key}"),
        msg.remote_addr(),
    )
    .await;
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_sensitive_key_honors_flag_and_suffix() {
        // Flag set → sensitive regardless of name.
        assert!(is_sensitive_key("PLAIN", 1));
        // SEC-060: suffix makes it sensitive even when the flag is clear.
        assert!(is_sensitive_key("STRIPE_SECRET", 0));
        assert!(is_sensitive_key("JWT_KEY", 0));
        // Neither flag nor suffix → not sensitive.
        assert!(!is_sensitive_key("SITE_NAME", 0));
    }

    #[test]
    fn validate_url_value_blocks_ssrf_and_allows_safe() {
        assert!(validate_url_value("").is_ok());
        assert!(validate_url_value("/relative/path").is_ok());
        assert!(validate_url_value("https://example.com/ok").is_ok());
        assert!(validate_url_value("http://localhost:8080").is_ok());
        // SSRF vectors.
        assert!(validate_url_value("http://example.com").is_err()); // not https
        assert!(validate_url_value("https://10.0.0.1/admin").is_err());
        assert!(validate_url_value("https://192.168.1.1").is_err());
        assert!(validate_url_value("https://127.0.0.1").is_err());
        assert!(validate_url_value("https://example.com\r\nHost: evil").is_err());
    }
}
