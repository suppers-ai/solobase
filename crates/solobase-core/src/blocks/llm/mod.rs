pub mod pages;

use crate::blocks::helpers::{
    self, err_bad_request, err_internal, err_not_found, json_map, ok_json,
};
use wafer_core::clients::config;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions};
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};

pub struct LlmBlock;

pub(crate) const SETTINGS_COLLECTION: &str = "suppers_ai__llm__settings";

const DEFAULT_PROVIDER_VAR: &str = "SUPPERS_AI__LLM__DEFAULT_PROVIDER";
const DEFAULT_MODEL_VAR: &str = "SUPPERS_AI__LLM__DEFAULT_MODEL";
const DEFAULT_PROVIDER: &str = "suppers-ai/provider-llm";

// ---------------------------------------------------------------------------
// Inter-block call helpers
// ---------------------------------------------------------------------------

/// Call the messages block to create an entry in a context.
async fn messages_create(
    ctx: &dyn Context,
    original_msg: &Message,
    context_id: &str,
    role: &str,
    content: &str,
) -> Option<serde_json::Value> {
    let body = serde_json::to_vec(&serde_json::json!({
        "kind": "message",
        "role": role,
        "content": content,
    }))
    .unwrap_or_default();

    let resource = format!("/b/messages/api/contexts/{context_id}/entries");
    let mut msg = Message::new(format!("create:{resource}"));
    msg.set_meta("req.action", "create");
    msg.set_meta("req.resource", &resource);
    msg.set_meta("http.method", "POST");
    msg.set_meta("http.path", &resource);
    msg.set_meta("req.content_type", "application/json");
    // Forward auth from original request
    let user_id = original_msg.user_id().to_string();
    if !user_id.is_empty() {
        msg.set_meta("auth.user_id", &user_id);
    }
    let user_email = original_msg.get_meta("auth.user_email").to_string();
    if !user_email.is_empty() {
        msg.set_meta("auth.user_email", &user_email);
    }
    let user_roles = original_msg.get_meta("auth.user_roles").to_string();
    if !user_roles.is_empty() {
        msg.set_meta("auth.user_roles", &user_roles);
    }

    let out = ctx
        .call_block("suppers-ai/messages", msg, InputStream::from_bytes(body))
        .await;
    if let Ok(buf) = out.collect_buffered().await {
        return serde_json::from_slice::<serde_json::Value>(&buf.body).ok();
    }
    None
}

/// Call the messages block to list entries in a context.
async fn messages_list(
    ctx: &dyn Context,
    original_msg: &Message,
    context_id: &str,
) -> Vec<serde_json::Value> {
    let resource = format!("/b/messages/api/contexts/{context_id}/entries?kind=message");
    let mut msg = Message::new(format!("retrieve:{resource}"));
    msg.set_meta("req.action", "retrieve");
    msg.set_meta("req.resource", &resource);
    msg.set_meta("http.method", "GET");
    msg.set_meta("http.path", &resource);
    let user_id = original_msg.user_id().to_string();
    if !user_id.is_empty() {
        msg.set_meta("auth.user_id", &user_id);
    }
    let user_roles = original_msg.get_meta("auth.user_roles").to_string();
    if !user_roles.is_empty() {
        msg.set_meta("auth.user_roles", &user_roles);
    }

    let out = ctx
        .call_block("suppers-ai/messages", msg, InputStream::empty())
        .await;
    if let Ok(buf) = out.collect_buffered().await {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&buf.body) {
            if let Some(records) = v.get("records").and_then(|r| r.as_array()) {
                return records.clone();
            }
        }
    }
    vec![]
}

