//! Shared request pipeline — the core solobase request handling logic.
//!
//! Both Cloudflare and native adapters call `handle_request()` after
//! converting their platform-specific HTTP types into a WAFER Message.

use wafer_run::context::Context;
use wafer_run::meta::*;
use wafer_run::types::*;

use crate::features::FeatureConfig;
use crate::routing::{self, BlockFactory};

/// Handle a solobase request.
///
/// This is the shared entry point that both CF and native adapters call
/// after building a Message from the incoming HTTP request.
///
/// Steps:
/// 1. Strip `/api` prefix (CF convention — native doesn't use it)
/// 2. Validate JWT and set auth meta
/// 3. Route to the appropriate solobase block
pub async fn handle_request(
    ctx: &dyn Context,
    msg: &mut Message,
    auth_header: Option<&str>,
    jwt_secret: &str,
    features: &dyn FeatureConfig,
    factory: &dyn BlockFactory,
) -> Result_ {
    // 1. Strip /api prefix from resource path
    let resource = msg.path().to_string();
    if let Some(stripped) = resource.strip_prefix("/api") {
        msg.set_meta(META_REQ_RESOURCE, stripped);
    }

    // 2. Validate JWT and set auth meta
    if let Some(header) = auth_header {
        crate::crypto::extract_auth_meta(header, jwt_secret, msg);
    }

    // 3. Route to block
    routing::route_to_block(ctx, msg, features, factory).await
}
