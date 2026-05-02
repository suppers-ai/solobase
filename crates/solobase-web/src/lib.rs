//! Solobase app compiled to WASM for running in the browser via Service Worker.
//!
//! Thin wasm-bindgen wrapper around the `solobase-browser` framework. Uses
//! `SolobaseBuilder` (from `solobase-core`) to wire up the full Solobase
//! block suite + the app-specific `BrowserLlmService`.

use std::sync::Arc;

use solobase_core::builder::{self, SolobaseBuilder};
use wafer_core::interfaces::config::service::ConfigService;
use wasm_bindgen::prelude::*;

pub mod config;

const SOLOBASE_CSP: &str = concat!(
    "default-src 'self'; ",
    "script-src 'self' 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval' https://cdn.jsdelivr.net; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data: blob: https:; ",
    "font-src 'self' https:; ",
    "connect-src 'self' https://cdn.jsdelivr.net https://esm.run https://huggingface.co ",
        "https://raw.githubusercontent.com https://*.huggingface.co https://*.hf.co https://*.xethub.hf.co; ",
    "frame-ancestors 'none'; ",
    "base-uri 'self'; ",
    "form-action 'self'",
);

#[wasm_bindgen]
pub async fn initialize() -> Result<(), JsValue> {
    if solobase_browser::is_initialized() {
        return Ok(());
    }

    solobase_browser::db_init().await;

    let vars = config::seed_and_load_variables();
    web_sys::console::log_1(
        &format!("solobase: {} variables loaded from database", vars.len()).into(),
    );

    let features = config::load_block_settings();

    let jwt_secret = vars
        .get("SUPPERS_AI__AUTH__JWT_SECRET")
        .cloned()
        .unwrap_or_default();

    let config_svc = wafer_block_config::service::EnvConfigService::new();
    for (key, value) in &vars {
        config_svc.set(key, value);
    }

    let browser_llm: Arc<dyn wafer_core::interfaces::llm::service::LlmService> =
        Arc::new(solobase_browser::llm::BrowserLlmService::new());

    let browser_vector: Arc<dyn wafer_core::interfaces::vector::service::VectorService> =
        Arc::new(solobase_browser::vector::BrowserVectorService::new());

    let browser_embedding: Arc<dyn wafer_core::interfaces::vector::service::EmbeddingService> =
        match solobase_browser::vector::BrowserEmbeddingService::new() {
            Ok(svc) => Arc::new(svc),
            Err(e) => {
                web_sys::console::error_1(&format!("BrowserEmbeddingService init: {e}").into());
                return Err(JsValue::from_str(&e));
            }
        };

    let (mut wafer, storage_block) = SolobaseBuilder::new()
        .database(solobase_browser::make_database_service())
        .storage(solobase_browser::make_storage_service())
        .config(Arc::new(config_svc))
        .crypto(solobase_browser::make_crypto_service(jwt_secret))
        .network(solobase_browser::make_network_service())
        .logger(solobase_browser::make_console_logger())
        .llm_service("browser", browser_llm)
        .vector_service(browser_vector)
        .embedding_service(browser_embedding)
        .block_settings(features)
        .block_config(
            "wafer-run/security-headers",
            serde_json::json!({ "csp": SOLOBASE_CSP }),
        )
        .build()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    wafer.set_asset_loader(solobase_browser::make_sw_asset_loader());

    wafer
        .start_without_bind()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    builder::post_start(&wafer, &storage_block);

    web_sys::console::log_1(&"solobase: WAFER runtime started".into());

    solobase_browser::store_wafer(wafer);

    Ok(())
}

#[wasm_bindgen]
pub async fn handle_request(request: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    solobase_browser::dispatch_request(request).await
}
