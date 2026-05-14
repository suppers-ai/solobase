//! Server-boot body lifted from the previous `main.rs`.
//!
//! `run()` is invoked by the sealed × native flow (and by the bare-`solobase`
//! shortcut path in `main.rs`). It owns the SQLite seeding, WAFER builder,
//! HTTP listener registration, and the `serve_until_shutdown` loop.

use std::{collections::HashMap, sync::Arc};

use solobase_core::builder::{self, SolobaseBuilder};
use solobase_native::{
    collect_app_env_vars, init_tracing, load_dotenv, register_http_listener,
    register_observability_hooks, serve_until_shutdown, InfraConfig,
};
use wafer_core::interfaces::config::service::ConfigService;

use crate::cli::server_config::{filter_to_declared_keys, load_block_settings, load_wrap_grants};

/// Boot the native server end-to-end. The body mirrors the previous
/// `main()` exactly; the signature is `pub async fn run()` so the new
/// dispatcher can `await` it as the sealed × native flow.
pub async fn run() -> anyhow::Result<()> {
    // 1. Load .env file (before reading any env vars)
    load_dotenv();

    // 2. Initialize tracing / logging
    let log_format = std::env::var("SOLOBASE_LOG_FORMAT").unwrap_or_else(|_| "text".into());
    init_tracing(&log_format);
    tracing::info!("solobase starting (Rust/WAFER runtime)");

    // 3. Read infrastructure config from SOLOBASE_* env vars
    let infra = InfraConfig::from_env();
    tracing::info!(
        listen = %infra.listen,
        db = %infra.db_type,
        db_path = %infra.db_path,
        storage = %infra.storage_type,
        "infrastructure config loaded"
    );

    // 4. Collect app config vars from env (non-SOLOBASE_* prefixed, filtered to declared keys)
    let env_vars = filter_to_declared_keys(collect_app_env_vars());

    // 5. Open SQLite directly, seed variables, read config
    let mut vars = seed_and_load_variables(&infra.db_path, &env_vars);
    tracing::info!(vars = vars.len(), "variables loaded from database");

    // 5a. Surface the `SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_*` env vars
    //     into `vars` for the bootstrap gate. `collect_app_env_vars` strips
    //     every `SOLOBASE_`-prefixed env var (it can't distinguish infra
    //     keys from `SOLOBASE_SHARED__*` app-config keys), so without this
    //     splice the gate below decides "no env vars set" on a fresh boot
    //     and the auto-bootstrap path injects a random password — which
    //     then becomes an `EnvConfigService` override that masks the real
    //     env var when the auth block reads it. The auth block itself
    //     reads these via `EnvConfigService::get`'s `std::env::var`
    //     fallback, so merging them in here matches the downstream view.
    {
        use solobase_core::blocks::auth::config::{
            BOOTSTRAP_ADMIN_EMAIL_KEY, BOOTSTRAP_ADMIN_PASSWORD_KEY, BOOTSTRAP_ADMIN_TOKEN_KEY,
        };
        for key in [
            BOOTSTRAP_ADMIN_EMAIL_KEY,
            BOOTSTRAP_ADMIN_PASSWORD_KEY,
            BOOTSTRAP_ADMIN_TOKEN_KEY,
        ] {
            if let Ok(val) = std::env::var(key) {
                if !val.is_empty() {
                    vars.entry(key.to_string()).or_insert(val);
                }
            }
        }
    }

    // 6. First-run admin policy. Runs before the auth block boots so the
    //    decision is visible in startup logs and a missing-config error
    //    stops the server cleanly. See [`auto_bootstrap_if_needed`].
    auto_bootstrap_if_needed(&infra.listen, &infra.db_path, &mut vars)?;

    // 7. Extract JWT secret and feature config from variables
    let jwt_secret = vars
        .get(solobase_core::blocks::auth::JWT_SECRET_KEY)
        .cloned()
        .unwrap_or_default();
    let features = load_block_settings(&infra.db_path);

    // 8. Build WAFER runtime via SolobaseBuilder
    let config_service = wafer_block_config::service::EnvConfigService::new();
    for (key, value) in &vars {
        config_service.set(key, value);
    }
    // Fan-out block_settings into the config snapshot so consumer blocks
    // (e.g. userportal) can read enablement state via `ctx.config_get`
    // without re-querying the `block_settings` SQLite table per request.
    config_service.set(
        solobase_core::features::BLOCK_SETTINGS_CONFIG_KEY,
        &features.to_config_json(),
    );

    let (mut wafer, storage_block) = SolobaseBuilder::new()
        .database(solobase_native::make_sqlite_database_service(
            &infra.db_path,
        ))
        .storage(solobase_native::make_local_storage_service(
            &infra.storage_root,
        ))
        .config(Arc::new(config_service))
        .crypto(solobase_native::make_jwt_crypto_service(jwt_secret))
        .network(solobase_native::make_fetch_network_service())
        .logger(solobase_native::make_tracing_logger())
        .block_settings(features)
        // Hand the SQLite path to the builder so the `native-embedding`
        // feature can open a dedicated connection for `SqliteVecService`.
        // Ignored when the feature is off.
        .sqlite_db_path(&infra.db_path)
        .build()
        .expect("failed to build solobase runtime");

    // 9. Native-only: register http-listener.
    //    solobase dispatches all HTTP traffic through the `site-main` flow
    //    (see crates/solobase-core/src/flows/site_main.rs).
    register_http_listener(&mut wafer, &infra.listen, "site-main");

    // 10. Register observability hooks
    register_observability_hooks(&mut wafer);

    // 11. Load custom WRAP grants from DB
    let db_grants = load_wrap_grants(&infra.db_path);
    if !db_grants.is_empty() {
        tracing::info!(
            count = db_grants.len(),
            "loaded custom WRAP grants from database"
        );
        wafer.add_wrap_grants(db_grants);
    }

    // 12. Start runtime
    let wafer = wafer.start().await.expect("failed to start WAFER runtime");

    // 13. Inject WRAP grants into storage block
    builder::post_start(&wafer, &storage_block);
    tracing::info!("WAFER runtime started — all blocks resolved");

    // 14. Wait for shutdown signal, then graceful shutdown
    serve_until_shutdown(&wafer).await;
    tracing::info!("solobase shutdown complete");

    Ok(())
}

