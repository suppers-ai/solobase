//! CloudflareContext — implements the WAFER Context trait backed by Cloudflare services.
//!
//! Routes `call_block` to:
//! - `@wafer/database` → D1 via D1DatabaseService
//! - `@wafer/storage` → R2 via R2StorageService
//! - `@wafer/config` → env vars / KV
//! - `@wafer/crypto` → argon2 + HMAC-SHA256 JWT
//! - `@wafer/network` → Worker fetch()
//! - `@wafer/logger` → console_log

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use wafer_core::interfaces::database::handler as db_handler;
use wafer_core::interfaces::storage::handler as storage_handler;
use wafer_run::context::Context;
use wafer_run::types::*;

use crate::database::D1DatabaseService;
use crate::storage::R2StorageService;

/// WAFER Context backed by Cloudflare Workers services (D1, R2, KV).
///
/// Solobase blocks call `ctx.call_block("@wafer/database", ...)` etc.
/// This context handles those calls by routing to the appropriate CF service.
pub struct CloudflareContext {
    db_service: Arc<D1DatabaseService>,
    storage_service: Arc<R2StorageService>,
    jwt_secret: String,
    env_vars: HashMap<String, String>,
}

// Safety: wasm32-unknown-unknown is single-threaded. Worker types (D1Database, Bucket)
// are !Send because they wrap JsValue, but no cross-thread sharing occurs.
unsafe impl Send for CloudflareContext {}
unsafe impl Sync for CloudflareContext {}

impl CloudflareContext {
    pub fn new(
        db: D1DatabaseService,
        storage: R2StorageService,
        jwt_secret: String,
        env_vars: HashMap<String, String>,
    ) -> Self {
        Self {
            db_service: Arc::new(db),
            storage_service: Arc::new(storage),
            jwt_secret,
            env_vars,
        }
    }
}

// ---------------------------------------------------------------------------
// Context implementation
// ---------------------------------------------------------------------------

#[async_trait::async_trait(?Send)]
impl Context for CloudflareContext {
    async fn call_block(&self, block_name: &str, msg: &mut Message) -> Result_ {
        match block_name {
            "@wafer/database" | "@db" | "solobase/d1" => {
                db_handler::handle_message(self.db_service.as_ref(), msg).await
            }
            "@wafer/storage" | "@storage" | "solobase/r2" => {
                storage_handler::handle_message(self.storage_service.as_ref(), msg).await
            }
            "@wafer/config" => self.handle_config(msg),
            "@wafer/crypto" => self.handle_crypto(msg),
            "@wafer/network" => self.handle_network(msg).await,
            "@wafer/logger" => self.handle_logger(msg),
            _ => err_result("not_found", format!("block '{}' not found", block_name)),
        }
    }

    fn is_cancelled(&self) -> bool {
        false
    }

