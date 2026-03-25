//! Thin Block wrappers for Cloudflare services (D1, R2, config, crypto, network, logger).
//!
//! These allow the Wafer runtime's `RuntimeContext` to route `call_block("wafer-run/database", ...)`
//! to the appropriate Cloudflare service, just like native blocks route to SQLite/local-storage.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use wafer_block::helpers::MessageExt;
use wafer_core::interfaces::database::handler as db_handler;
use wafer_core::interfaces::storage::handler as storage_handler;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;

use crate::database::D1DatabaseService;
use crate::storage::R2StorageService;

// Safety: wasm32-unknown-unknown is single-threaded. Worker types (D1Database, Bucket)
// are !Send because they wrap JsValue, but no cross-thread sharing occurs.
macro_rules! unsafe_send_sync {
    ($ty:ty) => {
        unsafe impl Send for $ty {}
        unsafe impl Sync for $ty {}
    };
}

fn block_info(name: &str, summary: &str) -> BlockInfo {
    BlockInfo {
        name: name.to_string(),
        version: "0.1.0".to_string(),
        interface: "service@v1".to_string(),
        summary: summary.to_string(),
        instance_mode: InstanceMode::Singleton,
        allowed_modes: Vec::new(),
        admin_ui: None,
        runtime: BlockRuntime::default(),
        requires: Vec::new(),
        collections: Vec::new(),
    }
}

fn err_result(code: &str, message: impl Into<String>) -> Result_ {
    Result_::error(WaferError::new(code, message))
}

fn respond_json<T: Serialize>(msg: &Message, data: &T) -> Result_ {
    match serde_json::to_vec(data) {
        Ok(body) => msg.clone().respond(Response {
            data: body,
            meta: Vec::new(),
        }),
        Err(e) => err_result("internal", e.to_string()),
    }
}

fn respond_empty(msg: &Message) -> Result_ {
    msg.clone().respond(Response {
        data: Vec::new(),
        meta: Vec::new(),
    })
}

/// Decode a request from the message, returning an error Result_ on failure.
macro_rules! decode_req {
    ($ty:ty, $msg:expr, $op:expr) => {
        match $msg.decode::<$ty>() {
            Ok(v) => v,
            Err(e) => return err_result("invalid_argument", format!("invalid {}: {e}", $op)),
        }
    };
}

// ---------------------------------------------------------------------------
// D1 Database Block
// ---------------------------------------------------------------------------

pub struct D1Block {
    service: Arc<D1DatabaseService>,
}
unsafe_send_sync!(D1Block);

impl D1Block {
    pub fn new(service: D1DatabaseService) -> Self {
        Self { service: Arc::new(service) }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for D1Block {
    fn info(&self) -> BlockInfo {
        block_info("wafer-run/d1", "Cloudflare D1 database service")
    }
    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        db_handler::handle_message(self.service.as_ref(), msg).await
    }
    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// R2 Storage Block
// ---------------------------------------------------------------------------

pub struct R2Block {
    service: Arc<R2StorageService>,
}
unsafe_send_sync!(R2Block);

impl R2Block {
    pub fn new(service: R2StorageService) -> Self {
        Self { service: Arc::new(service) }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for R2Block {
    fn info(&self) -> BlockInfo {
        block_info("wafer-run/r2", "Cloudflare R2 storage service")
    }
    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        storage_handler::handle_message(self.storage_service(), msg).await
    }
    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

impl R2Block {
    fn storage_service(&self) -> &dyn wafer_core::interfaces::storage::service::StorageService {
        self.service.as_ref()
    }
}

// ---------------------------------------------------------------------------
// Config Block
// ---------------------------------------------------------------------------

pub struct ConfigBlock {
    env_vars: HashMap<String, String>,
}
unsafe_send_sync!(ConfigBlock);

impl ConfigBlock {
    pub fn new(env_vars: HashMap<String, String>) -> Self {
        Self { env_vars }
    }
}

#[derive(Deserialize)]
struct ConfigGetReq { key: String }

#[derive(Serialize)]
struct ConfigGetResp { value: String }

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for ConfigBlock {
    fn info(&self) -> BlockInfo {
        block_info("wafer-run/config", "Configuration from environment variables")
    }
    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
            "config.set" => respond_empty(msg),
            other => err_result("unimplemented", format!("unknown config op: {other}")),
        }
    }
    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Crypto Block
// ---------------------------------------------------------------------------

pub struct CryptoBlock {
    jwt_secret: String,
}
unsafe_send_sync!(CryptoBlock);

impl CryptoBlock {
    pub fn new(jwt_secret: String) -> Self {
        Self { jwt_secret }
    }
}

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

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for CryptoBlock {
    fn info(&self) -> BlockInfo {
        block_info("wafer-run/crypto", "Cryptographic operations (argon2, JWT, random)")
    }
    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        match msg.kind.as_str() {
            "crypto.hash" => {
                let req = decode_req!(CryptoHashReq, msg, "crypto.hash");
                match solobase_core::crypto::hash_password(&req.password) {
                    Ok(hash) => respond_json(msg, &CryptoHashResp { hash }),
                    Err(e) => err_result("internal", e),
                }
            }
            "crypto.compare_hash" => {
                let req = decode_req!(CryptoCompareReq, msg, "crypto.compare_hash");
                let matches = solobase_core::crypto::verify_password(&req.password, &req.hash);
                respond_json(msg, &CryptoCompareResp { matches })
            }
            "crypto.sign" => {
                let req = decode_req!(CryptoSignReq, msg, "crypto.sign");
                let token = solobase_core::crypto::jwt_sign(&req.claims, Duration::from_secs(req.expiry_secs), &self.jwt_secret);
                respond_json(msg, &CryptoSignResp { token })
            }
            "crypto.verify" => {
                let req = decode_req!(CryptoVerifyReq, msg, "crypto.verify");
                match solobase_core::crypto::jwt_verify(&req.token, &self.jwt_secret) {
                    Ok(claims) => respond_json(msg, &CryptoVerifyResp { claims }),
                    Err(e) => err_result("unauthenticated", e),
                }
            }
            "crypto.random_bytes" => {
                let req = decode_req!(CryptoRandomReq, msg, "crypto.random_bytes");
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
    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Network Block
// ---------------------------------------------------------------------------

pub struct NetworkBlock;
unsafe_send_sync!(NetworkBlock);

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

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for NetworkBlock {
    fn info(&self) -> BlockInfo {
        block_info("wafer-run/network", "HTTP fetch via Worker runtime")
    }
    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        match msg.kind.as_str() {
            "network.do" => {
                let req = decode_req!(NetworkDoReq, msg, "network.do");
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
    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Logger Block
// ---------------------------------------------------------------------------

pub struct LoggerBlock;
unsafe_send_sync!(LoggerBlock);

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for LoggerBlock {
    fn info(&self) -> BlockInfo {
        block_info("wafer-run/logger", "Console logging for Workers")
    }
    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        #[derive(Deserialize)]
        struct LogReq { message: String }
        if let Ok(req) = msg.decode::<LogReq>() {
            let level = msg.kind.strip_prefix("logger.").unwrap_or("info");
            worker::console_log!("[{}] {}", level, req.message);
        }
        respond_empty(msg)
    }
    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
