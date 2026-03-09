//! Tenant configuration and routing.

use serde::{Deserialize, Serialize};

/// Per-tenant configuration stored in KV.
///
/// KV key: `tenant:{subdomain}:config`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    /// Unique tenant identifier.
    pub id: String,
    /// Postgres schema name for this tenant (e.g., `tenant_abc`).
    pub schema: String,
    /// Subdomain (e.g., `myapp` → `myapp.solobase.app`).
    pub subdomain: String,
    /// Billing plan.
    #[serde(default = "default_plan")]
    pub plan: String,
    /// Feature flags for this tenant.
    #[serde(default)]
    pub features: TenantFeatures,
    /// Custom WASM block names installed by this tenant.
    /// Each block's compiled .wasm is stored at KV key `tenant:{id}:block:{name}`.
    #[serde(default)]
    pub blocks: Vec<String>,
}

/// Feature flags per tenant.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TenantFeatures {
    #[serde(default = "default_true")]
    pub auth: bool,
    #[serde(default = "default_true")]
    pub admin: bool,
    #[serde(default = "default_true")]
    pub files: bool,
    #[serde(default = "default_true")]
    pub products: bool,
    #[serde(default = "default_true")]
    pub monitoring: bool,
    #[serde(default = "default_true")]
    pub legalpages: bool,
    #[serde(default = "default_true")]
    pub userportal: bool,
    #[serde(default = "default_true")]
    pub profile: bool,
}

fn default_plan() -> String {
    "hobby".to_string()
}

fn default_true() -> bool {
    true
}
