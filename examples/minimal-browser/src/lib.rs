//! Smallest-possible consumer of `solobase-browser`. No SolobaseBuilder;
//! no solobase-core. Builds a bare Wafer with browser platform services
//! and no registered blocks.
//!
//! Its purpose is to fail-loud at compile time if `solobase-browser`
//! accidentally grows a dependency on `solobase` or `solobase-core`, or if
//! the framework contract requires app-level types that non-solobase
//! consumers won't have.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn initialize() -> Result<(), JsValue> {
    if solobase_browser::is_initialized() {
        return Ok(());
    }

    // Init the sql.js + OPFS database before constructing services.
    solobase_browser::db_init().await;

    // Build a bare Wafer using only the wafer-run public API.
    // No SolobaseBuilder, no solobase-core, no solobase-specific blocks.
    let mut wafer = wafer_run::Wafer::new();

    // Wire up the browser platform services from solobase-browser factories.
    // These are the same factories solobase-web uses, but assembled directly
    // without the SolobaseBuilder layer.
    wafer.set_asset_loader(solobase_browser::make_sw_asset_loader());

    // Start the runtime (resolves block deps, snapshots introspection data).
    // An empty Wafer with no flows is valid — dispatch_request returns a
    // 503-shaped response when "site-main" is not found, which is the
    // expected behaviour for this smoke-test binary.
    wafer
        .start_without_bind()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    solobase_browser::store_wafer(wafer);

    Ok(())
}

#[wasm_bindgen]
pub async fn handle_request(req: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    solobase_browser::dispatch_request(req).await
}
