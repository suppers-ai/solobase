//! Tenant provisioning — create, update, and manage tenant configurations.
//!
//! Tenants are stored in Cloudflare KV with key pattern:
//! - `tenant:{subdomain}:config` → TenantConfig JSON
//! - `tenants:list` → JSON array of all subdomain strings

use serde_json;
use worker::*;

use crate::tenant::{TenantConfig, TenantFeatures};

/// Provision a new tenant.
///
/// 1. Generates a unique tenant ID
/// 2. Stores the tenant config in KV
/// 3. Adds the subdomain to the global tenant list
/// 4. Runs schema migrations for the tenant's tables
pub async fn create_tenant(
    kv: &kv::KvStore,
    db: &D1Database,
    subdomain: &str,
    plan: &str,
) -> Result<TenantConfig> {
    // Check if tenant already exists
    let key = format!("tenant:{}:config", subdomain);
    if kv.get(&key).json::<TenantConfig>().await?.is_some() {
        return Err(Error::RustError(format!(
            "tenant '{}' already exists",
            subdomain
        )));
    }

    let tenant_id = uuid::Uuid::new_v4().to_string();

    let config = TenantConfig {
        id: tenant_id,
        schema: subdomain.to_string(),
        subdomain: subdomain.to_string(),
        plan: plan.to_string(),
        features: TenantFeatures::default(),
        blocks: Vec::new(),
    };

    // Store tenant config
    let config_json = serde_json::to_string(&config)
        .map_err(|e| Error::RustError(format!("serialize config: {e}")))?;

    kv.put(&key, config_json)
        .map_err(|e| Error::RustError(format!("KV put error: {e}")))?
        .execute()
        .await?;

    // Add to tenant list
    add_to_tenant_list(kv, subdomain).await?;

    // Run schema migrations
    crate::schema::run_migrations(db).await?;

    console_log!("Tenant '{}' provisioned (id: {})", subdomain, config.id);
    Ok(config)
}

/// Delete a tenant and its data.
pub async fn delete_tenant(kv: &kv::KvStore, subdomain: &str) -> Result<()> {
    let key = format!("tenant:{}:config", subdomain);
    kv.delete(&key).await?;
    remove_from_tenant_list(kv, subdomain).await?;
    console_log!("Tenant '{}' deleted", subdomain);
    Ok(())
}

/// List all tenant subdomains.
pub async fn list_tenants(kv: &kv::KvStore) -> Result<Vec<String>> {
    let list = kv
        .get("tenants:list")
        .json::<Vec<String>>()
        .await?
        .unwrap_or_default();
    Ok(list)
}

/// Get a tenant's config.
pub async fn get_tenant(kv: &kv::KvStore, subdomain: &str) -> Result<Option<TenantConfig>> {
    let key = format!("tenant:{}:config", subdomain);
    kv.get(&key).json::<TenantConfig>().await.map_err(|e| e.into())
}

/// Update a tenant's config.
pub async fn update_tenant(
    kv: &kv::KvStore,
    subdomain: &str,
    config: &TenantConfig,
) -> Result<()> {
    let key = format!("tenant:{}:config", subdomain);
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

async fn add_to_tenant_list(kv: &kv::KvStore, subdomain: &str) -> Result<()> {
    let mut list = list_tenants(kv).await?;
    if !list.contains(&subdomain.to_string()) {
        list.push(subdomain.to_string());
        let json = serde_json::to_string(&list)
            .map_err(|e| Error::RustError(format!("serialize list: {e}")))?;
        kv.put("tenants:list", json)
            .map_err(|e| Error::RustError(format!("KV put error: {e}")))?
            .execute()
            .await?;
    }
    Ok(())
}

async fn remove_from_tenant_list(kv: &kv::KvStore, subdomain: &str) -> Result<()> {
    let mut list = list_tenants(kv).await?;
    list.retain(|s| s != subdomain);
    let json = serde_json::to_string(&list)
        .map_err(|e| Error::RustError(format!("serialize list: {e}")))?;
    kv.put("tenants:list", json)
        .map_err(|e| Error::RustError(format!("KV put error: {e}")))?
        .execute()
        .await?;
    Ok(())
}
