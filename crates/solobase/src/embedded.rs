//! Embedded frontend assets.
//!
//! Uses `rust-embed` to embed the built frontend SPA into the binary.
//! In dev mode the files are served from disk (hot-reloadable);
//! in release mode they are baked into the binary at compile time.

use axum::body::Body;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use rust_embed::RustEmbed;

/// Embedded frontend assets from the SvelteKit build output.
///
/// In debug mode, rust-embed serves files from disk (allowing hot-reload
/// without recompiling the Rust binary). In release mode the files are
/// embedded as static bytes.
#[derive(RustEmbed)]
#[folder = "frontend/build"]
struct FrontendAssets;

/// Create an axum Router that serves the embedded frontend assets.
///
/// This acts as a SPA fallback: known static files are served directly,
/// and any unknown path falls back to `index.html` so client-side routing works.
pub fn frontend_router() -> Router {
    Router::new()
        .fallback(get(serve_frontend))
}

/// Handler that serves embedded frontend files with multi-page SPA fallback.
///
/// Each block has its own HTML entry point. URL paths are mapped to the
/// correct entry:
///   /admin/login       → blocks/auth/frontend/index.html
///   /admin/monitoring* → blocks/monitoring/frontend/index.html
///   /admin/logs*       → blocks/logs/frontend/index.html
///   /admin/iam*        → blocks/admin/frontend/iam/index.html
///   /admin/ext/products* → blocks/products/frontend/index.html
///   /admin*            → blocks/admin/frontend/index.html  (default admin)
async fn serve_frontend(
    uri: axum::http::Uri,
) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try to serve the exact path first (static assets like JS, CSS, images)
    if let Some(response) = serve_asset(path) {
        return response;
    }

    // Multi-page SPA fallback: route to the correct block entry point
    let entry = resolve_spa_entry(uri.path());
    if let Some(response) = serve_asset(entry) {
        return response;
    }

    // Final fallback — try the admin entry
    if let Some(response) = serve_asset("blocks/admin/frontend/index.html") {
        return response;
    }

    (StatusCode::NOT_FOUND, "frontend not found").into_response()
}

/// Resolve a URL path to the correct block frontend entry point.
fn resolve_spa_entry(path: &str) -> &'static str {
    if path.starts_with("/admin/login") {
        "blocks/auth/frontend/index.html"
    } else if path.starts_with("/admin/monitoring") {
        "blocks/monitoring/frontend/index.html"
    } else if path.starts_with("/admin/logs") {
        "blocks/logs/frontend/index.html"
    } else if path.starts_with("/admin/iam") {
        "blocks/admin/frontend/iam/index.html"
    } else if path.starts_with("/admin/ext/products") {
        "blocks/products/frontend/index.html"
    } else {
        // Default: admin panel (handles /admin, /admin/users, /admin/database, etc.)
        "blocks/admin/frontend/index.html"
    }
}

/// Try to serve a specific asset from the embedded files.
fn serve_asset(path: &str) -> Option<Response> {
    let asset = FrontendAssets::get(path)?;

    // rust-embed's metadata includes the MIME type when the `mime-guess` feature is enabled
    let content_type = asset
        .metadata
        .mimetype()
        .to_string();

    // Immutable assets (hashed filenames) get long cache headers
    let cache_control = if path.contains("/_app/") || path.contains("/assets/") {
        "public, max-age=31536000, immutable"
    } else if path == "index.html" || path == "favicon.ico" {
        "public, max-age=0, must-revalidate"
    } else {
        "public, max-age=3600"
    };

    let body = Body::from(asset.data.into_owned());

    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CACHE_CONTROL, cache_control)
            .body(body)
            .unwrap(),
    )
}
