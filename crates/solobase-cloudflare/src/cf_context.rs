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
use std::time::Duration;

use serde::{Deserialize, Serialize};

use wafer_run::context::Context;
use wafer_run::types::*;

use crate::database::{self, D1DatabaseService, Filter, FilterOp, ListOptions, SortField};
use crate::storage::R2StorageService;

/// WAFER Context backed by Cloudflare Workers services (D1, R2, KV).
///
/// Solobase blocks call `ctx.call_block("@wafer/database", ...)` etc.
/// This context handles those calls by routing to the appropriate CF service.
pub struct CloudflareContext {
    db: D1DatabaseService,
    storage: R2StorageService,
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
        Self { db, storage, jwt_secret, env_vars }
    }
}

// ---------------------------------------------------------------------------
// Context implementation
// ---------------------------------------------------------------------------

#[async_trait::async_trait(?Send)]
impl Context for CloudflareContext {
    async fn call_block(&self, block_name: &str, msg: &mut Message) -> Result_ {
        match block_name {
            "@wafer/database" => self.handle_database(msg).await,
            "@wafer/storage" => self.handle_storage(msg).await,
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

// ---------------------------------------------------------------------------
// Database handler — routes to D1 via D1DatabaseService
// ---------------------------------------------------------------------------

// Wire-format request types (match wafer-core/src/blocks/database/block.rs)

#[derive(Deserialize)]
struct DbGetReq { collection: String, id: String }

#[derive(Deserialize)]
struct DbListReq {
    collection: String,
    #[serde(default)]
    filters: Vec<DbFilterDef>,
    #[serde(default)]
    sort: Vec<DbSortDef>,
    #[serde(default)]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

#[derive(Deserialize)]
struct DbCreateReq {
    collection: String,
    data: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct DbUpdateReq {
    collection: String,
    id: String,
    data: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct DbDeleteReq { collection: String, id: String }

#[derive(Deserialize)]
struct DbCountReq {
    collection: String,
    #[serde(default)]
    filters: Vec<DbFilterDef>,
}

#[derive(Deserialize)]
struct DbSumReq {
    collection: String,
    field: String,
    #[serde(default)]
    filters: Vec<DbFilterDef>,
}

#[derive(Deserialize)]
struct DbQueryRawReq {
    query: String,
    #[serde(default)]
    args: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct DbExecRawReq {
    query: String,
    #[serde(default)]
    args: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct DbFilterDef {
    field: String,
    #[serde(default = "default_op")]
    operator: String,
    #[serde(default)]
    value: serde_json::Value,
}

fn default_op() -> String { "eq".to_string() }

#[derive(Deserialize)]
struct DbSortDef {
    field: String,
    #[serde(default)]
    desc: bool,
}

#[derive(Serialize)]
struct CountResp { count: i64 }

#[derive(Serialize)]
struct SumResp { sum: f64 }

#[derive(Serialize)]
struct ExecRawResp { rows_affected: i64 }

fn parse_filter_op(op: &str) -> FilterOp {
    match op {
        "eq" | "=" | "equal" => FilterOp::Equal,
        "neq" | "!=" | "not_equal" => FilterOp::NotEqual,
        "gt" | ">" | "greater_than" => FilterOp::GreaterThan,
        "gte" | ">=" | "greater_equal" => FilterOp::GreaterEqual,
        "lt" | "<" | "less_than" => FilterOp::LessThan,
        "lte" | "<=" | "less_equal" => FilterOp::LessEqual,
        "like" => FilterOp::Like,
        "in" => FilterOp::In,
        "is_null" => FilterOp::IsNull,
        "is_not_null" => FilterOp::IsNotNull,
        _ => FilterOp::Equal,
    }
}

fn convert_filters(defs: Vec<DbFilterDef>) -> Vec<Filter> {
    defs.into_iter()
        .map(|f| Filter {
            field: f.field,
            operator: parse_filter_op(&f.operator),
            value: f.value,
        })
        .collect()
}

fn convert_sort(defs: Vec<DbSortDef>) -> Vec<SortField> {
    defs.into_iter()
        .map(|s| SortField { field: s.field, desc: s.desc })
        .collect()
}

/// Decode a request from the message, returning an error Result_ on failure.
fn decode_req<T: serde::de::DeserializeOwned>(msg: &mut Message, op: &str) -> Result<T, Result_> {
    msg.decode::<T>().map_err(|e| err_result("invalid_argument", format!("invalid {op}: {e}")))
}

impl CloudflareContext {
    async fn handle_database(&self, msg: &mut Message) -> Result_ {
        match msg.kind.as_str() {
            "database.get" => {
                let req = decode_req::<DbGetReq>(msg, "database.get")?;
                match self.db.get(&req.collection, &req.id).await {
                    Ok(record) => respond_json(msg, &record),
                    Err(_) => err_result("not_found", "record not found"),
                }
            }
            "database.list" => {
                let req = decode_req::<DbListReq>(msg, "database.list")?;
                let opts = ListOptions {
                    filters: convert_filters(req.filters),
                    sort: convert_sort(req.sort),
                    limit: req.limit,
                    offset: req.offset,
                };
                match self.db.list(&req.collection, &opts).await {
                    Ok(list) => respond_json(msg, &list),
                    Err(e) => err_result("internal", format!("database list error: {e}")),
                }
            }
            "database.create" => {
                let req = decode_req::<DbCreateReq>(msg, "database.create")?;
                match self.db.create(&req.collection, req.data).await {
                    Ok(record) => respond_json(msg, &record),
                    Err(e) => err_result("internal", format!("database create error: {e}")),
                }
            }
            "database.update" => {
                let req = decode_req::<DbUpdateReq>(msg, "database.update")?;
                match self.db.update(&req.collection, &req.id, req.data).await {
                    Ok(record) => respond_json(msg, &record),
                    Err(e) => err_result("internal", format!("database update error: {e}")),
                }
            }
            "database.delete" => {
                let req = decode_req::<DbDeleteReq>(msg, "database.delete")?;
                match self.db.delete(&req.collection, &req.id).await {
                    Ok(()) => respond_empty(msg),
                    Err(e) => err_result("internal", format!("database delete error: {e}")),
                }
            }
            "database.count" => {
                let req = decode_req::<DbCountReq>(msg, "database.count")?;
                let filters = convert_filters(req.filters);
                match self.db.count(&req.collection, &filters).await {
                    Ok(count) => respond_json(msg, &CountResp { count }),
                    Err(e) => err_result("internal", format!("database count error: {e}")),
                }
            }
            "database.sum" => {
                let req = decode_req::<DbSumReq>(msg, "database.sum")?;
                let col = database::sanitize_ident(&req.field);
                let table = database::sanitize_ident(&req.collection);
                let sql = format!("SELECT COALESCE(SUM({}), 0) as s FROM {}", col, table);
                match self.db.query_raw(&sql, &[]).await {
                    Ok(records) => {
                        let sum = records.first()
                            .and_then(|r| r.data.get("s"))
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        respond_json(msg, &SumResp { sum })
                    }
                    Err(e) => err_result("internal", format!("database sum error: {e}")),
                }
            }
            "database.query_raw" => {
                let req = decode_req::<DbQueryRawReq>(msg, "database.query_raw")?;
                match self.db.query_raw(&req.query, &req.args).await {
                    Ok(records) => respond_json(msg, &records),
                    Err(e) => err_result("internal", format!("database query_raw error: {e}")),
                }
            }
            "database.exec_raw" => {
                let req = decode_req::<DbExecRawReq>(msg, "database.exec_raw")?;
                match self.db.exec_raw(&req.query, &req.args).await {
                    Ok(()) => respond_json(msg, &ExecRawResp { rows_affected: 0 }),
                    Err(e) => err_result("internal", format!("database exec_raw error: {e}")),
                }
            }
            other => err_result("unimplemented", format!("unknown database op: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Storage handler — routes to R2 via R2StorageService
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct StoragePutReq { folder: String, key: String, data: Vec<u8>, #[serde(default = "default_ct")] content_type: String }
fn default_ct() -> String { "application/octet-stream".to_string() }

#[derive(Deserialize)]
struct StorageGetReq { folder: String, key: String }

#[derive(Deserialize)]
struct StorageDeleteReq { folder: String, key: String }

#[derive(Deserialize)]
struct StorageListReq { folder: String, #[serde(default)] prefix: String, #[serde(default)] limit: i64, #[serde(default)] offset: i64 }

#[derive(Deserialize)]
struct StorageCreateFolderReq { name: String, #[serde(default)] public: bool }

#[derive(Deserialize)]
struct StorageDeleteFolderReq { name: String }

#[derive(Serialize)]
struct StorageGetResp { data: Vec<u8>, info: crate::storage::ObjectInfo }

impl CloudflareContext {
    async fn handle_storage(&self, msg: &mut Message) -> Result_ {
        match msg.kind.as_str() {
            "storage.put" => {
                let req = decode_req::<StoragePutReq>(msg, "storage.put")?;
                match self.storage.put(&req.folder, &req.key, req.data, &req.content_type).await {
                    Ok(()) => respond_empty(msg),
                    Err(e) => err_result("internal", format!("storage put error: {e}")),
                }
            }
            "storage.get" => {
                let req = decode_req::<StorageGetReq>(msg, "storage.get")?;
                match self.storage.get(&req.folder, &req.key).await {
                    Ok((data, info)) => respond_json(msg, &StorageGetResp { data, info }),
                    Err(_) => err_result("not_found", "object not found"),
                }
            }
            "storage.delete" => {
                let req = decode_req::<StorageDeleteReq>(msg, "storage.delete")?;
                match self.storage.delete(&req.folder, &req.key).await {
                    Ok(()) => respond_empty(msg),
                    Err(e) => err_result("internal", format!("storage delete error: {e}")),
                }
            }
            "storage.list" => {
                let req = decode_req::<StorageListReq>(msg, "storage.list")?;
                let limit = if req.limit > 0 { req.limit as u32 } else { 100 };
                match self.storage.list(&req.folder, &req.prefix, limit).await {
                    Ok(list) => respond_json(msg, &list),
                    Err(e) => err_result("internal", format!("storage list error: {e}")),
                }
            }
            "storage.create_folder" | "storage.delete_folder" | "storage.list_folders" => {
                // R2 doesn't have real folders — no-ops
                respond_empty(msg)
            }
            other => err_result("unimplemented", format!("unknown storage op: {other}")),
        }
    }
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
                match crypto_hash_password(&req.password) {
                    Ok(hash) => respond_json(msg, &CryptoHashResp { hash }),
                    Err(e) => err_result("internal", e),
                }
            }
            "crypto.compare_hash" => {
                let req = decode_req::<CryptoCompareReq>(msg, "crypto.compare_hash")?;
                let matches = crypto_verify_password(&req.password, &req.hash);
                respond_json(msg, &CryptoCompareResp { matches })
            }
            "crypto.sign" => {
                let req = decode_req::<CryptoSignReq>(msg, "crypto.sign")?;
                let token = jwt_sign(&req.claims, Duration::from_secs(req.expiry_secs), &self.jwt_secret);
                respond_json(msg, &CryptoSignResp { token })
            }
            "crypto.verify" => {
                let req = decode_req::<CryptoVerifyReq>(msg, "crypto.verify")?;
                match jwt_verify(&req.token, &self.jwt_secret) {
                    Ok(claims) => respond_json(msg, &CryptoVerifyResp { claims }),
                    Err(e) => err_result("unauthenticated", e),
                }
            }
            "crypto.random_bytes" => {
                let req = decode_req::<CryptoRandomReq>(msg, "crypto.random_bytes")?;
                if req.n > 1_048_576 {
                    return err_result("invalid_argument", "random_bytes n exceeds 1 MiB limit");
                }
                let mut buf = vec![0u8; req.n];
                getrandom::getrandom(&mut buf).unwrap_or_default();
                respond_json(msg, &CryptoRandomResp { bytes: buf })
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

// ---------------------------------------------------------------------------
// Crypto implementation: argon2 + HMAC-SHA256 JWT
// ---------------------------------------------------------------------------

fn crypto_hash_password(password: &str) -> Result<String, String> {
    use argon2::{
        password_hash::SaltString,
        Argon2, PasswordHasher, Params,
    };
    // Use lower-cost params for Workers (4 MiB memory, 2 iterations)
    let params = Params::new(4096, 2, 1, None)
        .map_err(|e| format!("argon2 params: {e}"))?;
    // Generate salt using getrandom (JS crypto on wasm32)
    let mut salt_bytes = [0u8; 16];
    getrandom::getrandom(&mut salt_bytes).map_err(|e| format!("rng error: {e}"))?;
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|e| format!("salt encode: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("argon2 hash: {e}"))
}

fn crypto_verify_password(password: &str, hash: &str) -> bool {
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

// --- HMAC-SHA256 JWT ---

fn hmac_sha256(data: &[u8], key: &[u8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn base64_url_encode(input: &[u8]) -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(input)
}

fn base64_url_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.decode(input).map_err(|e| format!("invalid base64: {e}"))
}

fn jwt_sign(
    claims: &HashMap<String, serde_json::Value>,
    expiry: Duration,
    secret: &str,
) -> String {
    let now = chrono::Utc::now();
    let exp = now + chrono::Duration::seconds(expiry.as_secs() as i64);

    let mut payload = claims.clone();
    payload.insert("iat".to_string(), serde_json::json!(now.timestamp()));
    payload.insert("exp".to_string(), serde_json::json!(exp.timestamp()));

    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let header_b64 = base64_url_encode(header.as_bytes());
    let payload_json = serde_json::to_string(&payload).unwrap_or_default();
    let payload_b64 = base64_url_encode(payload_json.as_bytes());

    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let sig = hmac_sha256(signing_input.as_bytes(), secret.as_bytes());
    let sig_b64 = base64_url_encode(&sig);

    format!("{}.{}.{}", header_b64, payload_b64, sig_b64)
}

fn jwt_verify(
    token: &str,
    secret: &str,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("invalid JWT format".into());
    }

    // Verify signature
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected_sig = hmac_sha256(signing_input.as_bytes(), secret.as_bytes());
    let actual_sig = base64_url_decode(parts[2])?;
    if expected_sig != actual_sig {
        return Err("invalid JWT signature".into());
    }

    // Decode payload
    let payload = base64_url_decode(parts[1])?;
    let claims: HashMap<String, serde_json::Value> = serde_json::from_slice(&payload)
        .map_err(|e| format!("invalid JWT claims: {e}"))?;

    // Check expiration
    if let Some(exp) = claims.get("exp").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp();
        if exp < now {
            return Err("JWT expired".into());
        }
    }

    Ok(claims)
}

/// Public JWT verify function for use by lib.rs auth middleware.
pub fn verify_jwt_public(
    token: &str,
    secret: &str,
) -> Result<HashMap<String, serde_json::Value>, String> {
    jwt_verify(token, secret)
}
