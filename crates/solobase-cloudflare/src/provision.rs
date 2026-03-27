//! Project provisioning — create, update, and manage project configurations.
//!
//! Projects are stored in the platform D1 database (`projects` table).
//!
//! Provisioning creates a D1 database and uploads a user worker via the
//! Cloudflare API, then triggers schema migrations on the new worker.

use wasm_bindgen::JsValue;
use worker::*;

use crate::cf_api::{self, CfCredentials, WorkerBindings};
use crate::project::{project_from_row, ProjectConfig};

/// Provision a new project: create D1 + R2 bucket, upload user worker, run migrations.
pub async fn create_project(
    env: &Env,
    db: &D1Database,
    subdomain: &str,
    name: &str,
    plan: &str,
    owner_user_id: Option<&str>,
    platform: bool,
) -> Result<ProjectConfig> {
    // Check if project already exists
    let existing = db
        .prepare("SELECT id FROM projects WHERE subdomain = ?1")
        .bind(&[subdomain.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    if existing.is_some() {
        return Err(Error::RustError(format!("project '{}' already exists", subdomain)));
    }

    let creds = CfCredentials::from_env(env)?;
    let project_id = uuid::Uuid::new_v4().to_string();
    let environment = env.var("ENVIRONMENT")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "development".into());
    let db_name = format!("solobase-{}-{}", environment, subdomain);

    // 1. Create D1 database
    let db_id = cf_api::create_d1_database(&creds, &db_name).await?;
    console_log!("Created D1 '{}' (id: {})", db_name, db_id);

    // 2. Create per-project R2 bucket
    let r2_bucket_name = db_name.clone(); // same naming: solobase-{env}-{subdomain}
    if let Err(e) = cf_api::create_r2_bucket(&creds, &r2_bucket_name).await {
        console_log!("R2 bucket create failed for '{}', rolling back D1", subdomain);
        let _ = cf_api::delete_d1_database(&creds, &db_id).await;
        return Err(e);
    }
    console_log!("Created R2 bucket '{}'", r2_bucket_name);

    // 3. Read user worker artifacts from the shared platform R2 bucket
    let platform_bucket = env.bucket("STORAGE")
        .map_err(|e| Error::RustError(format!("R2: {e}")))?;

    let js_module = read_r2_bytes(&platform_bucket, "_system/worker/index.js").await
        .map_err(|e| Error::RustError(format!("read worker JS from R2: {e}")))?;
    let wasm_bytes = read_r2_bytes(&platform_bucket, "_system/worker/index_bg.wasm").await
        .map_err(|e| Error::RustError(format!("read worker WASM from R2: {e}")))?;

    // 4. Build worker bindings
    // Service binding to dispatcher only for platform projects (used by the projects block)
    let dispatch_worker_name = if platform {
        Some(format!("solobase-{}", environment))
    } else {
        None
    };
    let bindings = WorkerBindings {
        d1_database_id: db_id.clone(),
        r2_bucket_name: r2_bucket_name.clone(),
        secrets: Vec::new(),
        vars: vec![
            ("FEATURE_PROJECTS".to_string(), platform.to_string()),
        ],
        dispatch_worker_name,
    };

    // 5. Upload user worker to dispatch namespace
    if let Err(e) = cf_api::upload_user_worker(&creds, subdomain, &js_module, &wasm_bytes, &bindings).await {
        console_log!("Worker upload failed for '{}', rolling back D1 and R2", subdomain, );
        let _ = cf_api::delete_d1_database(&creds, &db_id).await;
        let _ = cf_api::delete_r2_bucket(&creds, &r2_bucket_name).await;
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

    // 6. Health check — verify the worker is responsive
    let health_req = Request::new("https://internal/health", Method::Get)?;
    let health_resp = dispatcher.get(subdomain)
        .map_err(|e| Error::RustError(format!("dispatch get: {e}")))?
        .fetch_request(health_req).await?;

    if health_resp.status_code() >= 400 {
        console_log!("Health check warning for '{}': status {}", subdomain, health_resp.status_code());
    } else {
        console_log!("Health check passed for '{}'", subdomain);
    }

    // 7. Store project config in D1
    let display_name = if name.is_empty() { subdomain } else { name };
    let owner = owner_user_id.unwrap_or("");
    let platform_int: u8 = if platform { 1 } else { 0 };

    db.prepare(
        "INSERT INTO projects (id, subdomain, name, plan, status, owner_user_id, db_id, platform) \
         VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, ?7)"
    )
    .bind(&[
        project_id.clone().into(),
        subdomain.into(),
        display_name.into(),
        plan.into(),
        owner.into(),
        db_id.clone().into(),
        JsValue::from(platform_int),
    ])?
    .run()
    .await?;

    let config = ProjectConfig {
        id: project_id.clone(),
        subdomain: subdomain.to_string(),
        name: if name.is_empty() { subdomain.to_string() } else { name.to_string() },
        plan: plan.to_string(),
        status: "active".to_string(),
        owner_user_id: owner_user_id.map(String::from),
        db_id: Some(db_id),
        platform,
        blocks: Vec::new(),
    };

    console_log!("Project '{}' fully provisioned (id: {})", subdomain, config.id);
    Ok(config)
}

/// Delete a project: remove user worker, D1 database, R2 bucket, and D1 row.
pub async fn delete_project(
    env: &Env,
    db: &D1Database,
    subdomain: &str,
) -> Result<()> {
    let creds = CfCredentials::from_env(env)?;
    let environment = env.var("ENVIRONMENT")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "development".into());

    // Look up project config to get db_id
    let config = get_project(db, subdomain).await?;

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

    // Delete per-project R2 bucket
    let r2_bucket_name = format!("solobase-{}-{}", environment, subdomain);
    if let Err(e) = cf_api::delete_r2_bucket(&creds, &r2_bucket_name).await {
        console_log!("Warning: failed to delete R2 bucket '{}': {e}", r2_bucket_name);
    }

    // Remove from D1
    db.prepare("DELETE FROM projects WHERE subdomain = ?1")
        .bind(&[subdomain.into()])?
        .run()
        .await?;

    console_log!("Project '{}' deleted", subdomain);
    Ok(())
}

/// List all project subdomains.
pub async fn list_projects(db: &D1Database) -> Result<Vec<String>> {
    let results = db
        .prepare("SELECT subdomain FROM projects")
        .bind(&[])?
        .all()
        .await?;

    let rows = results.results::<serde_json::Value>()?;
    let subdomains = rows
        .iter()
        .filter_map(|row| row.get("subdomain").and_then(|v| v.as_str().map(String::from)))
        .collect();
    Ok(subdomains)
}

/// Get a project's config.
pub async fn get_project(db: &D1Database, subdomain: &str) -> Result<Option<ProjectConfig>> {
    let row = db
        .prepare("SELECT * FROM projects WHERE subdomain = ?1")
        .bind(&[subdomain.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.as_ref().and_then(project_from_row))
}

/// Update a project's config.
pub async fn update_project(
    db: &D1Database,
    subdomain: &str,
    config: &ProjectConfig,
) -> Result<()> {
    let owner = config.owner_user_id.as_deref().unwrap_or("");
    let db_id = config.db_id.as_deref().unwrap_or("");
    let platform_int: u8 = if config.platform { 1 } else { 0 };

    db.prepare(
        "UPDATE projects SET name = ?1, plan = ?2, status = ?3, owner_user_id = ?4, \
         db_id = ?5, platform = ?6, updated_at = datetime('now') WHERE subdomain = ?7"
    )
    .bind(&[
        config.name.as_str().into(),
        config.plan.as_str().into(),
        config.status.as_str().into(),
        owner.into(),
        db_id.into(),
        JsValue::from(platform_int),
        subdomain.into(),
    ])?
    .run()
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
