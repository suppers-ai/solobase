use std::collections::HashMap;

use wafer_core::clients::database as db;
use wafer_run::{
    context::Context,
    types::{self, *},
    InputStream, OutputStream,
};

use crate::blocks::helpers::{
    self, err_bad_request, err_internal, err_not_found, json_map, ok_json, RecordExt,
};

/// Per-block enable/config settings (one row per block).
///
/// `pub` (not `pub(crate)`) because consumers outside `solobase-core`
/// reference this table by name.
pub const BLOCK_SETTINGS_TABLE: &str = "suppers_ai__admin__block_settings";

/// Admin-managed configuration variables (key/value/scope/sensitive).
///
/// `pub` (not `pub(crate)`) because consumers outside `solobase-core`
/// reference this table by name.
pub const VARIABLES_TABLE: &str = "suppers_ai__admin__variables";

const MASKED_VALUE: &str = "********";

/// SEC-060: unified sensitive-key check used by both `handle_list_full` and
/// `handle_list`. A value is sensitive if the row's `sensitive` flag is 1
/// OR the key name follows the `_SECRET` / `_KEY` suffix convention. Both
/// listing endpoints must agree on this — the previous code masked on the
/// suffix in `list_full` but only on the flag in `list`, so an admin who
/// forgot to flip the `sensitive` flag on a `*_SECRET` key would have it
/// leak through the lightweight listing.
fn is_sensitive_key(key: &str, sensitive_flag: i64) -> bool {
    sensitive_flag == 1 || key.ends_with("_SECRET") || key.ends_with("_KEY")
}

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
    match db::list_all(ctx, VARIABLES_TABLE, vec![]).await {
        Ok(records) => {
            let vars: Vec<_> = records
                .iter()
                .map(|record| {
                    let key = record.str_field("key").to_string();
                    let is_sensitive = is_sensitive_key(&key, record.i64_field("sensitive"));
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
        Err(e) => err_internal("Database error", e),
    }
}

async fn handle_list(ctx: &dyn Context) -> OutputStream {
    match db::list_all(ctx, VARIABLES_TABLE, vec![]).await {
        Ok(records) => {
            // Convert to key-value map, masking sensitive values
            let mut settings = HashMap::new();
            for record in &records {
                let key = record.str_field("key");
                let is_sensitive = is_sensitive_key(key, record.i64_field("sensitive"));
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
        Err(e) => err_internal("Database error", e),
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
        VARIABLES_TABLE,
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
        Err(e) => err_internal("Database error", e),
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
        VARIABLES_TABLE,
        "key",
        serde_json::Value::String(key.to_string()),
        data,
    )
    .await
    {
        Ok(record) => ok_json(&record),
        Err(e) => err_internal("Database error", e),
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
    match db::create(ctx, VARIABLES_TABLE, data).await {
        Ok(record) => ok_json(&record),
        Err(e) => err_internal("Database error", e),
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
        VARIABLES_TABLE,
        "key",
        serde_json::Value::String(key.to_string()),
    )
    .await
    {
        Ok(record) => match db::delete(ctx, VARIABLES_TABLE, &record.id).await {
            Ok(_) => ok_json(&serde_json::json!({"deleted": key})),
            Err(e) => err_internal("Database error", e),
        },
        Err(_) => err_not_found("Setting not found"),
    }
}

/// Full block name of the admin block — the `block_settings` row whose
/// `seed_defaults_hash` column gates this function.
const ADMIN_BLOCK_NAME: &str = "suppers-ai/admin";

/// Compute a deterministic SHA-256 hex digest over the declared shared
/// config vars. Anything that affects the seed outcome (key, name,
/// description, default, warning, sensitive flag) feeds the hash; sort by
/// key so map ordering can't make two equivalent inputs hash differently.
///
/// `var.auto_generate` / `var.optional` don't affect what `seed_defaults`
/// writes — they're consumed by `seed_auto_generated` (CF runner) and the
/// startup validator respectively — so they're intentionally omitted.
fn seed_payload_hash(vars: &[types::ConfigVar]) -> String {
    use std::fmt::Write as _;
    let mut keys: Vec<&types::ConfigVar> = vars.iter().collect();
    keys.sort_by(|a, b| a.key.cmp(&b.key));
    let mut buf = String::with_capacity(vars.len() * 128);
    for v in keys {
        let sensitive = if v.input_type == types::InputType::Password {
            1
        } else {
            0
        };
        // Fixed shape per var: `key\x1fname\x1fdescription\x1fdefault\x1fwarning\x1fsensitive\x1e`.
        // ASCII unit-separator (0x1f) + record-separator (0x1e) bracket
        // each field so embedded newlines / colons in description text
        // can't collide field boundaries across different var shapes.
        let _ = write!(
            &mut buf,
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1e}",
            v.key, v.name, v.description, v.default, v.warning, sensitive,
        );
    }
    crate::migration_helper::sha256_hex_bytes(buf.as_bytes())
}

pub async fn seed_defaults(ctx: &dyn Context) {
    let vars = crate::config_vars::shared_config_vars();

    // Hash-gate: if the cached `block_settings.seed_defaults_hash` row for
    // the admin block already matches the current declared-vars hash, every
    // shared var was seeded against the same metadata last time — there is
    // no outcome change possible and we can skip the entire seed (zero D1
    // queries). Mirrors `migration_helper::apply_if_blessed`'s gate; reads
    // the same in-memory snapshot the migration helper does, so warm cold
    // starts cost zero round-trips. See 2026-05-14 config-snapshot spec
    // § "Hash-gate seed_defaults like migrations" (PR 3).
    let code_hash = seed_payload_hash(&vars);
    let json = ctx
        .config_get(crate::features::BLOCK_SETTINGS_CONFIG_KEY)
        .unwrap_or("{}");
    let cached_hash =
        crate::features::BlockSettings::state_for(json, ADMIN_BLOCK_NAME).seed_defaults_hash;
    if cached_hash == code_hash && !code_hash.is_empty() {
        return;
    }

    // Single bulk fetch of every existing variable, then in-memory diff
    // per declared shared var. Replaces the per-var `get_by_field` loop
    // that issued 2× D1 queries per shared var × cold isolate (~5k D1
    // reads/day in prod — see 2026-05-14 config-snapshot spec). On a
    // bulk failure we treat every key as missing, which falls into the
    // create-with-INSERT-OR-IGNORE-equivalent path; consistent with the
    // prior code's silent-on-error stance.
    let existing: HashMap<String, _> = db::list_all(ctx, VARIABLES_TABLE, vec![])
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|record| (record.str_field("key").to_string(), record))
        .collect();

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

        match existing.get(&var.key) {
            Some(record) => {
                // Only refresh metadata when at least one declared field
                // actually differs. Without this guard every isolate cold-start
                // re-writes every shared config var (~80 vars × cold-starts/day
                // ≈ ~900 useless UPDATEs/day in prod).
                let same_name = record.str_field("name") == name.as_str();
                let same_desc = record.str_field("description") == var.description;
                let same_warn = record.str_field("warning") == var.warning;
                let same_sens = record.i64_field("sensitive") == sensitive as i64;
                if same_name && same_desc && same_warn && same_sens {
                    continue;
                }
                let data = json_map(serde_json::json!({
                    "name": name,
                    "description": var.description,
                    "warning": var.warning,
                    "sensitive": sensitive,
                }));
                let _ = db::upsert(
                    ctx,
                    VARIABLES_TABLE,
                    "key",
                    serde_json::Value::String(var.key.clone()),
                    data,
                )
                .await;
            }
            None => {
                // Seed from process env when set (lets `.env` bootstrap a
                // fresh deployment), otherwise fall back to the declared
                // default. Empty env values are treated as unset so that
                // `FOO=` doesn't accidentally clear a meaningful default.
                let seed_value = std::env::var(&var.key)
                    .ok()
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| var.default.clone());
                if !seed_value.is_empty() {
                    let data = json_map(serde_json::json!({
                        "key": var.key,
                        "name": name,
                        "description": var.description,
                        "value": seed_value,
                        "warning": var.warning,
                        "sensitive": sensitive,
                        "created_at": helpers::now_rfc3339()
                    }));
                    let _ = db::create(ctx, VARIABLES_TABLE, data).await;
                }
            }
        }
    }

    // Stamp the new hash on the admin block_settings row so the next cold
    // start short-circuits before issuing `list_all`. Failures here are
    // logged but non-fatal — the seed itself succeeded, and the worst case
    // is that the next isolate re-runs the bulk `list_all` (the same cost
    // we paid this run). Matches the "silent on error" stance of the
    // per-var upsert/create calls above; the `block_settings` row may not
    // exist yet (admin migrations create it on the same `Init` pass),
    // which is why we use `upsert_block_settings_fields` rather than
    // assuming a row.
    let mut patch = std::collections::HashMap::new();
    patch.insert(
        "seed_defaults_hash".to_string(),
        serde_json::Value::String(code_hash),
    );
    if let Err(e) =
        crate::migration_helper::upsert_block_settings_fields(ctx, ADMIN_BLOCK_NAME, patch).await
    {
        tracing::warn!(
            err = %e,
            "seed_defaults: failed to stamp seed_defaults_hash; next cold start will re-run the bulk list_all"
        );
    }
}

#[cfg(test)]
mod tests {
    use wafer_core::clients::database::{Filter, FilterOp};

    use super::*;
    use crate::test_support::TestContext;

    /// `seed_payload_hash` is independent of input order (sorts by `key`).
    #[test]
    fn payload_hash_independent_of_input_order() {
        let a = types::ConfigVar::new("AAA", "first", "1");
        let b = types::ConfigVar::new("BBB", "second", "2");
        let h1 = seed_payload_hash(&[a.clone(), b.clone()]);
        let h2 = seed_payload_hash(&[b, a]);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    /// Hash changes whenever any seed-relevant field changes.
    #[test]
    fn payload_hash_sensitive_to_each_field() {
        let base = vec![types::ConfigVar::new("KEY", "desc", "def")];
        let h_base = seed_payload_hash(&base);

        let mut name_changed = base.clone();
        name_changed[0].name = "label".into();
        assert_ne!(h_base, seed_payload_hash(&name_changed));

        let mut desc_changed = base.clone();
        desc_changed[0].description = "different".into();
        assert_ne!(h_base, seed_payload_hash(&desc_changed));

        let mut default_changed = base.clone();
        default_changed[0].default = "other".into();
        assert_ne!(h_base, seed_payload_hash(&default_changed));

        let mut warning_changed = base.clone();
        warning_changed[0].warning = "careful".into();
        assert_ne!(h_base, seed_payload_hash(&warning_changed));

        let mut sensitive_changed = base.clone();
        sensitive_changed[0].input_type = types::InputType::Password;
        assert_ne!(h_base, seed_payload_hash(&sensitive_changed));
    }

    /// End-to-end: after `seed_defaults` runs once, the admin
    /// `block_settings` row carries the current hash. Wiring that hash into
    /// the next cold-start's config snapshot short-circuits `seed_defaults`
    /// before it can touch the `variables` table — even if every row was
    /// deleted between starts.
    #[tokio::test]
    async fn second_call_with_matching_snapshot_hash_short_circuits() {
        let ctx = TestContext::new().await;

        // 1. Run admin migrations so the block_settings + variables tables
        //    exist (with the new seed_defaults_hash column).
        crate::blocks::admin::migrations::apply(&ctx)
            .await
            .expect("apply admin migrations");

        // 2. First seed run — populates variables + stamps the hash row.
        seed_defaults(&ctx).await;
        let var_count_after_first = db::list_all(&ctx, VARIABLES_TABLE, vec![])
            .await
            .expect("list variables")
            .len();
        assert!(
            var_count_after_first > 0,
            "first seed_defaults should populate at least one variable"
        );

        // 3. Read the stamped hash from the block_settings row directly.
        let admin_rows = db::list_all(
            &ctx,
            crate::blocks::admin::settings::BLOCK_SETTINGS_TABLE,
            vec![Filter {
                field: "block_name".into(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(ADMIN_BLOCK_NAME.to_string()),
            }],
        )
        .await
        .expect("list block_settings");
        assert_eq!(
            admin_rows.len(),
            1,
            "admin block_settings row should be present after first seed_defaults"
        );
        let stamped_hash = admin_rows[0].str_field("seed_defaults_hash").to_string();
        let code_hash = seed_payload_hash(&crate::config_vars::shared_config_vars());
        assert_eq!(
            stamped_hash, code_hash,
            "stamped seed_defaults_hash should match current declared vars",
        );

        // 4. Simulate a fresh cold start: build a new TestContext (fresh
        //    in-memory DB — no variables, no block_settings row), but
        //    pre-populate the config snapshot with the stamped hash. This
        //    mirrors what the production loader does on the next boot.
        let mut next_ctx = TestContext::new().await;
        crate::blocks::admin::migrations::apply(&next_ctx)
            .await
            .expect("apply admin migrations on next ctx");
        let snapshot = serde_json::json!({
            ADMIN_BLOCK_NAME: { "enabled": true, "seed_defaults_hash": stamped_hash }
        })
        .to_string();
        next_ctx.set_config(crate::features::BLOCK_SETTINGS_CONFIG_KEY, &snapshot);

        // 5. seed_defaults should short-circuit before any list_all on
        //    variables — leaving the (empty) variables table untouched.
        seed_defaults(&next_ctx).await;
        let var_count_after_second = db::list_all(&next_ctx, VARIABLES_TABLE, vec![])
            .await
            .expect("list variables on next ctx")
            .len();
        assert_eq!(
            var_count_after_second, 0,
            "seed_defaults should short-circuit when snapshot hash matches; \
             expected 0 rows in fresh variables table, got {}",
            var_count_after_second
        );
    }

    /// When the snapshot's cached hash differs from the current code hash
    /// (e.g. a new shared var was declared), `seed_defaults` runs again
    /// and re-stamps the row.
    #[tokio::test]
    async fn mismatched_snapshot_hash_re_runs_seed() {
        let mut ctx = TestContext::new().await;
        crate::blocks::admin::migrations::apply(&ctx)
            .await
            .expect("apply admin migrations");

        // Pre-populate the snapshot with a deliberately-wrong hash.
        let snapshot = serde_json::json!({
            ADMIN_BLOCK_NAME: {
                "enabled": true,
                "seed_defaults_hash": "deadbeef".to_string(),
            }
        })
        .to_string();
        ctx.set_config(crate::features::BLOCK_SETTINGS_CONFIG_KEY, &snapshot);

        seed_defaults(&ctx).await;
        let count = db::list_all(&ctx, VARIABLES_TABLE, vec![])
            .await
            .expect("list variables")
            .len();
        assert!(
            count > 0,
            "mismatched snapshot hash should still run the seed; got 0 rows"
        );
    }
}
