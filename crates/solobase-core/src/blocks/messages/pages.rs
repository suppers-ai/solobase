//! SSR pages for the messages block.
//!
//! Provides:
//! - Thread list page (`GET /b/messages/`)
//! - Thread view page (`GET /b/messages/threads/{id}`)
//! - Message HTML fragment (htmx partial for new messages)

use crate::blocks::helpers::RecordExt;
use crate::ui::{self, components, NavItem, SiteConfig, UserInfo};
use maud::{html, Markup};
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

use super::{MESSAGES_COLLECTION, THREADS_COLLECTION};

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

fn nav() -> Vec<NavItem> {
    vec![NavItem {
        label: "Threads".into(),
        href: "/b/messages/".into(),
        icon: "message-square",
    }]
}

/// Wrap content in the messages shell (sidebar + layout).
fn messages_page(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    content: Markup,
    msg: &mut Message,
) -> Result_ {
    let is_fragment = ui::is_htmx(msg);
    let markup =
        ui::layout::block_shell(title, config, &nav(), user, path, content, is_fragment);
    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// Message card fragment (also used inline in thread view)
// ---------------------------------------------------------------------------

/// Render a single message card. Used both on the full thread page and as
/// an htmx fragment when a new message is created.
pub fn message_card(record: &db::Record) -> Markup {
    let role = record.str_field("role");
    let content = record.str_field("content");
    let created_at = record.str_field("created_at");
    let date = created_at.get(..10).unwrap_or(created_at);

    let (bg_style, badge_class) = match role {
        "user" => (
            "background:#eff6ff;border-left:3px solid #3b82f6",
            "badge-info",
        ),
        "assistant" => (
            "background:#f8fafc;border-left:3px solid #94a3b8",
            "badge",
        ),
        "system" => (
            "background:#fefce8;border-left:3px solid #eab308",
            "badge-warning",
        ),
        _ => (
            "background:#f0fdf4;border-left:3px solid #22c55e",
            "badge-success",
        ),
    };

    html! {
        div .card style={"margin-bottom:0.75rem;" (bg_style)} {
            div style="display:flex;align-items:center;gap:0.5rem;margin-bottom:0.5rem" {
                span .badge .(badge_class) style="text-transform:capitalize" { (role) }
                @if !date.is_empty() {
                    span .text-muted style="font-size:0.75rem" { (date) }
                }
            }
            p style="margin:0;white-space:pre-wrap;word-break:break-word" { (content) }
        }
    }
}

// ---------------------------------------------------------------------------
// Thread list page
// ---------------------------------------------------------------------------

pub async fn thread_list_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
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

    let threads = match db::list(ctx, THREADS_COLLECTION, &opts).await {
        Ok(r) => r.records,
        Err(_) => vec![],
    };

    let content = html! {
        (components::page_header(
            "Messages",
            Some("Manage conversation threads"),
            None,
        ))

        // New thread form
        div .card style="margin-bottom:1.5rem" {
            h3 style="font-size:1rem;font-weight:600;margin:0 0 0.75rem" { "New Thread" }
            form
                hx-post="/b/messages/api/threads"
                hx-target="#thread-list"
                hx-swap="afterbegin"
                hx-on--after-request="if(event.detail.successful){this.reset()}"
            {
                div style="display:flex;gap:0.5rem" {
                    input .form-input
                        type="text"
                        name="title"
                        placeholder="Thread title"
                        required
                        style="flex:1"
                    ;
                    button .btn .btn-primary type="submit" { "Create" }
                }
            }
        }

        // Thread list
        div #thread-list {
            @if threads.is_empty() {
                div .text-center .text-muted style="padding:2rem" {
                    "No threads yet. Create one above."
                }
            } @else {
                @for thread in &threads {
                    @let id = thread.id.as_str();
                    @let title = thread.str_field("title");
                    @let updated_at = thread.str_field("updated_at");
                    @let date = updated_at.get(..10).unwrap_or(updated_at);
                    a .card href={"/b/messages/threads/" (id)}
                        style="display:block;text-decoration:none;margin-bottom:0.5rem;transition:box-shadow 0.15s"
                        onmouseover="this.style.boxShadow='0 2px 8px rgba(0,0,0,0.1)'"
                        onmouseout="this.style.boxShadow=''"
                    {
                        div style="display:flex;align-items:center;justify-content:space-between" {
                            span style="font-weight:500;color:var(--text-primary)" {
                                @if title.is_empty() { "Untitled thread" } @else { (title) }
                            }
                            @if !date.is_empty() {
                                span .text-muted style="font-size:0.8rem" { (date) }
                            }
                        }
                    }
                }
            }
        }
    };

    messages_page("Messages", &config, &path, user.as_ref(), content, msg)
}

// ---------------------------------------------------------------------------
// Thread view page
// ---------------------------------------------------------------------------

pub async fn thread_view_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    // Extract thread ID from path: /b/messages/threads/{id}
    let thread_id = path
        .strip_prefix("/b/messages/threads/")
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("");

    if thread_id.is_empty() {
        return ui::not_found_response(msg);
    }

    let thread = match db::get(ctx, THREADS_COLLECTION, thread_id).await {
        Ok(r) => r,
        Err(e) if e.code == ErrorCode::NotFound => return ui::not_found_response(msg),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

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
        limit: 200,
        ..Default::default()
    };

    let messages = match db::list(ctx, MESSAGES_COLLECTION, &opts).await {
        Ok(r) => r.records,
        Err(_) => vec![],
    };

    let thread_title = thread.str_field("title");
    let display_title = if thread_title.is_empty() {
        "Untitled thread"
    } else {
        thread_title
    };

    let post_url = format!("/b/messages/api/threads/{thread_id}/messages");

    let content = html! {
        // Header with back button
        div style="display:flex;align-items:center;gap:0.75rem;margin-bottom:1.5rem" {
            a .btn .btn-ghost .btn-sm href="/b/messages/" { "\u{2190} Back" }
            h1 .page-title style="margin:0" { (display_title) }
        }

        // Messages area
        div #messages-list style="margin-bottom:1.5rem;max-height:60vh;overflow-y:auto" {
            @if messages.is_empty() {
                div .text-center .text-muted style="padding:2rem" {
                    "No messages yet. Send the first one below."
                }
            } @else {
                @for m in &messages {
                    (message_card(m))
                }
            }
        }

        // New message form
        div .card {
            h3 style="font-size:0.9rem;font-weight:600;margin:0 0 0.75rem;color:var(--text-muted)" {
                "Add Message"
            }
            form
                hx-post=(post_url)
                hx-target="#messages-list"
                hx-swap="beforeend"
                hx-on--after-request="if(event.detail.successful){this.reset();var list=document.getElementById('messages-list');list.scrollTop=list.scrollHeight;}"
            {
                div style="margin-bottom:0.5rem" {
                    select .form-input name="role" style="width:auto" {
                        option value="user" { "user" }
                        option value="assistant" { "assistant" }
                        option value="system" { "system" }
                    }
                }
                div style="display:flex;gap:0.5rem;align-items:flex-end" {
                    textarea .form-input
                        name="content"
                        placeholder="Message content"
                        rows="3"
                        required
                        style="flex:1;resize:vertical"
                    {}
                    button .btn .btn-primary type="submit" { "Send" }
                }
            }
        }
    };

    messages_page(display_title, &config, &path, user.as_ref(), content, msg)
}
