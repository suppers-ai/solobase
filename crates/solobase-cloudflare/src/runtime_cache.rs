//! Per-isolate runtime cache. Builds the Wafer once per isolate (sealed, no
//! boot funnel — migrations/seeds happen at deploy via `/_deploy/init`),
//! stores it in a thread_local, and rebuilds when the KV config-version
//! stamp moves. Mirrors solobase-browser/src/runtime.rs's thread_local
//! pattern; `Rc` handles (not raw pointers) keep an in-flight request's
//! runtime alive across a swap. wasm32 is single-threaded, so the RefCell
//! borrows are never contended — but they are still never held across an
//! `.await` (interleaved fetch events resume at await points).
//!
//! Concurrent first requests may race to build; last store wins. A build
//! is pure CPU plus one KV-cached block_settings read, so a duplicate
//! build is wasteful-but-correct and only possible in the first instants
//! of an isolate's life. (YAGNI: no async build-guard until measurement
//! says otherwise.)

use std::{cell::RefCell, rc::Rc, sync::Arc};

use solobase_core::cache_key::CONFIG_VERSION_KEY;
use wafer_core::interfaces::database::service::DatabaseService;

pub(crate) struct ReadyRuntime {
    pub wafer: wafer_run::Wafer,
    pub db: Arc<dyn DatabaseService>,
    /// KV backend this runtime was built with. Held so the config-version
    /// probe on the request hot path reuses it instead of constructing a fresh
    /// `KvStore` handle from `env` on every request.
    pub kv: Arc<dyn crate::kv_cached_db::KvBackend>,
    pub version: String,
}

thread_local! {
    static RUNTIME: RefCell<Option<Rc<ReadyRuntime>>> = const { RefCell::new(None) };
}

fn cached() -> Option<Rc<ReadyRuntime>> {
    RUNTIME.with(|r| r.borrow().clone())
}

fn store(rt: Rc<ReadyRuntime>) {
    RUNTIME.with(|r| *r.borrow_mut() = Some(rt));
}

/// Read-only peek at the currently-cached runtime, if any. Used by `run` to
/// drain queued request-log rows through the cached runtime's DB handle in a
/// `waitUntil` without forcing a build.
pub(crate) fn peek() -> Option<Rc<ReadyRuntime>> {
    cached()
}

/// Current KV config-version stamp. Missing key ⇒ stamp a fresh one so all
/// isolates converge on the same generation.
async fn current_version(kv: &Arc<dyn crate::kv_cached_db::KvBackend>) -> String {
    match kv.get(CONFIG_VERSION_KEY).await {
        Ok(Some(v)) => v,
        _ => {
            let v = crate::kv_cached_db::new_version_stamp();
            let _ = kv
                .put_with_ttl(CONFIG_VERSION_KEY, &v, crate::kv_cached_db::CACHE_TTL_SECS)
                .await;
            v
        }
    }
}

/// Return the per-isolate cached runtime, rebuilding it if the KV
/// config-version stamp has moved (or if nothing is cached yet).
///
/// The `register_blocks` / `register_post_build` hooks are `FnOnce` and are
/// consumed only on the build path; on a cache hit they are dropped unused.
pub(crate) async fn get_or_build<F, G>(
    env: &worker::Env,
    register_blocks: F,
    register_post_build: G,
) -> Result<Rc<ReadyRuntime>, Box<dyn std::error::Error>>
where
    F: FnOnce(crate::SolobaseBuilder) -> Result<crate::SolobaseBuilder, Box<dyn std::error::Error>>,
    G: FnOnce(
        &mut wafer_run::Wafer,
        Arc<dyn wafer_core::interfaces::storage::service::StorageService>,
    ) -> Result<(), Box<dyn std::error::Error>>,
{
    // Hit path: probe the config-version through the CACHED runtime's own KV
    // backend — no fresh `KvStore` construction on the request hot path. One
    // KV `get` per request. On a version move we reuse this probed value to
    // tag the rebuild, so the mismatch path still costs a single KV `get`.
    let probed_version = if let Some(rt) = cached() {
        let version = current_version(&rt.kv).await;
        if rt.version == version {
            return Ok(rt);
        }
        tracing::info!(old = %rt.version, new = %version, "config version moved; rebuilding runtime");
        Some(version)
    } else {
        None
    };

    let mut built = crate::build_runtime(env, register_blocks, register_post_build, false).await?;

    // Cold isolate: nothing was cached to probe through, so read the version
    // now via the freshly-built runtime's own backend (still one KV `get`).
    let version = match probed_version {
        Some(v) => v,
        None => current_version(&built.kv).await,
    };

    // Dynamic WRAP grants must be registered before seal.
    crate::apply_db_wrap_grants(&mut built).await;

    built.wafer.seal().await.map_err(|e| format!("seal: {e}"))?;
    solobase_core::builder::post_start(&built.wafer, &built.storage_block);

    let rt = Rc::new(ReadyRuntime {
        wafer: built.wafer,
        db: built.db,
        kv: built.kv,
        version,
    });
    store(rt.clone());
    Ok(rt)
}