// ---------------------------------------------------------------------------
// SQLite variable seeding and loading
// ---------------------------------------------------------------------------

/// Canonical variables table name, sourced from the admin block so the
/// boot loader and the `/b/admin/settings/variables` UI always read and
/// write the same place. Earlier versions used a bare `variables` table
/// here, which drifted from the admin block's prefixed `CollectionSchema`
/// and silently divided the config into two stores. The constant lives
/// in `solobase-core::blocks::admin` so there's no second source of truth.
const VARIABLES_TABLE: &str = solobase_core::blocks::admin::VARIABLES_TABLE;

/// Ensure the variables table exists, seed from env vars, and return all variables.
fn seed_and_load_variables(
    db_path: &str,
    env_vars: &[(String, String)],
) -> HashMap<String, String> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            tracing::error!(
                "failed to create database directory {}: {e}",
                parent.display()
            );
            std::process::exit(1);
        });
    }

    let conn = rusqlite::Connection::open(db_path).unwrap_or_else(|e| {
        tracing::error!("failed to open SQLite at {db_path}: {e}");
        std::process::exit(1);
    });

    // Create variables table if it doesn't exist.
    //
    // Boot runs before WAFER, so the admin block's lifecycle hasn't created
    // the table yet — we have to pre-create it via raw SQL. Schema mirrors
    // the admin block's `CollectionSchema`: the user-visible columns are
    // declared there, and `ensure_table` adds the `id`/`created_at`/
    // `updated_at` columns the WAFER DB client expects. Pre-creating here
    // with the union of both is harmless (the columns line up; later
    // `ensure_table` calls see they exist and skip).
    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS \"{VARIABLES_TABLE}\" (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL UNIQUE,
            name TEXT DEFAULT '',
            description TEXT DEFAULT '',
            value TEXT DEFAULT '',
            warning TEXT DEFAULT '',
            sensitive INTEGER DEFAULT 0,
            updated_by TEXT DEFAULT '',
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );
        CREATE UNIQUE INDEX IF NOT EXISTS \"idx_{VARIABLES_TABLE}_key\" \
         ON \"{VARIABLES_TABLE}\" (key);"
    );
    conn.execute_batch(&create_sql).unwrap_or_else(|e| {
        tracing::error!("failed to create {VARIABLES_TABLE} table: {e}");
        std::process::exit(1);
    });

    // Seed from env vars (INSERT OR IGNORE — existing DB values take priority)
    {
        let insert_sql = format!(
            "INSERT OR IGNORE INTO \"{VARIABLES_TABLE}\" \
             (id, key, value, sensitive, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))"
        );
        let mut stmt = conn
            .prepare(&insert_sql)
            .expect("failed to prepare seed statement");

        for (key, value) in env_vars {
            let id = format!("var_{}", uuid::Uuid::new_v4());
            let sensitive = if key.ends_with("_SECRET") || key.ends_with("_KEY") {
                1
            } else {
                0
            };
            if let Err(e) = stmt.execute(rusqlite::params![id, key, value, sensitive]) {
                tracing::warn!(key = %key, error = %e, "failed to seed variable");
            }
        }
    }

    // Auto-generate secrets for config vars marked with auto_generate
    seed_auto_generated(&conn);

    // Load all variables
    let mut vars = HashMap::new();
    let select_sql = format!("SELECT key, value FROM \"{VARIABLES_TABLE}\"");
    let mut stmt = conn
        .prepare(&select_sql)
        .expect("failed to prepare SELECT variables");
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("failed to query variables");

    for (key, value) in rows.flatten() {
        if !key.is_empty() {
            vars.insert(key, value);
        }
    }

    vars
}

