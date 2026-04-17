pub mod ingestion;
pub mod pages;
pub mod service;

use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use crate::blocks::helpers;

pub struct VectorBlock;

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

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        // All endpoints require authentication. Task 15 fills in the indexes
        // CRUD routes; remaining routes (upsert, query, ingest, embed, stats,
        // delete-vector) still resolve to `Unimplemented` in `pages::route`.
        let user_id = msg.user_id().to_string();
        if user_id.is_empty() {
            return helpers::err_unauthorized("authentication required");
        }

        pages::route(ctx, &msg, input).await
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
