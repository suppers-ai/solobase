//! Cloudflare REST API client for D1 database and Workers for Platforms management.
//!
//! Used by the dispatch worker to:
//! - Create/delete D1 databases per project
//! - Upload/delete/update user workers in the dispatch namespace
//!
//! All calls go through `api.cloudflare.com` using `worker::Fetch`.

use serde::{Deserialize, Serialize};
use worker::*;

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

// ---------------------------------------------------------------------------
// Credentials helper
// ---------------------------------------------------------------------------

/// Cloudflare API credentials read from worker secrets.
pub struct CfCredentials {
    pub account_id: String,
    pub api_token: String,
    /// Dispatch namespace name (not UUID) — e.g. "solobase-workers-dev".
    pub namespace_name: String,
}

impl CfCredentials {
    /// Read credentials from the worker environment.
    pub fn from_env(env: &Env) -> Result<Self> {
        let account_id = env.secret("CF_ACCOUNT_ID")
            .map(|s| s.to_string())
            .map_err(|_| Error::RustError("CF_ACCOUNT_ID secret not set".into()))?;
        let api_token = env.secret("CF_API_TOKEN")
            .map(|s| s.to_string())
            .map_err(|_| Error::RustError("CF_API_TOKEN secret not set".into()))?;
        let namespace_name = env.secret("DISPATCHER_NAMESPACE")
            .or_else(|_| env.var("DISPATCHER_NAMESPACE"))
            .map(|s| s.to_string())
            .map_err(|_| Error::RustError("DISPATCHER_NAMESPACE secret/var not set".into()))?;
        Ok(Self { account_id, api_token, namespace_name })
    }
}

// ---------------------------------------------------------------------------
// D1 database management
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CfApiResponse<T> {
    success: bool,
    result: Option<T>,
    errors: Option<Vec<CfApiError>>,
}

#[derive(Deserialize)]
struct CfApiError {
    message: String,
}

#[derive(Deserialize)]
struct D1CreateResult {
    uuid: String,
}

/// Create a new D1 database.
pub async fn create_d1_database(creds: &CfCredentials, name: &str) -> Result<String> {
    let url = format!("{}/accounts/{}/d1/database", CF_API_BASE, creds.account_id);
    let body = serde_json::json!({ "name": name });

    let mut req = Request::new_with_init(
        &url,
        RequestInit::new()
            .with_method(Method::Post)
            .with_body(Some(wasm_bindgen::JsValue::from_str(&body.to_string()))),
    )?;
    req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;
    req.headers_mut()?.set("Content-Type", "application/json")?;

    let mut resp = Fetch::Request(req).send().await?;
    let result: CfApiResponse<D1CreateResult> = resp.json().await?;

    if !result.success {
        let msg = result.errors
            .and_then(|e| e.first().map(|e| e.message.clone()))
            .unwrap_or_else(|| "unknown error".into());
        return Err(Error::RustError(format!("D1 create failed: {msg}")));
    }

    result.result
        .map(|r| r.uuid)
        .ok_or_else(|| Error::RustError("D1 create: no result".into()))
}

