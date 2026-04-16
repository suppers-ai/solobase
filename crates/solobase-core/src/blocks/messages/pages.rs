//! SSR pages for the messages block.
//!
//! Provides:
//! - Context list page (`GET /b/messages/`)
//! - Context detail page (`GET /b/messages/contexts/{id}`)

use maud::{html, Markup};
use wafer_core::clients::database as db;
use wafer_run::{context::Context, types::*, OutputStream};

use super::service::{self, ListContextsParams, ListEntriesParams};
use crate::{
    blocks::helpers::{err_internal, RecordExt},
    ui::{self, components, NavItem, SiteConfig, UserInfo},
};

fn nav() -> Vec<NavItem> {
    vec![NavItem {
        label: "Contexts".into(),
        href: "/b/messages/".into(),
        icon: "message-square",
    }]
}

fn messages_page(
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

pub fn entry_card(record: &db::Record) -> Markup {
    let kind = record.str_field("kind");
    let role = record.str_field("role");
    let content = record.str_field("content");
    let content_type = record.str_field("content_type");
    let created_at = record.str_field("created_at");
    let date = created_at.get(..10).unwrap_or(created_at);

    let (bg_style, badge_class) = match kind {
        "artifact" => (
            "background:#fdf4ff;border-left:3px solid #a855f7",
            "badge-warning",
        ),
        "notification" => (
            "background:#fefce8;border-left:3px solid #eab308",
            "badge-warning",
        ),
        "status" => (
            "background:#f0f9ff;border-left:3px solid #0ea5e9",
            "badge-info",
        ),
        _ => match role {
            "user" => (
                "background:#eff6ff;border-left:3px solid #3b82f6",
                "badge-info",
            ),
            "agent" | "assistant" => ("background:#f8fafc;border-left:3px solid #94a3b8", "badge"),
            "system" => (
                "background:#fefce8;border-left:3px solid #eab308",
                "badge-warning",
            ),
            _ => (
                "background:#f0fdf4;border-left:3px solid #22c55e",
                "badge-success",
            ),
        },
    };

    html! {
        div .card style={"margin-bottom:0.75rem;" (bg_style)} {
            div style="display:flex;align-items:center;gap:0.5rem;margin-bottom:0.5rem" {
                span .badge .(badge_class) style="text-transform:capitalize" { (kind) }
                @if !role.is_empty() {
                    span .badge style="text-transform:capitalize" { (role) }
                }
                @if kind == "artifact" && !content_type.is_empty() && content_type != "text/plain" {
                    span .text-muted style="font-size:0.7rem" { (content_type) }
                }
                @if !date.is_empty() {
                    span .text-muted style="font-size:0.75rem;margin-left:auto" { (date) }
                }
            }
            p style="margin:0;white-space:pre-wrap;word-break:break-word" { (content) }
        }
    }
}

pub async fn context_list_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    let params = ListContextsParams {
        context_type: None,
        status: None,
        sender_id: None,
        parent_id: None,
        page_size: 50,
        offset: 0,
    };

    let contexts = match service::list_contexts(ctx, &params).await {
        Ok(r) => r.records,
        Err(_) => vec![],
    };

    let content = html! {
        (components::page_header(
            "Messages",
            Some("Manage contexts and entries"),
            None,
        ))

        div .card style="margin-bottom:1.5rem" {
            h3 style="font-size:1rem;font-weight:600;margin:0 0 0.75rem" { "New Context" }
            form
                hx-post="/b/messages/api/contexts"
                hx-target="#context-list"
                hx-swap="afterbegin"
                hx-on--after-request="if(event.detail.successful){this.reset()}"
            {
                div style="display:flex;gap:0.5rem" {
                    select .form-input name="type" style="width:auto" {
                        option value="conversation" { "Conversation" }
                        option value="task" { "Task" }
                        option value="notification" { "Notification" }
                    }
                    input .form-input
                        type="text"
                        name="title"
                        placeholder="Title"
                        required
                        style="flex:1"
                    ;
                    button .btn .btn-primary type="submit" { "Create" }
                }
            }
        }

        div #context-list {
            @if contexts.is_empty() {
                div .text-center .text-muted style="padding:2rem" {
                    "No contexts yet. Create one above."
                }
            } @else {
                @for context in &contexts {
                    @let id = context.id.as_str();
                    @let title = context.str_field("title");
                    @let context_type = context.str_field("type");
                    @let status = context.str_field("status");
                    @let updated_at = context.str_field("updated_at");
                    @let date = updated_at.get(..10).unwrap_or(updated_at);
                    a .card href={"/b/messages/contexts/" (id)}
                        style="display:block;text-decoration:none;margin-bottom:0.5rem;transition:box-shadow 0.15s"
                        onmouseover="this.style.boxShadow='0 2px 8px rgba(0,0,0,0.1)'"
                        onmouseout="this.style.boxShadow=''"
                    {
                        div style="display:flex;align-items:center;justify-content:space-between" {
                            div style="display:flex;align-items:center;gap:0.5rem" {
                                span .badge style="text-transform:capitalize" { (context_type) }
                                span style="font-weight:500;color:var(--text-primary)" {
                                    @if title.is_empty() { "Untitled" } @else { (title) }
                                }
                            }
                            div style="display:flex;align-items:center;gap:0.5rem" {
                                span .badge { (status) }
                                @if !date.is_empty() {
                                    span .text-muted style="font-size:0.8rem" { (date) }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    messages_page("Messages", &config, &path, user.as_ref(), content, msg)
}

pub async fn context_detail_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    let context_id = path
        .strip_prefix("/b/messages/contexts/")
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("");

    if context_id.is_empty() {
        return ui::not_found_response(msg);
    }

    let context = match service::get_context(ctx, context_id).await {
        Ok(r) => r,
        Err(e) if e.code == ErrorCode::NotFound => return ui::not_found_response(msg),
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };

    let entries_params = ListEntriesParams {
        kind: None,
        role: None,
        page_size: 200,
        offset: 0,
    };

    let entries = match service::list_entries(ctx, context_id, &entries_params).await {
        Ok(r) => r.records,
        Err(_) => vec![],
    };

    let context_title = context.str_field("title");
    let context_type = context.str_field("type");
    let context_status = context.str_field("status");
    let display_title = if context_title.is_empty() {
        "Untitled"
    } else {
        context_title
    };

    let post_url = format!("/b/messages/api/contexts/{context_id}/entries");

    let content = html! {
        div style="display:flex;align-items:center;gap:0.75rem;margin-bottom:1.5rem" {
            a .btn .btn-ghost .btn-sm href="/b/messages/" { "\u{2190} Back" }
            h1 .page-title style="margin:0" { (display_title) }
            span .badge style="text-transform:capitalize" { (context_type) }
            span .badge { (context_status) }
        }

        div #entries-list style="margin-bottom:1.5rem;max-height:60vh;overflow-y:auto" {
            @if entries.is_empty() {
                div .text-center .text-muted style="padding:2rem" {
                    "No entries yet. Add one below."
                }
            } @else {
                @for e in &entries {
                    (entry_card(e))
                }
            }
        }

        div .card {
            h3 style="font-size:0.9rem;font-weight:600;margin:0 0 0.75rem;color:var(--text-muted)" {
                "Add Entry"
            }
            form
                hx-post=(post_url)
                hx-target="#entries-list"
                hx-swap="beforeend"
                hx-on--after-request="if(event.detail.successful){this.reset();var list=document.getElementById('entries-list');list.scrollTop=list.scrollHeight;}"
            {
                div style="display:flex;gap:0.5rem;margin-bottom:0.5rem" {
                    select .form-input name="kind" style="width:auto" {
                        option value="message" { "message" }
                        option value="artifact" { "artifact" }
                        option value="notification" { "notification" }
                        option value="status" { "status" }
                    }
                    select .form-input name="role" style="width:auto" {
                        option value="user" { "user" }
                        option value="agent" { "agent" }
                        option value="system" { "system" }
                    }
                }
                div style="display:flex;gap:0.5rem;align-items:flex-end" {
                    textarea .form-input
                        name="content"
                        placeholder="Entry content"
                        rows="3"
                        required
                        style="flex:1;resize:vertical"
                    {}
                    button .btn .btn-primary type="submit" { "Add" }
                }
            }
        }
    };

    messages_page(display_title, &config, &path, user.as_ref(), content, msg)
}
