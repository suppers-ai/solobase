//! R2Block — Cloudflare R2 storage as a WAFER Block.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use wafer_run::block::{Block, BlockInfo};
use wafer_run::types::*;

use crate::storage::R2StorageService;

pub struct R2Block {
    storage: Arc<R2StorageService>,
}

unsafe impl Send for R2Block {}
unsafe impl Sync for R2Block {}

impl R2Block {
    pub fn new(storage: R2StorageService) -> Self {
        Self { storage: Arc::new(storage) }
    }
}

// --- Request/response types ---

#[derive(Deserialize)]
struct StoragePutReq { folder: String, key: String, data: Vec<u8>, #[serde(default = "default_ct")] content_type: String }
fn default_ct() -> String { "application/octet-stream".to_string() }

#[derive(Deserialize)]
struct StorageGetReq { folder: String, key: String }

#[derive(Deserialize)]
struct StorageDeleteReq { folder: String, key: String }

#[derive(Deserialize)]
struct StorageListReq { folder: String, #[serde(default)] prefix: String, #[serde(default)] limit: i64, #[serde(default)] offset: i64 }

#[derive(Serialize)]
struct StorageGetResp { data: Vec<u8>, info: crate::storage::ObjectInfo }

// --- Helpers ---

fn respond_json<T: Serialize>(msg: &Message, data: &T) -> Result_ {
    match serde_json::to_vec(data) {
        Ok(body) => msg.clone().respond(Response { data: body, meta: HashMap::new() }),
        Err(e) => err_result("internal", e.to_string()),
    }
}

fn respond_empty(msg: &Message) -> Result_ {
    msg.clone().respond(Response { data: Vec::new(), meta: HashMap::new() })
}

fn err_result(code: &str, message: impl Into<String>) -> Result_ {
    Result_::error(WaferError::new(code, message))
}

fn decode_req<T: serde::de::DeserializeOwned>(msg: &mut Message, op: &str) -> Result<T, Result_> {
    msg.decode::<T>().map_err(|e| err_result("invalid_argument", format!("invalid {op}: {e}")))
}

// --- Block implementation ---

#[async_trait::async_trait(?Send)]
impl Block for R2Block {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "solobase/r2".to_string(),
            version: "0.1.0".to_string(),
            interface: "storage@v1".to_string(),
            summary: "Cloudflare R2 storage block".to_string(),
            instance_mode: InstanceMode::PerNode,
            allowed_modes: Vec::new(),
            admin_ui: None,
            runtime: BlockRuntime::Native,
            requires: Vec::new(),
        }
    }

    async fn handle(&self, _ctx: &dyn wafer_run::context::Context, msg: &mut Message) -> Result_ {
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
                respond_empty(msg)
            }
            other => err_result("unimplemented", format!("unknown storage op: {other}")),
        }
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn wafer_run::context::Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
