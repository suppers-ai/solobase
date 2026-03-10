//! AppConfig — simplified configuration for solobase instances.
//!
//! Instead of the verbose blocks.json (700+ lines of schema definitions),
//! `app.json` lets you configure an instance in ~15 lines:
//!
//! ```json
//! {
//!     "version": 1,
//!     "app": "my-store",
//!     "listen": "0.0.0.0:8090",
//!     "database": { "type": "sqlite", "path": "data/app.db" },
//!     "storage": { "type": "local", "root": "data/storage" },
//!     "jwt_secret": "${JWT_SECRET}",
//!     "web_root": "./frontend/build",
//!     "auth": {},
//!     "products": {},
//!     "files": {},
//!     "legalpages": {}
//! }
//! ```
//!
//! Feature blocks are enabled by including their key (even as `{}`).
//! Omit the key to disable the feature. Set to `false` to explicitly disable.

use serde::Deserialize;
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// AppConfig struct
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    /// Config format version. Used to select the correct runtime binary
    /// (e.g. on Cloudflare where multiple major versions may coexist).
    #[serde(default = "default_version")]
    pub version: u32,

    /// Instance name (informational).
    #[serde(default)]
    pub app: Option<String>,

    /// HTTP listen address (default: "0.0.0.0:8090").
    #[serde(default)]
    pub listen: Option<String>,

    /// Database configuration.
    #[serde(default)]
    pub database: Option<DatabaseConfig>,

    /// File storage configuration.
    #[serde(default)]
    pub storage: Option<StorageConfig>,

    /// JWT signing secret. Supports `${ENV_VAR}` expansion.
    #[serde(default)]
    pub jwt_secret: Option<String>,

    /// Web root directory for the SPA frontend (default: "./frontend/build").
    #[serde(default)]
    pub web_root: Option<String>,

    // -- Feature blocks (present = enabled, absent = disabled) --

    /// Authentication & user accounts.
    #[serde(default)]
    pub auth: Option<Value>,

    /// Admin panel, IAM, settings, logs, custom tables.
    #[serde(default)]
    pub admin: Option<Value>,

    /// File storage & cloud storage.
    #[serde(default)]
    pub files: Option<Value>,

    /// Product catalog, pricing, purchases, Stripe.
    #[serde(default)]
    pub products: Option<Value>,

    /// Deployment management.
    #[serde(default)]
    pub deployments: Option<Value>,

    /// Legal pages (terms, privacy).
    #[serde(default)]
    pub legalpages: Option<Value>,

    /// User portal (branding, feature toggles).
    #[serde(default)]
    pub userportal: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    #[serde(rename = "type", default = "default_db_type")]
    pub db_type: String,
    #[serde(default = "default_db_path")]
    pub path: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    #[serde(rename = "type", default = "default_storage_type")]
    pub storage_type: String,
    #[serde(default = "default_storage_root")]
    pub root: String,
}

fn default_version() -> u32 { 0 }
fn default_db_type() -> String { "sqlite".into() }
fn default_db_path() -> String { "data/solobase.db".into() }
fn default_storage_type() -> String { "local".into() }
fn default_storage_root() -> String { "data/storage".into() }

// ---------------------------------------------------------------------------
// Feature detection
// ---------------------------------------------------------------------------

impl AppConfig {
    /// Returns true if the feature value means "enabled".
    /// `None` (absent) and `false` mean disabled. Object or `true` means enabled.
    fn is_enabled(val: &Option<Value>) -> bool {
        !matches!(val, None | Some(Value::Bool(false)) | Some(Value::Null))
    }

    pub fn auth_enabled(&self) -> bool { Self::is_enabled(&self.auth) }
    pub fn admin_enabled(&self) -> bool { Self::is_enabled(&self.admin) }
    pub fn files_enabled(&self) -> bool { Self::is_enabled(&self.files) }
    pub fn products_enabled(&self) -> bool { Self::is_enabled(&self.products) }
    pub fn deployments_enabled(&self) -> bool { Self::is_enabled(&self.deployments) }
    pub fn legalpages_enabled(&self) -> bool { Self::is_enabled(&self.legalpages) }
    pub fn userportal_enabled(&self) -> bool { Self::is_enabled(&self.userportal) }

    /// Returns the list of enabled feature names (for FEATURE_* env var gating).
    pub fn enabled_features(&self) -> Vec<&str> {
        // system and profile are always enabled
        let mut features = vec!["system", "profile"];
        if self.auth_enabled() { features.push("auth"); }
        if self.admin_enabled() { features.push("admin"); }
        if self.files_enabled() { features.push("files"); }
        if self.products_enabled() { features.push("products"); }
        if self.deployments_enabled() { features.push("deployments"); }
        if self.legalpages_enabled() { features.push("legalpages"); }
        if self.userportal_enabled() { features.push("userportal"); }
        features
    }

    /// Returns the list of disabled feature names (for FEATURE_*=false env vars).
    pub fn disabled_features(&self) -> Vec<&str> {
        let mut disabled = Vec::new();
        if !self.auth_enabled() { disabled.push("auth"); }
        if !self.admin_enabled() { disabled.push("admin"); }
        if !self.files_enabled() { disabled.push("files"); }
        if !self.products_enabled() { disabled.push("products"); }
        if !self.deployments_enabled() { disabled.push("deployments"); }
        if !self.legalpages_enabled() { disabled.push("legalpages"); }
        if !self.userportal_enabled() { disabled.push("userportal"); }
        disabled
    }
}