    fn config_get(&self, key: &str) -> Option<&str> {
        self.env_vars.get(key).map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

fn respond_json<T: Serialize>(msg: &Message, data: &T) -> Result_ {
    match serde_json::to_vec(data) {
        Ok(body) => msg.clone().respond(Response {
            data: body,
            meta: HashMap::new(),
        }),
        Err(e) => err_result("internal", e.to_string()),
    }
}

fn respond_empty(msg: &Message) -> Result_ {
    msg.clone().respond(Response {
        data: Vec::new(),
        meta: HashMap::new(),
    })
}

fn err_result(code: &str, message: impl Into<String>) -> Result_ {
    Result_::error(WaferError::new(code, message))
}

/// Decode a request from the message, returning an error Result_ on failure.
fn decode_req<T: serde::de::DeserializeOwned>(msg: &mut Message, op: &str) -> Result<T, Result_> {
    msg.decode::<T>().map_err(|e| err_result("invalid_argument", format!("invalid {op}: {e}")))
}

// ---------------------------------------------------------------------------
// Config handler — reads from env vars
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ConfigGetReq { key: String }

#[derive(Serialize)]
struct ConfigGetResp { value: String }

impl CloudflareContext {
    fn handle_config(&self, msg: &mut Message) -> Result_ {
        match msg.kind.as_str() {
            "config.get" => {
                let key = match msg.decode::<ConfigGetReq>() {
                    Ok(req) => req.key,
                    Err(_) => {
                        let meta_key = msg.get_meta("key");
                        if meta_key.is_empty() {
                            return err_result("invalid_argument", "config.get requires a 'key'");
                        }
                        meta_key.to_string()
                    }
                };
                match self.env_vars.get(&key) {
                    Some(val) => respond_json(msg, &ConfigGetResp { value: val.clone() }),
                    None => err_result("not_found", format!("config key not found: {key}")),
                }
            }
            "config.set" => {
                // Config is immutable on Workers (env vars are read-only)
                respond_empty(msg)
            }
            other => err_result("unimplemented", format!("unknown config op: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Crypto handler — argon2 + HMAC-SHA256 JWT
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CryptoHashReq { password: String }

#[derive(Serialize)]
struct CryptoHashResp { hash: String }

#[derive(Deserialize)]
struct CryptoCompareReq { password: String, hash: String }

#[derive(Serialize)]
struct CryptoCompareResp { #[serde(rename = "match")] matches: bool }

#[derive(Deserialize)]
struct CryptoSignReq {
    claims: HashMap<String, serde_json::Value>,
    #[serde(default = "default_expiry")]
    expiry_secs: u64,
}
fn default_expiry() -> u64 { 3600 }

#[derive(Serialize)]
struct CryptoSignResp { token: String }

#[derive(Deserialize)]
struct CryptoVerifyReq { token: String }

#[derive(Serialize)]
struct CryptoVerifyResp { claims: HashMap<String, serde_json::Value> }

#[derive(Deserialize)]
struct CryptoRandomReq { #[serde(default = "default_rand_n")] n: usize }
fn default_rand_n() -> usize { 32 }

#[derive(Serialize)]
struct CryptoRandomResp { bytes: Vec<u8> }

impl CloudflareContext {
    fn handle_crypto(&self, msg: &mut Message) -> Result_ {
        match msg.kind.as_str() {
            "crypto.hash" => {
                let req = decode_req::<CryptoHashReq>(msg, "crypto.hash")?;
                match solobase_core::crypto::hash_password(&req.password) {
                    Ok(hash) => respond_json(msg, &CryptoHashResp { hash }),
                    Err(e) => err_result("internal", e),
                }
            }
            "crypto.compare_hash" => {
                let req = decode_req::<CryptoCompareReq>(msg, "crypto.compare_hash")?;
                let matches = solobase_core::crypto::verify_password(&req.password, &req.hash);
                respond_json(msg, &CryptoCompareResp { matches })
            }
            "crypto.sign" => {
                let req = decode_req::<CryptoSignReq>(msg, "crypto.sign")?;
                let token = solobase_core::crypto::jwt_sign(&req.claims, Duration::from_secs(req.expiry_secs), &self.jwt_secret);
                respond_json(msg, &CryptoSignResp { token })
            }
            "crypto.verify" => {
                let req = decode_req::<CryptoVerifyReq>(msg, "crypto.verify")?;
                match solobase_core::crypto::jwt_verify(&req.token, &self.jwt_secret) {
                    Ok(claims) => respond_json(msg, &CryptoVerifyResp { claims }),
                    Err(e) => err_result("unauthenticated", e),
                }
            }
            "crypto.random_bytes" => {
                let req = decode_req::<CryptoRandomReq>(msg, "crypto.random_bytes")?;
                if req.n > 1_048_576 {
                    return err_result("invalid_argument", "random_bytes n exceeds 1 MiB limit");
                }
                match solobase_core::crypto::random_bytes(req.n) {
                    Ok(bytes) => respond_json(msg, &CryptoRandomResp { bytes }),
                    Err(e) => err_result("internal", e),
                }
            }
            other => err_result("unimplemented", format!("unknown crypto op: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Network handler — Worker fetch()
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct NetworkDoReq {
    method: String,
    url: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: Option<Vec<u8>>,
}

#[derive(Serialize)]
struct NetworkDoResp {
    status_code: u16,
    headers: HashMap<String, Vec<String>>,
    body: Vec<u8>,
}

impl CloudflareContext {
    async fn handle_network(&self, msg: &mut Message) -> Result_ {
        match msg.kind.as_str() {
            "network.do" => {
                let req = decode_req::<NetworkDoReq>(msg, "network.do")?;

                let method = match req.method.to_uppercase().as_str() {
                    "GET" => worker::Method::Get,
                    "POST" => worker::Method::Post,
                    "PUT" => worker::Method::Put,
                    "PATCH" => worker::Method::Patch,
                    "DELETE" => worker::Method::Delete,
                    "HEAD" => worker::Method::Head,
                    _ => worker::Method::Get,
                };

                let mut init = worker::RequestInit::new();
                init.with_method(method);

                if let Some(body) = req.body {
                    // Convert body bytes to a JsValue string (works for JSON API calls)
                    let body_str = String::from_utf8_lossy(&body);
                    init.with_body(Some(wasm_bindgen::JsValue::from_str(&body_str)));
                }

                let mut worker_req = match worker::Request::new_with_init(&req.url, &init) {
                    Ok(r) => r,
                    Err(e) => return err_result("internal", format!("fetch init error: {e}")),
                };

                if let Ok(headers) = worker_req.headers_mut() {
                    for (k, v) in &req.headers {
                        let _ = headers.set(k, v);
                    }
                }

                let mut resp = match worker::Fetch::Request(worker_req).send().await {
                    Ok(r) => r,
                    Err(e) => return err_result("unavailable", format!("fetch error: {e}")),
                };

                let status_code = resp.status_code();
                let resp_body = resp.bytes().await.unwrap_or_default();

                let mut resp_headers: HashMap<String, Vec<String>> = HashMap::new();
                for (k, v) in resp.headers() {
                    resp_headers.entry(k).or_default().push(v);
                }

                respond_json(msg, &NetworkDoResp { status_code, headers: resp_headers, body: resp_body })
            }
            other => err_result("unimplemented", format!("unknown network op: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Logger handler — console_log
// ---------------------------------------------------------------------------

impl CloudflareContext {
    fn handle_logger(&self, msg: &mut Message) -> Result_ {
        #[derive(Deserialize)]
        struct LogReq { message: String }

        if let Ok(req) = msg.decode::<LogReq>() {
            let level = msg.kind.strip_prefix("logger.").unwrap_or("info");
            worker::console_log!("[{}] {}", level, req.message);
        }
        respond_empty(msg)
    }
}