/// Call a provider block (provider-llm or local-llm) with a chat request.
async fn provider_chat(
    ctx: &dyn Context,
    provider_block: &str,
    messages: Vec<serde_json::Value>,
    model: &str,
    provider_id: &str,
) -> Result<(String, String), String> {
    let body = serde_json::to_vec(&serde_json::json!({
        "messages": messages,
        "model": model,
        "provider_id": provider_id,
    }))
    .map_err(|e| e.to_string())?;

    let resource = "/b/provider-llm/api/chat";
    let mut msg = Message::new(format!("create:{resource}"));
    msg.set_meta("req.action", "create");
    msg.set_meta("req.resource", resource);
    msg.set_meta("http.method", "POST");
    msg.set_meta("http.path", resource);
    msg.set_meta("req.content_type", "application/json");

    let out = ctx
        .call_block(provider_block, msg, InputStream::from_bytes(body))
        .await;
    match out.collect_buffered().await {
        Ok(buf) => {
            let v: serde_json::Value =
                serde_json::from_slice(&buf.body).map_err(|e| e.to_string())?;
            let content = v
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let actual_model = v
                .get("model")
                .and_then(|m| m.as_str())
                .unwrap_or(model)
                .to_string();
            Ok((content, actual_model))
        }
        Err(e) => Err(format!("Provider error: {:?}", e)),
    }
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

impl LlmBlock {
    // --- Chat ---

    async fn handle_chat(
        &self,
        ctx: &dyn Context,
        msg: &Message,
        input: InputStream,
    ) -> OutputStream {
        #[derive(serde::Deserialize)]
        struct ChatRequest {
            thread_id: String,
            message: String,
            provider: Option<String>,
            model: Option<String>,
        }

        let raw = input.collect_to_bytes().await;
        let body: ChatRequest = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        let thread_id = body.thread_id.clone();

        // 1. Write user message to messages block
        let _ = messages_create(ctx, msg, &thread_id, "user", &body.message).await;

        // 2. Read thread history
        let history = messages_list(ctx, msg, &thread_id).await;

        // Convert stored messages to the format providers expect
        let provider_messages: Vec<serde_json::Value> = history
            .iter()
            .filter_map(|m| {
                let role = m
                    .get("data")
                    .and_then(|d| d.get("role"))
                    .or_else(|| m.get("role"))
                    .and_then(|r| r.as_str())?;
                let content = m
                    .get("data")
                    .and_then(|d| d.get("content"))
                    .or_else(|| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                Some(serde_json::json!({ "role": role, "content": content }))
            })
            .collect();

        // 3. Determine provider and model
        // Priority: per-thread override → request override → default config
        let (provider_block, model) =
            self.resolve_provider(ctx, &thread_id, body.provider.as_deref(), body.model.as_deref())
                .await;

        // Need a default provider_id — look up first enabled provider from provider-llm
        let provider_id = self.get_default_provider_id(ctx).await;

        // 4. Call provider
        let (content, actual_model) =
            match provider_chat(ctx, &provider_block, provider_messages, &model, &provider_id)
                .await
            {
                Ok(r) => r,
                Err(e) => return err_internal(&format!("Provider error: {e}")),
            };

        // 5. Write assistant response to messages block
        let saved = messages_create(ctx, msg, &thread_id, "assistant", &content).await;
        let message_id = saved
            .as_ref()
            .and_then(|v| {
                v.get("id")
                    .or_else(|| v.get("data").and_then(|d| d.get("id")))
            })
            .and_then(|id| id.as_str())
            .unwrap_or("")
            .to_string();

        // 6. Return response
        ok_json(&serde_json::json!({
            "content": content,
            "message_id": message_id,
            "model": actual_model,
        }))
    }

    /// Resolve which provider block and model to use for a request.
    async fn resolve_provider(
        &self,
        ctx: &dyn Context,
        thread_id: &str,
        req_provider: Option<&str>,
        req_model: Option<&str>,
    ) -> (String, String) {
        // Check per-thread override first
        let thread_setting = self.get_thread_setting(ctx, thread_id).await;

        let provider_block = thread_setting
            .as_ref()
            .and_then(|s| s.get("provider_block").and_then(|v| v.as_str()))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| req_provider.map(|s| s.to_string()))
            .unwrap_or_else(|| {
                // Will be filled below from config
                String::new()
            });

        let model = thread_setting
            .as_ref()
            .and_then(|s| s.get("model").and_then(|v| v.as_str()))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| req_model.map(|s| s.to_string()))
            .unwrap_or_default();

        let default_provider = config::get_default(ctx, DEFAULT_PROVIDER_VAR, DEFAULT_PROVIDER).await;
        let default_model = config::get_default(ctx, DEFAULT_MODEL_VAR, "").await;

        let final_provider = if provider_block.is_empty() {
            default_provider
        } else {
            provider_block
        };

        let final_model = if model.is_empty() {
            default_model
        } else {
            model
        };

        (final_provider, final_model)
    }

    /// Get per-thread settings record from DB, if any.
    async fn get_thread_setting(
        &self,
        ctx: &dyn Context,
        thread_id: &str,
    ) -> Option<std::collections::HashMap<String, serde_json::Value>> {
        let opts = ListOptions {
            filters: vec![Filter {
                field: "thread_id".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(thread_id.to_string()),
            }],
            limit: 1,
            ..Default::default()
        };
        let result = db::list(ctx, SETTINGS_COLLECTION, &opts).await.ok()?;
        let record = result.records.into_iter().next()?;
        Some(record.data)
    }

    /// Get the first enabled provider ID from the provider-llm block DB.
    async fn get_default_provider_id(&self, ctx: &dyn Context) -> String {
        use wafer_core::clients::database::{Filter, FilterOp, ListOptions};

        const PROVIDERS_COLLECTION: &str = "suppers_ai__provider_llm__providers";
        let opts = ListOptions {
            filters: vec![Filter {
                field: "enabled".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::json!(1),
            }],
            limit: 1,
            ..Default::default()
        };
        match db::list(ctx, PROVIDERS_COLLECTION, &opts).await {
            Ok(r) => r.records.into_iter().next().map(|rec| rec.id).unwrap_or_default(),
            Err(_) => String::new(),
        }
    }

    // --- Config ---

    async fn handle_get_config(&self, ctx: &dyn Context) -> OutputStream {
        let default_provider =
            config::get_default(ctx, DEFAULT_PROVIDER_VAR, DEFAULT_PROVIDER).await;
        let default_model = config::get_default(ctx, DEFAULT_MODEL_VAR, "").await;
        ok_json(&serde_json::json!({
            "default_provider": default_provider,
            "default_model": default_model,
        }))
    }

    async fn handle_post_config(
        &self,
        ctx: &dyn Context,
        input: InputStream,
    ) -> OutputStream {
        #[derive(serde::Deserialize)]
        struct ConfigUpdate {
            thread_id: Option<String>,
            default_provider: Option<String>,
            default_model: Option<String>,
            provider_block: Option<String>,
            model: Option<String>,
        }

        let raw = input.collect_to_bytes().await;
        let body: ConfigUpdate = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        // Per-thread override update
        if let Some(thread_id) = body.thread_id {
            let existing = self.get_thread_setting(ctx, &thread_id).await;

            if let Some(mut data) = existing {
                // Update existing record — find record ID
                let opts = ListOptions {
                    filters: vec![Filter {
                        field: "thread_id".to_string(),
                        operator: FilterOp::Equal,
                        value: serde_json::Value::String(thread_id.clone()),
                    }],
                    limit: 1,
                    ..Default::default()
                };
                let result = match db::list(ctx, SETTINGS_COLLECTION, &opts).await {
                    Ok(r) => r,
                    Err(e) => return err_internal(&format!("Database error: {e}")),
                };
                if let Some(record) = result.records.into_iter().next() {
                    if let Some(pb) = body.provider_block {
                        data.insert("provider_block".to_string(), serde_json::json!(pb));
                    }
                    if let Some(m) = body.model {
                        data.insert("model".to_string(), serde_json::json!(m));
                    }
                    helpers::stamp_updated(&mut data);
                    match db::update(ctx, SETTINGS_COLLECTION, &record.id, data).await {
                        Ok(r) => return ok_json(&r),
                        Err(e) => return err_internal(&format!("Database error: {e}")),
                    }
                }
            } else {
                // Create new per-thread setting
                let mut data = json_map(serde_json::json!({
                    "thread_id": thread_id,
                    "provider_block": body.provider_block.unwrap_or_default(),
                    "model": body.model.unwrap_or_default(),
                }));
                helpers::stamp_created(&mut data);
                match db::create(ctx, SETTINGS_COLLECTION, data).await {
                    Ok(r) => return ok_json(&r),
                    Err(e) => return err_internal(&format!("Database error: {e}")),
                }
            }
        }

        // Global default update would go via the config system (admin only),
        // but here we just acknowledge since config writes go through wafer-run/config.
        if body.default_provider.is_some() || body.default_model.is_some() {
            return err_bad_request(
                "Global default provider/model must be set via environment variables: SUPPERS_AI__LLM__DEFAULT_PROVIDER and SUPPERS_AI__LLM__DEFAULT_MODEL",
            );
        }

        ok_json(&serde_json::json!({"updated": true}))
    }

    // --- Providers / Models aggregation ---

    async fn handle_list_providers(&self, ctx: &dyn Context, msg: &Message) -> OutputStream {
        // Aggregate from provider-llm block
        let resource = "/b/provider-llm/api/providers";
        let mut call_msg = Message::new(format!("retrieve:{resource}"));
        call_msg.set_meta("req.action", "retrieve");
        call_msg.set_meta("req.resource", resource);
        call_msg.set_meta("http.method", "GET");
        call_msg.set_meta("http.path", resource);
        // Forward auth
        let user_id = msg.user_id().to_string();
        if !user_id.is_empty() {
            call_msg.set_meta("auth.user_id", &user_id);
        }
        let roles = msg.get_meta("auth.user_roles").to_string();
        if !roles.is_empty() {
            call_msg.set_meta("auth.user_roles", &roles);
        }

        let out = ctx
            .call_block("suppers-ai/provider-llm", call_msg, InputStream::empty())
            .await;
        if let Ok(buf) = out.collect_buffered().await {
            if let Ok(providers) = serde_json::from_slice::<serde_json::Value>(&buf.body) {
                return ok_json(&serde_json::json!({ "providers": providers }));
            }
        }
        ok_json(&serde_json::json!({ "providers": [] }))
    }

    async fn handle_list_models(&self, ctx: &dyn Context) -> OutputStream {
        let resource = "/b/provider-llm/api/models";
        let mut call_msg = Message::new(format!("retrieve:{resource}"));
        call_msg.set_meta("req.action", "retrieve");
        call_msg.set_meta("req.resource", resource);
        call_msg.set_meta("http.method", "GET");
        call_msg.set_meta("http.path", resource);

        let out = ctx
            .call_block("suppers-ai/provider-llm", call_msg, InputStream::empty())
            .await;
        if let Ok(buf) = out.collect_buffered().await {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&buf.body) {
                return ok_json(&v);
            }
        }
        ok_json(&serde_json::json!({ "models": [] }))
    }
}

