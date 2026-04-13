//! Solobase compiled to WASM for running in the browser via Service Worker.
//!
//! Exposes two wasm-bindgen entry points:
//! - `initialize()` — called once when the Service Worker starts; loads the
//!   database, seeds variables, registers all WAFER blocks, and starts the runtime.
//! - `handle_request(request)` — called on each SW fetch event; converts the
//!   browser `Request` into a WAFER `Message`, dispatches it through the
//!   `site-main` flow, and returns a browser `Response`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use wasm_bindgen::prelude::*;

pub mod bridge;
pub mod config;
pub mod convert;
pub mod crypto;
pub mod database;
pub mod logger;
pub mod network;
pub mod storage;

use solobase_core::blocks::router::{NativeBlockFactory, SolobaseRouterBlock};
use solobase_core::features::FeatureConfig;

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

// WASM is single-threaded — a thread_local RefCell is safe and avoids
// needing Send + Sync on Wafer (which is not Send on wasm32).
thread_local! {
    static RUNTIME: RefCell<Option<wafer_run::Wafer>> = const { RefCell::new(None) };
}

// ---------------------------------------------------------------------------
// initialize()
// ---------------------------------------------------------------------------

/// Initialize the Solobase WAFER runtime.
///
/// Must be called exactly once when the Service Worker starts, before any
/// `handle_request()` calls.  This is async because it awaits `bridge::dbInit()`
/// and `wafer.start_without_bind()`.
#[wasm_bindgen]
pub async fn initialize() -> Result<(), JsValue> {
    // 1. Load sql.js WASM + open/create the OPFS database.
    bridge::dbInit().await;

    // 2. Seed the variables table (create if not exists, generate secrets).
    let vars = config::seed_and_load_variables();
    web_sys::console::log_1(
        &format!("solobase: {} variables loaded from database", vars.len()).into(),
    );

    // 3. Load feature flag settings from the block_settings table.
    let features = config::load_block_settings();

    // 4. Extract the JWT secret.
    let jwt_secret = vars
        .get("SUPPERS_AI__AUTH__JWT_SECRET")
        .cloned()
        .unwrap_or_default();

    // 5. Create the WAFER runtime.
    let mut wafer = wafer_run::Wafer::new();
    wafer.set_admin_block("suppers-ai/admin");

    // 6. Register service blocks with browser implementations.
    register_service_blocks(&mut wafer, &vars, &jwt_secret)?;

    // 7. Register middleware / infrastructure blocks.
    register_middleware_blocks(&mut wafer)?;

    // 8. Register solobase feature blocks.
    let shared_blocks = solobase_core::blocks::create_blocks(|name| features.is_enabled(name));
    solobase_core::blocks::register_shared_blocks(&mut wafer, &shared_blocks);

    // 8b. Email block (always registered, not feature-gated).
    wafer
        .register_block(
            "suppers-ai/email",
            Arc::new(solobase_core::blocks::email::EmailBlock),
        )
        .map_err(|e| JsValue::from_str(&e))?;

    // 9. Build and register the solobase router block.
    let feature_config: Arc<dyn FeatureConfig> = Arc::new(features);
    let factory = NativeBlockFactory::new(shared_blocks);
    let router = SolobaseRouterBlock::new(jwt_secret, feature_config, factory);
    wafer
        .register_block("suppers-ai/router", Arc::new(router))
        .map_err(|e| JsValue::from_str(&e))?;
    wafer.add_block_config("suppers-ai/router", solobase_core::routing::routes_config());

    // 10. Register the site-main flow and routes.
    register_site_main_flow(&mut wafer)?;

    // 11. Start the runtime (without bind — bind() is native-only).
    wafer
        .start_without_bind()
        .await
        .map_err(|e| JsValue::from_str(&e))?;

    web_sys::console::log_1(&"solobase: WAFER runtime started".into());

    // 12. Store in global.
    RUNTIME.with(|r| {
        *r.borrow_mut() = Some(wafer);
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// handle_request()
// ---------------------------------------------------------------------------

/// Handle an incoming fetch request from the Service Worker.
///
/// Converts the browser `Request` into a WAFER `Message`, dispatches it
/// through the `site-main` flow, and returns a browser `Response`.
#[wasm_bindgen]
pub async fn handle_request(request: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    let mut msg = convert::request_to_message(&request).await?;

    // Dispatch through the site-main flow.
    //
    // We use a raw pointer to avoid holding a RefCell borrow across await.
    // The take()-and-put-back pattern breaks when concurrent fetch events
    // interleave at await points (the second event finds None).
    //
    // SAFETY: wasm32 is single-threaded, and the RefCell value is never
    // replaced after initialize() stores it.
    let wafer_ptr = RUNTIME.with(|r| {
        let borrow = r.borrow();
        match borrow.as_ref() {
            Some(w) => Ok(w as *const wafer_run::Wafer),
            None => Err(JsValue::from_str(
                "solobase: runtime not initialized — call initialize() first",
            )),
        }
    })?;
    let wafer = unsafe { &*wafer_ptr };
    let result = wafer.run("site-main", &mut msg).await;

    convert::result_to_response(result)
}

// ---------------------------------------------------------------------------
// Service block registration
// ---------------------------------------------------------------------------

fn register_service_blocks(
    wafer: &mut wafer_run::Wafer,
    vars: &HashMap<String, String>,
    jwt_secret: &str,
) -> Result<(), JsValue> {
    use wafer_core::interfaces::config::service::ConfigService;

    // Database — BrowserDatabaseService backed by sql.js / OPFS
    let db_service = Arc::new(database::BrowserDatabaseService);
    wafer_core::service_blocks::database::register_with(wafer, db_service)
        .map_err(|e| JsValue::from_str(&e))?;
    wafer.add_alias("db", "wafer-run/database");

    // Storage — BrowserStorageService backed by OPFS
    let storage_service = Arc::new(storage::BrowserStorageService);
    wafer_core::service_blocks::storage::register_with(wafer, storage_service)
        .map_err(|e| JsValue::from_str(&e))?;
    wafer.add_alias("storage", "wafer-run/storage");

    // Config — pre-populated from variables table (no env vars in browser)
    let config_service = wafer_block_config::service::EnvConfigService::new();
    for (key, value) in vars {
        config_service.set(key, value);
    }
    wafer_core::service_blocks::config::register_with(wafer, Arc::new(config_service))
        .map_err(|e| JsValue::from_str(&e))?;

    // Crypto — PBKDF2 password hashing + HS256 JWT (browser-optimized, fast in WASM)
    let crypto_service = Arc::new(crypto::BrowserCryptoService::new(jwt_secret.to_string()));
    wafer_core::service_blocks::crypto::register_with(wafer, crypto_service)
        .map_err(|e| JsValue::from_str(&e))?;

    // Network — BrowserNetworkService backed by browser fetch()
    let network_service = Arc::new(network::BrowserNetworkService);
    wafer_core::service_blocks::network::register_with(wafer, network_service)
        .map_err(|e| JsValue::from_str(&e))?;

    // Logger — ConsoleLogger (console.log / console.warn / console.error)
    let logger_service = Arc::new(logger::ConsoleLogger);
    wafer_core::service_blocks::logger::register_with(wafer, logger_service)
        .map_err(|e| JsValue::from_str(&e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Middleware / infrastructure block registration
// ---------------------------------------------------------------------------

fn register_middleware_blocks(wafer: &mut wafer_run::Wafer) -> Result<(), JsValue> {
    wafer_block_auth_validator::register(wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer_block_cors::register(wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer_block_iam_guard::register(wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer_block_inspector::register(wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer.add_block_config(
        "wafer-run/inspector",
        serde_json::json!({ "allow_anonymous": false }),
    );
    wafer_block_readonly_guard::register(wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer_block_router::register(wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer_block_security_headers::register(wafer).map_err(|e| JsValue::from_str(&e))?;

    // wafer-run/web — static file server via OPFS storage (SPA support)
    wafer_block_web::register(wafer).map_err(|e| JsValue::from_str(&e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Flow registration (mirrors solobase::flows::register_site_main)
// ---------------------------------------------------------------------------

/// Register the site-main flow.
///
/// This is the browser-side equivalent of `solobase::flows::register_site_main`.
/// We inline the same flow JSON and route configuration here to avoid
/// depending on the `solobase` crate (which requires tokio for `server` feature).
fn register_site_main_flow(wafer: &mut wafer_run::Wafer) -> Result<(), JsValue> {
    // Default routes — API goes to suppers-ai/router, everything else to wafer-run/web
    let routes = serde_json::json!([
        { "path": "/b/**",                    "block": "suppers-ai/router" },
        { "path": "/health",                  "block": "suppers-ai/router" },
        { "path": "/openapi.json",            "block": "suppers-ai/router" },
        { "path": "/.well-known/agent.json",  "block": "suppers-ai/router" },
        { "path": "/**",                      "block": "wafer-run/web", "config": { "web_root": "site", "web_spa": "true", "web_index": "index.html" } }
    ]);

    wafer.add_block_config("wafer-run/router", serde_json::json!({ "routes": routes }));
    wafer.add_block_config(
        "wafer-run/web",
        serde_json::json!({
            "web_root": "site",
            "web_spa": "true",
            "web_index": "index.html"
        }),
    );

    // The site-main flow definition
    let flow_json = r#"{
        "id": "site-main",
        "name": "Site Main",
        "version": "0.1.0",
        "description": "Top-level HTTP dispatch — API router + frontend SPA",
        "steps": [
            { "id": "security-headers", "block": "wafer-run/security-headers", "config": { "csp": "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval' https://cdn.jsdelivr.net; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https:; font-src 'self' https:; connect-src 'self' https://cdn.jsdelivr.net https://esm.run https://huggingface.co https://raw.githubusercontent.com https://*.huggingface.co https://*.hf.co https://*.xethub.hf.co; frame-ancestors 'none'; base-uri 'self'; form-action 'self'" } },
            { "id": "cors", "block": "wafer-run/cors" },
            { "id": "readonly-guard", "block": "wafer-run/readonly-guard" },
            { "id": "router", "block": "wafer-run/router" }
        ],
        "config": { "on_error": "stop" },
        "config_map": {
            "routes": { "target": "wafer-run/router", "key": "routes" }
        }
    }"#;

    wafer
        .add_flow_json(flow_json)
        .map_err(|e| JsValue::from_str(&format!("invalid site-main flow JSON: {e}")))
}
