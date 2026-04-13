pub mod pages;
pub mod service;

use crate::blocks::helpers::{self, json_map};
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub struct MessagesBlock;

pub(crate) const THREADS_COLLECTION: &str = "suppers_ai__messages__threads";
pub(crate) const MESSAGES_COLLECTION: &str = "suppers_ai__messages__messages";

// ---------------------------------------------------------------------------
// Path extraction helpers
// ---------------------------------------------------------------------------

/// Extract thread ID from paths like:
/// - `/b/messages/api/threads/{id}`
/// - `/b/messages/api/threads/{id}/messages`
fn extract_thread_id(msg: &Message) -> &str {
    let var = msg.var("id");
    if !var.is_empty() {
        return var;
    }
    let path = msg.path();
    let suffix = path
        .strip_prefix("/b/messages/api/threads/")
        .unwrap_or("");
    suffix.split('/').next().unwrap_or("")
}

/// Extract message ID from paths like `/b/messages/api/messages/{id}`.
fn extract_message_id(msg: &Message) -> &str {
    let var = msg.var("id");
    if !var.is_empty() {
        return var;
    }
    let path = msg.path();
    let suffix = path
        .strip_prefix("/b/messages/api/messages/")
        .unwrap_or("");
    suffix.split('/').next().unwrap_or("")
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

impl MessagesBlock {
    // --- Thread handlers ---

    async fn list_threads(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let (_, page_size, offset) = msg.pagination_params(20);
        let opts = ListOptions {
            sort: vec![SortField {
                field: "updated_at".to_string(),
                desc: true,
            }],
            limit: page_size as i64,
            offset: offset as i64,
            ..Default::default()
        };
        match db::list(ctx, THREADS_COLLECTION, &opts).await {
            Ok(result) => json_respond(msg, &result),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn create_thread(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        #[derive(serde::Deserialize)]
        struct CreateThread {
            title: String,
            metadata: Option<serde_json::Value>,
        }
        let body: CreateThread = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };
        let metadata = body
            .metadata
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

        let mut data = json_map(serde_json::json!({
            "title": body.title,
            "metadata": metadata,
        }));
        helpers::stamp_created(&mut data);

        match db::create(ctx, THREADS_COLLECTION, data).await {
            Ok(record) => json_respond(msg, &record),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn get_thread(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let id = extract_thread_id(msg);
        if id.is_empty() {
            return err_bad_request(msg, "Missing thread ID");
        }
        match db::get(ctx, THREADS_COLLECTION, id).await {
            Ok(record) => json_respond(msg, &record),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Thread not found"),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn update_thread(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let id = extract_thread_id(msg);
        if id.is_empty() {
            return err_bad_request(msg, "Missing thread ID");
        }
        let body: std::collections::HashMap<String, serde_json::Value> = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };
        // Only allow title and metadata fields
        let mut data = std::collections::HashMap::new();
        if let Some(v) = body.get("title") {
            data.insert("title".to_string(), v.clone());
        }
        if let Some(v) = body.get("metadata") {
            data.insert("metadata".to_string(), v.clone());
        }
        helpers::stamp_updated(&mut data);

        match db::update(ctx, THREADS_COLLECTION, id, data).await {
            Ok(record) => json_respond(msg, &record),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Thread not found"),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn delete_thread(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let id = extract_thread_id(msg);
        if id.is_empty() {
            return err_bad_request(msg, "Missing thread ID");
        }
        // Cascade delete messages in this thread first
        let filters = vec![Filter {
            field: "thread_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(id.to_string()),
        }];
        if let Err(e) = db::delete_by_filters(ctx, MESSAGES_COLLECTION, filters).await {
            tracing::warn!("Failed to cascade delete messages for thread {id}: {e}");
        }

        match db::delete(ctx, THREADS_COLLECTION, id).await {
            Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Thread not found"),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    // --- Message handlers ---

    async fn list_messages(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let thread_id = extract_thread_id(msg);
        if thread_id.is_empty() {
            return err_bad_request(msg, "Missing thread ID");
        }
        let (_, page_size, offset) = msg.pagination_params(100);
        let opts = ListOptions {
            filters: vec![Filter {
                field: "thread_id".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(thread_id.to_string()),
            }],
            sort: vec![SortField {
                field: "created_at".to_string(),
                desc: false,
            }],
            limit: page_size as i64,
            offset: offset as i64,
        };
        match db::list(ctx, MESSAGES_COLLECTION, &opts).await {
            Ok(result) => json_respond(msg, &result),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn create_message(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let thread_id = extract_thread_id(msg);
        if thread_id.is_empty() {
            return err_bad_request(msg, "Missing thread ID");
        }

        #[derive(serde::Deserialize)]
        struct CreateMessage {
            role: String,
            content: Option<String>,
            metadata: Option<serde_json::Value>,
        }
        let body: CreateMessage = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        let metadata = body
            .metadata
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
        let content = body.content.unwrap_or_default();

        let now = helpers::now_rfc3339();
        let data = json_map(serde_json::json!({
            "thread_id": thread_id,
            "role": body.role,
            "content": content,
            "metadata": metadata,
            "created_at": now,
            "updated_at": now,
        }));

        let record = match db::create(ctx, MESSAGES_COLLECTION, data).await {
            Ok(r) => r,
            Err(e) => return err_internal(msg, &format!("Database error: {e}")),
        };

        // Bump thread updated_at so list sorts correctly
        let mut thread_update = std::collections::HashMap::new();
        helpers::stamp_updated(&mut thread_update);
        if let Err(e) = db::update(ctx, THREADS_COLLECTION, thread_id, thread_update).await {
            tracing::warn!("Failed to update thread updated_at after message create: {e}");
        }

        json_respond(msg, &record)
    }

    async fn get_message(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let id = extract_message_id(msg);
        if id.is_empty() {
            return err_bad_request(msg, "Missing message ID");
        }
        match db::get(ctx, MESSAGES_COLLECTION, id).await {
            Ok(record) => json_respond(msg, &record),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Message not found"),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn update_message(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let id = extract_message_id(msg);
        if id.is_empty() {
            return err_bad_request(msg, "Missing message ID");
        }
        let body: std::collections::HashMap<String, serde_json::Value> = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };
        // Only allow content and metadata fields
        let mut data = std::collections::HashMap::new();
        if let Some(v) = body.get("content") {
            data.insert("content".to_string(), v.clone());
        }
        if let Some(v) = body.get("metadata") {
            data.insert("metadata".to_string(), v.clone());
        }
        helpers::stamp_updated(&mut data);

        match db::update(ctx, MESSAGES_COLLECTION, id, data).await {
            Ok(record) => json_respond(msg, &record),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Message not found"),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn delete_message(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let id = extract_message_id(msg);
        if id.is_empty() {
            return err_bad_request(msg, "Missing message ID");
        }
        match db::delete(ctx, MESSAGES_COLLECTION, id).await {
            Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Message not found"),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Block trait implementation
// ---------------------------------------------------------------------------

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for MessagesBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::types::CollectionSchema;
        use wafer_run::AuthLevel;

        BlockInfo::new(
            "suppers-ai/messages",
            "0.0.1",
            "http-handler@v1",
            "Generic message threads and messages",
        )
        .instance_mode(InstanceMode::Singleton)
        .requires(vec!["wafer-run/database".into()])
        .collections(vec![
            CollectionSchema::new(THREADS_COLLECTION)
                .field("title", "string")
                .field_default("metadata", "text", "{}")
                .index(&["updated_at"]),
            CollectionSchema::new(MESSAGES_COLLECTION)
                .field_ref(
                    "thread_id",
                    "string",
                    &format!("{}.id", THREADS_COLLECTION),
                )
                .field("role", "string")
                .field_default("content", "text", "")
                .field_default("metadata", "text", "{}")
                .index(&["thread_id"])
                .index(&["thread_id", "created_at"]),
        ])
        .category(wafer_run::BlockCategory::Feature)
        .description(
            "Generic message thread system. Stores threads (conversations) and messages \
             within threads. Each message has a role (user, assistant, system, etc.) and \
             content. Designed for extensibility via metadata fields.",
        )
        .endpoints(vec![
            // Threads
            BlockEndpoint::get("/b/messages/api/threads")
                .summary("List threads")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/messages/api/threads")
                .summary("Create thread")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/messages/api/threads/{id}")
                .summary("Get thread")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::patch("/b/messages/api/threads/{id}")
                .summary("Update thread")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::delete("/b/messages/api/threads/{id}")
                .summary("Delete thread and its messages")
                .auth(AuthLevel::Authenticated),
            // Messages
            BlockEndpoint::get("/b/messages/api/threads/{id}/messages")
                .summary("List messages in thread")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/messages/api/threads/{id}/messages")
                .summary("Create message in thread")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/messages/api/messages/{id}")
                .summary("Get message")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::patch("/b/messages/api/messages/{id}")
                .summary("Update message")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::delete("/b/messages/api/messages/{id}")
                .summary("Delete message")
                .auth(AuthLevel::Authenticated),
        ])
        .can_disable(true)
        .default_enabled(true)
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![wafer_run::UiRoute::authenticated("/")]
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let action = msg.action();
        let path = msg.path();
        let is_api = path.contains("/api/");
        let user_id = msg.user_id().to_string();

        // All endpoints require authentication
        if user_id.is_empty() {
            return crate::ui::forbidden_response(msg);
        }

        // UI pages require admin role
        if !is_api {
            let is_admin = msg
                .get_meta("auth.user_roles")
                .split(',')
                .any(|r| r.trim() == "admin");
            if !is_admin {
                return crate::ui::forbidden_response(msg);
            }
        }

        match (action, path) {
            // UI pages
            ("retrieve", "/b/messages/") => pages::thread_list_page(ctx, msg).await,
            ("retrieve", _)
                if path.starts_with("/b/messages/threads/") && !path.contains("/api/") =>
            {
                pages::thread_view_page(ctx, msg).await
            }

            // Threads CRUD
            ("retrieve", "/b/messages/api/threads") => self.list_threads(ctx, msg).await,
            ("create", "/b/messages/api/threads") => self.create_thread(ctx, msg).await,
            ("retrieve", _) if path.starts_with("/b/messages/api/threads/") && !path["/b/messages/api/threads/".len()..].contains('/') => {
                self.get_thread(ctx, msg).await
            }
            ("update", _) if path.starts_with("/b/messages/api/threads/") && !path["/b/messages/api/threads/".len()..].contains('/') => {
                self.update_thread(ctx, msg).await
            }
            ("delete", _) if path.starts_with("/b/messages/api/threads/") && !path["/b/messages/api/threads/".len()..].contains('/') => {
                self.delete_thread(ctx, msg).await
            }

            // Messages within a thread
            ("retrieve", _) if path.starts_with("/b/messages/api/threads/") && path.ends_with("/messages") => {
                self.list_messages(ctx, msg).await
            }
            ("create", _) if path.starts_with("/b/messages/api/threads/") && path.ends_with("/messages") => {
                self.create_message(ctx, msg).await
            }

            // Direct message access
            ("retrieve", _) if path.starts_with("/b/messages/api/messages/") => {
                self.get_message(ctx, msg).await
            }
            ("update", _) if path.starts_with("/b/messages/api/messages/") => {
                self.update_message(ctx, msg).await
            }
            ("delete", _) if path.starts_with("/b/messages/api/messages/") => {
                self.delete_message(ctx, msg).await
            }

            _ => err_not_found(msg, "not found"),
        }
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
