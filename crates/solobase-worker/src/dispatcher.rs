//! Dispatcher block — forwards requests to the dispatch worker via service binding.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_block::helpers::MessageExt;

pub struct DispatcherBlock {
    fetcher: worker::Fetcher,
}

unsafe impl Send for DispatcherBlock {}
unsafe impl Sync for DispatcherBlock {}

impl DispatcherBlock {
    pub fn new(fetcher: worker::Fetcher) -> Self {
        Self { fetcher }
    }
}

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

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for DispatcherBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "solobase/dispatcher".to_string(),
            version: "0.0.1".to_string(),
            interface: "service@v1".to_string(),
            summary: "Forward requests to dispatch worker via service binding".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: Vec::new(),
            admin_ui: None,
            runtime: BlockRuntime::default(),
            requires: Vec::new(),
            collections: Vec::new(),
            config_schema: None,
        }
    }

    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        match msg.kind.as_str() {
            "network.do" => {
                let req = match msg.decode::<NetworkDoReq>() {
                    Ok(v) => v,
                    Err(e) => return err_result("invalid_argument", format!("invalid network.do: {e}")),
                };
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
                let mut resp = match self.fetcher.fetch_request(worker_req).await {
                    Ok(r) => r,
                    Err(e) => return err_result("unavailable", format!("dispatcher fetch error: {e}")),
                };
                let status_code = resp.status_code();
                let resp_body = resp.bytes().await.unwrap_or_default();
                let mut resp_headers: HashMap<String, Vec<String>> = HashMap::new();
                for (k, v) in resp.headers() {
                    resp_headers.entry(k).or_default().push(v);
                }
                respond_json(msg, &NetworkDoResp { status_code, headers: resp_headers, body: resp_body })
            }
            other => err_result("unimplemented", format!("unknown dispatcher op: {other}")),
        }
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
