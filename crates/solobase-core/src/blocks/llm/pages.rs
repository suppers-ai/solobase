//! SSR pages for the LLM orchestrator block.
//!
//! Provides:
//! - Chat page (`GET /b/llm/`) — thread list + message history + input form
//! - Thread page (`GET /b/llm/threads/{id}`) — focused thread view
//! - Settings page (`GET /b/llm/settings`) — default provider/model config

use maud::{html, Markup};
use wafer_core::clients::{
    config, database as db,
    database::{Filter, FilterOp, ListOptions, SortField},
};
use wafer_run::{context::Context, types::*, InputStream, OutputStream};

use super::SETTINGS_COLLECTION;
use crate::{
    blocks::helpers::{err_internal, RecordExt},
    ui::{self, components, icons, NavItem, SiteConfig, UserInfo},
};

const DEFAULT_PROVIDER_VAR: &str = "SUPPERS_AI__LLM__DEFAULT_PROVIDER";
const DEFAULT_MODEL_VAR: &str = "SUPPERS_AI__LLM__DEFAULT_MODEL";
const DEFAULT_PROVIDER: &str = "suppers-ai/provider-llm";

const CONTEXTS_COLLECTION: &str = "suppers_ai__messages__contexts";
const ENTRIES_COLLECTION: &str = "suppers_ai__messages__entries";

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

fn nav() -> Vec<NavItem> {
    vec![
        NavItem {
            label: "Chat".into(),
            href: "/b/llm/".into(),
            icon: "message-circle",
        },
        NavItem {
            label: "Settings".into(),
            href: "/b/llm/settings".into(),
            icon: "settings",
        },
    ]
}

fn llm_page(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    content: Markup,
    msg: &Message,
) -> OutputStream {
    let is_fragment = ui::is_htmx(msg);
    let markup = ui::layout::block_shell(title, config, &nav(), user, path, content, is_fragment);
    ui::html_response(markup)
}

// ---------------------------------------------------------------------------
// Chat page
// ---------------------------------------------------------------------------

pub async fn chat_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    let opts = ListOptions {
        sort: vec![SortField {
            field: "updated_at".to_string(),
            desc: true,
        }],
        limit: 50,
        ..Default::default()
    };

    let threads = match db::list(ctx, CONTEXTS_COLLECTION, &opts).await {
        Ok(r) => r.records,
        Err(_) => vec![],
    };

    // Load available remote models for the picker — aggregated across all
    // registered LLM backends via the `wafer-run/llm` service block.
    let models = load_models(ctx, msg).await;
    let default_model = config::get_default(ctx, DEFAULT_MODEL_VAR, "").await;

    let content = html! {
        (components::page_header(
            "Chat",
            Some("Start a conversation with your LLM providers"),
            None,
        ))

        // Model loading progress bar (hidden by default)
        div #model-progress-container style="display:none;margin-bottom:1rem" {
            div .card style="padding:0.75rem" {
                div style="display:flex;align-items:center;gap:0.75rem;margin-bottom:0.5rem" {
                    span style="font-size:0.875rem;font-weight:500" { "Loading model..." }
                    button #model-unload-btn .btn.btn-sm.btn-ghost onclick="unloadLocalModel()" style="margin-left:auto" { "Cancel" }
                }
                div style="background:var(--bg-secondary);border-radius:0.25rem;height:6px;overflow:hidden" {
                    div #model-progress-bar style="height:100%;background:var(--primary, #3b82f6);width:0%;transition:width 0.3s" {}
                }
                div #model-progress-text .text-muted style="font-size:0.75rem;margin-top:0.25rem" { "" }
            }
        }

        div style="display:grid;grid-template-columns:280px 1fr;gap:1.5rem;height:calc(100vh - 200px)" {
            // --- Sidebar: thread list ---
            div style="display:flex;flex-direction:column;gap:0.75rem;overflow:hidden" {
                div style="display:flex;align-items:center;justify-content:space-between" {
                    h3 style="font-size:0.875rem;font-weight:600;color:var(--text-muted);margin:0;text-transform:uppercase;letter-spacing:0.05em" {
                        "Threads"
                    }
                    button
                        .btn.btn-sm.btn-primary
                        onclick="createNewThread()"
                    {
                        (icons::plus())
                    }
                }
                div #thread-list style="overflow-y:auto;flex:1" {
                    (thread_list_items(&threads))
                }
            }

            // --- Main: message area + input ---
            div style="display:flex;flex-direction:column;overflow:hidden" {
                // Model picker — populated server-side from
                // `wafer-run/llm` aggregated `list_models`. Values use the
                // `"{backend_id}:{model_id}"` format so the chat API can
                // forward them unchanged via the `model` field.
                div style="margin-bottom:1rem;display:flex;align-items:center;gap:0.75rem;flex-wrap:wrap" {
                    label .form-label style="margin:0;font-size:0.875rem" { "Model:" }
                    select
                        #model-picker
                        .form-input
                        style="max-width:280px"
                        name="model"
                        onchange="onModelChange(this.value)"
                    {
                        // Remote models group
                        optgroup label="Remote" {
                            option value="" selected[default_model.is_empty()] { "Default (remote)" }
                            (render_model_picker(&models, default_model.as_str()))
                        }
                        // Local models group — populated by JS if WebGPU available
                        optgroup #local-models-group label="Local (WebLLM)" {}
                    }
                    // Status indicator for local model
                    span #model-status .text-muted style="font-size:0.75rem" {}
                }

                div #messages-area
                    style="flex:1;overflow-y:auto;padding:0.5rem;background:var(--bg-secondary);border-radius:0.5rem;margin-bottom:1rem"
                {
                    div #no-thread-prompt .text-center style="padding:3rem 1rem" {
                        div style="font-size:2.5rem;margin-bottom:0.75rem" { "\u{1f4ac}" }
                        p style="font-size:1.1rem;color:var(--text-primary);margin:0 0 0.5rem" { "Start a new conversation" }
                        p .text-muted style="margin:0 0 1.5rem" { "Click the " strong { "+" } " button to create a thread, then type your message." }
                    }
                }

                // Chat input form (disabled until a thread is selected)
                form
                    id="chat-form"
                    onsubmit="return handleChatSubmit(event)"
                    style="opacity:0.4;pointer-events:none"
                    data-thread=""
                {
                    input type="hidden" name="thread_id" id="active-thread-id" value="";
                    div style="display:flex;gap:0.5rem;align-items:flex-end" {
                        div style="flex:1;position:relative" {
                            textarea
                                .form-input
                                #chat-input
                                name="message"
                                placeholder="Create a thread first..."
                                rows="3"
                                required
                                disabled
                                style="resize:none;width:100%"
                                onkeydown="if(event.key==='Enter'&&!event.shiftKey){event.preventDefault();this.closest('form').requestSubmit();}"
                            {}
                        }
                        div style="display:flex;flex-direction:column;align-items:center;gap:0.25rem" {
                            button #send-btn .btn.btn-primary type="submit" disabled style="height:fit-content" {
                                "Send"
                            }
                            span #send-status .text-muted style="font-size:0.7rem;white-space:nowrap" {}
                        }
                    }
                }
            }
        }

        // Pulse animation for thinking indicator + blinking cursor for typing
        style { "@keyframes pulse{0%,100%{opacity:1}50%{opacity:.4}} @keyframes blink{0%,100%{opacity:1}50%{opacity:0}} .typing-cursor{display:inline-block;width:0.5em;height:1.1em;background:var(--text-primary,#333);vertical-align:text-bottom;margin-left:2px;animation:blink 0.8s step-end infinite}" }
        // Load marked.js for markdown rendering
        script src="https://cdn.jsdelivr.net/npm/marked@14/marked.min.js" {}
        // Load ai-bridge.js for local model inference
        script type="module" src="/ai-bridge.js" {}

        // Shared JS (markdown, message rendering, model management, chat logic)
        script {
            (maud::PreEscaped(SHARED_JS))
        }
        // Page-specific JS (thread selection)
        script {
            (maud::PreEscaped(CHAT_JS))
        }
    };

    llm_page("Chat", &config, &path, user.as_ref(), content, msg)
}

