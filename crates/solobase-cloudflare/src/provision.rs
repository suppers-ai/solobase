//! Project provisioning — create, update, and manage project configurations.
//!
//! Projects are stored in Cloudflare KV with key pattern:
//! - `project:{subdomain}:config` → ProjectConfig JSON
//! - `projects:list` → JSON array of all subdomain strings

use worker::*;

use crate::project::{ProjectConfig, ProjectAppConfig};

/// Provision a new project.
pub async fn create_project(
    kv: &kv::KvStore,
    subdomain: &str,
    name: &str,
    plan: &str,
    owner_user_id: Option<&str>,
    app_config: Option<ProjectAppConfig>,
) -> Result<ProjectConfig> {
    let key = format!("project:{}:config", subdomain);
    if kv.get(&key).json::<ProjectConfig>().await?.is_some() {
        return Err(Error::RustError(format!("project '{}' already exists", subdomain)));
    }

    let project_id = uuid::Uuid::new_v4().to_string();

    let config = ProjectConfig {
        id: project_id.clone(),
        subdomain: subdomain.to_string(),
        name: if name.is_empty() { subdomain.to_string() } else { name.to_string() },
        plan: plan.to_string(),
        status: "active".to_string(),
        owner_user_id: owner_user_id.map(String::from),
        db_id: None,
        db_binding: None,
        config: app_config.unwrap_or_else(ProjectAppConfig::all_enabled),
        blocks: Vec::new(),
    };

    // Store project config
    let config_json = serde_json::to_string(&config)
        .map_err(|e| Error::RustError(format!("serialize config: {e}")))?;

    kv.put(&key, config_json)
        .map_err(|e| Error::RustError(format!("KV put error: {e}")))?
        .execute()
        .await?;

    add_to_project_list(kv, subdomain).await?;

    console_log!("Project '{}' provisioned (id: {})", subdomain, config.id);
    Ok(config)
}

/// Delete a project.
pub async fn delete_project(kv: &kv::KvStore, subdomain: &str) -> Result<()> {
    let key = format!("project:{}:config", subdomain);
    kv.delete(&key).await?;
    remove_from_project_list(kv, subdomain).await?;
    console_log!("Project '{}' deleted", subdomain);
    Ok(())
}

/// List all project subdomains.
pub async fn list_projects(kv: &kv::KvStore) -> Result<Vec<String>> {
    let list = kv
        .get("projects:list")
        .json::<Vec<String>>()
        .await?
        .unwrap_or_default();
    Ok(list)
}

/// Get a project's config.
pub async fn get_project(kv: &kv::KvStore, subdomain: &str) -> Result<Option<ProjectConfig>> {
    let key = format!("project:{}:config", subdomain);
    kv.get(&key).json::<ProjectConfig>().await.map_err(|e| e.into())
}

/// Update a project's config.
pub async fn update_project(
    kv: &kv::KvStore,
    subdomain: &str,
    config: &ProjectConfig,
) -> Result<()> {
    let key = format!("project:{}:config", subdomain);
    let config_json = serde_json::to_string(config)
        .map_err(|e| Error::RustError(format!("serialize config: {e}")))?;

    kv.put(&key, config_json)
        .map_err(|e| Error::RustError(format!("KV put error: {e}")))?
        .execute()
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async fn add_to_project_list(kv: &kv::KvStore, subdomain: &str) -> Result<()> {
    let mut list = list_projects(kv).await?;
    if !list.contains(&subdomain.to_string()) {
        list.push(subdomain.to_string());
        let json = serde_json::to_string(&list)
            .map_err(|e| Error::RustError(format!("serialize list: {e}")))?;
        kv.put("projects:list", json)
            .map_err(|e| Error::RustError(format!("KV put error: {e}")))?
            .execute()
            .await?;
    }
    Ok(())
}

async fn remove_from_project_list(kv: &kv::KvStore, subdomain: &str) -> Result<()> {
    let mut list = list_projects(kv).await?;
    list.retain(|s| s != subdomain);
    let json = serde_json::to_string(&list)
        .map_err(|e| Error::RustError(format!("serialize list: {e}")))?;
    kv.put("projects:list", json)
        .map_err(|e| Error::RustError(format!("KV put error: {e}")))?
        .execute()
        .await?;
    Ok(())
}
