//! Project provisioning — create, update, and manage project configurations.
//!
//! Projects are stored in Cloudflare KV with key pattern:
//! - `project:{subdomain}:config` → ProjectConfig JSON
//! - `projects:list` → JSON array of all subdomain strings
//!
//! Provisioning creates a D1 database and uploads a user worker via the
//! Cloudflare API, then triggers schema migrations on the new worker.

use worker::*;

use crate::cf_api::{self, CfCredentials, WorkerBindings};
use crate::project::ProjectConfig;

/// Provision a new project: create D1, upload user worker, run migrations.
pub async fn create_project(
    env: &Env,
    kv: &kv::KvStore,
    subdomain: &str,
    name: &str,
    plan: &str,
    owner_user_id: Option<&str>,
    app_config: Option<serde_json::Value>,
) -> Result<ProjectConfig> {
    let key = format!("project:{}:config", subdomain);
    if kv.get(&key).json::<ProjectConfig>().await?.is_some() {
        return Err(Error::RustError(format!("project '{}' already exists", subdomain)));
    }

    let creds = CfCredentials::from_env(env)?;
    let project_id = uuid::Uuid::new_v4().to_string();
    let db_name = format!("solobase-proj-{}", subdomain);

    // 1. Create D1 database
    let db_id = cf_api::create_d1_database(&creds, &db_name).await?;
    console_log!("Created D1 '{}' (id: {})", db_name, db_id);

    // 2. Read user worker artifacts from R2
    let bucket = env.bucket("STORAGE")
        .map_err(|e| Error::RustError(format!("R2: {e}")))?;

    let js_module = read_r2_bytes(&bucket, "_system/worker/index.js").await
        .map_err(|e| Error::RustError(format!("read worker JS from R2: {e}")))?;
    let wasm_bytes = read_r2_bytes(&bucket, "_system/worker/index_bg.wasm").await
        .map_err(|e| Error::RustError(format!("read worker WASM from R2: {e}")))?;

    // 3. Collect secrets to forward to the user worker
    let config_json = app_config
        .unwrap_or_else(|| all_features_enabled())
        .to_string();

    let r2_bucket_name = env.var("R2_BUCKET_NAME")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "solobase-storage".into());

    // Only infrastructure bindings — all config lives in the D1 variables table.
    let bindings = WorkerBindings {
        d1_database_id: db_id.clone(),
        r2_bucket_name,
        secrets: Vec::new(),
        vars: vec![
            ("PROJECT_ID".to_string(), project_id.clone()),
        ],
    };

    // 4. Upload user worker to dispatch namespace
    if let Err(e) = cf_api::upload_user_worker(&creds, subdomain, &js_module, &wasm_bytes, &bindings).await {
        // Rollback: delete the D1 database we just created
        console_log!("Worker upload failed for '{}', rolling back D1 '{}'", subdomain, db_id);
        let _ = cf_api::delete_d1_database(&creds, &db_id).await;
        return Err(e);
    }
    console_log!("Uploaded user worker '{}'", subdomain);

    // 5. Trigger schema migrations via the dispatcher
    let dispatcher = env.dynamic_dispatcher("DISPATCHER")
        .map_err(|e| Error::RustError(format!("dispatcher: {e}")))?;

    let migrate_req = Request::new(
        "https://internal/_internal/migrate",
        Method::Post,
    )?;
    let migrate_resp = dispatcher.get(subdomain)
        .map_err(|e| Error::RustError(format!("dispatch get: {e}")))?
        .fetch_request(migrate_req).await?;

    if migrate_resp.status_code() >= 400 {
        console_log!("Migration warning for '{}': status {}", subdomain, migrate_resp.status_code());
    }

    // 6. Store project config in KV
    let config = ProjectConfig {
        id: project_id.clone(),
        subdomain: subdomain.to_string(),
        name: if name.is_empty() { subdomain.to_string() } else { name.to_string() },
        plan: plan.to_string(),
        status: "active".to_string(),
        owner_user_id: owner_user_id.map(String::from),
        db_id: Some(db_id),
        db_binding: None, // User workers use their own DB binding, not the dispatch's
        config: serde_json::from_str(&config_json).unwrap_or_default(),
        blocks: Vec::new(),
    };

    let config_str = serde_json::to_string(&config)
        .map_err(|e| Error::RustError(format!("serialize config: {e}")))?;

    kv.put(&format!("project:{}:config", subdomain), config_str)
        .map_err(|e| Error::RustError(format!("KV put: {e}")))?
        .execute()
        .await?;

    add_to_project_list(kv, subdomain).await?;

    console_log!("Project '{}' fully provisioned (id: {})", subdomain, config.id);
    Ok(config)
}

/// Delete a project: remove user worker, D1 database, and KV config.
pub async fn delete_project(
    env: &Env,
    kv: &kv::KvStore,
    subdomain: &str,
) -> Result<()> {
    let creds = CfCredentials::from_env(env)?;

    // Look up project config to get db_id
    let key = format!("project:{}:config", subdomain);
    let config = kv.get(&key).json::<ProjectConfig>().await?;

    // Delete user worker from namespace
    if let Err(e) = cf_api::delete_user_worker(&creds, subdomain).await {
        console_log!("Warning: failed to delete worker '{}': {e}", subdomain);
    }

    // Delete D1 database
    if let Some(ref cfg) = config {
        if let Some(ref db_id) = cfg.db_id {
            if let Err(e) = cf_api::delete_d1_database(&creds, db_id).await {
                console_log!("Warning: failed to delete D1 '{}': {e}", db_id);
            }
        }
    }

    // Remove from KV
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
        .map_err(|e| Error::RustError(format!("KV put: {e}")))?
        .execute()
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async fn read_r2_bytes(bucket: &Bucket, key: &str) -> Result<Vec<u8>> {
    let obj = bucket.get(key).execute().await?
        .ok_or_else(|| Error::RustError(format!("R2 object '{}' not found", key)))?;
    let body = obj.body()
        .ok_or_else(|| Error::RustError(format!("R2 object '{}' has no body", key)))?;
    body.bytes().await
}

fn all_features_enabled() -> serde_json::Value {
    serde_json::json!({
        "version": 1,
        "auth": {},
        "admin": {},
        "files": {},
        "products": {},
        "deployments": {},
        "legalpages": {},
        "userportal": {}
    })
}

async fn add_to_project_list(kv: &kv::KvStore, subdomain: &str) -> Result<()> {
    let mut list = list_projects(kv).await?;
    if !list.contains(&subdomain.to_string()) {
        list.push(subdomain.to_string());
        let json = serde_json::to_string(&list)
            .map_err(|e| Error::RustError(format!("serialize list: {e}")))?;
        kv.put("projects:list", json)
            .map_err(|e| Error::RustError(format!("KV put: {e}")))?
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
        .map_err(|e| Error::RustError(format!("KV put: {e}")))?
        .execute()
        .await?;
    Ok(())
}