fn thread_list_items(threads: &[db::Record]) -> Markup {
    html! {
        @if threads.is_empty() {
            div .text-center .text-muted style="padding:1rem;font-size:0.875rem" {
                "No threads yet."
            }
        } @else {
            @for thread in threads {
                @let id = thread.id.as_str();
                @let title = thread.str_field("title");
                @let updated_at = thread.str_field("updated_at");
                @let date = updated_at.get(..10).unwrap_or(updated_at);
                div
                    .card
                    style="margin-bottom:0.375rem;cursor:pointer;padding:0.625rem 0.75rem;transition:box-shadow 0.15s"
                    data-thread-id=(id)
                    onclick={"selectThread('" (id) "')"}
                    onmouseover="this.style.boxShadow='0 2px 8px rgba(0,0,0,0.1)'"
                    onmouseout="this.style.boxShadow=''"
                {
                    div style="display:flex;align-items:center;justify-content:space-between;gap:0.5rem" {
                        span style="font-weight:500;font-size:0.875rem;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1" {
                            @if title.is_empty() { "Untitled" } @else { (title) }
                        }
                        @if !date.is_empty() {
                            span .text-muted style="font-size:0.75rem;flex-shrink:0" { (date) }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Thread page (focused view)
// ---------------------------------------------------------------------------

pub async fn thread_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    let thread_id = path
        .strip_prefix("/b/llm/threads/")
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("");

    if thread_id.is_empty() {
        return ui::not_found_response(msg);
    }

    let thread = match db::get(ctx, CONTEXTS_COLLECTION, thread_id).await {
        Ok(r) => r,
        Err(e) if e.code == ErrorCode::NotFound => return ui::not_found_response(msg),
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };

    let messages_opts = ListOptions {
        filters: vec![Filter {
            field: "context_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(thread_id.to_string()),
        }],
        sort: vec![SortField {
            field: "created_at".to_string(),
            desc: false,
        }],
        limit: 200,
        ..Default::default()
    };

    let messages = match db::list(ctx, ENTRIES_COLLECTION, &messages_opts).await {
        Ok(r) => r.records,
        Err(_) => vec![],
    };

    let models = load_models(ctx, msg).await;
    let default_model = config::get_default(ctx, DEFAULT_MODEL_VAR, "").await;

    let thread_title = thread.str_field("title");
    let display_title = if thread_title.is_empty() {
        "Chat"
    } else {
        thread_title
    };

    // Build messages JSON for the page JS to pick up
    let messages_json: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m.str_field("role"),
                "content": m.str_field("content"),
                "created_at": m.str_field("created_at"),
            })
        })
        .collect();
    let messages_json_str = serde_json::to_string(&messages_json).unwrap_or_else(|_| "[]".into());

    let content = html! {
        div style="display:flex;align-items:center;gap:0.75rem;margin-bottom:1rem" {
            a .btn.btn-ghost.btn-sm href="/b/llm/" { "\u{2190} Back" }
            h1 .page-title style="margin:0" { (display_title) }
        }

        // Model loading progress bar (hidden by default)
        div #model-progress-container style="display:none;margin-bottom:1rem" {
            div .card style="padding:0.75rem" {
                div style="display:flex;align-items:center;gap:0.75rem;margin-bottom:0.5rem" {
                    span style="font-size:0.875rem;font-weight:500" { "Loading model..." }
                    button #model-unload-btn .btn.btn-sm.btn-ghost onclick="unloadLocalModel()" style="margin-left:auto" { "Cancel" }
                }
                div style="background:var(--bg-secondary);border-radius:0.25rem;height:6px;overflow:hidden" {
                    div #model-progress-bar style="height:100%;background:var(--primary, #3b82f6);width:0%;transition:width 0.3s" {}
                }
                div #model-progress-text .text-muted style="font-size:0.75rem;margin-top:0.25rem" { "" }
            }
        }

        // Model picker — populated server-side from `wafer-run/llm`
        // aggregated `list_models`. See `render_model_picker` for the
        // `"{backend_id}:{model_id}"` value shape.
        div style="margin-bottom:1rem;display:flex;align-items:center;gap:0.75rem;flex-wrap:wrap" {
            label .form-label style="margin:0;font-size:0.875rem" { "Model:" }
            select
                #model-picker
                .form-input
                style="max-width:280px"
                name="model"
                onchange="onModelChange(this.value)"
            {
                optgroup label="Remote" {
                    option value="" selected[default_model.is_empty()] { "Default (remote)" }
                    (render_model_picker(&models, default_model.as_str()))
                }
                optgroup #local-models-group label="Local (WebLLM)" {}
            }
            span #model-status .text-muted style="font-size:0.75rem" {}
        }

        // Messages list
        div
            #messages-area
            style="margin-bottom:1rem;max-height:60vh;overflow-y:auto;padding:0.5rem;background:var(--bg-secondary);border-radius:0.5rem"
        {
            @if messages.is_empty() {
                div .text-center .text-muted style="padding:2rem" {
                    "No messages yet. Send the first one below."
                }
            }
            // Messages will be rendered by JS for consistent markdown rendering
        }

        // Chat input form
        form
            id="chat-form"
            onsubmit="return handleChatSubmit(event)"
        {
            input type="hidden" name="thread_id" id="active-thread-id" value=(thread_id);
            div style="display:flex;gap:0.5rem;align-items:flex-end" {
                div style="flex:1;position:relative" {
                    textarea
                        .form-input
                        #chat-input
                        name="message"
                        placeholder="Type your message..."
                        rows="3"
                        required
                        style="resize:none;width:100%"
                        onkeydown="if(event.key==='Enter'&&!event.shiftKey){event.preventDefault();this.closest('form').requestSubmit();}"
                    {}
                }
                button #send-btn .btn.btn-primary type="submit" style="height:fit-content" {
                    "Send"
                }
            }
        }

        // Pulse animation for thinking indicator + blinking cursor for typing
        style { "@keyframes pulse{0%,100%{opacity:1}50%{opacity:.4}} @keyframes blink{0%,100%{opacity:1}50%{opacity:0}} .typing-cursor{display:inline-block;width:0.5em;height:1.1em;background:var(--text-primary,#333);vertical-align:text-bottom;margin-left:2px;animation:blink 0.8s step-end infinite}" }
        // Load marked.js for markdown rendering
        script src="https://cdn.jsdelivr.net/npm/marked@14/marked.min.js" {}
        // Load ai-bridge.js for local model inference
        script type="module" src="/ai-bridge.js" {}

        // Pass initial messages to JS
        script {
            (maud::PreEscaped(format!(
                "window._threadMessages = {messages_json_str};"
            )))
        }

        // Shared JS (markdown, message rendering, model management, chat logic)
        script {
            (maud::PreEscaped(SHARED_JS))
        }
        // Page-specific JS (initial message rendering)
        script {
            (maud::PreEscaped(THREAD_JS))
        }
    };

    llm_page(display_title, &config, &path, user.as_ref(), content, msg)
}

