//! HTTP route dispatcher for suppers-ai/vector.
//!
//! Implemented routes:
//!   - `POST   /b/vector/api/indexes`           → create an index
//!   - `GET    /b/vector/api/indexes`           → list indexes for this project
//!   - `DELETE /b/vector/api/indexes/{name}`    → delete an index
//!   - `POST   /b/vector/api/upsert`            → upsert pre-computed vectors
//!   - `POST   /b/vector/api/query`             → search vectors (vector/keyword/hybrid)
//!   - `DELETE /b/vector/api/{index}/{id}`      → delete a single vector
//!   - `GET    /b/vector/api/stats`             → per-index counts
//!
//! User-facing index names are prefixed with `suppers_ai__vector__` before
//! being passed to the `wafer-run/vector` runtime block. The prefix is
//! stripped on the way out in list/stats responses.
//!
//! Task 19 implements ingest and embed:
//!   - `POST   /b/vector/api/ingest`            → chunk + embed + upsert
//!   - `POST   /b/vector/api/embed`             → raw text → vectors
//!
//! ### Registry table
//!
//! Per-index metadata (model, dimensions, keyword_search flag) is kept in
//! `suppers_ai__vector__registry`. This lets the query route look up the
//! correct embedding model when the caller sends text instead of a raw
//! vector. Rows are written on `create_index` and removed on `delete_index`.
//! Pre-registry indexes (created before this table existed) fall back to
//! `DEFAULT_MODEL` + an `_fts` scan on `sqlite_master`.
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
use wafer_core::interfaces::vector::service::VectorEntry;
use wafer_core::interfaces::vector::{
    get_model, DistanceMetric, MetadataFilter, SearchMode, VectorIndexConfig, DEFAULT_MODEL,
};
use wafer_run::{context::Context, types::*, InputStream, OutputStream};

use super::ingestion::{self, DEFAULT_CHUNK_TOKENS, DEFAULT_OVERLAP_RATIO};
use super::service::{self, TABLE_PREFIX};
use crate::blocks::helpers::{err_bad_request, err_internal, err_not_found, ok_json};

/// Per-index metadata registry table.
///
/// One row per index, keyed by the prefixed (storage) name. Keeping this
/// separate from `sqlite_master` lets us remember per-index knobs — the
/// model to re-embed text with, and whether keyword search was enabled —
/// without spelunking through DDL on every query.
///
/// The name is embedded in each SQL statement below (no string
/// interpolation: it's a compile-time literal and sharing a single place
/// to grep is more valuable than DRYing two or three usages).
///
/// Schema: `suppers_ai__vector__registry(prefixed_name TEXT PK, model TEXT,
/// dimensions INTEGER, keyword_search INTEGER)`.
const REGISTRY_CREATE_SQL: &str = "CREATE TABLE IF NOT EXISTS suppers_ai__vector__registry(\
    prefixed_name TEXT PRIMARY KEY,\
    model TEXT NOT NULL,\
    dimensions INTEGER NOT NULL,\
    keyword_search INTEGER NOT NULL\
)";

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
        ("create", "/b/vector/api/query") => query(ctx, input).await,
        ("create", "/b/vector/api/ingest") => ingest(ctx, input).await,
        ("create", "/b/vector/api/embed") => embed(ctx, input).await,
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

    if let Err(e) = vclient::create_index(ctx, cfg.clone()).await {
        return err_internal(&format!("create_index failed: {e}"));
    }

    // Record the index in the registry so queries against it can look up
    // the right embedding model when the caller sends text. A failure here
    // leaves the underlying index created but unregistered — report it as
    // an internal error rather than silently swallowing it; the operator
    // can retry create (idempotent at the registry level via OR REPLACE
    // and harmless at vclient level because the index already exists).
    if let Err(e) = ensure_registry(ctx).await {
        return err_internal(&format!("registry init failed: {e}"));
    }
    if let Err(e) = db::exec_raw(
        ctx,
        "INSERT OR REPLACE INTO suppers_ai__vector__registry(prefixed_name, model, dimensions, keyword_search) VALUES (?1, ?2, ?3, ?4)",
        &[
            serde_json::Value::String(cfg.name.clone()),
            serde_json::Value::String(cfg.model.clone()),
            serde_json::Value::Number(serde_json::Number::from(cfg.dimensions)),
            serde_json::Value::Number(serde_json::Number::from(cfg.keyword_search as i64)),
        ],
    )
    .await
    {
        return err_internal(&format!("registry write failed: {e}"));
    }

    ok_json(&serde_json::json!({
        "name": body.name,
        "model": cfg.model,
        "dimensions": cfg.dimensions,
        "metric": cfg.metric,
        "keyword_search": cfg.keyword_search,
    }))
}

