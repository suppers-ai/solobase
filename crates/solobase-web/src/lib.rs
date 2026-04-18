//! Solobase compiled to WASM for running in the browser via Service Worker.
//!
//! Exposes two wasm-bindgen entry points:
//! - `initialize()` — called once when the Service Worker starts; loads the
//!   database, seeds variables, registers all WAFER blocks, and starts the runtime.
//! - `handle_request(request)` — called on each SW fetch event; converts the
//!   browser `Request` into a WAFER `Message`, dispatches it through the
//!   `site-main` flow, and returns a browser `Response`.

use std::{cell::RefCell, sync::Arc};

use solobase::builder::{self, SolobaseBuilder};
use wafer_core::interfaces::config::service::ConfigService;
use wasm_bindgen::prelude::*;

pub mod asset_loader;
pub mod bridge;
pub mod config;
pub mod convert;
pub mod crypto;
pub mod database;
pub mod logger;
pub mod network;
pub mod storage;

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
    // Guard against double initialization
    let already_init = RUNTIME.with(|r| r.borrow().is_some());
    if already_init {
        return Ok(());
    }

    // 1. Load sql.js WASM + open/create the OPFS database.
    bridge::dbInit().await;

    // 2. Seed variables and load config.
    let vars = config::seed_and_load_variables();
    web_sys::console::log_1(
        &format!("solobase: {} variables loaded from database", vars.len()).into(),
    );

    // 3. Load feature flag settings.
    let features = config::load_block_settings();

    // 4. Extract JWT secret.
    let jwt_secret = vars
        .get("SUPPERS_AI__AUTH__JWT_SECRET")
        .cloned()
        .unwrap_or_default();

    // 5. Build config service.
    let config_svc = wafer_block_config::service::EnvConfigService::new();
    for (key, value) in &vars {
        config_svc.set(key, value);
    }

    // 6. Build WAFER runtime via SolobaseBuilder.
    let (mut wafer, storage_block) = SolobaseBuilder::new()
        .database(Arc::new(database::BrowserDatabaseService))
        .storage(Arc::new(storage::BrowserStorageService))
        .config(Arc::new(config_svc))
        .crypto(Arc::new(crypto::BrowserCryptoService::new(jwt_secret)))
        .network(Arc::new(network::BrowserNetworkService))
        .logger(Arc::new(logger::ConsoleLogger))
        .block_settings(features)
        .block_config("wafer-run/security-headers", serde_json::json!({
            "csp": concat!(
                "default-src 'self'; ",
                "script-src 'self' 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval' https://cdn.jsdelivr.net; ",
                "style-src 'self' 'unsafe-inline'; ",
                "img-src 'self' data: blob: https:; ",
                "font-src 'self' https:; ",
                "connect-src 'self' https://cdn.jsdelivr.net https://esm.run https://huggingface.co ",
                    "https://raw.githubusercontent.com https://*.huggingface.co https://*.hf.co https://*.xethub.hf.co; ",
                "frame-ancestors 'none'; ",
                "base-uri 'self'; ",
                "form-action 'self'"
            )
        }))
        .build()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 6b. Register the SW-side external-asset loader before start so any
    // block init that triggers an asset load sees the real loader (not the
    // NoopAssetLoader default).
    wafer.set_asset_loader(Arc::new(asset_loader::SwAssetLoader::new()));

    // 7. Start runtime.
    wafer
        .start_without_bind()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 8. Inject WRAP grants.
    builder::post_start(&wafer, &storage_block);

    web_sys::console::log_1(&"solobase: WAFER runtime started".into());

    // 9. Store in global.
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
    let (msg, input) = convert::request_to_message(&request).await?;

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
    let output = wafer.run("site-main", msg, input).await;

    convert::output_to_response(output).await
}
