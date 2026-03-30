//! Periodic sync of R2 and D1 storage usage from the Cloudflare API.
//!
//! Called by the scheduled (cron) handler. Queries the CF API for actual
//! storage sizes and writes them to the platform `project_usage` table.

use serde::Deserialize;
use worker::*;

use crate::cf_api::CfCredentials;

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

// ---------------------------------------------------------------------------
// CF API response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CfApiResponse<T> {
    #[allow(dead_code)]
    success: bool,
    result: Option<T>,
}

#[derive(Deserialize)]
struct D1DatabaseInfo {
    file_size: Option<u64>,
}

#[derive(Deserialize)]
struct R2BucketUsage {
    #[serde(rename = "payloadSize")]
    payload_size: Option<u64>,
}

// ---------------------------------------------------------------------------
// Sync logic
// ---------------------------------------------------------------------------

/// Sync storage usage for all active projects.
pub async fn sync_all(env: &Env) -> Result<()> {
    let db = env.d1("DB").map_err(|e| Error::RustError(format!("D1: {e}")))?;
    let creds = CfCredentials::from_env(env)?;
    let month = current_month();

    // Get all active projects with their D1 database IDs
    let projects = match db
        .prepare("SELECT id, subdomain, db_id FROM projects WHERE status = 'active'")
        .bind(&[])?
        .all()
        .await
    {
        Ok(result) => {
            let rows: Vec<serde_json::Value> = result.results()
                .unwrap_or_default();
            rows
        }
        Err(e) => {
            console_log!("storage_sync: failed to list projects: {e}");
            return Ok(());
        }
    };

    console_log!("storage_sync: syncing {} active projects", projects.len());

    for project in &projects {
        let project_id = project.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let subdomain = project.get("subdomain").and_then(|v| v.as_str()).unwrap_or("");
        let db_id = project.get("db_id").and_then(|v| v.as_str()).unwrap_or("");

        if project_id.is_empty() {
            continue;
        }

        // Query D1 database size
        let d1_bytes = if !db_id.is_empty() {
            get_d1_size(&creds, db_id).await.unwrap_or(0)
        } else {
            0
        };

        // Query R2 bucket size (bucket name = project subdomain)
        let r2_bytes = if !subdomain.is_empty() {
            get_r2_size(&creds, subdomain).await.unwrap_or(0)
        } else {
            0
        };

        // Upsert into project_usage
        let usage_id = format!("{project_id}:{month}");
        let result = db.prepare(
            "INSERT INTO project_usage (id, project_id, month, requests, r2_bytes, d1_bytes) \
             VALUES (?1, ?2, ?3, 0, ?4, ?5) \
             ON CONFLICT (project_id, month) DO UPDATE SET r2_bytes = ?4, d1_bytes = ?5"
        )
        .bind(&[
            usage_id.into(),
            project_id.into(),
            month.clone().into(),
            wasm_bindgen::JsValue::from_f64(r2_bytes as f64),
            wasm_bindgen::JsValue::from_f64(d1_bytes as f64),
        ]);

        match result {
            Ok(stmt) => {
                if let Err(e) = stmt.run().await {
                    console_log!("storage_sync: failed to update usage for {project_id}: {e}");
                }
            }
            Err(e) => {
                console_log!("storage_sync: failed to prepare usage update for {project_id}: {e}");
            }
        }
    }

    // Reset addon columns for the new month (addons are per billing month).
    // The current month's row is created fresh by `increment_usage`, so old
    // months keep their addon values for historical tracking. We only need
    // to ensure the current month starts clean if no usage row exists yet.
    // The upsert above already handles this — new rows get addon = 0.

    console_log!("storage_sync: done");
    Ok(())
}

// ---------------------------------------------------------------------------
// CF API queries
// ---------------------------------------------------------------------------

/// Get D1 database file size in bytes.
async fn get_d1_size(creds: &CfCredentials, database_id: &str) -> Result<u64> {
    let url = format!(
        "{}/accounts/{}/d1/database/{}",
        CF_API_BASE, creds.account_id, database_id
    );

    let mut req = Request::new_with_init(
        &url,
        RequestInit::new().with_method(Method::Get),
    )?;
    req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;

    let mut resp = Fetch::Request(req).send().await?;
    let result: CfApiResponse<D1DatabaseInfo> = resp.json().await?;

    Ok(result.result.and_then(|r| r.file_size).unwrap_or(0))
}

/// Get R2 bucket total size in bytes.
///
/// Uses the R2 bucket usage endpoint. Falls back to 0 if the bucket
/// doesn't exist or the API returns an error.
async fn get_r2_size(creds: &CfCredentials, bucket_name: &str) -> Result<u64> {
    // The R2 usage API endpoint — returns bucket storage metrics.
    // Note: This endpoint may require specific permissions on the API token.
    let url = format!(
        "{}/accounts/{}/r2/buckets/{}/usage",
        CF_API_BASE, creds.account_id, bucket_name
    );

    let mut req = Request::new_with_init(
        &url,
        RequestInit::new().with_method(Method::Get),
    )?;
    req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;

    let mut resp = Fetch::Request(req).send().await?;

    if resp.status_code() != 200 {
        // Bucket may not exist yet or API endpoint not available
        return Ok(0);
    }

    let result: CfApiResponse<R2BucketUsage> = resp.json().await?;
    Ok(result.result.and_then(|r| r.payload_size).unwrap_or(0))
}

fn current_month() -> String {
    chrono::Utc::now().format("%Y-%m").to_string()
}