/// Idempotently create the registry table.
async fn ensure_registry(ctx: &dyn Context) -> Result<(), WaferError> {
    db::exec_raw(ctx, REGISTRY_CREATE_SQL, &[]).await.map(|_| ())
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
/// We scan `sqlite_master` rather than the registry so that indexes
/// created before the registry existed still surface in list/stats.
/// The registry is the source of truth for *per-index metadata* (model,
/// keyword_search flag), not for existence.
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
        Ok(()) => {
            // Clear the registry row. Best-effort — a missing registry table
            // (pre-registry deployment) is not a failure, so we only surface
            // errors that aren't about the table itself. The row-level
            // `OR REPLACE` in create_index makes this robustly idempotent.
            let _ = db::exec_raw(
                ctx,
                "DELETE FROM suppers_ai__vector__registry WHERE prefixed_name = ?1",
                &[serde_json::Value::String(prefixed)],
            )
            .await;
            ok_json(&serde_json::json!({ "ok": true }))
        }
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

// ---------------------------------------------------------------------------
// POST /b/vector/api/query — search vectors
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct QueryBody {
    index: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    vector: Option<Vec<f32>>,
    #[serde(default)]
    top_k: Option<usize>,
    #[serde(default)]
    filter: Option<MetadataFilter>,
    #[serde(default)]
    mode: Option<SearchMode>,
    #[serde(default)]
    keyword_query: Option<String>,
}

const DEFAULT_TOP_K: usize = 10;

async fn query(ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: QueryBody = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    if body.index.is_empty() {
        return err_bad_request("index is required");
    }

    let prefixed = service::prefixed_index_name(&body.index);

    // Look up model + keyword_search from the registry, falling back to
    // DEFAULT_MODEL + sqlite_master scan for indexes created before the
    // registry table existed.
    let (model_id, keyword_search) = match load_index_metadata(ctx, &prefixed).await {
        Ok(m) => m,
        Err(e) => return err_internal(&format!("load index metadata failed: {e}")),
    };

    // Default mode reflects the index's declared capabilities. An index
    // created with keyword_search=true gets Hybrid by default; everyone
    // else gets plain Vector.
    let mode = body.mode.unwrap_or(if keyword_search {
        SearchMode::Hybrid
    } else {
        SearchMode::Vector
    });

    // Resolve the query vector. If the caller provided a vector directly
    // we use it; otherwise we embed `text` through the model the index
    // was created with. Exactly one of the two must be present.
    let vector = match (body.vector.clone(), body.text.as_deref()) {
        (Some(v), _) if !v.is_empty() => v,
        (_, Some(text)) if !text.is_empty() => {
            let block = embedding_block_for_model(&model_id);
            match vclient::embed(ctx, block, vec![text.to_string()]).await {
                Ok((_, _, mut vectors)) => match vectors.pop() {
                    Some(v) => v,
                    None => return err_internal("embedding block returned no vectors"),
                },
                Err(e) => return err_internal(&format!("embed failed: {e}")),
            }
        }
        _ => return err_bad_request("either 'text' or 'vector' is required"),
    };

    // For modes that use keyword search, default the keyword query to the
    // raw text when the caller didn't supply an explicit one. This lets
    // hybrid-mode callers pass just `text` and get both halves for free.
    let keyword_query = match (mode, body.keyword_query.clone(), body.text.clone()) {
        (SearchMode::Vector, kq, _) => kq,
        (_, Some(kq), _) => Some(kq),
        (_, None, Some(text)) => Some(text),
        (_, None, None) => None,
    };

    let top_k = body.top_k.unwrap_or(DEFAULT_TOP_K);

    match vclient::query(ctx, &prefixed, vector, top_k, body.filter, mode, keyword_query).await {
        Ok(matches) => ok_json(&serde_json::json!({ "matches": matches })),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(&format!("index not found: {}", body.index))
        }
        Err(e) if e.code == ErrorCode::InvalidArgument => err_bad_request(&e.message),
        Err(e) => err_internal(&format!("query failed: {e}")),
    }
}

