//! HTTP route dispatcher for suppers-ai/vector.
//!
//! Implemented routes:
//!   - `POST   /b/vector/api/indexes`           → create an index
//!   - `GET    /b/vector/api/indexes`           → list indexes for this project
//!   - `DELETE /b/vector/api/indexes/{name}`    → delete an index
//!   - `POST   /b/vector/api/upsert`            → upsert pre-computed vectors
//!   - `DELETE /b/vector/api/{index}/{id}`      → delete a single vector
//!   - `GET    /b/vector/api/stats`             → per-index counts
//!
//! User-facing index names are prefixed with `suppers_ai__vector__` before
//! being passed to the `wafer-run/vector` runtime block. The prefix is
//! stripped on the way out in list/stats responses.
//!
//! Remaining routes (query, ingest, embed) still resolve to `Unimplemented`;
//! they land in Tasks 17 and 19.
//!
//! ### Route ordering note
//!
//! `DELETE /b/vector/api/indexes/{name}` and `DELETE /b/vector/api/{index}/{id}`
//! both map to the `delete` action and both live under `/b/vector/api/`.
//! The dispatcher checks the `indexes/` prefix first so the more specific
//! route wins; only after that do we fall through to the generic
//! `{index}/{id}` handler.

use wafer_core::clients::database as db;
use wafer_core::clients::vector as vclient;
use wafer_core::interfaces::vector::{get_model, DistanceMetric, VectorIndexConfig, DEFAULT_MODEL};
use wafer_core::interfaces::vector::service::VectorEntry;
use wafer_run::{context::Context, types::*, InputStream, OutputStream};

use super::service::{self, TABLE_PREFIX};
use crate::blocks::helpers::{err_bad_request, err_internal, err_not_found, ok_json};

/// Route dispatcher for the `suppers-ai/vector` block.
///
/// Matches on the normalized action (GET→`retrieve`, POST→`create`,
/// DELETE→`delete`, etc.) and the request path. Anything that does not
/// match a handled route resolves to `Unimplemented`, which lets the block
/// compile and be registered while the remaining handlers land in future
/// tasks.
pub async fn route(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("create", "/b/vector/api/indexes") => create_index(ctx, input).await,
        ("retrieve", "/b/vector/api/indexes") => list_indexes(ctx).await,
        ("create", "/b/vector/api/upsert") => upsert(ctx, input).await,
        ("retrieve", "/b/vector/api/stats") => stats(ctx).await,
        // NOTE: the `indexes/` guard must come before the generic
        // `{index}/{id}` guard so `/indexes/foo` resolves to `delete_index`
        // rather than being matched as `index=indexes, id=foo`.
        ("delete", p) if p.starts_with("/b/vector/api/indexes/") => delete_index(ctx, msg).await,
        ("delete", p) if p.starts_with("/b/vector/api/") && p != "/b/vector/api/" => {
            delete_single(ctx, msg).await
        }
        _ => OutputStream::error(WaferError {
            code: ErrorCode::Unimplemented,
            message: format!("vector route not yet implemented: {action} {path}"),
            meta: vec![],
        }),
    }
}

// ---------------------------------------------------------------------------
// POST /b/vector/api/indexes — create an index
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct CreateIndexBody {
    name: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    dimensions: Option<u32>,
    #[serde(default)]
    metric: Option<DistanceMetric>,
    #[serde(default)]
    keyword_search: bool,
}

async fn create_index(ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: CreateIndexBody = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    if body.name.is_empty() {
        return err_bad_request("index name is required");
    }

    let model_id = body
        .model
        .as_deref()
        .unwrap_or(DEFAULT_MODEL)
        .to_string();
    let model = match get_model(&model_id) {
        Some(m) => m,
        None => return err_bad_request(&format!("unknown embedding model: {model_id}")),
    };

    if let Some(requested) = body.dimensions {
        if requested != model.dimensions {
            return err_bad_request(&format!(
                "dimensions mismatch: model {} has {} dimensions, got {}",
                model.id, model.dimensions, requested
            ));
        }
    }

    let cfg = VectorIndexConfig {
        name: service::prefixed_index_name(&body.name),
        model: model.id.to_string(),
        dimensions: model.dimensions,
        metric: body.metric.unwrap_or(DistanceMetric::Cosine),
        keyword_search: body.keyword_search,
    };

    match vclient::create_index(ctx, cfg.clone()).await {
        Ok(()) => ok_json(&serde_json::json!({
            "name": body.name,
            "model": cfg.model,
            "dimensions": cfg.dimensions,
            "metric": cfg.metric,
            "keyword_search": cfg.keyword_search,
        })),
        Err(e) => err_internal(&format!("create_index failed: {e}")),
    }
}

// ---------------------------------------------------------------------------
// GET /b/vector/api/indexes — list indexes
// ---------------------------------------------------------------------------

async fn list_indexes(ctx: &dyn Context) -> OutputStream {
    match discover_indexes(ctx).await {
        Ok(indexes) => ok_json(&serde_json::json!({ "indexes": indexes })),
        Err(e) => err_internal(&format!("list indexes failed: {e}")),
    }
}