// ---------------------------------------------------------------------------
// Settings page
// ---------------------------------------------------------------------------

pub async fn settings_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    let default_provider = config::get_default(ctx, DEFAULT_PROVIDER_VAR, DEFAULT_PROVIDER).await;
    let default_model = config::get_default(ctx, DEFAULT_MODEL_VAR, "").await;

    // Load per-thread overrides
    let overrides_opts = ListOptions {
        limit: 100,
        ..Default::default()
    };
    let overrides = match db::list(ctx, SETTINGS_COLLECTION, &overrides_opts).await {
        Ok(r) => r.records,
        Err(_) => vec![],
    };

    let content = html! {
        (components::page_header(
            "LLM Settings",
            Some("Configure default provider and model"),
            None,
        ))

        // Global defaults — read-only display; set via env vars
        div .card style="margin-bottom:1.5rem" {
            h3 style="font-size:1rem;font-weight:600;margin:0 0 1rem" { "Global Defaults" }
            p .text-muted style="font-size:0.875rem;margin-bottom:1rem" {
                "Global defaults are configured via environment variables."
            }
            div style="display:grid;grid-template-columns:1fr 1fr;gap:1rem" {
                div .form-group {
                    label .form-label { "Default Provider" }
                    input
                        .form-input
                        type="text"
                        value=(default_provider)
                        readonly
                        style="background:var(--bg-secondary)"
                    ;
                    p .form-hint {
                        "Set via " code { (DEFAULT_PROVIDER_VAR) }
                    }
                }
                div .form-group {
                    label .form-label { "Default Model" }
                    input
                        .form-input
                        type="text"
                        value=(default_model)
                        placeholder="(provider default)"
                        readonly
                        style="background:var(--bg-secondary)"
                    ;
                    p .form-hint {
                        "Set via " code { (DEFAULT_MODEL_VAR) }
                    }
                }
            }
        }

        // Per-thread overrides
        div .card {
            h3 style="font-size:1rem;font-weight:600;margin:0 0 1rem" { "Per-Thread Overrides" }
            @if overrides.is_empty() {
                div .text-center .text-muted style="padding:1.5rem" {
                    "No thread overrides configured."
                }
            } @else {
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Thread ID" }
                                th { "Provider Block" }
                                th { "Model" }
                                th { "Updated" }
                                th { "Actions" }
                            }
                        }
                        tbody {
                            @for ov in &overrides {
                                @let tid = ov.str_field("thread_id");
                                @let pb = ov.str_field("provider_block");
                                @let model = ov.str_field("model");
                                @let updated = ov.str_field("updated_at");
                                @let date = updated.get(..10).unwrap_or(updated);
                                tr {
                                    td {
                                        a href={"/b/llm/threads/" (tid)} style="font-family:monospace;font-size:0.8rem" {
                                            (tid)
                                        }
                                    }
                                    td {
                                        @if pb.is_empty() {
                                            span .text-muted { "(default)" }
                                        } @else {
                                            code style="font-size:0.8rem" { (pb) }
                                        }
                                    }
                                    td {
                                        @if model.is_empty() {
                                            span .text-muted { "(default)" }
                                        } @else {
                                            code style="font-size:0.8rem" { (model) }
                                        }
                                    }
                                    td .text-muted style="font-size:0.8rem" { (date) }
                                    td {
                                        button
                                            .btn.btn-sm.btn-danger
                                            hx-delete={"/b/llm/api/config/" (ov.id)}
                                            hx-confirm={"Remove override for thread " (tid) "?"}
                                            hx-target="closest tr"
                                            hx-swap="outerHTML"
                                        {
                                            (icons::trash())
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    llm_page("LLM Settings", &config, &path, user.as_ref(), content, msg)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load the aggregated list of available models from the `wafer-run/llm`
/// service block.
///
/// Mirrors the logic of [`super::routes::list_models`] so page rendering can
/// inline the picker options without an extra HTTP roundtrip. The service
/// block returns a bare `Vec<ModelInfo>` whose JSON fields are `backend_id`,
/// `model_id`, `display_name`, `capabilities`.
///
/// Returns an empty vec on any failure — the picker falls back to the
/// "Default (remote)" option and the user can still send a request.
async fn load_models(ctx: &dyn Context, original_msg: &Message) -> Vec<serde_json::Value> {
    let mut call_msg = Message::new(wafer_run::common::ServiceOp::LLM_LIST_MODELS);
    call_msg.set_meta("req.action", wafer_run::common::ServiceOp::LLM_LIST_MODELS);
    let user_id = original_msg.user_id().to_string();
    if !user_id.is_empty() {
        call_msg.set_meta("auth.user_id", &user_id);
    }
    let user_roles = original_msg.get_meta("auth.user_roles").to_string();
    if !user_roles.is_empty() {
        call_msg.set_meta("auth.user_roles", &user_roles);
    }

    let out = ctx
        .call_block("wafer-run/llm", call_msg, InputStream::empty())
        .await;
    let Ok(buf) = out.collect_buffered().await else {
        return vec![];
    };
    // The service block returns `Vec<ModelInfo>` as a bare JSON array. A
    // decode failure is treated as "no models" so the picker still renders —
    // the user can fall back to the default remote option.
    serde_json::from_slice::<Vec<serde_json::Value>>(&buf.body).unwrap_or_default()
}

/// Render the `<option>` list for the remote-model picker.
///
/// Each option's `value` is `"{backend_id}:{model_id}"` — a single string so
/// the existing thread-setting shape (single `model` field) stays compatible.
/// The visible label prefers `display_name`, falling back to `model_id`. The
/// `backend_id` is appended in parens when non-empty so users can disambiguate
/// the same model id hosted on different backends.
///
/// Entries missing both `model_id` and `id` (legacy key) are skipped — the
/// resulting `<option value="">` would collide with the "Default (remote)"
/// entry and pick the wrong model on submit.
fn render_model_picker(models: &[serde_json::Value], default_model: &str) -> Markup {
    html! {
        @for m in models {
            @let backend_id = m
                .get("backend_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            // Accept `model_id` (current `ModelInfo` shape) or `id` (legacy).
            @let model_id = m
                .get("model_id")
                .or_else(|| m.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            @if !model_id.is_empty() {
                @let value = if backend_id.is_empty() {
                    model_id.to_string()
                } else {
                    format!("{backend_id}:{model_id}")
                };
                @let display = m
                    .get("display_name")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(model_id);
                option
                    value=(value)
                    selected[value == default_model]
                {
                    (display)
                    @if !backend_id.is_empty() {
                        " (" (backend_id) ")"
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared JS utilities (markdown, message rendering, model management)
// ---------------------------------------------------------------------------

/// JavaScript shared between the chat page and thread page.
/// Contains markdown rendering, message card rendering, model management,
/// and chat submission logic.
const SHARED_JS: &str = r#"
// ---------------------------------------------------------------------------
// Markdown rendering
// ---------------------------------------------------------------------------

function renderMarkdown(text) {
    if (typeof marked !== 'undefined' && marked.parse) {
        try {
            return marked.parse(text, { breaks: true });
        } catch(e) {}
    }
    // Fallback: escape HTML and preserve whitespace
    return escHtml(text).replace(/\n/g, '<br>');
}

function escHtml(s) {
    return String(s)
        .replace(/&/g,'&amp;')
        .replace(/</g,'&lt;')
        .replace(/>/g,'&gt;')
        .replace(/"/g,'&quot;');
}

// ---------------------------------------------------------------------------
// Message card rendering
// ---------------------------------------------------------------------------

function messageCardHtml(role, content, date, opts) {
    opts = opts || {};
    var isMarkdown = (role === 'assistant');
    var rendered = isMarkdown ? renderMarkdown(content) : escHtml(content);

    var bg, badge;
    if (role === 'user') {
        bg = 'background:#eff6ff;border-left:3px solid #3b82f6';
        badge = 'badge-info';
    } else if (role === 'assistant') {
        bg = 'background:#f8fafc;border-left:3px solid #94a3b8';
        badge = 'badge';
    } else if (role === 'system') {
        bg = 'background:#fefce8;border-left:3px solid #eab308';
        badge = 'badge-warning';
    } else {
        bg = 'background:#f0fdf4;border-left:3px solid #22c55e';
        badge = 'badge-success';
    }

    var modelBadge = '';
    if (opts.model) {
        modelBadge = ' <span class="badge badge-info" style="font-size:0.7rem">' + escHtml(opts.model) + '</span>';
    }

    var contentStyle = isMarkdown
        ? 'margin:0;word-break:break-word;line-height:1.6'
        : 'margin:0;white-space:pre-wrap;word-break:break-word';

    var id = opts.id ? ' id="' + opts.id + '"' : '';

    return '<div class="card"' + id + ' style="margin-bottom:0.75rem;' + bg + '">'
        + '<div style="display:flex;align-items:center;gap:0.5rem;margin-bottom:0.5rem">'
        + '<span class="badge ' + badge + '" style="text-transform:capitalize">' + role + '</span>'
        + (date ? '<span class="text-muted" style="font-size:0.75rem">' + escHtml(date) + '</span>' : '')
        + modelBadge
        + '</div>'
        + '<div style="' + contentStyle + '">' + rendered + '</div>'
        + '</div>';
}

function appendMessageCard(role, content, opts) {
    var area = document.getElementById('messages-area');
    if (!area) return null;
    // Clear placeholder text
    var placeholder = area.querySelector('.text-center.text-muted');
    if (placeholder) placeholder.remove();

    var wrapper = document.createElement('div');
    var date = new Date().toISOString().slice(0, 10);
    wrapper.innerHTML = messageCardHtml(role, content, date, opts);
    var card = wrapper.firstChild;
    area.appendChild(card);
    area.scrollTop = area.scrollHeight;
    return card;
}

// ---------------------------------------------------------------------------
// Local model management
// ---------------------------------------------------------------------------

var _localModelLoading = false;

async function populateLocalModels() {
    if (!window.solobaseAI) return;
    var status = window.solobaseAI.getStatus();
    if (!status.webgpu_supported) {
        var group = document.getElementById('local-models-group');
        if (group) group.label = 'Local (WebGPU not available)';
        return;
    }
    var models = await window.solobaseAI.getAvailableModels();
    var group = document.getElementById('local-models-group');
    if (!group) return;
    group.innerHTML = '';
    models.forEach(function(m) {
        var opt = document.createElement('option');
        opt.value = 'local:' + m.id;
        opt.textContent = m.name;
        group.appendChild(opt);
    });
}

function onModelChange(value) {
    if (value && value.startsWith('local:')) {
        var modelId = value.slice(6);
        loadLocalModel(modelId);
    } else {
        updateModelStatus('');
    }
}

function loadLocalModel(modelId) {
    if (!window.solobaseAI) {
        updateModelStatus('WebLLM not loaded yet. Wait for page to finish loading.');
        return;
    }
    var status = window.solobaseAI.getStatus();
    if (status.loaded_model === modelId) {
        updateModelStatus('Ready');
        return;
    }

    _localModelLoading = true;
    showModelProgress(true);
    updateModelStatus('Loading...');

    window.solobaseAI.loadModel(modelId, function(progress) {
        var pct = Math.round(progress.progress * 100);
        var bar = document.getElementById('model-progress-bar');
        var text = document.getElementById('model-progress-text');
        if (bar) bar.style.width = pct + '%';
        if (text) text.textContent = progress.text;
    }).then(function() {
        _localModelLoading = false;
        showModelProgress(false);
        updateModelStatus('Ready');
    }).catch(function(err) {
        _localModelLoading = false;
        showModelProgress(false);
        updateModelStatus('Error: ' + err.message);
        console.error('[solobase] Model load error:', err);
    });
}

function unloadLocalModel() {
    if (!window.solobaseAI) return;
    window.solobaseAI.unloadModel().then(function() {
        _localModelLoading = false;
        showModelProgress(false);
        updateModelStatus('');
        // Reset picker to default
        var picker = document.getElementById('model-picker');
        if (picker) picker.value = '';
    });
}

function showModelProgress(show) {
    var container = document.getElementById('model-progress-container');
    if (container) container.style.display = show ? 'block' : 'none';
}

function updateModelStatus(text) {
    var el = document.getElementById('model-status');
    if (el) el.textContent = text;
}

// ---------------------------------------------------------------------------
// Chat submission
// ---------------------------------------------------------------------------

var _chatBusy = false;

function handleChatSubmit(e) {
    e.preventDefault();
    if (_chatBusy) return false;

    var form = document.getElementById('chat-form');
    var textarea = document.getElementById('chat-input');
    var threadId = document.getElementById('active-thread-id').value;
    var userText = textarea.value.trim();

    if (!userText || !threadId) return false;

    _chatBusy = true;
    setSendEnabled(false);
    textarea.value = '';

    // 1. Show user message immediately
    appendMessageCard('user', userText);

    // 2. Determine if local or remote
    var picker = document.getElementById('model-picker');
    var model = picker ? picker.value : '';

    var chatPromise;
    if (model.startsWith('local:')) {
        // Local path: save user message ourselves (the SW chat endpoint is not involved)
        chatPromise = fetch('/b/messages/api/threads/' + threadId + '/messages', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ role: 'user', content: userText })
        }).then(function() {
            return handleLocalChat(threadId, model.slice(6));
        });
    } else {
        // Remote path: /b/llm/api/chat saves user + assistant messages server-side
        chatPromise = handleRemoteChat(threadId, userText, model);
    }

    chatPromise.catch(function(err) {
        appendMessageCard('system', 'Error: ' + err.message);
    }).finally(function() {
        _chatBusy = false;
        setSendEnabled(true);
    });

    return false;
}

function handleLocalChat(threadId, modelId) {
    if (!window.solobaseAI) {
        appendMessageCard('system', 'WebLLM not loaded. Select a local model first.');
        return Promise.resolve();
    }

    // Get full thread history for context
    return fetch('/b/messages/api/threads/' + threadId + '/messages')
        .then(function(r) { return r.json(); })
        .then(function(data) {
            var records = data.records || [];
            var messages = records.map(function(m) {
                var d = m.data || m;
                return { role: d.role, content: d.content };
            });

            // Create streaming assistant card with thinking indicator
            var card = appendMessageCard('assistant', '', { id: 'streaming-msg' });
            var contentDiv = card ? card.querySelector('div:last-child') : null;
            if (contentDiv) contentDiv.innerHTML = '<span class="text-muted" style="animation:pulse 1.5s infinite">Thinking...</span>';
            setSendStatus('AI is thinking...');

            return window.solobaseAI.chat(messages, function(delta, full) {
                setSendStatus('AI is typing...');
                // Update the card with streaming content + blinking cursor
                if (contentDiv) {
                    contentDiv.innerHTML = renderMarkdown(full) + '<span class="typing-cursor"></span>';
                    var area = document.getElementById('messages-area');
                    if (area) area.scrollTop = area.scrollHeight;
                }
            });
        })
        .then(function(result) {
            // Remove the streaming ID and blinking cursor
            var streamCard = document.getElementById('streaming-msg');
            if (streamCard) {
                streamCard.removeAttribute('id');
                var cursor = streamCard.querySelector('.typing-cursor');
                if (cursor) cursor.remove();
                // Re-render without cursor
                var cd = streamCard.querySelector('div:last-child');
                if (cd && result.content) cd.innerHTML = renderMarkdown(result.content);
            }

            // Save assistant message
            return fetch('/b/messages/api/threads/' + threadId + '/messages', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ role: 'assistant', content: result.content })
            });
        });
}

function handleRemoteChat(threadId, userText, model) {
    // Create a placeholder assistant card with animated thinking indicator
    var card = appendMessageCard('assistant', '', { id: 'streaming-msg' });
    var contentDiv = card ? card.querySelector('div:last-child') : null;
    if (contentDiv) contentDiv.innerHTML = '<span class="text-muted" style="animation:pulse 1.5s infinite">Thinking...</span>';
    setSendStatus('Waiting for response...');

    var body = { thread_id: threadId, message: userText };
    if (model) body.model = model;

    return fetch('/b/llm/api/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
    })
    .then(function(r) { return r.json(); })
    .then(function(data) {
        // Replace the placeholder with actual response
        var streamCard = document.getElementById('streaming-msg');
        if (streamCard) {
            var contentDiv = streamCard.querySelector('div:last-child');
            if (contentDiv) {
                contentDiv.innerHTML = renderMarkdown(data.content || 'No response');
                contentDiv.style.margin = '0';
                contentDiv.style.wordBreak = 'break-word';
                contentDiv.style.lineHeight = '1.6';
            }
            // Add model badge if available
            if (data.model) {
                var header = streamCard.querySelector('div:first-child');
                if (header) {
                    var badge = document.createElement('span');
                    badge.className = 'badge badge-info';
                    badge.style.fontSize = '0.7rem';
                    badge.textContent = data.model;
                    header.appendChild(badge);
                }
            }
            streamCard.removeAttribute('id');
        }
    })
    .catch(function(err) {
        var streamCard = document.getElementById('streaming-msg');
        if (streamCard) streamCard.remove();
        appendMessageCard('system', 'Error: ' + err.message);
    });
}

function setSendEnabled(enabled) {
    var btn = document.getElementById('send-btn');
    var input = document.getElementById('chat-input');
    if (btn) { btn.disabled = !enabled; btn.textContent = enabled ? 'Send' : 'Sending...'; }
    if (input) input.disabled = !enabled;
    if (enabled) setSendStatus('');
}

function setSendStatus(text) {
    var el = document.getElementById('send-status');
    if (el) el.textContent = text;
}

// ---------------------------------------------------------------------------
// Thread creation
// ---------------------------------------------------------------------------

function createNewThread() {
    fetch('/b/messages/api/threads', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ title: 'New Chat' })
    })
    .then(function(r) { return r.json(); })
    .then(function(data) {
        var id = data.id || (data.data && data.data.id);
        if (id) {
            // Add thread to sidebar
            var list = document.getElementById('thread-list');
            if (list) {
                var placeholder = list.querySelector('.text-center.text-muted');
                if (placeholder) placeholder.remove();
                var date = new Date().toISOString().slice(0, 10);
                var html = '<div class="card" style="margin-bottom:0.375rem;cursor:pointer;padding:0.625rem 0.75rem;transition:box-shadow 0.15s" '
                    + 'data-thread-id="' + id + '" '
                    + 'onclick="selectThread(\'' + id + '\')" '
                    + 'onmouseover="this.style.boxShadow=\'0 2px 8px rgba(0,0,0,0.1)\'" '
                    + 'onmouseout="this.style.boxShadow=\'\'">'
                    + '<div style="display:flex;align-items:center;justify-content:space-between;gap:0.5rem">'
                    + '<span style="font-weight:500;font-size:0.875rem;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1">New Chat</span>'
                    + '<span class="text-muted" style="font-size:0.75rem;flex-shrink:0">' + date + '</span>'
                    + '</div></div>';
                list.insertAdjacentHTML('afterbegin', html);
            }
            selectThread(id);
        }
    })
    .catch(function(err) {
        console.error('[solobase] Error creating thread:', err);
    });
}
"#;

// ---------------------------------------------------------------------------
// Chat page JS (includes shared + chat-specific)
// ---------------------------------------------------------------------------

const CHAT_JS: &str = r#"
// ---------------------------------------------------------------------------
// Thread selection (chat page only)
// ---------------------------------------------------------------------------

function selectThread(id) {
    // Update active thread ID in form
    document.getElementById('active-thread-id').value = id;

    // Enable the form
    var form = document.getElementById('chat-form');
    form.style.opacity = '1';
    form.style.pointerEvents = 'auto';
    var input = document.getElementById('chat-input');
    if (input) { input.disabled = false; input.placeholder = 'Type your message...'; input.focus(); }
    var btn = document.getElementById('send-btn');
    if (btn) btn.disabled = false;
    // Remove the "create a thread" prompt
    var prompt = document.getElementById('no-thread-prompt');
    if (prompt) prompt.remove();

    // Load thread messages
    fetch('/b/messages/api/threads/' + id + '/messages')
        .then(function(r) { return r.json(); })
        .then(function(data) {
            var records = data.records || [];
            var area = document.getElementById('messages-area');
            if (!area) return;

            if (records.length === 0) {
                area.innerHTML = '<div class="text-center text-muted" style="padding:2rem">No messages yet.</div>';
            } else {
                var html = records.map(function(m) {
                    var d = m.data || m;
                    var role = d.role || 'user';
                    var content = d.content || '';
                    var date = (d.created_at || '').slice(0, 10);
                    return messageCardHtml(role, content, date);
                }).join('');
                area.innerHTML = html;
            }
            area.scrollTop = area.scrollHeight;
        })
        .catch(function(err) {
            console.error('[solobase] Error loading messages:', err);
        });

    // Highlight active thread
    document.querySelectorAll('[data-thread-id]').forEach(function(el) {
        if (el.dataset.threadId === id) {
            el.style.borderColor = 'var(--primary)';
            el.style.background = 'var(--primary-light, #eff6ff)';
        } else {
            el.style.borderColor = '';
            el.style.background = '';
        }
    });

    // Update URL so thread survives navigation/refresh without conflicting with thread_view_page
    history.replaceState({}, '', '/b/llm/?thread=' + id);
}

// Auto-select thread from URL on page load
(function() {
    var threadId = new URLSearchParams(location.search).get('thread');
    if (threadId) {
        setTimeout(function() { selectThread(threadId); }, 100);
    }
})();

// Populate local models when ai-bridge loads
setTimeout(function() { populateLocalModels(); }, 1500);
// Retry in case CDN import is slow
setTimeout(function() { populateLocalModels(); }, 5000);
"#;

// ---------------------------------------------------------------------------
// Thread page JS (includes shared + thread-specific)
// ---------------------------------------------------------------------------

const THREAD_JS: &str = r#"
// Render initial messages on load
document.addEventListener('DOMContentLoaded', function() {
    var messages = window._threadMessages || [];
    var area = document.getElementById('messages-area');
    if (!area || messages.length === 0) return;

    area.innerHTML = messages.map(function(m) {
        var date = (m.created_at || '').slice(0, 10);
        return messageCardHtml(m.role, m.content, date);
    }).join('');
    area.scrollTop = area.scrollHeight;
});

// Populate local models when ai-bridge loads
setTimeout(function() { populateLocalModels(); }, 1500);
setTimeout(function() { populateLocalModels(); }, 5000);
"#;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty input produces no `<option>` tags — the "Default (remote)" entry
    /// rendered alongside this helper stays the only option.
    #[test]
    fn render_model_picker_empty_list_emits_no_options() {
        let markup = render_model_picker(&[], "");
        let html = markup.into_string();
        assert!(
            !html.contains("<option"),
            "expected zero <option> tags for empty model list, got: {html}"
        );
    }

    /// A typical aggregated `Vec<ModelInfo>` payload renders one option per
    /// entry with the `"{backend_id}:{model_id}"` value and `display_name`
    /// label.
    #[test]
    fn render_model_picker_typical_list_emits_one_option_per_model() {
        let models = vec![
            serde_json::json!({
                "backend_id": "openai",
                "model_id": "gpt-4o",
                "display_name": "GPT-4o",
            }),
            serde_json::json!({
                "backend_id": "anthropic",
                "model_id": "claude-3-5-sonnet",
                "display_name": "Claude 3.5 Sonnet",
            }),
        ];
        let markup = render_model_picker(&models, "anthropic:claude-3-5-sonnet");
        let html = markup.into_string();

        // One <option> per model entry.
        assert_eq!(
            html.matches("<option").count(),
            2,
            "expected 2 <option> tags, got: {html}"
        );
        // Composite value uses `backend_id:model_id`.
        assert!(
            html.contains(r#"value="openai:gpt-4o""#),
            "missing openai value: {html}"
        );
        assert!(
            html.contains(r#"value="anthropic:claude-3-5-sonnet""#),
            "missing anthropic value: {html}"
        );
        // Human label comes from display_name.
        assert!(html.contains("GPT-4o"), "missing GPT-4o label: {html}");
        assert!(
            html.contains("Claude 3.5 Sonnet"),
            "missing Claude label: {html}"
        );
        // Backend id is appended in parens for disambiguation.
        assert!(html.contains("(openai)"), "missing backend suffix: {html}");
        // The entry matching the default_model string is pre-selected.
        assert!(
            html.contains(r#"value="anthropic:claude-3-5-sonnet" selected"#),
            "expected selected attr on matching default_model entry: {html}"
        );
    }

    /// Malformed entries — missing both `model_id` and `id`, or with a blank
    /// `model_id` — must be skipped rather than rendered as a junk
    /// `value=""` option (which would collide with the "Default (remote)"
    /// entry rendered alongside). Missing `display_name` falls back to the
    /// model id. Missing `backend_id` renders a value without the `:`
    /// prefix and no parenthesized suffix.
    #[test]
    fn render_model_picker_skips_malformed_entries() {
        let models = vec![
            // Missing model_id entirely — skip.
            serde_json::json!({ "backend_id": "openai", "display_name": "Orphan" }),
            // Blank model_id — skip.
            serde_json::json!({ "backend_id": "openai", "model_id": "" }),
            // Legacy `id` field (pre-Task-16 shape) is accepted as a fallback.
            serde_json::json!({ "backend_id": "legacy", "id": "legacy-model" }),
            // Missing display_name — label falls back to model_id.
            serde_json::json!({ "backend_id": "openai", "model_id": "gpt-4o-mini" }),
            // Missing backend_id — value has no `:` prefix, no parens suffix.
            serde_json::json!({ "model_id": "solo-model", "display_name": "Solo" }),
        ];
        let markup = render_model_picker(&models, "");
        let html = markup.into_string();

        // Three valid entries out of five.
        assert_eq!(
            html.matches("<option").count(),
            3,
            "expected 3 <option> tags, got: {html}"
        );
        // Legacy `id` fallback wired up correctly.
        assert!(
            html.contains(r#"value="legacy:legacy-model""#),
            "expected legacy-id fallback to produce backend:id value: {html}"
        );
        // Missing display_name falls back to model id as the visible label.
        assert!(
            html.contains(r#"value="openai:gpt-4o-mini""#),
            "expected openai:gpt-4o-mini value: {html}"
        );
        assert!(
            html.contains("gpt-4o-mini"),
            "expected gpt-4o-mini label fallback: {html}"
        );
        // No backend_id — value is the bare model_id, no `(...)` suffix.
        assert!(
            html.contains(r#"value="solo-model""#),
            "expected bare-model value when backend_id is missing: {html}"
        );
        assert!(
            !html.contains("(solo-model)"),
            "expected no parens suffix when backend_id is missing: {html}"
        );
        // The orphan with no model_id must NOT appear as value="".
        assert!(
            !html.contains(r#"value="""#),
            "expected no empty-value <option>, got: {html}"
        );
    }
}
