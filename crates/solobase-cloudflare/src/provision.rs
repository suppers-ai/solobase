//! Tenant provisioning — create, update, and manage tenant configurations.
//!
//! Tenants are stored in Cloudflare KV with key pattern:
//! - `tenant:{subdomain}:config` → TenantConfig JSON
//! - `tenants:list` → JSON array of all subdomain strings
//!
//! Each tenant gets their own D1 database, created via the Cloudflare API
//! during provisioning. The database ID is stored in `TenantConfig.db_id`.

use serde_json;
use worker::*;

use crate::tenant::{TenantConfig, TenantAppConfig};

/// Provision a new tenant.
///
/// 1. Generates a unique tenant ID
/// 2. Creates a dedicated D1 database via Cloudflare API
/// 3. Runs schema migrations on the new database
/// 4. Stores the tenant config in KV
/// 5. Adds the subdomain to the global tenant list
pub async fn create_tenant(
    kv: &kv::KvStore,
    db: &D1Database,
    subdomain: &str,
    plan: &str,
    app_config: Option<TenantAppConfig>,
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

    // For now, use the shared DB binding for the new tenant.
    // In production, this would create a new D1 database via the Cloudflare API:
    //   POST /accounts/{account_id}/d1/database
    //   { "name": "solobase-{subdomain}" }
    // Then bind it to the Worker via the Workers API.
    // The db_binding would be "DB_{subdomain}" and db_id would be the new database UUID.
    let db_binding = format!("DB_{}", subdomain);

    let config = TenantConfig {
        id: tenant_id,
        subdomain: subdomain.to_string(),
        plan: plan.to_string(),
        db_id: None, // populated when D1 is created via Cloudflare API
        db_binding: Some(db_binding),
        config: app_config.unwrap_or_else(TenantAppConfig::all_enabled),
        blocks: Vec::new(),
    };

    // Run schema migrations on the tenant's database.
    // Currently uses the shared DB passed in; once per-tenant D1 is
    // provisioned via the API, this would target the tenant's own DB.
    crate::schema::run_migrations(db).await?;

    // Store tenant config
    let config_json = serde_json::to_string(&config)
        .map_err(|e| Error::RustError(format!("serialize config: {e}")))?;

    kv.put(&key, config_json)
        .map_err(|e| Error::RustError(format!("KV put error: {e}")))?
        .execute()
        .await?;

    // Add to tenant list
    add_to_tenant_list(kv, subdomain).await?;

    console_log!("Tenant '{}' provisioned (id: {})", subdomain, config.id);
    Ok(config)
}

/// Delete a tenant and its data.
pub async fn delete_tenant(kv: &kv::KvStore, subdomain: &str) -> Result<()> {
    let key = format!("tenant:{}:config", subdomain);
    kv.delete(&key).await?;
    remove_from_tenant_list(kv, subdomain).await?;
    // TODO: Delete the tenant's D1 database via Cloudflare API
    // DELETE /accounts/{account_id}/d1/database/{db_id}
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