/// Auto-generate values for config vars marked with `auto_generate: true`.
///
/// Reads all block config var declarations, finds those needing auto-generation,
/// and generates random values for any that don't already exist in the variables table.
fn seed_auto_generated(conn: &rusqlite::Connection) {
    let block_infos = solobase_core::blocks::all_block_infos();
    let all_vars = solobase_core::config_vars::collect_all_config_vars(&block_infos);

    let insert_sql = format!(
        "INSERT OR IGNORE INTO \"{VARIABLES_TABLE}\" \
         (id, key, name, description, value, warning, sensitive, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))"
    );
    let mut stmt = conn
        .prepare(&insert_sql)
        .expect("failed to prepare auto-generate statement");

    for var in &all_vars {
        if !var.auto_generate {
            continue;
        }

        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("failed to generate random secret");
        let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();

        let id = format!("var_{}", uuid::Uuid::new_v4());
        let sensitive: i32 = if var.is_sensitive() { 1 } else { 0 };

        let affected = stmt
            .execute(rusqlite::params![
                id,
                var.key,
                var.name,
                var.description,
                secret,
                var.warning,
                sensitive
            ])
            .unwrap_or(0);

        if affected > 0 {
            tracing::warn!(key = %var.key, "auto-generated secret (not found in variables table)");
        }
    }
}

// ---------------------------------------------------------------------------
// First-run admin bootstrap policy
// ---------------------------------------------------------------------------

