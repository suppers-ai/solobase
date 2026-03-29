use std::collections::HashMap;
use wafer_core::interfaces::network::service::{NetworkError, NetworkService, Request, Response};

/// NetworkService using CF Worker's fetch API.
pub struct WorkerFetchService;

// Safety: wasm32-unknown-unknown is single-threaded.
unsafe impl Send for WorkerFetchService {}
unsafe impl Sync for WorkerFetchService {}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl NetworkService for WorkerFetchService {
    async fn do_request(&self, req: &Request) -> Result<Response, NetworkError> {
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
        if let Some(ref body) = req.body {
            let body_str = String::from_utf8_lossy(body);
            init.with_body(Some(wasm_bindgen::JsValue::from_str(&body_str)));
        }

        let mut worker_req = worker::Request::new_with_init(&req.url, &init)
            .map_err(|e| NetworkError::RequestError(format!("fetch init error: {e}")))?;

        if let Ok(headers) = worker_req.headers_mut() {
            for (k, v) in &req.headers {
                let _ = headers.set(k, v);
            }
        }

        let mut resp = worker::Fetch::Request(worker_req)
            .send()
            .await
            .map_err(|e| NetworkError::RequestError(format!("fetch error: {e}")))?;

        let status_code = resp.status_code();
        let resp_body = resp.bytes().await.unwrap_or_default();
        let mut resp_headers: HashMap<String, Vec<String>> = HashMap::new();
        for (k, v) in resp.headers() {
            resp_headers.entry(k).or_default().push(v);
        }

        Ok(Response {
            status_code,
            headers: resp_headers,
            body: resp_body,
        })
    }
}