// ---------------------------------------------------------------------------
// Block trait implementation
// ---------------------------------------------------------------------------

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for LlmBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::types::CollectionSchema;
        use wafer_run::AuthLevel;

        BlockInfo::new(
            "suppers-ai/llm",
            "0.0.1",
            "http-handler@v1",
            "LLM orchestrator — routes to provider or local backends",
        )
        .instance_mode(InstanceMode::Singleton)
        .requires(vec![
            "suppers-ai/messages".into(),
            "suppers-ai/provider-llm".into(),
            "wafer-run/database".into(),
            "wafer-run/config".into(),
        ])
        .collections(vec![CollectionSchema::new(SETTINGS_COLLECTION)
            .field("thread_id", "string")
            .field_default("provider_block", "string", "")
            .field_default("model", "string", "")
            .index(&["thread_id"])])
        .category(wafer_run::BlockCategory::Feature)
        .description(
            "LLM orchestrator. Routes chat requests to provider-llm or local-llm backends, \
             manages thread history via the messages block, and provides the main chat UI.",
        )
        .endpoints(vec![
            BlockEndpoint::post("/b/llm/api/chat")
                .summary("Send a chat message")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/llm/api/providers")
                .summary("List available LLM providers")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/llm/api/models")
                .summary("List available models")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/llm/api/config")
                .summary("Get default provider/model config")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/llm/api/config")
                .summary("Update per-thread provider/model override")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/llm/")
                .summary("Chat UI")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/llm/settings")
                .summary("LLM settings page")
                .auth(AuthLevel::Admin),
        ])
        .config_keys(vec![
            ConfigVar::new(
                DEFAULT_PROVIDER_VAR,
                "Default LLM provider block (suppers-ai/provider-llm or suppers-ai/local-llm)",
                DEFAULT_PROVIDER,
            )
            .name("Default Provider"),
            ConfigVar::new(DEFAULT_MODEL_VAR, "Default model to use (empty = provider default)", "")
                .name("Default Model"),
        ])
        .can_disable(true)
        .default_enabled(true)
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![wafer_run::UiRoute::authenticated("/")]
    }

    async fn handle(
        &self,
        ctx: &dyn Context,
        msg: Message,
        input: InputStream,
    ) -> OutputStream {
        let action = msg.action();
        let path = msg.path();
        let is_api = path.contains("/api/");
        let user_id = msg.user_id().to_string();

        // All endpoints require authentication
        if user_id.is_empty() {
            return crate::ui::forbidden_response(&msg);
        }

        // UI pages require admin role
        if !is_api {
            let is_admin = helpers::is_admin(&msg);
            if !is_admin {
                return crate::ui::forbidden_response(&msg);
            }
        }

        match (action, path) {
            // UI pages
            ("retrieve", "/b/llm/") | ("retrieve", "/b/llm") => {
                pages::chat_page(ctx, &msg).await
            }
            ("retrieve", _) if path.starts_with("/b/llm/threads/") => {
                pages::thread_page(ctx, &msg).await
            }
            ("retrieve", "/b/llm/settings") => pages::settings_page(ctx, &msg).await,

            // Chat API
            ("create", "/b/llm/api/chat") => self.handle_chat(ctx, &msg, input).await,

            // Aggregation
            ("retrieve", "/b/llm/api/providers") => self.handle_list_providers(ctx, &msg).await,
            ("retrieve", "/b/llm/api/models") => self.handle_list_models(ctx).await,

            // Config
            ("retrieve", "/b/llm/api/config") => self.handle_get_config(ctx).await,
            ("create", "/b/llm/api/config") => self.handle_post_config(ctx, input).await,

            _ => err_not_found("not found"),
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