/// First-run admin provisioning.
///
/// Decision table when the users table is empty:
///
/// | env BOOTSTRAP_ADMIN_*  | listen      | action                      |
/// |------------------------|-------------|-----------------------------|
/// | any set                | any         | no-op (auth block handles)  |
/// | none set               | loopback    | auto-generate + log + inject|
/// | none set               | non-loopback| `bail!` — refuse to start   |
///
/// When the users table already has rows we always no-op: re-injecting
/// bootstrap creds against an initialized DB would be misleading (the auth
/// block's bootstrap skips when users exist, so the log line would refer to a
/// credential that was never installed).
fn auto_bootstrap_if_needed(
    listen: &str,
    db_path: &str,
    vars: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    use solobase_core::blocks::auth::config::{
        BOOTSTRAP_ADMIN_EMAIL_KEY, BOOTSTRAP_ADMIN_PASSWORD_KEY, BOOTSTRAP_ADMIN_TOKEN_KEY,
    };

    if !users_table_is_empty(db_path) {
        return Ok(());
    }

    let any_bootstrap_set = [
        BOOTSTRAP_ADMIN_EMAIL_KEY,
        BOOTSTRAP_ADMIN_PASSWORD_KEY,
        BOOTSTRAP_ADMIN_TOKEN_KEY,
    ]
    .iter()
    .any(|k| vars.get(*k).is_some_and(|v| !v.is_empty()));
    if any_bootstrap_set {
        return Ok(());
    }

    if !is_loopback(listen) {
        anyhow::bail!(
            "no admin configured (users table is empty) and listen address {listen} \
             is not loopback. Set {email_key} and {password_key} (or {token_key}) to \
             provision the first admin, or bind to 127.0.0.1 to allow auto-bootstrap.",
            email_key = BOOTSTRAP_ADMIN_EMAIL_KEY,
            password_key = BOOTSTRAP_ADMIN_PASSWORD_KEY,
            token_key = BOOTSTRAP_ADMIN_TOKEN_KEY,
        );
    }

    let password = random_password();
    let email = "admin@example.com";
    vars.insert(BOOTSTRAP_ADMIN_EMAIL_KEY.to_string(), email.to_string());
    vars.insert(BOOTSTRAP_ADMIN_PASSWORD_KEY.to_string(), password.clone());
    tracing::warn!(
        email = %email,
        password = %password,
        "auto-bootstrapped admin (loopback bind, no bootstrap env vars set). \
         Set {} to override on the next boot, or note these credentials now — \
         they are not persisted in plaintext and will not be re-logged.",
        BOOTSTRAP_ADMIN_PASSWORD_KEY,
    );
    Ok(())
}

/// True when the auth `users` table is missing or has no rows. Used to gate
/// first-run admin provisioning. Errors opening the DB are treated as
/// "empty" so a brand-new install proceeds; the subsequent SQLite seed step
/// in [`seed_and_load_variables`] is the canonical place to fail on real
/// DB problems.
fn users_table_is_empty(db_path: &str) -> bool {
    use solobase_core::blocks::auth::repo::users::TABLE;

    let Ok(conn) = rusqlite::Connection::open(db_path) else {
        return true;
    };
    let table_exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [TABLE],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if table_exists == 0 {
        return true;
    }
    // TABLE is a hard-coded const, not user input — no injection surface.
    let count: i64 = conn
        .query_row(&format!("SELECT count(*) FROM \"{TABLE}\""), [], |row| {
            row.get(0)
        })
        .unwrap_or(0);
    count == 0
}

/// True if `listen`'s host portion is a loopback address (127.0.0.0/8, ::1)
/// or the literal `localhost`. Anything else (including 0.0.0.0 and external
/// IPs) is non-loopback.
fn is_loopback(listen: &str) -> bool {
    let host = parse_listen_host(listen);
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    match host.parse::<std::net::IpAddr>() {
        Ok(ip) => ip.is_loopback(),
        Err(_) => false,
    }
}

