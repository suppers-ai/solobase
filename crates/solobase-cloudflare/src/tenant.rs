//! Tenant configuration and routing.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Per-tenant configuration stored in KV.
///
/// KV key: `tenant:{subdomain}:config`
///
/// The `config` field holds the tenant's `app.json` — the same format used
/// by the standalone binary.  Feature blocks are present = enabled, absent
/// = disabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    /// Unique tenant identifier.
    pub id: String,
    /// Subdomain (e.g., `myapp` → `myapp.solobase.app`).
    pub subdomain: String,
    /// Billing plan.
    #[serde(default = "default_plan")]
    pub plan: String,
    /// D1 database UUID for this tenant (created via Cloudflare API).
    /// Each tenant gets its own isolated D1 database.
    #[serde(default)]
    pub db_id: Option<String>,
    /// Worker binding name for this tenant's D1 (e.g., `DB_myapp`).
    /// Falls back to the shared `DB` binding if not set.
    #[serde(default)]
    pub db_binding: Option<String>,
    /// The tenant's app config (same schema as `app.json`).
    #[serde(default)]
    pub config: TenantAppConfig,
    /// Custom WASM block names installed by this tenant.
    /// Each block's compiled .wasm is stored at KV key `tenant:{id}:block:{name}`.
    #[serde(default)]
    pub blocks: Vec<String>,
}

/// Embedded app config — mirrors the standalone `AppConfig` struct but
/// without infrastructure fields that are handled by the Worker (D1, R2, etc.).
///
/// Feature presence rules (same as standalone `app.json`):
/// - Present as `{}` or `true` → enabled
/// - Absent or `false` or `null` → disabled
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TenantAppConfig {
    /// Config format version (default 0 = beta).
    #[serde(default)]
    pub version: u32,

    /// Instance name (informational).
    #[serde(default)]
    pub app: Option<String>,

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

fn default_plan() -> String {
    "hobby".to_string()
}

// ---------------------------------------------------------------------------
// Feature detection (same logic as standalone AppConfig)
// ---------------------------------------------------------------------------

impl TenantAppConfig {
    /// Returns true if the feature value means "enabled".
    fn is_enabled(val: &Option<Value>) -> bool {
        match val {
            None => false,
            Some(Value::Bool(false)) => false,
            Some(Value::Null) => false,
            _ => true,
        }
    }

    pub fn auth_enabled(&self) -> bool { Self::is_enabled(&self.auth) }
    pub fn admin_enabled(&self) -> bool { Self::is_enabled(&self.admin) }
    pub fn files_enabled(&self) -> bool { Self::is_enabled(&self.files) }
    pub fn products_enabled(&self) -> bool { Self::is_enabled(&self.products) }
    pub fn deployments_enabled(&self) -> bool { Self::is_enabled(&self.deployments) }
    pub fn legalpages_enabled(&self) -> bool { Self::is_enabled(&self.legalpages) }
    pub fn userportal_enabled(&self) -> bool { Self::is_enabled(&self.userportal) }
}

impl TenantAppConfig {
    /// Create a config with all features enabled (the default for new tenants).
    pub fn all_enabled() -> Self {
        let on = Some(Value::Object(Default::default()));
        Self {
            version: 0,
            app: None,
            auth: on.clone(),
            admin: on.clone(),
            files: on.clone(),
            products: on.clone(),
            deployments: on.clone(),
            legalpages: on.clone(),
            userportal: on,
        }
    }
}
