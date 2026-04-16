mod pages;

use std::collections::HashMap;

use wafer_core::clients::{
    config, database as db,
    database::{ListOptions, SortField},
    network,
};
use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use crate::blocks::helpers::{
    self, err_bad_request, err_internal, err_not_found, json_map, ok_json,
};

pub struct ProviderLlmBlock;

pub(crate) const PROVIDERS_COLLECTION: &str = "suppers_ai__provider_llm__providers";

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn extract_provider_id(msg: &Message) -> &str {
    let var = msg.var("id");
    if !var.is_empty() {
        return var;
    }
    let path = msg.path();
    let suffix = path
        .strip_prefix("/b/provider-llm/api/providers/")
        .unwrap_or("");
    suffix.split('/').next().unwrap_or("")
}

// ---------------------------------------------------------------------------
// Chat
// ---------------------------------------------------------------------------

impl ProviderLlmBlock {
    async fn handle_chat(&self, ctx: &dyn Context, input: InputStream) -> OutputStream {
        #[derive(serde::Deserialize)]
        struct ChatRequest {
            messages: Vec<serde_json::Value>,
            model: String,
            provider_id: String,
        }

        let raw = input.collect_to_bytes().await;
        let body: ChatRequest = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        // Load provider record
        let provider = match db::get(ctx, PROVIDERS_COLLECTION, &body.provider_id).await {
            Ok(r) => r,
            Err(e) if e.code == ErrorCode::NotFound => return err_not_found("Provider not found"),
            Err(e) => return err_internal(&format!("Database error: {e}")),
        };

        let provider_type = provider
            .data
            .get("provider_type")
            .and_then(|v| v.as_str())
            .unwrap_or("openai");
        let endpoint = provider
            .data
            .get("endpoint")
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.openai.com/v1");

        let (api_url, req_body, api_key_var) = match provider_type {
            "anthropic" => {
                let url = format!("{}/messages", endpoint);
                let rb = serde_json::json!({
                    "model": body.model,
                    "messages": body.messages,
                    "max_tokens": 4096,
                });
                (url, rb, "SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY")
            }
            _ => {
                // openai-compatible
                let url = format!("{}/chat/completions", endpoint);
                let rb = serde_json::json!({
                    "model": body.model,
                    "messages": body.messages,
                });
                (url, rb, "SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY")
            }
        };

        let api_key = config::get_default(ctx, api_key_var, "").await;
        if api_key.is_empty() {
            return err_bad_request(&format!(
                "API key not configured. Set {api_key_var} in config."
            ));
        }

        let req_bytes = match serde_json::to_vec(&req_body) {
            Ok(b) => b,
            Err(e) => return err_internal(&format!("Failed to serialize request: {e}")),
        };

        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        if provider_type == "anthropic" {
            headers.insert("x-api-key".to_string(), api_key);
            headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        } else {
            headers.insert("Authorization".to_string(), format!("Bearer {api_key}"));
        }

        let resp =
            match network::do_request(ctx, "POST", &api_url, &headers, Some(&req_bytes)).await {
                Ok(r) => r,
                Err(e) => return err_internal(&format!("Network error: {e}")),
            };

        if resp.status_code < 200 || resp.status_code >= 300 {
            let body_str = String::from_utf8_lossy(&resp.body);
            return err_internal(&format!("LLM API error {}: {}", resp.status_code, body_str));
        }

        let resp_json: serde_json::Value = match serde_json::from_slice(&resp.body) {
            Ok(v) => v,
            Err(e) => return err_internal(&format!("Failed to parse LLM response: {e}")),
        };

        let (content, input_tokens, output_tokens) = if provider_type == "anthropic" {
            let text = resp_json
                .pointer("/content/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let input_t = resp_json
                .pointer("/usage/input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let output_t = resp_json
                .pointer("/usage/output_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            (text, input_t, output_t)
        } else {
            let text = resp_json
                .pointer("/choices/0/message/content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let input_t = resp_json
                .pointer("/usage/prompt_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let output_t = resp_json
                .pointer("/usage/completion_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            (text, input_t, output_t)
        };

        ok_json(&serde_json::json!({
            "content": content,
            "model": body.model,
            "usage": {
                "input_tokens": input_tokens,
                "output_tokens": output_tokens,
            }
        }))
    }

    // ---------------------------------------------------------------------------
    // Provider CRUD
    // ---------------------------------------------------------------------------

    async fn handle_list_providers(&self, ctx: &dyn Context, msg: &Message) -> OutputStream {
        let (_, page_size, offset) = msg.pagination_params(50);
        let opts = ListOptions {
            sort: vec![SortField {
                field: "name".to_string(),
                desc: false,
            }],
            limit: page_size as i64,
            offset: offset as i64,
            ..Default::default()
        };
        match db::list(ctx, PROVIDERS_COLLECTION, &opts).await {
            Ok(result) => ok_json(&result),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    async fn handle_create_provider(&self, ctx: &dyn Context, input: InputStream) -> OutputStream {
        #[derive(serde::Deserialize)]
        struct CreateProvider {
            name: String,
            provider_type: String,
            endpoint: Option<String>,
            models: Option<serde_json::Value>,
            enabled: Option<i64>,
        }

        let raw = input.collect_to_bytes().await;
        let body: CreateProvider = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        let default_endpoint = match body.provider_type.as_str() {
            "anthropic" => "https://api.anthropic.com/v1",
            _ => "https://api.openai.com/v1",
        };

        let models_str = match &body.models {
            Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()),
            None => "[]".to_string(),
        };

        let mut data = json_map(serde_json::json!({
            "name": body.name,
            "provider_type": body.provider_type,
            "endpoint": body.endpoint.unwrap_or_else(|| default_endpoint.to_string()),
            "models": models_str,
            "enabled": body.enabled.unwrap_or(1),
        }));
        helpers::stamp_created(&mut data);

        match db::create(ctx, PROVIDERS_COLLECTION, data).await {
            Ok(record) => ok_json(&record),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    async fn handle_update_provider(
        &self,
        ctx: &dyn Context,
        msg: &Message,
        input: InputStream,
    ) -> OutputStream {
        let id = extract_provider_id(msg).to_string();
        if id.is_empty() {
            return err_bad_request("Missing provider ID");
        }

        let raw = input.collect_to_bytes().await;
        let body: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        let mut data = body;
        helpers::stamp_updated(&mut data);

        match db::update(ctx, PROVIDERS_COLLECTION, &id, data).await {
            Ok(record) => ok_json(&record),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found("Provider not found"),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    async fn handle_delete_provider(&self, ctx: &dyn Context, msg: &Message) -> OutputStream {
        let id = extract_provider_id(msg).to_string();
        if id.is_empty() {
            return err_bad_request("Missing provider ID");
        }
        match db::delete(ctx, PROVIDERS_COLLECTION, &id).await {
            Ok(()) => ok_json(&serde_json::json!({"deleted": true})),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found("Provider not found"),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    // ---------------------------------------------------------------------------
    // Aggregate models list
    // ---------------------------------------------------------------------------

    async fn handle_list_models(&self, ctx: &dyn Context) -> OutputStream {
        use wafer_core::clients::database::{Filter, FilterOp};

        let opts = ListOptions {
            filters: vec![Filter {
                field: "enabled".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::json!(1),
            }],
            sort: vec![SortField {
                field: "name".to_string(),
                desc: false,
            }],
            limit: 200,
            ..Default::default()
        };

        let providers = match db::list(ctx, PROVIDERS_COLLECTION, &opts).await {
            Ok(r) => r,
            Err(e) => return err_internal(&format!("Database error: {e}")),
        };

        let mut models: Vec<serde_json::Value> = Vec::new();
        for provider in &providers.records {
            let provider_id = &provider.id;
            let provider_name = provider
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let provider_type = provider
                .data
                .get("provider_type")
                .and_then(|v| v.as_str())
                .unwrap_or("openai");
            let models_json = provider
                .data
                .get("models")
                .and_then(|v| v.as_str())
                .unwrap_or("[]");

            let model_ids: Vec<String> = serde_json::from_str(models_json).unwrap_or_default();

            for model_id in model_ids {
                models.push(serde_json::json!({
                    "id": model_id,
                    "provider_id": provider_id,
                    "provider_name": provider_name,
                    "provider_type": provider_type,
                }));
            }
        }

        ok_json(&serde_json::json!({ "models": models }))
    }

    // ---------------------------------------------------------------------------
    // Lifecycle: seed default providers
    // ---------------------------------------------------------------------------

    async fn seed_defaults(&self, ctx: &dyn Context) {
        let count = db::count(ctx, PROVIDERS_COLLECTION, &[]).await.unwrap_or(0);
        if count > 0 {
            return;
        }

        let now = helpers::now_rfc3339();
        let defaults: &[(&str, &str, &str, &str)] = &[
            (
                "OpenAI",
                "openai",
                "https://api.openai.com/v1",
                r#"["gpt-4o","gpt-4o-mini"]"#,
            ),
            (
                "Anthropic",
                "anthropic",
                "https://api.anthropic.com/v1",
                r#"["claude-sonnet-4-20250514","claude-haiku-4-5-20251001"]"#,
            ),
        ];

        for (name, provider_type, endpoint, models) in defaults {
            let data = json_map(serde_json::json!({
                "name": name,
                "provider_type": provider_type,
                "endpoint": endpoint,
                "models": models,
                "enabled": 1,
                "created_at": now,
                "updated_at": now,
            }));
            if let Err(e) = db::create(ctx, PROVIDERS_COLLECTION, data).await {
                tracing::warn!("Failed to seed default provider '{name}': {e}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Block trait implementation
// ---------------------------------------------------------------------------

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for ProviderLlmBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::{types::CollectionSchema, AuthLevel};

        BlockInfo::new(
            "suppers-ai/provider-llm",
            "0.0.1",
            "http-handler@v1",
            "Remote LLM API providers (OpenAI, Anthropic)",
        )
        .instance_mode(InstanceMode::Singleton)
        .requires(vec![
            "wafer-run/network".into(),
            "wafer-run/config".into(),
            "wafer-run/database".into(),
        ])
        .collections(vec![CollectionSchema::new(PROVIDERS_COLLECTION)
            .field("name", "string")
            .field("provider_type", "string")
            .field_default("endpoint", "string", "https://api.openai.com/v1")
            .field_default("models", "text", "[]")
            .field_default("enabled", "int", "1")
            .index(&["provider_type", "enabled"])])
        .category(wafer_run::BlockCategory::Feature)
        .description(
            "Manage remote LLM API providers (OpenAI, Anthropic) and route chat completions.",
        )
        .endpoints(vec![
            BlockEndpoint::post("/b/provider-llm/api/chat").summary("Chat completion via provider"),
            BlockEndpoint::get("/b/provider-llm/api/providers")
                .summary("List providers")
                .auth(AuthLevel::Admin),
            BlockEndpoint::post("/b/provider-llm/api/providers")
                .summary("Create provider")
                .auth(AuthLevel::Admin),
            BlockEndpoint::patch("/b/provider-llm/api/providers/{id}")
                .summary("Update provider")
                .auth(AuthLevel::Admin),
            BlockEndpoint::delete("/b/provider-llm/api/providers/{id}")
                .summary("Delete provider")
                .auth(AuthLevel::Admin),
            BlockEndpoint::get("/b/provider-llm/api/models").summary("Aggregate model list"),
            BlockEndpoint::get("/b/provider-llm/admin")
                .summary("Admin UI")
                .auth(AuthLevel::Admin),
        ])
        .config_keys(vec![
            ConfigVar::new("SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY", "OpenAI API key", "")
                .name("OpenAI API Key")
                .input_type(InputType::Password),
            ConfigVar::new(
                "SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY",
                "Anthropic API key",
                "",
            )
            .name("Anthropic API Key")
            .input_type(InputType::Password),
        ])
        .admin_url("/b/provider-llm/admin")
        .can_disable(true)
        .default_enabled(true)
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let action = msg.action();
        let path = msg.path();
        let user_id = msg.user_id().to_string();

        // All endpoints require authentication
        if user_id.is_empty() {
            return crate::ui::forbidden_response(&msg);
        }

        // UI pages and provider config API require admin role.
        // Only /api/chat and /api/models are open to any authenticated user.
        let needs_admin = !path.contains("/api/")
            || path.contains("/api/providers")
            || path == "/b/provider-llm/admin"
            || path == "/b/provider-llm/admin/";
        if needs_admin {
            let is_admin = helpers::is_admin(&msg);
            if !is_admin {
                return crate::ui::forbidden_response(&msg);
            }
        }

        match (action, path) {
            // Admin UI
            ("retrieve", "/b/provider-llm/admin") | ("retrieve", "/b/provider-llm/admin/") => {
                pages::admin_page(ctx, &msg).await
            }

            // Chat API
            ("create", "/b/provider-llm/api/chat") => self.handle_chat(ctx, input).await,

            // Provider CRUD
            ("retrieve", "/b/provider-llm/api/providers") => {
                self.handle_list_providers(ctx, &msg).await
            }
            ("create", "/b/provider-llm/api/providers") => {
                self.handle_create_provider(ctx, input).await
            }
            ("update", _) if path.starts_with("/b/provider-llm/api/providers/") => {
                self.handle_update_provider(ctx, &msg, input).await
            }
            ("delete", _) if path.starts_with("/b/provider-llm/api/providers/") => {
                self.handle_delete_provider(ctx, &msg).await
            }

            // Models aggregate
            ("retrieve", "/b/provider-llm/api/models") => self.handle_list_models(ctx).await,

            _ => err_not_found("not found"),
        }
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            self.seed_defaults(ctx).await;
        }
        Ok(())
    }
}
