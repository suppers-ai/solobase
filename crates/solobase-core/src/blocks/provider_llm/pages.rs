//! SSR admin pages for the provider-llm block.
//!
//! Provides a simple admin UI for managing LLM providers (OpenAI, Anthropic, etc.).

use crate::blocks::helpers::RecordExt;
use crate::ui::{self, icons, NavItem, SiteConfig, UserInfo};
use maud::{html, Markup};
use wafer_core::clients::database as db;
use wafer_core::clients::database::ListOptions;
use wafer_run::context::Context;
use wafer_run::types::*;

use super::PROVIDERS_COLLECTION;

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

fn nav() -> Vec<NavItem> {
    vec![
        NavItem {
            label: "Providers".into(),
            href: "/b/provider-llm/admin".into(),
            icon: "server",
        },
        NavItem {
            label: "API Keys".into(),
            href: "/b/provider-llm/admin/keys".into(),
            icon: "key",
        },
    ]
}

fn provider_llm_page(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    content: Markup,
    msg: &mut Message,
) -> Result_ {
    let is_fragment = ui::is_htmx(msg);
    let markup = ui::layout::block_shell(title, config, &nav(), user, path, content, is_fragment);
    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// Providers list page
// ---------------------------------------------------------------------------

pub async fn admin_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    let opts = ListOptions {
        sort: vec![wafer_core::clients::database::SortField {
            field: "name".to_string(),
            desc: false,
        }],
        limit: 100,
        ..Default::default()
    };

    let providers = db::list(ctx, PROVIDERS_COLLECTION, &opts)
        .await
        .unwrap_or_else(|_| wafer_core::clients::database::RecordList {
            records: vec![],
            total_count: 0,
            page: 1,
            page_size: 100,
        });

    let content = html! {
        div .page-header {
            div {
                h1 .page-title { "LLM Providers" }
                p .page-subtitle { "Manage OpenAI, Anthropic, and compatible API providers." }
            }
            button
                .btn.btn-primary
                onclick="document.getElementById('add-provider-modal').classList.add('modal-open')"
            {
                (icons::plus())
                " Add Provider"
            }
        }

        // Config key reminder
        div .card style="margin-bottom:1.5rem;padding:1rem 1.25rem;background:var(--bg-card);border:1px solid var(--border)" {
            p style="margin:0;font-size:0.875rem;color:var(--text-muted)" {
                (icons::info())
                " API keys are configured via environment variables: "
                code { "SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY" }
                " and "
                code { "SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY" }
                ". Set them in your project settings."
            }
        }

        // Providers table
        @if providers.records.is_empty() {
            div .empty-state {
                div .empty-state-icon { (icons::server()) }
                h3 { "No providers yet" }
                p { "Add OpenAI, Anthropic, or any OpenAI-compatible provider to get started." }
            }
        } @else {
            div .table-container {
                table .table {
                    thead {
                        tr {
                            th { "Name" }
                            th { "Type" }
                            th { "Endpoint" }
                            th { "Models" }
                            th { "Status" }
                            th { "Actions" }
                        }
                    }
                    tbody {
                        @for provider in &providers.records {
                            @let name = provider.str_field("name");
                            @let ptype = provider.str_field("provider_type");
                            @let endpoint = provider.str_field("endpoint");
                            @let models_json = provider.str_field("models");
                            @let enabled = provider.i64_field("enabled");
                            @let model_count = count_models(models_json);
                            tr {
                                td { strong { (name) } }
                                td {
                                    span .badge.badge-info { (ptype) }
                                }
                                td style="font-size:0.8rem;color:var(--text-muted);max-width:220px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap" {
                                    (endpoint)
                                }
                                td {
                                    span .badge.badge-info { (model_count) " model(s)" }
                                }
                                td {
                                    @if enabled != 0 {
                                        span .badge.badge-success { "Enabled" }
                                    } @else {
                                        span .badge.badge-warning { "Disabled" }
                                    }
                                }
                                td {
                                    div style="display:flex;gap:0.5rem" {
                                        button
                                            .btn.btn-sm.btn-secondary
                                            onclick={
                                                "openEditModal('"
                                                (provider.id)
                                                "','"
                                                (js_escape(name))
                                                "','"
                                                (ptype)
                                                "','"
                                                (js_escape(endpoint))
                                                "','"
                                                (js_escape(models_json))
                                                "',"
                                                (enabled)
                                                ")"
                                            }
                                        {
                                            (icons::edit())
                                        }
                                        button
                                            .btn.btn-sm.btn-danger
                                            hx-delete={"/b/provider-llm/api/providers/" (provider.id)}
                                            hx-confirm={"Delete provider \"" (name) "\"?"}
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

        // Add Provider Modal
        div #add-provider-modal .modal {
            div .modal-dialog {
                div .modal-content {
                    div .modal-header {
                        h3 .modal-title { "Add Provider" }
                        button .modal-close onclick="document.getElementById('add-provider-modal').classList.remove('modal-open')" { "\u{00d7}" }
                    }
                    form
                        hx-post="/b/provider-llm/api/providers"
                        hx-target="body"
                        hx-swap="none"
                        hx-on--after-request="if(event.detail.successful){location.reload()}"
                    {
                        div .modal-body {
                            (provider_form_fields(None))
                        }
                        div .modal-footer {
                            button .btn.btn-secondary type="button"
                                onclick="document.getElementById('add-provider-modal').classList.remove('modal-open')"
                            { "Cancel" }
                            button .btn.btn-primary type="submit" { "Add Provider" }
                        }
                    }
                }
            }
        }

        // Edit Provider Modal
        div #edit-provider-modal .modal {
            div .modal-dialog {
                div .modal-content {
                    div .modal-header {
                        h3 .modal-title { "Edit Provider" }
                        button .modal-close onclick="document.getElementById('edit-provider-modal').classList.remove('modal-open')" { "\u{00d7}" }
                    }
                    form
                        id="edit-provider-form"
                        hx-patch="/b/provider-llm/api/providers/__ID__"
                        hx-target="body"
                        hx-swap="none"
                        hx-on--after-request="if(event.detail.successful){location.reload()}"
                    {
                        div .modal-body {
                            (provider_form_fields(Some("edit")))
                        }
                        div .modal-footer {
                            button .btn.btn-secondary type="button"
                                onclick="document.getElementById('edit-provider-modal').classList.remove('modal-open')"
                            { "Cancel" }
                            button .btn.btn-primary type="submit" { "Save Changes" }
                        }
                    }
                }
            }
        }

        script {
            (maud::PreEscaped(ADMIN_JS))
        }
    };

    provider_llm_page("LLM Providers", &config, &path, user.as_ref(), content, msg)
}

// ---------------------------------------------------------------------------
// Reusable form fields
// ---------------------------------------------------------------------------

fn provider_form_fields(prefix: Option<&str>) -> Markup {
    let id_prefix = prefix.unwrap_or("new");
    html! {
        div .form-group {
            label .form-label for={(id_prefix) "-name"} { "Display Name" }
            input
                .form-input
                type="text"
                name="name"
                id={(id_prefix) "-name"}
                placeholder="e.g. OpenAI, My Ollama"
                required;
        }
        div .form-group {
            label .form-label for={(id_prefix) "-type"} { "Provider Type" }
            select .form-select name="provider_type" id={(id_prefix) "-type"} {
                option value="openai" { "OpenAI (or compatible)" }
                option value="anthropic" { "Anthropic" }
            }
        }
        div .form-group {
            label .form-label for={(id_prefix) "-endpoint"} { "Endpoint URL" }
            input
                .form-input
                type="url"
                name="endpoint"
                id={(id_prefix) "-endpoint"}
                placeholder="https://api.openai.com/v1";
        }
        div .form-group {
            label .form-label for={(id_prefix) "-models"} { "Models (JSON array)" }
            input
                .form-input
                type="text"
                name="models"
                id={(id_prefix) "-models"}
                placeholder="[\"gpt-4o\",\"gpt-4o-mini\"]";
            p .form-hint { "Comma-separated list of model IDs available via this provider." }
        }
        div .form-group {
            label style="display:flex;align-items:center;gap:0.5rem;cursor:pointer" {
                input type="checkbox" name="enabled_check" id={(id_prefix) "-enabled"} checked;
                " Enabled"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn count_models(models_json: &str) -> usize {
    let v: serde_json::Value = serde_json::from_str(models_json).unwrap_or(serde_json::json!([]));
    v.as_array().map(|a| a.len()).unwrap_or(0)
}

fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

// ---------------------------------------------------------------------------
// Inline JS for admin modal interactions
// ---------------------------------------------------------------------------

const ADMIN_JS: &str = r#"
function openEditModal(id, name, ptype, endpoint, models, enabled) {
    var form = document.getElementById('edit-provider-form');
    form.setAttribute('hx-patch', '/b/provider-llm/api/providers/' + id);
    // Re-init htmx on the form so the new action takes effect
    if (window.htmx) htmx.process(form);

    var modal = document.getElementById('edit-provider-modal');
    modal.querySelector('#edit-name').value = name;
    var sel = modal.querySelector('#edit-type');
    for (var i = 0; i < sel.options.length; i++) {
        if (sel.options[i].value === ptype) { sel.selectedIndex = i; break; }
    }
    modal.querySelector('#edit-endpoint').value = endpoint;
    modal.querySelector('#edit-models').value = models;
    modal.querySelector('#edit-enabled').checked = (enabled !== 0);
    modal.classList.add('modal-open');
}
"#;
