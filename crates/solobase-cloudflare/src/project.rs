//! Project configuration, resolution, and plan limits.
//!
//! Projects are the multi-tenant units in Solobase Cloud. Each project
//! maps to a subdomain ({project}.solobase.dev) and has its own D1 database.
//!
//! KV key: `project:{subdomain}:config` → ProjectConfig JSON

use serde::{Deserialize, Serialize};
use serde_json::Value;
use solobase_core::features;

/// Reserved subdomains that cannot be used as project names.
pub const RESERVED_SUBDOMAINS: &[&str] = &[
    "admin", "api", "app", "auth", "billing", "blog", "cdn", "cloud",
    "console", "dashboard", "dev", "docs", "help", "internal", "login",
    "mail", "manage", "platform", "settings", "staging", "status",
    "support", "test", "www",
];

/// Check if a subdomain is reserved.
pub fn is_reserved_subdomain(subdomain: &str) -> bool {
    RESERVED_SUBDOMAINS.contains(&subdomain.to_lowercase().as_str())
}

/// Check if the host is the platform host.
///
/// Platform hosts serve the shared DB (auth, billing, dashboard).
/// Project hosts ({project}.solobase.dev) serve per-project data.
pub fn is_platform_host(host: &str) -> bool {
    let host_no_port = host.split(':').next().unwrap_or(host);
    host_no_port == "localhost"
        || host_no_port == "127.0.0.1"
        || host_no_port == "cloud.solobase.dev"
        || host_no_port.ends_with(".workers.dev") // workers.dev preview URLs
}

/// Per-project configuration stored in KV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Unique project identifier.
    pub id: String,
    /// Subdomain (e.g., `myapp` → `myapp.solobase.dev`).
    pub subdomain: String,
    /// Display name of the project.
    #[serde(default)]
    pub name: String,
    /// Billing plan: "free", "starter", "pro", "platform".
    #[serde(default = "default_plan")]
    pub plan: String,
    /// Project status: "active" or "inactive".
    #[serde(default = "default_status")]
    pub status: String,
    /// The user who owns this project.
    #[serde(default)]
    pub owner_user_id: Option<String>,
    /// D1 database UUID for this project.
    #[serde(default)]
    pub db_id: Option<String>,
    /// Worker binding name for this project's D1.
    #[serde(default)]
    pub db_binding: Option<String>,
    /// The project's app config (feature flags).
    #[serde(default)]
    pub config: ProjectAppConfig,
    /// Custom WASM block names installed by this project.
    #[serde(default)]
    pub blocks: Vec<String>,
}

fn default_plan() -> String { "free".to_string() }
fn default_status() -> String { "active".to_string() }

/// App config — mirrors solobase.json feature flags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectAppConfig {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub app: Option<String>,
    #[serde(default)]
    pub auth: Option<Value>,
    #[serde(default)]
    pub admin: Option<Value>,
    #[serde(default)]
    pub files: Option<Value>,
    #[serde(default)]
    pub products: Option<Value>,
    #[serde(default)]
    pub deployments: Option<Value>,
    #[serde(default)]
    pub legalpages: Option<Value>,
    #[serde(default)]
    pub userportal: Option<Value>,
}

impl features::FeatureConfig for ProjectAppConfig {
    fn auth_enabled(&self) -> bool { features::is_feature_enabled(&self.auth) }
    fn admin_enabled(&self) -> bool { features::is_feature_enabled(&self.admin) }
    fn files_enabled(&self) -> bool { features::is_feature_enabled(&self.files) }
    fn products_enabled(&self) -> bool { features::is_feature_enabled(&self.products) }
    fn deployments_enabled(&self) -> bool { features::is_feature_enabled(&self.deployments) }
    fn legalpages_enabled(&self) -> bool { features::is_feature_enabled(&self.legalpages) }
    fn userportal_enabled(&self) -> bool { features::is_feature_enabled(&self.userportal) }
}

impl ProjectAppConfig {
    /// Create a config with all features enabled.
    pub fn all_enabled() -> Self {
        let on = Some(Value::Object(Default::default()));
        Self {
            version: 1,
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

// ---------------------------------------------------------------------------
// Plan limits
// ---------------------------------------------------------------------------

/// Plan limits for resource enforcement.
#[allow(dead_code)]
pub struct PlanLimits {
    pub max_projects: usize,
    pub max_requests_per_month: u64,
    pub max_r2_storage_bytes: u64,
    pub max_d1_storage_bytes: u64,
}

/// Get limits for a plan name.
pub fn get_plan_limits(plan: &str) -> PlanLimits {
    match plan {
        "starter" => PlanLimits {
            max_projects: 2,
            max_requests_per_month: 500_000,
            max_r2_storage_bytes: 1_073_741_824, // 1 GB
            max_d1_storage_bytes: 524_288_000,   // 500 MB
        },
        "pro" => PlanLimits {
            max_projects: usize::MAX,
            max_requests_per_month: 3_000_000,
            max_r2_storage_bytes: 10_737_418_240, // 10 GB
            max_d1_storage_bytes: 5_368_709_120,  // 5 GB
        },
        "platform" => PlanLimits {
            max_projects: usize::MAX,
            max_requests_per_month: u64::MAX,
            max_r2_storage_bytes: u64::MAX,
            max_d1_storage_bytes: u64::MAX,
        },
        // "free" or unknown
        _ => PlanLimits {
            max_projects: 0,
            max_requests_per_month: 0,
            max_r2_storage_bytes: 0,
            max_d1_storage_bytes: 0,
        },
    }
}

// ---------------------------------------------------------------------------
// Project resolution
// ---------------------------------------------------------------------------

/// Resolve a project from hostname via KV lookup.
pub async fn resolve_project(
    hostname: &str,
    kv: &worker::kv::KvStore,
    is_dev: bool,
) -> std::result::Result<ProjectConfig, String> {
    let host_no_port = hostname.split(':').next().unwrap_or(hostname);
    let subdomain = host_no_port
        .split('.')
        .next()
        .ok_or_else(|| "invalid hostname".to_string())?;

    // Dev mode: return a dev project for localhost
    if is_dev && (subdomain == "localhost" || subdomain == "127") {
        return Ok(ProjectConfig {
            id: "dev".to_string(),
            subdomain: "localhost".to_string(),
            name: "Development".to_string(),
            plan: "platform".to_string(),
            status: "active".to_string(),
            owner_user_id: None,
            db_id: None,
            db_binding: Some("DB".to_string()),
            config: ProjectAppConfig::all_enabled(),
            blocks: Vec::new(),
        });
    }

    if is_reserved_subdomain(subdomain) {
        return Err(format!("subdomain '{}' is reserved", subdomain));
    }

    let key = format!("project:{}:config", subdomain);
    let config = kv
        .get(&key)
        .json::<ProjectConfig>()
        .await
        .map_err(|e| format!("KV get error: {e}"))?
        .ok_or_else(|| format!("project '{}' not found", subdomain))?;

    Ok(config)
}
