//! Project configuration, resolution, and plan limits.
//!
//! Projects are the multi-tenant units in Solobase Cloud. Each project
//! maps to a subdomain ({project}.solobase.dev) and has its own D1 database.
//!
//! Project data is stored in the platform D1 database (`projects` table).

use serde::{Deserialize, Serialize};
use worker::D1Database;

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
        || host_no_port == "cloud.solobase-dev.dev"
}

/// Per-project configuration stored in the platform D1 database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Unique project identifier.
    pub id: String,
    /// Subdomain (e.g., `myapp` → `myapp.solobase.dev`).
    pub subdomain: String,
    /// Display name of the project.
    #[serde(default)]
    pub name: String,
    /// Billing plan: "free", "starter", "pro".
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
    /// Whether this is a platform project (has FEATURE_PROJECTS enabled).
    #[serde(default)]
    pub platform: bool,
    /// Custom WASM block names installed by this project.
    #[serde(default)]
    pub blocks: Vec<String>,
}

fn default_plan() -> String { "free".to_string() }
fn default_status() -> String { "active".to_string() }

/// Build a ProjectConfig from a D1 row returned as serde_json::Value.
pub fn project_from_row(row: &serde_json::Value) -> Option<ProjectConfig> {
    Some(ProjectConfig {
        id: row.get("id")?.as_str()?.to_string(),
        subdomain: row.get("subdomain")?.as_str()?.to_string(),
        name: row.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        plan: row.get("plan").and_then(|v| v.as_str()).unwrap_or("free").to_string(),
        status: row.get("status").and_then(|v| v.as_str()).unwrap_or("active").to_string(),
        owner_user_id: row.get("owner_user_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from),
        db_id: row.get("db_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from),
        platform: row.get("platform").and_then(|v| v.as_i64()).unwrap_or(0) != 0,
        blocks: Vec::new(),
    })
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
            max_r2_storage_bytes: 2_147_483_648,  // 2 GB
            max_d1_storage_bytes: 524_288_000,    // 500 MB
        },
        "pro" => PlanLimits {
            max_projects: 10,
            max_requests_per_month: 3_000_000,
            max_r2_storage_bytes: 21_474_836_480, // 20 GB
            max_d1_storage_bytes: 5_368_709_120,  // 5 GB
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

/// Resolve a project from hostname via D1 lookup.
pub async fn resolve_project(
    hostname: &str,
    db: &D1Database,
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
            plan: "pro".to_string(),
            status: "active".to_string(),
            owner_user_id: None,
            db_id: None,
            platform: true,
            blocks: Vec::new(),
        });
    }

    if is_reserved_subdomain(subdomain) {
        return Err(format!("subdomain '{}' is reserved", subdomain));
    }

    let row = db
        .prepare("SELECT * FROM projects WHERE subdomain = ?1")
        .bind(&[subdomain.into()])
        .map_err(|e| format!("D1 bind error: {e}"))?
        .first::<serde_json::Value>(None)
        .await
        .map_err(|e| format!("D1 query error: {e}"))?
        .ok_or_else(|| format!("project '{}' not found", subdomain))?;

    project_from_row(&row)
        .ok_or_else(|| format!("failed to parse project '{}' from D1", subdomain))
}