/// Extract the host portion from a `host:port` or `[host]:port` listen string.
/// Bare IPv6 (`::1` with no port) is returned as-is.
fn parse_listen_host(listen: &str) -> &str {
    let s = listen.trim();
    if let Some(rest) = s.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            return &rest[..end];
        }
    }
    if let Some(idx) = s.rfind(':') {
        // Bare IPv6 without brackets — multiple ':' means no port suffix.
        if s[..idx].contains(':') {
            return s;
        }
        return &s[..idx];
    }
    s
}

/// 16-char password from a confusion-resistant base32-ish alphabet (no 0/O/1/I/l).
fn random_password() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).expect("getrandom for bootstrap password");
    bytes
        .iter()
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use solobase_core::blocks::auth::{
        config::{
            BOOTSTRAP_ADMIN_EMAIL_KEY, BOOTSTRAP_ADMIN_PASSWORD_KEY, BOOTSTRAP_ADMIN_TOKEN_KEY,
        },
        repo::users::TABLE as USERS_TABLE,
    };

    use super::*;

    #[test]
    fn parse_listen_host_handles_ipv4_ipv6_and_localhost() {
        assert_eq!(parse_listen_host("127.0.0.1:8090"), "127.0.0.1");
        assert_eq!(parse_listen_host("0.0.0.0:8090"), "0.0.0.0");
        assert_eq!(parse_listen_host("localhost:8090"), "localhost");
        assert_eq!(parse_listen_host("[::1]:8090"), "::1");
        assert_eq!(parse_listen_host("::1"), "::1");
        assert_eq!(parse_listen_host("127.0.0.1"), "127.0.0.1");
    }

    #[test]
    fn is_loopback_matches_loopback_addresses() {
        assert!(is_loopback("127.0.0.1:8090"));
        assert!(is_loopback("127.5.6.7:8090"));
        assert!(is_loopback("[::1]:8090"));
        assert!(is_loopback("localhost:8090"));
        assert!(is_loopback("LocalHost:8090"));
        assert!(!is_loopback("0.0.0.0:8090"));
        assert!(!is_loopback("192.168.1.10:8090"));
        assert!(!is_loopback("nonsense"));
    }

    #[test]
    fn random_password_is_16_chars_and_changes() {
        let a = random_password();
        let b = random_password();
        assert_eq!(a.len(), 16);
        assert_eq!(b.len(), 16);
        assert_ne!(a, b);
    }

    fn temp_db_with_users(rows: usize) -> tempfile::NamedTempFile {
        let f = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();
        if rows > 0 {
            let conn = rusqlite::Connection::open(f.path()).unwrap();
            conn.execute_batch(&format!(
                "CREATE TABLE \"{USERS_TABLE}\" (id TEXT PRIMARY KEY);"
            ))
            .unwrap();
            for i in 0..rows {
                conn.execute(
                    &format!("INSERT INTO \"{USERS_TABLE}\" (id) VALUES (?1)"),
                    [format!("u{i}")],
                )
                .unwrap();
            }
        }
        f
    }

    #[test]
    fn users_table_is_empty_when_missing_or_empty() {
        let f = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();
        // table missing → empty
        assert!(users_table_is_empty(f.path().to_str().unwrap()));
        // table exists, 0 rows → empty
        let conn = rusqlite::Connection::open(f.path()).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE \"{USERS_TABLE}\" (id TEXT PRIMARY KEY);"
        ))
        .unwrap();
        drop(conn);
        assert!(users_table_is_empty(f.path().to_str().unwrap()));
    }

    #[test]
    fn users_table_is_not_empty_when_rows_present() {
        let f = temp_db_with_users(1);
        assert!(!users_table_is_empty(f.path().to_str().unwrap()));
    }

    #[test]
    fn auto_bootstrap_noop_when_users_exist() {
        let f = temp_db_with_users(1);
        let mut vars = HashMap::new();
        auto_bootstrap_if_needed("0.0.0.0:8090", f.path().to_str().unwrap(), &mut vars).unwrap();
        // Did not inject creds, did not error.
        assert!(!vars.contains_key(BOOTSTRAP_ADMIN_EMAIL_KEY));
        assert!(!vars.contains_key(BOOTSTRAP_ADMIN_PASSWORD_KEY));
    }

    #[test]
    fn auto_bootstrap_injects_creds_on_loopback() {
        let f = temp_db_with_users(0);
        let mut vars = HashMap::new();
        auto_bootstrap_if_needed("127.0.0.1:8090", f.path().to_str().unwrap(), &mut vars).unwrap();
        assert_eq!(
            vars.get(BOOTSTRAP_ADMIN_EMAIL_KEY).map(String::as_str),
            Some("admin@example.com")
        );
        let pw = vars.get(BOOTSTRAP_ADMIN_PASSWORD_KEY).unwrap();
        assert_eq!(pw.len(), 16);
    }

    #[test]
    fn auto_bootstrap_errors_on_non_loopback_with_no_env() {
        let f = temp_db_with_users(0);
        let mut vars = HashMap::new();
        let err = auto_bootstrap_if_needed("0.0.0.0:8090", f.path().to_str().unwrap(), &mut vars)
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("no admin configured"), "msg = {msg}");
        assert!(msg.contains(BOOTSTRAP_ADMIN_EMAIL_KEY), "msg = {msg}");
        assert!(msg.contains("loopback"), "msg = {msg}");
    }

    #[test]
    fn auto_bootstrap_noop_when_email_password_already_set() {
        let f = temp_db_with_users(0);
        let mut vars = HashMap::new();
        vars.insert(BOOTSTRAP_ADMIN_EMAIL_KEY.to_string(), "x@y.z".to_string());
        vars.insert(BOOTSTRAP_ADMIN_PASSWORD_KEY.to_string(), "pw".to_string());
        auto_bootstrap_if_needed("0.0.0.0:8090", f.path().to_str().unwrap(), &mut vars).unwrap();
        // Values untouched.
        assert_eq!(vars.get(BOOTSTRAP_ADMIN_EMAIL_KEY).unwrap(), "x@y.z");
        assert_eq!(vars.get(BOOTSTRAP_ADMIN_PASSWORD_KEY).unwrap(), "pw");
    }

    #[test]
    fn auto_bootstrap_noop_when_token_already_set() {
        let f = temp_db_with_users(0);
        let mut vars = HashMap::new();
        vars.insert(
            BOOTSTRAP_ADMIN_TOKEN_KEY.to_string(),
            "deadbeef".to_string(),
        );
        // Non-loopback should still be fine because the operator chose token mode.
        auto_bootstrap_if_needed("0.0.0.0:8090", f.path().to_str().unwrap(), &mut vars).unwrap();
        assert!(!vars.contains_key(BOOTSTRAP_ADMIN_EMAIL_KEY));
        assert!(!vars.contains_key(BOOTSTRAP_ADMIN_PASSWORD_KEY));
    }

    #[test]
    fn auto_bootstrap_treats_empty_string_env_as_unset() {
        let f = temp_db_with_users(0);
        let mut vars = HashMap::new();
        // shell exports like FOO="" round-trip through the DB as empty strings;
        // they should NOT count as "set" for bootstrap gating.
        vars.insert(BOOTSTRAP_ADMIN_EMAIL_KEY.to_string(), String::new());
        vars.insert(BOOTSTRAP_ADMIN_PASSWORD_KEY.to_string(), String::new());
        vars.insert(BOOTSTRAP_ADMIN_TOKEN_KEY.to_string(), String::new());
        auto_bootstrap_if_needed("127.0.0.1:8090", f.path().to_str().unwrap(), &mut vars).unwrap();
        assert_eq!(
            vars.get(BOOTSTRAP_ADMIN_EMAIL_KEY).map(String::as_str),
            Some("admin@example.com")
        );
        assert_eq!(vars.get(BOOTSTRAP_ADMIN_PASSWORD_KEY).unwrap().len(), 16);
    }
}