// ---------------------------------------------------------------------------
// Load from file
// ---------------------------------------------------------------------------

impl AppConfig {
    /// Load from a JSON file, expanding `${ENV_VAR}` references.
    pub fn load(path: &str) -> Result<Self, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {path}: {e}"))?;
        let expanded = wafer_run::helpers::expand_env_vars(&data);
        serde_json::from_str(&expanded)
            .map_err(|e| format!("invalid app config in {path}: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Expand to blocks.json value
// ---------------------------------------------------------------------------

impl AppConfig {
    /// Expand this config into blocks + aliases that WAFER expects.
    ///
    /// Returns `(block_configs, aliases)`. The aliases map canonical names
    /// like `@db` → `@wafer/database` so feature blocks can use short,
    /// backend-agnostic names.
    pub fn to_blocks_json(&self) -> (Map<String, Value>, Vec<(String, String)>) {
        let mut blocks = Map::new();
        let mut aliases: Vec<(String, String)> = Vec::new();

        // -- Infrastructure blocks --

        let listen = self.listen.clone().unwrap_or_else(|| "0.0.0.0:8090".into());
        blocks.insert("@wafer/http-listener".into(), json!({
            "flow": "site-main",
            "listen": listen
        }));

        let web_root = self.web_root.clone().unwrap_or_else(|| "./frontend/build".into());
        blocks.insert("@wafer/web".into(), json!({
            "web_root": web_root,
            "web_spa": "true",
            "web_index": "index.html"
        }));

        // Database — register the specific backend block and alias @db to it
        let db = self.database.as_ref();
        let db_type = db.map(|d| d.db_type.as_str()).unwrap_or("sqlite");
        let db_block_name = match db_type {
            "postgres" | "postgresql" => "solobase/postgres",
            _ => "solobase/sqlite",
        };
        let mut db_config = json!({
            "path": db.map(|d| d.path.as_str()).unwrap_or("data/solobase.db")
        });
        if let Some(d) = db {
            if let Some(ref url) = d.url {
                db_config["url"] = json!(url);
            }
        }
        blocks.insert(db_block_name.into(), db_config);
        aliases.push(("@db".into(), db_block_name.into()));
        // Keep @wafer/database as an alias too for backward compatibility
        aliases.push(("@wafer/database".into(), db_block_name.into()));

        // Storage — register the specific backend block and alias @storage to it
        let storage = self.storage.as_ref();
        let storage_type = storage.map(|s| s.storage_type.as_str()).unwrap_or("local");
        let storage_block_name = match storage_type {
            "s3" => "solobase/s3",
            _ => "solobase/local-storage",
        };
        let storage_config = json!({
            "root": storage.map(|s| s.root.as_str()).unwrap_or("data/storage")
        });
        blocks.insert(storage_block_name.into(), storage_config);
        aliases.push(("@storage".into(), storage_block_name.into()));
        // Keep @wafer/storage as an alias too for backward compatibility
        aliases.push(("@wafer/storage".into(), storage_block_name.into()));

        let jwt = self.jwt_secret.clone().unwrap_or_default();
        blocks.insert("@wafer/crypto".into(), json!({ "jwt_secret": jwt }));

        blocks.insert("@wafer/network".into(), json!({}));
        blocks.insert("@wafer/logger".into(), json!({}));
        blocks.insert("@wafer/config".into(), json!({}));

        // -- Feature blocks (only if enabled) --

        if self.auth_enabled() {
            blocks.insert("@solobase/auth".into(), schema_with_config(
                schemas::AUTH_SCHEMA,
                &self.auth,
            ));
        }

        if self.admin_enabled() {
            blocks.insert("@solobase/admin".into(), schema_with_config(
                schemas::ADMIN_SCHEMA,
                &self.admin,
            ));
        }

        if self.files_enabled() {
            blocks.insert("@solobase/files".into(), schema_with_config(
                schemas::FILES_SCHEMA,
                &self.files,
            ));
        }

        if self.products_enabled() {
            blocks.insert("@solobase/products".into(), schema_with_config(
                schemas::PRODUCTS_SCHEMA,
                &self.products,
            ));
        }

        if self.deployments_enabled() {
            blocks.insert("@solobase/deployments".into(), schema_with_config(
                schemas::DEPLOYMENTS_SCHEMA,
                &self.deployments,
            ));
        }

        if self.legalpages_enabled() {
            blocks.insert("@solobase/legalpages".into(), schema_with_config(
                schemas::LEGALPAGES_SCHEMA,
                &self.legalpages,
            ));
        }

        (blocks, aliases)
    }
}

/// Merge the built-in schema with any user-provided config overrides.
fn schema_with_config(schema_json: &str, user_config: &Option<Value>) -> Value {
    let mut schema: Value = serde_json::from_str(schema_json)
        .expect("built-in schema must be valid JSON");

    // If the user provided an object, merge its keys into the schema
    if let Some(Value::Object(overrides)) = user_config {
        if let Value::Object(ref mut map) = schema {
            for (k, v) in overrides {
                // Don't let user overrides clobber the "uses" schema
                if k != "uses" {
                    map.insert(k.clone(), v.clone());
                }
            }
        }
    }

    schema
}

// ---------------------------------------------------------------------------
// Built-in schemas for each feature block
// ---------------------------------------------------------------------------

mod schemas {
    pub const AUTH_SCHEMA: &str = r#"{
    "uses": {
        "@wafer/database": {
            "collections": {
                "auth_users": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "email": { "type": "string", "unique": true },
                        "password_hash": { "type": "string" },
                        "name": { "type": "string", "optional": true },
                        "disabled": { "type": "bool", "default": false },
                        "username": { "type": "string", "optional": true },
                        "confirmed": { "type": "int", "default": 0 },
                        "first_name": { "type": "string", "optional": true },
                        "last_name": { "type": "string", "optional": true },
                        "display_name": { "type": "string", "optional": true },
                        "phone": { "type": "string", "optional": true },
                        "location": { "type": "string", "optional": true },
                        "confirm_token": { "type": "string", "optional": true },
                        "confirm_selector": { "type": "string", "optional": true },
                        "recover_token": { "type": "string", "optional": true },
                        "recover_token_exp": { "type": "datetime", "optional": true },
                        "recover_selector": { "type": "string", "optional": true },
                        "attempt_count": { "type": "int", "default": 0 },
                        "last_attempt": { "type": "datetime", "optional": true },
                        "last_login_at": { "type": "datetime", "optional": true },
                        "metadata": { "type": "text", "optional": true },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true },
                        "deleted_at": { "type": "datetime", "optional": true },
                        "totp_secret": { "type": "string", "optional": true },
                        "totp_secret_backup": { "type": "string", "optional": true },
                        "sms_phone_number": { "type": "string", "optional": true },
                        "recovery_codes": { "type": "text", "optional": true }
                    },
                    "indexes": [
                        { "fields": ["confirm_selector"] },
                        { "fields": ["recover_selector"] },
                        { "fields": ["deleted_at"] }
                    ]
                },
                "auth_tokens": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "user_id": { "type": "string", "ref": "auth_users.id" },
                        "token_hash": { "type": "string", "optional": true },
                        "token": { "type": "string", "optional": true },
                        "type": { "type": "string", "optional": true },
                        "family_id": { "type": "string", "optional": true },
                        "provider": { "type": "string", "optional": true },
                        "provider_uid": { "type": "string", "optional": true },
                        "access_token": { "type": "string", "optional": true },
                        "oauth_expiry": { "type": "datetime", "optional": true },
                        "expires_at": { "type": "datetime", "optional": true },
                        "used_at": { "type": "datetime", "optional": true },
                        "revoked_at": { "type": "datetime", "optional": true },
                        "created_at": { "type": "datetime", "auto": true },
                        "device_info": { "type": "string", "optional": true },
                        "ip_address": { "type": "string", "optional": true }
                    },
                    "indexes": [
                        { "fields": ["user_id"] },
                        { "fields": ["token_hash"] },
                        { "fields": ["token"] },
                        { "fields": ["type"] },
                        { "fields": ["family_id"] },
                        { "fields": ["provider_uid"] },
                        { "fields": ["expires_at"] },
                        { "fields": ["revoked_at"] }
                    ]
                },
                "api_keys": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "user_id": { "type": "string", "ref": "auth_users.id" },
                        "name": { "type": "string" },
                        "key_prefix": { "type": "string" },
                        "key_hash": { "type": "string", "unique": true },
                        "scopes": { "type": "text", "optional": true },
                        "expires_at": { "type": "datetime", "optional": true },
                        "last_used_at": { "type": "datetime", "optional": true },
                        "last_used_ip": { "type": "string", "optional": true },
                        "revoked_at": { "type": "datetime", "optional": true },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["user_id"] },
                        { "fields": ["key_prefix"] },
                        { "fields": ["revoked_at"] }
                    ]
                }
            }
        }
    }
}"#;

    pub const ADMIN_SCHEMA: &str = r#"{
    "uses": {
        "@wafer/database": {
            "collections": {
                "iam_roles": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "name": { "type": "string", "unique": true },
                        "display_name": { "type": "string", "optional": true },
                        "description": { "type": "text", "optional": true },
                        "is_system": { "type": "bool", "default": false },
                        "type": { "type": "string", "optional": true },
                        "metadata": { "type": "text", "optional": true },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    }
                },
                "iam_user_roles": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "user_id": { "type": "string" },
                        "role_id": { "type": "string", "optional": true },
                        "role_name": { "type": "string", "optional": true },
                        "granted_by": { "type": "string", "optional": true },
                        "granted_at": { "type": "datetime", "auto": true },
                        "expires_at": { "type": "datetime", "optional": true }
                    },
                    "indexes": [
                        { "fields": ["user_id"] },
                        { "fields": ["role_id"] },
                        { "fields": ["role_name"] },
                        { "fields": ["user_id", "role_id"], "unique": true }
                    ]
                },
                "iam_policies": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "ptype": { "type": "string" },
                        "v0": { "type": "string", "optional": true },
                        "v1": { "type": "string", "optional": true },
                        "v2": { "type": "string", "optional": true },
                        "v3": { "type": "string", "optional": true },
                        "v4": { "type": "string", "optional": true },
                        "v5": { "type": "string", "optional": true },
                        "created_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["ptype"] },
                        { "fields": ["v0"] }
                    ]
                },
                "iam_audit_logs": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "user_id": { "type": "string", "optional": true },
                        "action": { "type": "string", "optional": true },
                        "resource": { "type": "string", "optional": true },
                        "result": { "type": "string", "optional": true },
                        "reason": { "type": "string", "optional": true },
                        "ip_address": { "type": "string", "optional": true },
                        "user_agent": { "type": "string", "optional": true },
                        "metadata": { "type": "text", "optional": true },
                        "created_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["user_id"] },
                        { "fields": ["created_at"] }
                    ]
                },
                "sys_logs": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "level": { "type": "string" },
                        "message": { "type": "text" },
                        "fields": { "type": "text", "optional": true },
                        "user_id": { "type": "string", "optional": true },
                        "trace_id": { "type": "string", "optional": true },
                        "created_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["level"] },
                        { "fields": ["user_id"] },
                        { "fields": ["trace_id"] },
                        { "fields": ["created_at"] }
                    ]
                },
                "sys_message_logs": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "chain_id": { "type": "string" },
                        "block_name": { "type": "string" },
                        "message_kind": { "type": "string" },
                        "action": { "type": "string" },
                        "duration_ms": { "type": "int" },
                        "trace_id": { "type": "string", "optional": true },
                        "error": { "type": "text", "optional": true },
                        "user_id": { "type": "string", "optional": true },
                        "meta_snapshot": { "type": "text", "optional": true },
                        "created_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["chain_id"] },
                        { "fields": ["block_name"] },
                        { "fields": ["message_kind"] },
                        { "fields": ["action"] },
                        { "fields": ["trace_id"] },
                        { "fields": ["created_at"] }
                    ]
                },
                "sys_settings": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "key": { "type": "string", "unique": true },
                        "value": { "type": "text", "optional": true },
                        "type": { "type": "string", "default": "string" },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true },
                        "deleted_at": { "type": "datetime", "optional": true }
                    },
                    "indexes": [
                        { "fields": ["deleted_at"] }
                    ]
                },
                "custom_table_definitions": {
                    "fields": {
                        "id": { "type": "int", "primary": true, "auto": true },
                        "name": { "type": "string", "unique": true },
                        "display_name": { "type": "string", "optional": true },
                        "description": { "type": "text", "optional": true },
                        "fields": { "type": "text", "optional": true },
                        "indexes": { "type": "text", "optional": true },
                        "options": { "type": "text", "optional": true },
                        "created_by": { "type": "string", "optional": true },
                        "status": { "type": "string", "default": "active" },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    }
                },
                "custom_table_migrations": {
                    "fields": {
                        "id": { "type": "int", "primary": true, "auto": true },
                        "table_id": { "type": "int", "optional": true },
                        "version": { "type": "int", "optional": true },
                        "migration_type": { "type": "string", "optional": true },
                        "old_schema": { "type": "text", "optional": true },
                        "new_schema": { "type": "text", "optional": true },
                        "executed_by": { "type": "string", "optional": true },
                        "executed_at": { "type": "datetime", "auto": true },
                        "rollback_at": { "type": "datetime", "optional": true },
                        "status": { "type": "string", "optional": true },
                        "error_message": { "type": "text", "optional": true }
                    },
                    "indexes": [
                        { "fields": ["table_id"] }
                    ]
                }
            }
        }
    }
}"#;

    pub const FILES_SCHEMA: &str = r#"{
    "uses": {
        "@wafer/database": {
            "collections": {
                "storage_buckets": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "name": { "type": "string", "unique": true },
                        "public": { "type": "int", "default": 0 },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    }
                },
                "storage_objects": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "bucket_name": { "type": "string" },
                        "object_name": { "type": "string" },
                        "parent_folder_id": { "type": "string", "optional": true },
                        "size": { "type": "int", "optional": true },
                        "content_type": { "type": "string", "optional": true },
                        "checksum": { "type": "string", "optional": true },
                        "metadata": { "type": "text", "optional": true },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true },
                        "last_viewed": { "type": "datetime", "optional": true },
                        "user_id": { "type": "string", "optional": true },
                        "app_id": { "type": "string", "optional": true },
                        "is_folder": { "type": "int", "default": 0 }
                    },
                    "indexes": [
                        { "fields": ["bucket_name"] },
                        { "fields": ["object_name"] },
                        { "fields": ["parent_folder_id"] },
                        { "fields": ["checksum"] },
                        { "fields": ["last_viewed"] },
                        { "fields": ["user_id"] },
                        { "fields": ["app_id"] }
                    ]
                },
                "storage_upload_tokens": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "token": { "type": "string", "unique": true },
                        "bucket": { "type": "string" },
                        "parent_folder_id": { "type": "string", "optional": true },
                        "object_name": { "type": "string" },
                        "user_id": { "type": "string", "optional": true },
                        "max_size": { "type": "int", "optional": true },
                        "content_type": { "type": "string", "optional": true },
                        "bytes_uploaded": { "type": "int", "default": 0 },
                        "completed": { "type": "int", "default": 0 },
                        "object_id": { "type": "string", "optional": true },
                        "expires_at": { "type": "datetime" },
                        "created_at": { "type": "datetime", "auto": true },
                        "completed_at": { "type": "datetime", "optional": true },
                        "client_ip": { "type": "string", "optional": true }
                    }
                },
                "storage_download_tokens": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "token": { "type": "string", "unique": true },
                        "file_id": { "type": "string" },
                        "bucket": { "type": "string" },
                        "parent_folder_id": { "type": "string", "optional": true },
                        "object_name": { "type": "string" },
                        "user_id": { "type": "string", "optional": true },
                        "file_size": { "type": "int", "optional": true },
                        "bytes_served": { "type": "int", "default": 0 },
                        "completed": { "type": "int", "default": 0 },
                        "expires_at": { "type": "datetime" },
                        "created_at": { "type": "datetime", "auto": true },
                        "callback_at": { "type": "datetime", "optional": true },
                        "client_ip": { "type": "string", "optional": true }
                    }
                },
                "ext_cloudstorage_storage_shares": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "object_id": { "type": "string" },
                        "shared_with_user_id": { "type": "string", "optional": true },
                        "shared_with_email": { "type": "string", "optional": true },
                        "permission_level": { "type": "string", "default": "view" },
                        "inherit_to_children": { "type": "bool", "default": true },
                        "share_token": { "type": "string", "optional": true, "unique": true },
                        "is_public": { "type": "bool", "default": false },
                        "expires_at": { "type": "datetime", "optional": true },
                        "created_by": { "type": "string" },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["object_id"] },
                        { "fields": ["shared_with_user_id"] }
                    ]
                },
                "ext_cloudstorage_storage_access_logs": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "object_id": { "type": "string" },
                        "user_id": { "type": "string", "optional": true },
                        "ip_address": { "type": "string", "optional": true },
                        "action": { "type": "string" },
                        "user_agent": { "type": "string", "optional": true },
                        "metadata": { "type": "text", "default": "{}" },
                        "created_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["object_id"] },
                        { "fields": ["user_id"] }
                    ]
                },
                "ext_cloudstorage_storage_quotas": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "user_id": { "type": "string", "unique": true },
                        "max_storage_bytes": { "type": "int", "default": 5368709120 },
                        "max_bandwidth_bytes": { "type": "int", "default": 10737418240 },
                        "storage_used": { "type": "int", "default": 0 },
                        "bandwidth_used": { "type": "int", "default": 0 },
                        "reset_bandwidth_at": { "type": "datetime", "optional": true },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    }
                },
                "ext_cloudstorage_role_quotas": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "role_id": { "type": "string", "unique": true },
                        "role_name": { "type": "string" },
                        "max_storage_bytes": { "type": "int", "default": 5368709120 },
                        "max_bandwidth_bytes": { "type": "int", "default": 10737418240 },
                        "max_upload_size": { "type": "int", "default": 104857600 },
                        "max_files_count": { "type": "int", "default": 1000 },
                        "allowed_extensions": { "type": "string", "optional": true },
                        "blocked_extensions": { "type": "string", "optional": true },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["role_name"] }
                    ]
                },
                "ext_cloudstorage_user_quota_overrides": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "user_id": { "type": "string", "unique": true },
                        "max_storage_bytes": { "type": "int", "optional": true },
                        "max_bandwidth_bytes": { "type": "int", "optional": true },
                        "max_upload_size": { "type": "int", "optional": true },
                        "max_files_count": { "type": "int", "optional": true },
                        "allowed_extensions": { "type": "string", "optional": true },
                        "blocked_extensions": { "type": "string", "optional": true },
                        "reason": { "type": "text", "optional": true },
                        "expires_at": { "type": "datetime", "optional": true },
                        "created_by": { "type": "string" },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["expires_at"] },
                        { "fields": ["created_by"] }
                    ]
                }
            }
        }
    }
}"#;

    pub const PRODUCTS_SCHEMA: &str = r#"{
    "uses": {
        "@wafer/database": {
            "collections": {
                "ext_products_variables": {
                    "fields": {
                        "id": { "type": "int", "primary": true, "auto": true },
                        "name": { "type": "string", "unique": true },
                        "display_name": { "type": "string", "optional": true },
                        "value_type": { "type": "string", "optional": true },
                        "type": { "type": "string", "optional": true },
                        "default_value": { "type": "string", "optional": true },
                        "description": { "type": "text", "optional": true },
                        "status": { "type": "string", "default": "active" },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    }
                },
                "ext_products_group_templates": {
                    "fields": {
                        "id": { "type": "int", "primary": true, "auto": true },
                        "name": { "type": "string", "unique": true },
                        "display_name": { "type": "string", "optional": true },
                        "description": { "type": "text", "optional": true },
                        "icon": { "type": "string", "optional": true },
                        "filter_fields_schema": { "type": "json", "optional": true },
                        "status": { "type": "string", "default": "active" },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    }
                },
                "ext_products_groups": {
                    "fields": {
                        "id": { "type": "int", "primary": true, "auto": true },
                        "user_id": { "type": "string" },
                        "group_template_id": { "type": "int", "ref": "ext_products_group_templates.id" },
                        "name": { "type": "string" },
                        "description": { "type": "text", "optional": true },
                        "filter_numeric_1": { "type": "float", "optional": true },
                        "filter_numeric_2": { "type": "float", "optional": true },
                        "filter_numeric_3": { "type": "float", "optional": true },
                        "filter_numeric_4": { "type": "float", "optional": true },
                        "filter_numeric_5": { "type": "float", "optional": true },
                        "filter_text_1": { "type": "string", "optional": true },
                        "filter_text_2": { "type": "string", "optional": true },
                        "filter_text_3": { "type": "string", "optional": true },
                        "filter_text_4": { "type": "string", "optional": true },
                        "filter_text_5": { "type": "string", "optional": true },
                        "filter_boolean_1": { "type": "bool", "optional": true },
                        "filter_boolean_2": { "type": "bool", "optional": true },
                        "filter_boolean_3": { "type": "bool", "optional": true },
                        "filter_boolean_4": { "type": "bool", "optional": true },
                        "filter_boolean_5": { "type": "bool", "optional": true },
                        "filter_enum_1": { "type": "string", "optional": true },
                        "filter_enum_2": { "type": "string", "optional": true },
                        "filter_enum_3": { "type": "string", "optional": true },
                        "filter_enum_4": { "type": "string", "optional": true },
                        "filter_enum_5": { "type": "string", "optional": true },
                        "filter_location_1": { "type": "string", "optional": true },
                        "filter_location_2": { "type": "string", "optional": true },
                        "filter_location_3": { "type": "string", "optional": true },
                        "filter_location_4": { "type": "string", "optional": true },
                        "filter_location_5": { "type": "string", "optional": true },
                        "custom_fields": { "type": "json", "optional": true },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["user_id"] },
                        { "fields": ["group_template_id"] },
                        { "fields": ["filter_numeric_1"] },
                        { "fields": ["filter_numeric_2"] },
                        { "fields": ["filter_text_1"] },
                        { "fields": ["filter_text_2"] },
                        { "fields": ["filter_boolean_1"] },
                        { "fields": ["filter_enum_1"] }
                    ]
                },
                "ext_products_product_templates": {
                    "fields": {
                        "id": { "type": "int", "primary": true, "auto": true },
                        "name": { "type": "string", "unique": true },
                        "display_name": { "type": "string", "optional": true },
                        "description": { "type": "text", "optional": true },
                        "category": { "type": "string", "optional": true },
                        "icon": { "type": "string", "optional": true },
                        "filter_fields_schema": { "type": "json", "optional": true },
                        "custom_fields_schema": { "type": "json", "optional": true },
                        "pricing_templates": { "type": "json", "optional": true },
                        "billing_mode": { "type": "string", "default": "instant" },
                        "billing_type": { "type": "string", "default": "one-time" },
                        "billing_recurring_interval": { "type": "string", "optional": true },
                        "billing_recurring_interval_count": { "type": "int", "default": 1 },
                        "status": { "type": "string", "default": "active" },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    }
                },
                "ext_products_products": {
                    "fields": {
                        "id": { "type": "int", "primary": true, "auto": true },
                        "group_id": { "type": "int", "ref": "ext_products_groups.id" },
                        "product_template_id": { "type": "int", "ref": "ext_products_product_templates.id" },
                        "name": { "type": "string" },
                        "description": { "type": "text", "optional": true },
                        "base_price": { "type": "float", "optional": true },
                        "base_price_cents": { "type": "int", "optional": true },
                        "currency": { "type": "string", "default": "USD" },
                        "filter_numeric_1": { "type": "float", "optional": true },
                        "filter_numeric_2": { "type": "float", "optional": true },
                        "filter_numeric_3": { "type": "float", "optional": true },
                        "filter_numeric_4": { "type": "float", "optional": true },
                        "filter_numeric_5": { "type": "float", "optional": true },
                        "filter_text_1": { "type": "string", "optional": true },
                        "filter_text_2": { "type": "string", "optional": true },
                        "filter_text_3": { "type": "string", "optional": true },
                        "filter_text_4": { "type": "string", "optional": true },
                        "filter_text_5": { "type": "string", "optional": true },
                        "filter_boolean_1": { "type": "bool", "optional": true },
                        "filter_boolean_2": { "type": "bool", "optional": true },
                        "filter_boolean_3": { "type": "bool", "optional": true },
                        "filter_boolean_4": { "type": "bool", "optional": true },
                        "filter_boolean_5": { "type": "bool", "optional": true },
                        "filter_enum_1": { "type": "string", "optional": true },
                        "filter_enum_2": { "type": "string", "optional": true },
                        "filter_enum_3": { "type": "string", "optional": true },
                        "filter_enum_4": { "type": "string", "optional": true },
                        "filter_enum_5": { "type": "string", "optional": true },
                        "filter_location_1": { "type": "string", "optional": true },
                        "filter_location_2": { "type": "string", "optional": true },
                        "filter_location_3": { "type": "string", "optional": true },
                        "filter_location_4": { "type": "string", "optional": true },
                        "filter_location_5": { "type": "string", "optional": true },
                        "custom_fields": { "type": "json", "optional": true },
                        "variables": { "type": "json", "optional": true },
                        "pricing_formula": { "type": "text", "optional": true },
                        "active": { "type": "bool", "default": true },
                        "status": { "type": "string", "default": "draft" },
                        "created_by": { "type": "string", "optional": true },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["group_id"] },
                        { "fields": ["product_template_id"] },
                        { "fields": ["active"] },
                        { "fields": ["status"] },
                        { "fields": ["created_by"] },
                        { "fields": ["filter_numeric_1"] },
                        { "fields": ["filter_numeric_2"] },
                        { "fields": ["filter_text_1"] },
                        { "fields": ["filter_text_2"] },
                        { "fields": ["filter_boolean_1"] },
                        { "fields": ["filter_enum_1"] }
                    ]
                },
                "ext_products_pricing_templates": {
                    "fields": {
                        "id": { "type": "int", "primary": true, "auto": true },
                        "name": { "type": "string", "unique": true },
                        "display_name": { "type": "string", "optional": true },
                        "description": { "type": "text", "optional": true },
                        "price_formula": { "type": "text" },
                        "condition_formula": { "type": "text", "optional": true },
                        "variables": { "type": "json", "optional": true },
                        "category": { "type": "string", "optional": true },
                        "status": { "type": "string", "default": "active" },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    }
                },
                "ext_products_purchases": {
                    "fields": {
                        "id": { "type": "int", "primary": true, "auto": true },
                        "user_id": { "type": "string" },
                        "provider": { "type": "string", "default": "stripe" },
                        "provider_session_id": { "type": "string", "optional": true },
                        "provider_payment_intent_id": { "type": "string", "optional": true },
                        "provider_subscription_id": { "type": "string", "optional": true },
                        "line_items": { "type": "json", "optional": true },
                        "product_metadata": { "type": "json", "optional": true },
                        "tax_items": { "type": "json", "optional": true },
                        "amount_cents": { "type": "int", "optional": true },
                        "tax_cents": { "type": "int", "optional": true },
                        "total_cents": { "type": "int", "optional": true },
                        "currency": { "type": "string", "default": "USD" },
                        "status": { "type": "string", "default": "pending" },
                        "requires_approval": { "type": "bool", "default": false },
                        "approved_at": { "type": "datetime", "optional": true },
                        "approved_by": { "type": "string", "optional": true },
                        "refunded_at": { "type": "datetime", "optional": true },
                        "refund_reason": { "type": "text", "optional": true },
                        "refund_amount": { "type": "int", "optional": true },
                        "cancelled_at": { "type": "datetime", "optional": true },
                        "cancel_reason": { "type": "text", "optional": true },
                        "success_url": { "type": "string", "optional": true },
                        "cancel_url": { "type": "string", "optional": true },
                        "customer_email": { "type": "string", "optional": true },
                        "customer_name": { "type": "string", "optional": true },
                        "billing_address": { "type": "json", "optional": true },
                        "shipping_address": { "type": "json", "optional": true },
                        "payment_method_types": { "type": "json", "optional": true },
                        "expires_at": { "type": "datetime", "optional": true },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true }
                    },
                    "indexes": [
                        { "fields": ["user_id"] },
                        { "fields": ["provider_session_id"] },
                        { "fields": ["provider_payment_intent_id"] },
                        { "fields": ["provider_subscription_id"] },
                        { "fields": ["status"] }
                    ]
                }
            }
        }
    }
}"#;

    pub const DEPLOYMENTS_SCHEMA: &str = r#"{
    "uses": {
        "@wafer/database": {
            "collections": {
                "ext_deployments": {
                    "fields": {
                        "user_id": { "type": "string" },
                        "name": { "type": "string" },
                        "slug": { "type": "string" },
                        "status": { "type": "string" },
                        "plan_id": { "type": "string", "optional": true },
                        "purchase_id": { "type": "string", "optional": true },
                        "region": { "type": "string" },
                        "tenant_id": { "type": "string", "optional": true },
                        "subdomain": { "type": "string", "optional": true },
                        "provision_error": { "type": "string", "optional": true },
                        "config": { "type": "json", "optional": true },
                        "created_at": { "type": "datetime" },
                        "updated_at": { "type": "datetime" },
                        "deleted_at": { "type": "datetime", "optional": true }
                    }
                }
            }
        }
    }
}"#;

    pub const LEGALPAGES_SCHEMA: &str = r#"{
    "uses": {
        "@wafer/database": {
            "collections": {
                "ext_legalpages_legal_documents": {
                    "fields": {
                        "id": { "type": "string", "primary": true },
                        "doc_type": { "type": "string" },
                        "title": { "type": "string" },
                        "content": { "type": "text", "optional": true },
                        "version": { "type": "int", "default": 1 },
                        "status": { "type": "string", "default": "draft" },
                        "created_at": { "type": "datetime", "auto": true },
                        "updated_at": { "type": "datetime", "auto": true },
                        "created_by": { "type": "string", "optional": true }
                    },
                    "indexes": [
                        { "fields": ["doc_type", "status"] }
                    ]
                }
            }
        }
    }
}"#;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_config(json: &str) -> AppConfig {
        serde_json::from_str(json).expect("valid JSON config")
    }

    #[test]
    fn test_minimal_config() {
        let config = parse_config("{}");
        assert!(!config.auth_enabled());
        assert!(!config.admin_enabled());
        assert!(!config.files_enabled());
        assert!(!config.products_enabled());
        assert!(!config.deployments_enabled());
        assert!(!config.legalpages_enabled());
        assert!(!config.userportal_enabled());
    }

    #[test]
    fn test_features_enabled_with_empty_object() {
        let config = parse_config(r#"{"auth": {}, "admin": {}, "files": {}}"#);
        assert!(config.auth_enabled());
        assert!(config.admin_enabled());
        assert!(config.files_enabled());
        assert!(!config.products_enabled());
    }

    #[test]
    fn test_features_enabled_with_true() {
        let config = parse_config(r#"{"auth": true, "products": true}"#);
        assert!(config.auth_enabled());
        assert!(config.products_enabled());
        assert!(!config.files_enabled());
    }

    #[test]
    fn test_features_disabled_with_false() {
        let config = parse_config(r#"{"auth": false, "admin": false}"#);
        assert!(!config.auth_enabled());
        assert!(!config.admin_enabled());
    }

    #[test]
    fn test_features_disabled_with_null() {
        let config = parse_config(r#"{"auth": null}"#);
        assert!(!config.auth_enabled());
    }

    #[test]
    fn test_features_enabled_with_config_object() {
        let config = parse_config(r#"{"auth": {"enable_signup": true}}"#);
        assert!(config.auth_enabled());
    }

    #[test]
    fn test_enabled_features_list() {
        let config = parse_config(r#"{"auth": {}, "files": {}, "legalpages": {}}"#);
        let features = config.enabled_features();
        assert!(features.contains(&"system")); // always present
        assert!(features.contains(&"profile")); // always present
        assert!(features.contains(&"auth"));
        assert!(features.contains(&"files"));
        assert!(features.contains(&"legalpages"));
        assert!(!features.contains(&"products"));
        assert!(!features.contains(&"admin"));
    }

    #[test]
    fn test_disabled_features_list() {
        let config = parse_config(r#"{"auth": {}, "files": {}}"#);
        let disabled = config.disabled_features();
        assert!(!disabled.contains(&"auth"));
        assert!(!disabled.contains(&"files"));
        assert!(disabled.contains(&"admin"));
        assert!(disabled.contains(&"products"));
        assert!(disabled.contains(&"deployments"));
    }

    #[test]
    fn test_database_config_defaults() {
        let config = parse_config("{}");
        assert!(config.database.is_none());

        let config = parse_config(r#"{"database": {}}"#);
        let db = config.database.unwrap();
        assert_eq!(db.db_type, "sqlite");
        assert_eq!(db.path, "data/solobase.db");
        assert!(db.url.is_none());
    }

    #[test]
    fn test_database_config_custom() {
        let config = parse_config(r#"{"database": {"type": "postgres", "url": "postgres://localhost/db"}}"#);
        let db = config.database.unwrap();
        assert_eq!(db.db_type, "postgres");
        assert_eq!(db.url.unwrap(), "postgres://localhost/db");
    }

    #[test]
    fn test_storage_config_defaults() {
        let config = parse_config(r#"{"storage": {}}"#);
        let storage = config.storage.unwrap();
        assert_eq!(storage.storage_type, "local");
        assert_eq!(storage.root, "data/storage");
    }

    #[test]
    fn test_full_config() {
        let config = parse_config(r#"{
            "version": 1,
            "app": "my-store",
            "listen": "127.0.0.1:3000",
            "database": {"type": "sqlite", "path": "my.db"},
            "storage": {"type": "local", "root": "/data"},
            "jwt_secret": "test-secret",
            "web_root": "./dist",
            "auth": {},
            "admin": {},
            "files": {},
            "products": {},
            "legalpages": {}
        }"#);

        assert_eq!(config.version, 1);
        assert_eq!(config.app.as_deref().unwrap(), "my-store");
        assert_eq!(config.listen.as_deref().unwrap(), "127.0.0.1:3000");
        assert_eq!(config.jwt_secret.as_deref().unwrap(), "test-secret");
        assert_eq!(config.web_root.as_deref().unwrap(), "./dist");
        assert!(config.auth_enabled());
        assert!(config.admin_enabled());
        assert!(config.files_enabled());
        assert!(config.products_enabled());
        assert!(!config.deployments_enabled());
        assert!(config.legalpages_enabled());
        assert!(!config.userportal_enabled());
    }

    #[test]
    fn test_to_blocks_json_produces_valid_output() {
        let config = parse_config(r#"{"auth": {}, "admin": {}}"#);
        let (blocks, aliases) = config.to_blocks_json();

        // Should have infrastructure blocks
        assert!(blocks.contains_key("@wafer/http-listener"));

        // Should contain the specific backend blocks (defaults: sqlite + local-storage)
        assert!(blocks.contains_key("solobase/sqlite"));
        assert!(blocks.contains_key("solobase/local-storage"));

        // Should contain the solobase feature blocks
        assert!(blocks.contains_key("@solobase/auth"));
        assert!(blocks.contains_key("@solobase/admin"));

        // Should have @db and @storage aliases pointing to specific backends
        assert!(aliases.iter().any(|(a, t)| a == "@db" && t == "solobase/sqlite"));
        assert!(aliases.iter().any(|(a, t)| a == "@storage" && t == "solobase/local-storage"));
        // Backward-compat aliases
        assert!(aliases.iter().any(|(a, t)| a == "@wafer/database" && t == "solobase/sqlite"));
        assert!(aliases.iter().any(|(a, t)| a == "@wafer/storage" && t == "solobase/local-storage"));
    }

    #[test]
    fn test_default_version_is_zero() {
        let config = parse_config("{}");
        assert_eq!(config.version, 0);
    }
}