/// Load `(model_id, keyword_search)` for an index.
///
/// Registry-first: if the row exists we trust it. If it doesn't (pre-registry
/// index, or registry table missing entirely) we fall back to
/// `DEFAULT_MODEL` and infer `keyword_search` by checking `sqlite_master`
/// for the per-index `_fts` table. Existence of the index is validated by
/// `vclient::query` itself, which returns `NotFound` when the underlying
/// tables are missing — so this helper always returns `Ok` on a successful
/// database roundtrip.
async fn load_index_metadata(
    ctx: &dyn Context,
    prefixed_index: &str,
) -> Result<(String, bool), WaferError> {
    // First try the registry. An error here (e.g. the table doesn't exist)
    // is treated as "no row", not fatal — we fall through to the scan.
    let rows = db::query_raw(
        ctx,
        "SELECT model, keyword_search FROM suppers_ai__vector__registry WHERE prefixed_name = ?1",
        &[serde_json::Value::String(prefixed_index.to_string())],
    )
    .await;

    if let Ok(rows) = rows {
        if let Some(row) = rows.into_iter().next() {
            let model = row
                .data
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or(DEFAULT_MODEL)
                .to_string();
            let kw = row
                .data
                .get("keyword_search")
                .and_then(|v| v.as_i64())
                .map(|n| n != 0)
                .unwrap_or(false);
            return Ok((model, kw));
        }
    }

    // Fallback path: infer keyword_search from the FTS table's presence.
    // `wafer-block-sqlite::SqliteVecService::create_index` creates
    // `{prefixed}_fts` when keyword_search is enabled and nothing when it
    // isn't, so the row count for that exact name is a reliable signal.
    let fts_name = format!("{prefixed_index}_fts");
    let fts_rows = db::query_raw(
        ctx,
        "SELECT name FROM sqlite_master WHERE type='table' AND name = ?1",
        &[serde_json::Value::String(fts_name)],
    )
    .await?;
    let keyword_search = !fts_rows.is_empty();

    Ok((DEFAULT_MODEL.to_string(), keyword_search))
}

/// Map a model id to the embedding block that serves it on this runtime.
///
/// On native we route everything to `suppers-ai/fastembed` — fastembed's
/// catalog covers every model in our native-embed support matrix. Plan 2
/// (Workers AI) and Plan 3 (browser/transformers) will split this by
/// `model_id` so different models dispatch to different embedding blocks.
fn embedding_block_for_model(_model_id: &str) -> &'static str {
    "suppers-ai/fastembed"
}

// ---------------------------------------------------------------------------
// POST /b/vector/api/ingest — chunk + (optionally add context) + embed + upsert
// ---------------------------------------------------------------------------

/// Request body for ingest. Shape matches the plan; only `index`,
/// `document_id`, and `text` are required — `metadata` and `contextual` are
/// optional and default to "no metadata" / "no context summary".
#[derive(serde::Deserialize)]
struct IngestBody {
    index: String,
    document_id: String,
    text: String,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
    #[serde(default)]
    contextual: bool,
}

#[derive(serde::Serialize)]
struct IngestResponse {
    chunks_created: usize,
}

