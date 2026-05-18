pub mod ingestion;
mod migrations;
pub mod pages;
pub mod pages_ui;
pub mod service;

use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use crate::blocks::helpers;

pub struct VectorBlock;

impl VectorBlock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VectorBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for VectorBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new(
            "suppers-ai/vector",
            "0.0.1",
            "http-handler@v1",
            "Vector search, RAG ingestion, and embedding generation",
        )
        .instance_mode(InstanceMode::Singleton)
        .requires(vec!["wafer-run/vector".into()])
        .category(wafer_run::BlockCategory::Feature)
        .endpoints(vec![
            BlockEndpoint::get("/b/vector/")
                .summary("Vector indexes admin list")
                .auth(AuthLevel::Admin),
            BlockEndpoint::get("/b/vector/{name}/")
                .summary("Vector index detail")
                .auth(AuthLevel::Admin),
            BlockEndpoint::post("/b/vector/api/indexes")
                .summary("Create a vector index")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/vector/api/indexes")
                .summary("List indexes")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::delete("/b/vector/api/indexes/{name}")
                .summary("Delete an index")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/vector/api/upsert")
                .summary("Upsert pre-computed vectors")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/vector/api/query")
                .summary("Search vectors")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/vector/api/ingest")
                .summary("Chunk + embed + upsert a document")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/vector/api/embed")
                .summary("Generate embeddings for raw text")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::delete("/b/vector/api/{index}/{id}")
                .summary("Delete a single vector")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/vector/api/stats")
                .summary("Index stats and usage")
                .auth(AuthLevel::Authenticated),
        ])
        .can_disable(true)
        .default_enabled(true)
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::admin("/"),
            wafer_run::UiRoute::admin("/{name}/"),
        ]
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        // All endpoints require authentication. Task 15 fills in the indexes
        // CRUD routes; remaining routes (upsert, query, ingest, embed, stats,
        // delete-vector) still resolve to `Unimplemented` in `pages::route`.
        let user_id = msg.user_id().to_string();
        if user_id.is_empty() {
            return helpers::err_unauthorized("authentication required");
        }

        // UI pages — admin-only. The JSON dispatch in `pages::route` handles
        // all `/b/vector/api/...` paths; UI routes live alongside on the
        // bare base path and on `/b/vector/{name}/`.
        let action = msg.action();
        let path = msg.path();
        if action == "retrieve" {
            let is_ui = path == "/b/vector/"
                || path == "/b/vector"
                || (path.starts_with("/b/vector/")
                    && !path.starts_with("/b/vector/api/")
                    && path != "/b/vector/api"
                    && path != "/b/vector/api/");
            if is_ui {
                if !helpers::is_admin(&msg) {
                    return crate::ui::forbidden_response(&msg);
                }
                if path == "/b/vector/" || path == "/b/vector" {
                    return pages_ui::index_list_page(ctx, &msg).await;
                }
                // /b/vector/{name}[/...] → detail page. Strip the prefix and
                // any trailing slashes; reject empty segments to avoid
                // routing `/b/vector//` or similar.
                let rest = path.trim_start_matches("/b/vector/").trim_matches('/');
                if rest.is_empty() || rest.contains('/') {
                    return crate::blocks::helpers::err_not_found("not found");
                }
                return pages_ui::index_detail_page(ctx, &msg, rest).await;
            }
        }

        pages::route(ctx, &msg, input).await
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            migrations::apply(ctx).await.map_err(|e| {
                WaferError::new(
                    wafer_run::ErrorCode::Internal,
                    format!("vector migrations: {e}"),
                )
            })?;
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
::wafer_run::register_static_block!("suppers-ai/vector", VectorBlock);
