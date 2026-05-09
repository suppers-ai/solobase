//! Per-isolate cache for config loaded from D1.
//!
//! On Cloudflare Workers, isolates are reused across requests within their
//! lifetime. We exploit that by holding the loaded `(env_vars, block_settings)`
//! in a static `TtlCache`. The first request in a fresh isolate triggers two
//! D1 reads; subsequent requests read from RAM.
//!
//! `wasm32-unknown-unknown` can't tick `std::time::Instant`, so we use a
//! `Duration::MAX` TTL — effectively "load once per isolate". Admin write
//! paths can call `invalidate()` to force a fresh load on the next request
//! handled by *this* isolate; other isolates pick up changes when they
//! recycle naturally (or by future PR work that adds proper signalling).

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use solobase_core::cache::TtlCache;
use solobase_core::features::BlockSettings;

pub type ConfigSnapshot = (HashMap<String, String>, BlockSettings);

static CACHE: OnceLock<TtlCache<ConfigSnapshot>> = OnceLock::new();

fn cache() -> &'static TtlCache<ConfigSnapshot> {
    CACHE.get_or_init(|| TtlCache::new(Duration::MAX))
}

pub async fn get_or_load<F, Fut>(loader: F) -> Arc<ConfigSnapshot>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ConfigSnapshot>,
{
    cache().get_or_load(loader).await
}

#[allow(dead_code)]
pub fn invalidate() {
    if let Some(c) = CACHE.get() {
        c.invalidate();
    }
}