/// Scan sqlite_master for the per-index `_meta` tables created by
/// `SqliteVecService::create_index` and return the user-facing index
/// names (prefix + `_meta` suffix stripped).
///
/// Task 17 introduces a dedicated registry table — once that lands this
/// query becomes a simple list of registry rows. Until then, the metadata
/// table is the canonical marker that an index exists.
async fn discover_indexes(ctx: &dyn Context) -> Result<Vec<String>, WaferError> {
    let pattern = format!("{TABLE_PREFIX}%_meta");
    let rows = db::query_raw(
        ctx,
        "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE ? ORDER BY name",
        &[serde_json::Value::String(pattern)],
    )
    .await?;

    let mut indexes: Vec<String> = Vec::with_capacity(rows.len());
    for row in rows {
        let table = row
            .data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if let Some(stored) = table.strip_suffix("_meta") {
            if let Some(user_name) = stored.strip_prefix(TABLE_PREFIX) {
                if !user_name.is_empty() {
                    indexes.push(user_name.to_string());
                }
            }
        }
    }
    Ok(indexes)
}

// ---------------------------------------------------------------------------
// DELETE /b/vector/api/indexes/{name} — delete an index
// ---------------------------------------------------------------------------

async fn delete_index(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let name = extract_index_name(msg);
    if name.is_empty() {
        return err_bad_request("index name is required");
    }

    let prefixed = service::prefixed_index_name(name);
    match vclient::delete_index(ctx, &prefixed).await {
        Ok(()) => ok_json(&serde_json::json!({ "ok": true })),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(&format!("index not found: {name}"))
        }
        Err(e) => err_internal(&format!("delete_index failed: {e}")),
    }
}

/// Extract `{name}` from `/b/vector/api/indexes/{name}`.
///
/// Prefers the router-populated `name` path variable when available, falling
/// back to string-splitting for direct handler invocation (e.g. in tests).
fn extract_index_name(msg: &Message) -> &str {
    let var = msg.var("name");
    if !var.is_empty() {
        return var;
    }
    msg.path()
        .strip_prefix("/b/vector/api/indexes/")
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("")
}

// ---------------------------------------------------------------------------
// POST /b/vector/api/upsert — upsert pre-computed vectors
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct UpsertBody {
    index: String,
    entries: Vec<VectorEntry>,
}

async fn upsert(ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: UpsertBody = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    if body.index.is_empty() {
        return err_bad_request("index is required");
    }

    let prefixed = service::prefixed_index_name(&body.index);
    match vclient::upsert(ctx, &prefixed, body.entries).await {
        Ok(()) => ok_json(&serde_json::json!({ "ok": true })),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(&format!("index not found: {}", body.index))
        }
        Err(e) if e.code == ErrorCode::InvalidArgument => err_bad_request(&e.message),
        Err(e) => err_internal(&format!("upsert failed: {e}")),
    }
}

// ---------------------------------------------------------------------------
// DELETE /b/vector/api/{index}/{id} — delete a single vector by ID
// ---------------------------------------------------------------------------

async fn delete_single(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let (index, id) = extract_index_and_id(msg);
    if index.is_empty() {
        return err_bad_request("index is required");
    }
    if id.is_empty() {
        return err_bad_request("id is required");
    }

    let prefixed = service::prefixed_index_name(index);
    match vclient::delete(ctx, &prefixed, vec![id.to_string()]).await {
        Ok(()) => ok_json(&serde_json::json!({ "ok": true })),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(&format!("index not found: {index}"))
        }
        Err(e) => err_internal(&format!("delete failed: {e}")),
    }
}

/// Extract `{index}` and `{id}` from `/b/vector/api/{index}/{id}`.
///
/// Prefers the router-populated path variables when available, falling back
/// to string-splitting for direct handler invocation (e.g. in tests).
fn extract_index_and_id(msg: &Message) -> (&str, &str) {
    let index = msg.var("index");
    let id = msg.var("id");
    if !index.is_empty() && !id.is_empty() {
        return (index, id);
    }

    let rest = msg
        .path()
        .strip_prefix("/b/vector/api/")
        .unwrap_or("");
    let mut parts = rest.split('/');
    let index_part = parts.next().unwrap_or("");
    let id_part = parts.next().unwrap_or("");
    (index_part, id_part)
}

// ---------------------------------------------------------------------------
// GET /b/vector/api/stats — per-index counts
// ---------------------------------------------------------------------------

async fn stats(ctx: &dyn Context) -> OutputStream {
    let indexes = match discover_indexes(ctx).await {
        Ok(v) => v,
        Err(e) => return err_internal(&format!("stats failed: {e}")),
    };

    let mut out: Vec<serde_json::Value> = Vec::with_capacity(indexes.len());
    for name in indexes {
        let prefixed = service::prefixed_index_name(&name);
        // If count fails for a single index (e.g. table was dropped between
        // discovery and count), fall back to 0 and keep going — stats should
        // not 500 on a transient partial-state issue.
        let count = vclient::count(ctx, &prefixed).await.unwrap_or(0);
        out.push(serde_json::json!({
            "name": name,
            "count": count,
        }));
    }

    ok_json(&serde_json::json!({ "indexes": out }))
}
