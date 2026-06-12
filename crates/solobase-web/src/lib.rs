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

    // ── Phase 1 ─────────────────────────────────────────────────────────────
    // Build the runtime with EMPTY config + EMPTY block_settings + EMPTY
    // ConfigSource. We can't fill any of these from OPFS yet because the
    // `suppers_ai__admin__variables` / `suppers_ai__admin__block_settings`
    // tables only exist after admin's lazy `lifecycle(Init)` runs its
    // migrations — and admin can't run until the wafer is built and sealed.
    //
    // The schema-drift class of bug (#210/#211) came from this crate trying
    // to short-cut that chicken-and-egg with `CREATE TABLE IF NOT EXISTS`
    // pre-creates that duplicated the admin migration schema by hand. Any
    // drift between the two schemas was silent until the first per-block
    // `migration_helper::write_state` upserted into the stale table and
    // failed on a missing column, taking the whole runtime with it.
    //
    // The proper fix is what the native CLI and Cloudflare runner already
    // do: defer seeding until *after* `init_block(admin)`. Admin's migration
    // is the single source of schema truth; this crate just reads back what
    // it created.

    let config_svc: Arc<dyn ConfigService> =
        Arc::new(wafer_core::service_blocks::config::EnvConfigService::new());
    // Empty initial BlockSettings — every block defaults to enabled. We rewrite
    // this via the handle below in Phase 3 once the real settings are loaded.
    let initial_block_settings =
        solobase_core::features::BlockSettings::from_map(std::collections::HashMap::new());
    // Empty StaticConfigSource: blocks that look up their declared keys via
    // the runtime's ConfigSource at lifecycle(Init) payload-build time will
    // see nothing. That's fine because solobase blocks read their keys via
    // `config_client::get` (which hits `wafer-run/config` → ConfigService)
    // rather than the Init payload, and we populate `config_svc` in Phase 3
    // below before triggering any block's Init.
    let cfg_source: Arc<dyn wafer_run::ConfigSource> =
        Arc::new(wafer_run::StaticConfigSource::default());

    let browser_llm: Arc<dyn wafer_core::interfaces::llm::service::LlmService> =
        Arc::new(solobase_browser::llm::BrowserLlmService::new());
    let browser_image: Arc<dyn wafer_core::interfaces::image::service::ImageService> =
        Arc::new(solobase_browser::image::BrowserImageService::new());
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

    // JWT secret can't be loaded yet (variables table doesn't exist).
    // Construct the concrete `BrowserCryptoService` so we keep a typed Arc
    // for `set_jwt_secret` in Phase 3; the same Arc-coerced trait object
    // gets handed to the builder. Both Arcs point at the same allocation,
    // so the rotation is observed by every block via the existing service.
    let crypto_concrete = Arc::new(solobase_browser::crypto::BrowserCryptoService::new(
        String::new(),
    ));
    let crypto_svc: Arc<dyn wafer_core::interfaces::crypto::service::CryptoService> =
        crypto_concrete.clone();

    let builder = SolobaseBuilder::new()
        .database(solobase_browser::make_database_service())
        .storage(solobase_browser::make_storage_service())
        .config(config_svc.clone())
        .crypto(crypto_svc)
        .network(solobase_browser::make_network_service())
        .logger(solobase_browser::make_console_logger())
        .llm_service("browser", browser_llm)
        .image_service("browser", browser_image)
        .vector_service(browser_vector)
        .embedding_service(browser_embedding)
        .block_settings(initial_block_settings)
        .block_config(
            "wafer-run/security-headers",
            serde_json::json!({ "csp": SOLOBASE_CSP }),
        )
        .config_source(cfg_source);
    let block_settings_handle = builder.block_settings_handle();

    let (mut wafer, storage_block) = builder
        .build()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    wafer.set_asset_loader(&solobase_browser::make_sw_asset_loader());

    wafer
        .seal()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // ── Phase 2 ─────────────────────────────────────────────────────────────
    // Run admin's `lifecycle(Init)` so its migrations create the canonical
    // `suppers_ai__admin__variables` + `suppers_ai__admin__block_settings`
    // tables with the strict schema declared in
    // `crates/solobase-core/src/blocks/admin/migrations/001_admin_schema.sqlite.sql`.
    // Mirrors the native CLI's `start_with_priority(&[admin])` and the
    // Cloudflare runner's `init_block(admin)` step.
    if let Err(e) = wafer
        .init_block(solobase_core::blocks::admin::ADMIN_BLOCK_ID)
        .await
    {
        web_sys::console::error_1(&format!("solobase: admin block Init failed: {e}").into());
        return Err(JsValue::from_str(&format!("admin init failed: {e}")));
    }

    // ── Phase 3 ─────────────────────────────────────────────────────────────
    // Now the variables + block_settings tables exist with the canonical
    // schema. Seed/load against them, then publish the results into the
    // services the wafer already holds:
    //  - `config_svc` is shared via Arc, mutated in place via `.set()`
    //  - `block_settings_handle` is the same `Arc<RwLock<…>>` the router's
    //    `FeatureConfig` reads from, so writes here are visible to the
    //    subsequent `init_all_blocks()` and to every later request.
    //  - `crypto_svc` exposes `set_jwt_secret` so the rotated secret takes
    //    effect for any block that hasn't initialised yet.
    let vars = config::seed_and_load_variables()?;
    web_sys::console::log_1(
        &format!("solobase: {} variables loaded from database", vars.len()).into(),
    );
    let features = config::load_block_settings()?;

    for (key, value) in &vars {
        config_svc.set(key, value);
    }
    config_svc.set(
        solobase_core::features::BLOCK_SETTINGS_CONFIG_KEY,
        &features.to_config_json(),
    );
    *block_settings_handle
        .write()
        .expect("BlockSettings RwLock poisoned") = features;
    if let Some(secret) = vars.get(solobase_core::blocks::auth::JWT_SECRET_KEY) {
        crypto_concrete.set_jwt_secret(secret.clone());
    }

    // ── Phase 4 ─────────────────────────────────────────────────────────────
    // Eager init the remaining blocks. With config_svc + block_settings now
    // populated, auth's `bootstrap::run` reads BOOTSTRAP_ADMIN_{EMAIL,PASSWORD}
    // via `config_client::get_default` and creates the admin user on a fresh
    // OPFS. Slot caching makes admin's second init a no-op.
    wafer.init_all_blocks().await;

    builder::post_start(&wafer, &storage_block);

    web_sys::console::log_1(&"solobase: WAFER runtime started".into());

    solobase_browser::store_wafer(wafer).map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(())
}

#[wasm_bindgen]
pub async fn handle_request(request: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    solobase_browser::dispatch_request(request).await
}
