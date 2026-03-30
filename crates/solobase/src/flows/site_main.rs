/// The top-level flow that wafer-run/http-listener dispatches all requests to.
///
/// API requests are routed to suppers-ai/router, which delegates to the
/// shared solobase-core pipeline (JWT validation, feature gates, admin checks,
/// block dispatch). Non-API requests fall through to wafer-run/web for SPA serving.
///
/// Infrastructure blocks (security headers, CORS, readonly guard)
/// are applied inline before routing.
pub const JSON: &str = r#"{
    "id": "site-main",
    "name": "Site Main",
    "version": "0.1.0",
    "description": "Top-level HTTP dispatch — API router + frontend SPA",
    "steps": [
        { "id": "security-headers", "block": "wafer-run/security-headers" },
        { "id": "cors", "block": "wafer-run/cors" },
        { "id": "readonly-guard", "block": "wafer-run/readonly-guard" },
        { "id": "router", "block": "wafer-run/router" }
    ],
    "config": { "on_error": "stop" },
    "config_map": {
        "routes": { "target": "wafer-run/router", "key": "routes" }
    }
}"#;

/// Default routes for the site-main flow.
///
/// Block SSR pages + API go through `suppers-ai/router`.
/// Static assets are embedded and served by the system block via `/static/`.
/// User's own site content is served by `wafer-run/web` as fallback.
pub fn default_routes() -> serde_json::Value {
    serde_json::json!([
        { "path": "/_inspector/**", "block": "wafer-run/inspector" },
        { "path": "/_inspector",    "block": "wafer-run/inspector" },
        { "path": "/static/**",     "block": "suppers-ai/router" },
        { "path": "/b/**",          "block": "suppers-ai/router" },
        { "path": "/auth/**",       "block": "suppers-ai/router" },
        { "path": "/internal/**",   "block": "suppers-ai/router" },
        { "path": "/admin/**",      "block": "suppers-ai/router" },
        { "path": "/settings/**",   "block": "suppers-ai/router" },
        { "path": "/storage/**",    "block": "suppers-ai/router" },
        { "path": "/profile/**",    "block": "suppers-ai/router" },
        { "path": "/health",        "block": "suppers-ai/router" },
        { "path": "/nav",           "block": "suppers-ai/router" },
        { "path": "/debug/**",      "block": "suppers-ai/router" },
        { "path": "/**",            "block": "wafer-run/web", "config": { "web_root": "site", "web_spa": "true", "web_index": "index.html" } }
    ])
}
