pub mod a2a;
pub mod pages;
pub mod rest;
pub mod service;

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub struct MessagesBlock;

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
            "Unified message and context system",
        )
        .instance_mode(InstanceMode::Singleton)
        .requires(vec!["wafer-run/database".into()])
        .collections(vec![
            CollectionSchema::new(service::CONTEXTS_COLLECTION)
                .field("type", "string")
                .field_default("status", "string", "active")
                .field_default("title", "string", "")
                .field_default("sender_id", "string", "")
                .field_default("recipient_id", "string", "")
                .field_optional("parent_id", "string")
                .field_default("metadata", "text", "{}")
                .index(&["type"])
                .index(&["status"])
                .index(&["parent_id"])
                .index(&["updated_at"])
                .index(&["sender_id"]),
            CollectionSchema::new(service::ENTRIES_COLLECTION)
                .field_ref(
                    "context_id",
                    "string",
                    &format!("{}.id", service::CONTEXTS_COLLECTION),
                )
                .field_default("kind", "string", "message")
                .field_default("role", "string", "")
                .field_default("status", "string", "")
                .field_default("sender_id", "string", "")
                .field_default("content", "text", "")
                .field_default("content_type", "string", "text/plain")
                .field_default("metadata", "text", "{}")
                .index(&["context_id"])
                .index(&["context_id", "created_at"])
                .index(&["kind"])
                .index(&["context_id", "kind"]),
        ])
        .category(wafer_run::BlockCategory::Feature)
        .description(
            "Protocol-agnostic context + entry system. Supports chat conversations, \
             A2A task lifecycle, notifications, and future protocols. Contexts are \
             containers (conversations, tasks, channels). Entries are the universal \
             primitive (messages, artifacts, notifications, status changes).",
        )
        .endpoints(vec![
            // Contexts
            BlockEndpoint::get("/b/messages/api/contexts")
                .summary("List contexts")
                .description("List contexts with optional filters by type, status, sender_id, parent_id")
                .auth(AuthLevel::Authenticated)
                .query_params_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "type": {"type": "string", "description": "Filter by context type (conversation, task, notification)"},
                        "status": {"type": "string", "description": "Filter by status"},
                        "sender_id": {"type": "string", "description": "Filter by sender"},
                        "parent_id": {"type": "string", "description": "Filter by parent context"},
                        "page": {"type": "integer", "default": 1},
                        "page_size": {"type": "integer", "default": 20}
                    }
                }))
                .output_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "records": {"type": "array", "items": {"type": "object"}},
                        "total_count": {"type": "integer"}
                    }
                }))
                .tags(&["contexts"]),
            BlockEndpoint::post("/b/messages/api/contexts")
                .summary("Create context")
                .auth(AuthLevel::Authenticated)
                .input_schema(serde_json::json!({
                    "type": "object",
                    "required": ["type"],
                    "properties": {
                        "type": {"type": "string", "description": "Context type: conversation, task, notification, etc."},
                        "title": {"type": "string", "default": ""},
                        "sender_id": {"type": "string", "default": ""},
                        "recipient_id": {"type": "string", "default": ""},
                        "parent_id": {"type": "string", "description": "Parent context ID for sub-tasks/threads"},
                        "metadata": {"type": "object", "default": {}}
                    }
                }))
                .tags(&["contexts"]),
            BlockEndpoint::get("/b/messages/api/contexts/{id}")
                .summary("Get context")
                .auth(AuthLevel::Authenticated)
                .path_params_schema(serde_json::json!({
                    "type": "object",
                    "required": ["id"],
                    "properties": {
                        "id": {"type": "string", "description": "Context ID"}
                    }
                }))
                .tags(&["contexts"]),
            BlockEndpoint::patch("/b/messages/api/contexts/{id}")
                .summary("Update context")
                .auth(AuthLevel::Authenticated)
                .input_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "status": {"type": "string"},
                        "title": {"type": "string"},
                        "metadata": {"type": "object"}
                    }
                }))
                .tags(&["contexts"]),
            BlockEndpoint::delete("/b/messages/api/contexts/{id}")
                .summary("Delete context and its entries")
                .auth(AuthLevel::Authenticated)
                .tags(&["contexts"]),
            // Entries
            BlockEndpoint::get("/b/messages/api/contexts/{id}/entries")
                .summary("List entries in context")
                .auth(AuthLevel::Authenticated)
                .query_params_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "kind": {"type": "string", "description": "Filter by kind (message, artifact, notification, status)"},
                        "role": {"type": "string", "description": "Filter by role (user, agent, system)"},
                        "page": {"type": "integer", "default": 1},
                        "page_size": {"type": "integer", "default": 100}
                    }
                }))
                .tags(&["entries"]),
            BlockEndpoint::post("/b/messages/api/contexts/{id}/entries")
                .summary("Add entry to context")
                .auth(AuthLevel::Authenticated)
                .input_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "kind": {"type": "string", "default": "message", "description": "Entry kind: message, artifact, notification, status"},
                        "role": {"type": "string", "default": "", "description": "Sender role: user, agent, system"},
                        "sender_id": {"type": "string", "default": ""},
                        "content": {"type": "string", "default": ""},
                        "content_type": {"type": "string", "default": "text/plain"},
                        "metadata": {"type": "object", "default": {}}
                    }
                }))
                .tags(&["entries"]),
            BlockEndpoint::get("/b/messages/api/entries/{id}")
                .summary("Get entry")
                .auth(AuthLevel::Authenticated)
                .tags(&["entries"]),
            BlockEndpoint::delete("/b/messages/api/entries/{id}")
                .summary("Delete entry")
                .auth(AuthLevel::Authenticated)
                .tags(&["entries"]),
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
            ("retrieve", "/b/messages/") => pages::context_list_page(ctx, msg).await,
            ("retrieve", _)
                if path.starts_with("/b/messages/contexts/") && !path.contains("/api/") =>
            {
                pages::context_detail_page(ctx, msg).await
            }

            // Context CRUD
            ("retrieve", "/b/messages/api/contexts") => rest::list_contexts(ctx, msg).await,
            ("create", "/b/messages/api/contexts") => rest::create_context(ctx, msg).await,
            ("retrieve", _)
                if path.starts_with("/b/messages/api/contexts/")
                    && !path["/b/messages/api/contexts/".len()..].contains('/') =>
            {
                rest::get_context(ctx, msg).await
            }
            ("update", _)
                if path.starts_with("/b/messages/api/contexts/")
                    && !path["/b/messages/api/contexts/".len()..].contains('/') =>
            {
                rest::update_context(ctx, msg).await
            }
            ("delete", _)
                if path.starts_with("/b/messages/api/contexts/")
                    && !path["/b/messages/api/contexts/".len()..].contains('/') =>
            {
                rest::delete_context(ctx, msg).await
            }

            // Entries within a context
            ("retrieve", _)
                if path.starts_with("/b/messages/api/contexts/")
                    && path.ends_with("/entries") =>
            {
                rest::list_entries(ctx, msg).await
            }
            ("create", _)
                if path.starts_with("/b/messages/api/contexts/")
                    && path.ends_with("/entries") =>
            {
                rest::add_entry(ctx, msg).await
            }

            // Direct entry access
            ("retrieve", _) if path.starts_with("/b/messages/api/entries/") => {
                rest::get_entry(ctx, msg).await
            }
            ("delete", _) if path.starts_with("/b/messages/api/entries/") => {
                rest::delete_entry(ctx, msg).await
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