/// Delete a D1 database.
pub async fn delete_d1_database(creds: &CfCredentials, database_id: &str) -> Result<()> {
    let url = format!(
        "{}/accounts/{}/d1/database/{}",
        CF_API_BASE, creds.account_id, database_id
    );

    let mut req = Request::new_with_init(
        &url,
        RequestInit::new().with_method(Method::Delete),
    )?;
    req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;

    let mut resp = Fetch::Request(req).send().await?;
    let result: CfApiResponse<serde_json::Value> = resp.json().await?;

    if !result.success {
        let msg = result.errors
            .and_then(|e| e.first().map(|e| e.message.clone()))
            .unwrap_or_else(|| "unknown error".into());
        return Err(Error::RustError(format!("D1 delete failed: {msg}")));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// R2 bucket management
// ---------------------------------------------------------------------------

/// Create a new R2 bucket. Succeeds silently if the bucket already exists.
pub async fn create_r2_bucket(creds: &CfCredentials, name: &str) -> Result<()> {
    let url = format!("{}/accounts/{}/r2/buckets", CF_API_BASE, creds.account_id);
    let body = serde_json::json!({ "name": name });

    let mut req = Request::new_with_init(
        &url,
        RequestInit::new()
            .with_method(Method::Post)
            .with_body(Some(wasm_bindgen::JsValue::from_str(&body.to_string()))),
    )?;
    req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;
    req.headers_mut()?.set("Content-Type", "application/json")?;

    let mut resp = Fetch::Request(req).send().await?;
    let result: CfApiResponse<serde_json::Value> = resp.json().await?;

    if !result.success {
        // Ignore "already exists" errors (code 10004) — makes provisioning idempotent
        let is_already_exists = result.errors.as_ref()
            .and_then(|e| e.first())
            .map(|e| e.message.contains("already exists"))
            .unwrap_or(false);
        if !is_already_exists {
            let msg = result.errors
                .and_then(|e| e.first().map(|e| e.message.clone()))
                .unwrap_or_else(|| "unknown error".into());
            return Err(Error::RustError(format!("R2 bucket create failed: {msg}")));
        }
    }
    Ok(())
}

/// Delete an R2 bucket (must be empty).
pub async fn delete_r2_bucket(creds: &CfCredentials, name: &str) -> Result<()> {
    let url = format!("{}/accounts/{}/r2/buckets/{}", CF_API_BASE, creds.account_id, name);

    let mut req = Request::new_with_init(
        &url,
        RequestInit::new().with_method(Method::Delete),
    )?;
    req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;

    let resp = Fetch::Request(req).send().await?;
    if resp.status_code() >= 400 && resp.status_code() != 404 {
        return Err(Error::RustError(format!("R2 bucket delete failed: status {}", resp.status_code())));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// User worker management
// ---------------------------------------------------------------------------

/// Bindings to attach to a user worker at upload time.
#[derive(Debug, Clone, Serialize)]
pub struct WorkerBindings {
    pub d1_database_id: String,
    pub r2_bucket_name: String,
    pub secrets: Vec<(String, String)>,
    pub vars: Vec<(String, String)>,
    /// If set, adds a service binding named "DISPATCHER" pointing to this worker.
    pub dispatch_worker_name: Option<String>,
}

impl WorkerBindings {
    /// Serialize bindings to the CF API metadata format.
    fn to_metadata_bindings(&self) -> Vec<serde_json::Value> {
        let mut bindings = vec![
            serde_json::json!({
                "type": "d1",
                "name": "DB",
                "id": self.d1_database_id,
            }),
            serde_json::json!({
                "type": "r2_bucket",
                "name": "STORAGE",
                "bucket_name": self.r2_bucket_name,
            }),
        ];

        for (key, value) in &self.secrets {
            bindings.push(serde_json::json!({
                "type": "secret_text",
                "name": key,
                "text": value,
            }));
        }

        for (key, value) in &self.vars {
            bindings.push(serde_json::json!({
                "type": "plain_text",
                "name": key,
                "text": value,
            }));
        }

        if let Some(ref worker_name) = self.dispatch_worker_name {
            bindings.push(serde_json::json!({
                "type": "service",
                "name": "DISPATCHER",
                "service": worker_name,
            }));
        }

        bindings
    }
}

/// Upload a user worker to the dispatch namespace.
///
/// The worker script is a JS module that imports the WASM module. Both the JS
/// shim and WASM binary are provided as byte slices (read from R2).
pub async fn upload_user_worker(
    creds: &CfCredentials,
    script_name: &str,
    js_module: &[u8],
    wasm_bytes: &[u8],
    bindings: &WorkerBindings,
) -> Result<()> {
    let url = format!(
        "{}/accounts/{}/workers/dispatch/namespaces/{}/scripts/{}",
        CF_API_BASE, creds.account_id, creds.namespace_name, script_name
    );

    let metadata = serde_json::json!({
        "main_module": "index.js",
        "compatibility_date": "2026-03-01",
        "bindings": bindings.to_metadata_bindings(),
    });

    // Build multipart form body manually
    let boundary = "----SolobaseWorkerUpload";
    let mut body = Vec::new();

    // Part 1: metadata
    append_form_part(
        &mut body,
        boundary,
        "metadata",
        "application/json",
        metadata.to_string().as_bytes(),
    );

    // Part 2: JS module (main entry point)
    append_form_part(
        &mut body,
        boundary,
        "index.js",
        "application/javascript+module",
        js_module,
    );

    // Part 3: WASM module
    append_form_part(
        &mut body,
        boundary,
        "index_bg.wasm",
        "application/wasm",
        wasm_bytes,
    );

    // Close boundary
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    let js_body = js_sys::Uint8Array::from(body.as_slice());

    let mut req = Request::new_with_init(
        &url,
        RequestInit::new()
            .with_method(Method::Put)
            .with_body(Some(js_body.into())),
    )?;
    req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;
    req.headers_mut()?.set(
        "Content-Type",
        &format!("multipart/form-data; boundary={boundary}"),
    )?;

    let mut resp = Fetch::Request(req).send().await?;
    let result: CfApiResponse<serde_json::Value> = resp.json().await?;

    if !result.success {
        let msg = result.errors
            .and_then(|e| e.first().map(|e| e.message.clone()))
            .unwrap_or_else(|| "unknown error".into());
        return Err(Error::RustError(format!("worker upload failed: {msg}")));
    }
    Ok(())
}

/// Delete a user worker from the dispatch namespace.
pub async fn delete_user_worker(creds: &CfCredentials, script_name: &str) -> Result<()> {
    let url = format!(
        "{}/accounts/{}/workers/dispatch/namespaces/{}/scripts/{}",
        CF_API_BASE, creds.account_id, creds.namespace_name, script_name
    );

    let mut req = Request::new_with_init(
        &url,
        RequestInit::new().with_method(Method::Delete),
    )?;
    req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;

    let mut resp = Fetch::Request(req).send().await?;
    let result: CfApiResponse<serde_json::Value> = resp.json().await?;

    if !result.success {
        let msg = result.errors
            .and_then(|e| e.first().map(|e| e.message.clone()))
            .unwrap_or_else(|| "unknown error".into());
        return Err(Error::RustError(format!("worker delete failed: {msg}")));
    }
    Ok(())
}

/// Update all user workers in the namespace with new code, preserving bindings.
///
/// Returns the names of successfully updated workers.
pub async fn update_all_workers(
    creds: &CfCredentials,
    js_module: &[u8],
    wasm_bytes: &[u8],
) -> Result<Vec<String>> {
    // List all scripts in the namespace
    let list_url = format!(
        "{}/accounts/{}/workers/dispatch/namespaces/{}/scripts",
        CF_API_BASE, creds.account_id, creds.namespace_name
    );

    let mut list_req = Request::new_with_init(
        &list_url,
        RequestInit::new().with_method(Method::Get),
    )?;
    list_req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;

    let mut list_resp = Fetch::Request(list_req).send().await?;

    #[derive(Deserialize)]
    struct ScriptInfo {
        id: String,
    }
    let list_result: CfApiResponse<Vec<ScriptInfo>> = list_resp.json().await?;

    let scripts = list_result.result.unwrap_or_default();
    let mut updated = Vec::new();

    for script in &scripts {
        let url = format!(
            "{}/accounts/{}/workers/dispatch/namespaces/{}/scripts/{}",
            CF_API_BASE, creds.account_id, creds.namespace_name, script.id
        );

        let metadata = serde_json::json!({
            "main_module": "index.js",
            "compatibility_date": "2026-03-01",
            "keep_bindings": ["d1", "r2_bucket", "secret_text", "plain_text", "service"],
        });

        let boundary = "----SolobaseWorkerUpdate";
        let mut body = Vec::new();

        append_form_part(&mut body, boundary, "metadata", "application/json", metadata.to_string().as_bytes());
        append_form_part(&mut body, boundary, "index.js", "application/javascript+module", js_module);
        append_form_part(&mut body, boundary, "index_bg.wasm", "application/wasm", wasm_bytes);
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        let js_body = js_sys::Uint8Array::from(body.as_slice());

        let mut req = Request::new_with_init(
            &url,
            RequestInit::new()
                .with_method(Method::Put)
                .with_body(Some(js_body.into())),
        )?;
        req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;
        req.headers_mut()?.set(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )?;

        let resp = Fetch::Request(req).send().await?;
        if resp.status_code() < 400 {
            updated.push(script.id.clone());
        } else {
            console_log!("Failed to update worker '{}': status {}", script.id, resp.status_code());
        }
    }

    Ok(updated)
}

// ---------------------------------------------------------------------------
// D1 query (read from a project's database via CF API)
// ---------------------------------------------------------------------------

/// Query a D1 database via the Cloudflare REST API.
/// Returns the first row's results as a Vec of JSON objects.
pub async fn query_d1(
    creds: &CfCredentials,
    database_id: &str,
    sql: &str,
    params: &[&str],
) -> Result<Vec<serde_json::Value>> {
    let url = format!(
        "{}/accounts/{}/d1/database/{}/query",
        CF_API_BASE, creds.account_id, database_id
    );

    let body = serde_json::json!({
        "sql": sql,
        "params": params,
    });

    let mut req = Request::new_with_init(
        &url,
        RequestInit::new()
            .with_method(Method::Post)
            .with_body(Some(wasm_bindgen::JsValue::from_str(&body.to_string()))),
    )?;
    req.headers_mut()?.set("Authorization", &format!("Bearer {}", creds.api_token))?;
    req.headers_mut()?.set("Content-Type", "application/json")?;

    let mut resp = Fetch::Request(req).send().await?;
    let result: CfApiResponse<Vec<D1QueryResult>> = resp.json().await?;

    if !result.success {
        let msg = result.errors
            .and_then(|e| e.first().map(|e| e.message.clone()))
            .unwrap_or_else(|| "unknown error".into());
        return Err(Error::RustError(format!("D1 query failed: {msg}")));
    }

    Ok(result.result
        .and_then(|r| r.into_iter().next())
        .map(|r| r.results)
        .unwrap_or_default())
}

#[derive(Deserialize)]
struct D1QueryResult {
    results: Vec<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Multipart form helpers
// ---------------------------------------------------------------------------

fn append_form_part(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    content_type: &str,
    data: &[u8],
) {
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{name}\"\r\n").as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(data);
    body.extend_from_slice(b"\r\n");
}