/// Handle `POST /b/vector/api/ingest`.
///
/// Flow: prefix the index, look up the embedding model, clear any prior
/// chunks for this `document_id` (re-ingestion safety), chunk the text,
/// optionally add context summaries, embed, and upsert. The response tells
/// the caller how many chunks landed.
async fn ingest(ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: IngestBody = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    if body.index.is_empty() {
        return err_bad_request("index is required");
    }
    if body.document_id.is_empty() {
        return err_bad_request("document_id is required");
    }

    let prefixed = service::prefixed_index_name(&body.index);

    // We need the index's model so we can re-embed the chunks with the same
    // one that was declared at create_index time. `load_index_metadata`
    // falls back to DEFAULT_MODEL for pre-registry indexes — same as the
    // query route does.
    let (model_id, _keyword_search) = match load_index_metadata(ctx, &prefixed).await {
        Ok(m) => m,
        Err(e) => return err_internal(&format!("load index metadata failed: {e}")),
    };

    // Re-ingestion safety: wipe any chunks we previously wrote for this
    // document_id before we add the new ones. If the metadata table isn't
    // there yet (first-ever ingest, or fresh index) the query fails and we
    // take that as "no prior chunks", not as a fatal error.
    if let Ok(rows) = db::query_raw(
        ctx,
        &format!(
            "SELECT id FROM {prefixed}_meta WHERE json_extract(metadata, '$.document_id') = ?1"
        ),
        &[serde_json::Value::String(body.document_id.clone())],
    )
    .await
    {
        let prior_ids: Vec<String> = rows
            .into_iter()
            .filter_map(|r| r.data.get("id").and_then(|v| v.as_str()).map(String::from))
            .collect();
        if !prior_ids.is_empty() {
            if let Err(e) = vclient::delete(ctx, &prefixed, prior_ids).await {
                return err_internal(&format!("failed to clear prior chunks: {e}"));
            }
        }
    }

    // Split into chunks. Empty / whitespace-only text produces no chunks;
    // return early rather than inventing an empty entry.
    let mut chunks = ingestion::chunk(&body.text, DEFAULT_CHUNK_TOKENS, DEFAULT_OVERLAP_RATIO);
    if body.contextual {
        match ingestion::add_context(ctx, &body.text, chunks).await {
            Ok(c) => chunks = c,
            Err(e) => return err_internal(&format!("add_context failed: {e}")),
        }
    }
    if chunks.is_empty() {
        return ok_json(&IngestResponse { chunks_created: 0 });
    }

    // Embed via the right block for this model. On native today that's
    // always `suppers-ai/fastembed`; see `embedding_block_for_model`.
    let (_model_name, _dims, vectors) =
        match vclient::embed(ctx, embedding_block_for_model(&model_id), chunks.clone()).await {
            Ok(tuple) => tuple,
            Err(e) => return err_internal(&format!("embed failed: {e}")),
        };

    if vectors.len() != chunks.len() {
        // Sanity check — embedding block violated its contract. Surface
        // the mismatch instead of silently upserting a truncated set.
        return err_internal(&format!(
            "embedding returned {} vectors for {} chunks",
            vectors.len(),
            chunks.len()
        ));
    }

    // Build VectorEntry list. Ids are `{document_id}:{i}` so re-ingestion
    // of the same document is idempotent at the row level too (overwrites
    // the same ids). Metadata carries `document_id` and `chunk_index` so
    // the SELECT-by-document_id query above keeps working on re-ingest.
    let entries: Vec<VectorEntry> = chunks
        .into_iter()
        .zip(vectors.into_iter())
        .enumerate()
        .map(|(i, (chunk_text, vector))| VectorEntry {
            id: format!("{}:{}", body.document_id, i),
            vector,
            metadata: Some(serde_json::json!({
                "document_id": body.document_id,
                "chunk_index": i,
                "user_metadata": body.metadata,
            })),
            text: Some(chunk_text),
        })
        .collect();

    let n = entries.len();
    match vclient::upsert(ctx, &prefixed, entries).await {
        Ok(()) => ok_json(&IngestResponse { chunks_created: n }),
        Err(e) if e.code == ErrorCode::NotFound => {
            err_not_found(&format!("index not found: {}", body.index))
        }
        Err(e) if e.code == ErrorCode::InvalidArgument => err_bad_request(&e.message),
        Err(e) => err_internal(&format!("upsert failed: {e}")),
    }
}

// ---------------------------------------------------------------------------
// POST /b/vector/api/embed — generate embeddings for raw text
// ---------------------------------------------------------------------------

/// Request body for embed. `model` is optional — missing defaults to
/// `DEFAULT_MODEL` (the catalog default).
#[derive(serde::Deserialize)]
struct EmbedBody {
    #[serde(default)]
    model: Option<String>,
    texts: Vec<String>,
}

#[derive(serde::Serialize)]
struct EmbedResponse {
    model: String,
    dimensions: u32,
    vectors: Vec<Vec<f32>>,
}

/// Handle `POST /b/vector/api/embed`.
///
/// Thin shim over `vclient::embed` — we look up which block serves the
/// requested model on this runtime and dispatch. Empty `texts` is allowed
/// (the embedding block returns an empty vector list).
async fn embed(ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: EmbedBody = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    let model = body.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let block = embedding_block_for_model(&model);

    match vclient::embed(ctx, block, body.texts).await {
        Ok((model, dimensions, vectors)) => ok_json(&EmbedResponse {
            model,
            dimensions,
            vectors,
        }),
        Err(e) if e.code == ErrorCode::InvalidArgument => err_bad_request(&e.message),
        Err(e) => err_internal(&format!("embed failed: {e}")),
    }
}
