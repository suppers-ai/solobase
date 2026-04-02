//! SSR admin pages for the legal pages block.
//!
//! Provides a tabbed admin UI with:
//! - Privacy Policy editor (Quill rich text editor)
//! - Terms of Service editor (Quill rich text editor)
//! - API endpoints reference

use crate::blocks::helpers::{self, json_map, RecordExt};
use crate::ui::{self, components, icons, NavItem, SiteConfig, UserInfo};
use maud::{html, Markup, PreEscaped};
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

const COLLECTION: &str = super::COLLECTION;

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

fn nav() -> Vec<NavItem> {
    vec![
        NavItem {
            label: "Privacy Policy".into(),
            href: "/b/legalpages/admin/privacy".into(),
            icon: "shield",
        },
        NavItem {
            label: "Terms of Service".into(),
            href: "/b/legalpages/admin/terms".into(),
            icon: "file-text",
        },
        NavItem {
            label: "Endpoints".into(),
            href: "/b/legalpages/admin/endpoints".into(),
            icon: "globe",
        },
    ]
}

/// Wrap content in the legalpages admin shell (sidebar + layout).
fn legalpages_page(
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
// Document lookup
// ---------------------------------------------------------------------------

/// Find the current document for a given type.
/// Prefers the latest draft (so admin sees their in-progress edits),
/// then falls back to the published version.
async fn find_current_doc(ctx: &dyn Context, doc_type: &str) -> Option<db::Record> {
    // First try latest draft
    let opts = ListOptions {
        filters: vec![
            Filter {
                field: "doc_type".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!(doc_type),
            },
            Filter {
                field: "status".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!("draft"),
            },
        ],
        sort: vec![SortField {
            field: "updated_at".into(),
            desc: true,
        }],
        limit: 1,
        ..Default::default()
    };
    if let Ok(result) = db::list(ctx, COLLECTION, &opts).await {
        if let Some(record) = result.records.into_iter().next() {
            return Some(record);
        }
    }

    // Fall back to published
    let opts = ListOptions {
        filters: vec![
            Filter {
                field: "doc_type".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!(doc_type),
            },
            Filter {
                field: "status".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!("published"),
            },
        ],
        sort: vec![SortField {
            field: "updated_at".into(),
            desc: true,
        }],
        limit: 1,
        ..Default::default()
    };
    if let Ok(result) = db::list(ctx, COLLECTION, &opts).await {
        result.records.into_iter().next()
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Editor page (Privacy / Terms)
// ---------------------------------------------------------------------------

pub async fn editor_page(ctx: &dyn Context, msg: &mut Message, doc_type: &str) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let doc = find_current_doc(ctx, doc_type).await;
    let default_title = if doc_type == "privacy" {
        "Privacy Policy"
    } else {
        "Terms of Service"
    };

    let (doc_id, title, content, status, updated_at) = match &doc {
        Some(d) => {
            let t = d.str_field("title");
            let title = if t.is_empty() { default_title } else { t };
            (
                d.id.as_str(),
                title,
                d.str_field("content"),
                d.str_field("status"),
                d.str_field("updated_at"),
            )
        }
        None => ("", default_title, "", "none", ""),
    };

    let badge_class = match status {
        "published" => "badge-success",
        "draft" => "badge-warning",
        _ => "badge-info",
    };
    let badge_text = match status {
        "published" => "Published",
        "draft" => "Draft",
        "archived" => "Archived",
        _ => "No document",
    };

    let page_content = html! {
        (components::page_header(default_title, Some("Edit and publish your legal document"), None))

        // Status bar
        div style="display:flex;align-items:center;gap:0.5rem;margin-bottom:1rem" {
            span .text-muted { "Status:" }
            span #status-badge .badge .(badge_class) { (badge_text) }
            @if !updated_at.is_empty() {
                span .text-muted style="font-size:0.85rem" {
                    " \u{00b7} Last updated: " (updated_at.get(..19).unwrap_or(updated_at))
                }
            }
        }

        // Title input
        div style="margin-bottom:1rem" {
            label .form-label for="title-input" { "Title" }
            input #title-input .form-input
                type="text"
                name="title"
                value=(title)
                placeholder="Document title";
        }

        // Hidden fields
        input #doc-type type="hidden" value=(doc_type);
        input #doc-id type="hidden" value=(doc_id);

        // Quill editor container with custom styling
        style { (PreEscaped(EDITOR_CSS)) }
        div .card .editor-card {
            div #editor { (PreEscaped(content)) }
        }

        // Action buttons
        div style="display:flex;gap:0.5rem;margin-top:1rem" {
            button #btn-save .btn .btn-secondary onclick="saveDocument(false)" {
                "Save Draft"
            }
            button #btn-publish .btn .btn-primary onclick="saveDocument(true)" {
                "Publish"
            }
            a .btn .btn-ghost
                href={"/b/legalpages/" (doc_type)}
                target="_blank"
            {
                (icons::eye()) " Preview"
            }
        }

        // Editor initialization script (loads Quill dynamically)
        script { (PreEscaped(EDITOR_JS)) }
    };

    legalpages_page(
        default_title,
        &config,
        &format!("/b/legalpages/admin/{doc_type}"),
        user.as_ref(),
        page_content,
        msg,
    )
}

const EDITOR_CSS: &str = r#"
.editor-card { padding: 0; overflow: hidden; }
.editor-card .ql-toolbar { border: none; border-bottom: 1px solid var(--border-color, #e2e8f0); background: var(--bg-secondary, #f8fafc); }
.editor-card .ql-container { border: none; font-size: 1rem; min-height: 400px; }
.editor-card .ql-editor { min-height: 400px; line-height: 1.7; padding: 1.25rem; }
.editor-card .ql-editor h1 { font-size: 1.75rem; margin-bottom: 0.5rem; }
.editor-card .ql-editor h2 { font-size: 1.4rem; margin: 1.5rem 0 0.5rem; }
.editor-card .ql-editor h3 { font-size: 1.15rem; margin: 1rem 0 0.5rem; }
.editor-card .ql-editor p { margin-bottom: 0.75rem; }
.editor-card .ql-editor blockquote { border-left: 3px solid var(--border-color, #e2e8f0); padding-left: 1rem; color: var(--text-muted, #64748b); }
"#;

const EDITOR_JS: &str = r#"
(function() {
    function loadQuill(cb) {
        if (!document.querySelector('link[href*="quill.snow"]')) {
            var link = document.createElement('link');
            link.rel = 'stylesheet';
            link.href = 'https://cdn.jsdelivr.net/npm/quill@2/dist/quill.snow.css';
            document.head.appendChild(link);
        }
        if (typeof Quill !== 'undefined') { cb(); return; }
        var s = document.createElement('script');
        s.src = 'https://cdn.jsdelivr.net/npm/quill@2/dist/quill.js';
        s.onload = cb;
        document.head.appendChild(s);
    }

    loadQuill(function() {
        window._quill = new Quill('#editor', {
            theme: 'snow',
            modules: {
                toolbar: [
                    [{ 'header': [1, 2, 3, false] }],
                    ['bold', 'italic', 'underline', 'strike'],
                    [{ 'list': 'ordered' }, { 'list': 'bullet' }],
                    ['blockquote', 'link'],
                    [{ 'align': [] }],
                    ['clean']
                ]
            },
            placeholder: 'Start writing your legal document...'
        });
    });

    window.saveDocument = function(publish) {
        if (!window._quill) return;
        var title = document.getElementById('title-input').value;
        var content = window._quill.getSemanticHTML();
        var docType = document.getElementById('doc-type').value;
        var docId = document.getElementById('doc-id').value;
        var url = publish ? '/b/legalpages/admin/publish' : '/b/legalpages/admin/save';

        var btn = document.getElementById(publish ? 'btn-publish' : 'btn-save');
        var origText = btn.textContent;
        btn.disabled = true;
        btn.textContent = publish ? 'Publishing...' : 'Saving...';

        fetch(url, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ doc_type: docType, title: title, content: content, doc_id: docId })
        })
        .then(function(r) { return r.json(); })
        .then(function(data) {
            document.body.dispatchEvent(new CustomEvent('showToast', {
                detail: { message: data.message || (data.error || 'Done'), type: data.error ? 'error' : 'success' }
            }));
            if (data.doc_id) document.getElementById('doc-id').value = data.doc_id;
            if (data.status) {
                var badge = document.getElementById('status-badge');
                if (badge) {
                    badge.className = 'badge ' + (data.status === 'published' ? 'badge-success' : 'badge-warning');
                    badge.textContent = data.status.charAt(0).toUpperCase() + data.status.slice(1);
                }
            }
        })
        .catch(function(err) {
            document.body.dispatchEvent(new CustomEvent('showToast', {
                detail: { message: 'Error: ' + err.message, type: 'error' }
            }));
        })
        .finally(function() {
            btn.disabled = false;
            btn.textContent = origText;
        });
    };
})();
"#;

// ---------------------------------------------------------------------------
// Endpoints page
// ---------------------------------------------------------------------------

pub async fn endpoints_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let content = html! {
        (components::page_header("API Endpoints", Some("Available endpoints for legal pages"), None))

        // Public endpoints
        h3 style="font-size:1rem;font-weight:600;margin-bottom:0.5rem" { "Public Endpoints" }
        p .text-muted style="font-size:0.875rem;margin-bottom:0.75rem" {
            "These endpoints are publicly accessible and return formatted HTML pages."
        }
        div .table-container style="margin-bottom:2rem" {
            table .table {
                thead {
                    tr {
                        th style="width:80px" { "Method" }
                        th { "Endpoint" }
                        th { "Description" }
                    }
                }
                tbody {
                    tr {
                        td { span .badge .badge-success { "GET" } }
                        td { code { "/b/legalpages/terms" } }
                        td { "View published Terms of Service page" }
                    }
                    tr {
                        td { span .badge .badge-success { "GET" } }
                        td { code { "/b/legalpages/privacy" } }
                        td { "View published Privacy Policy page" }
                    }
                }
            }
        }

        // Admin API
        h3 style="font-size:1rem;font-weight:600;margin-bottom:0.5rem" { "Admin API Endpoints" }
        p .text-muted style="font-size:0.875rem;margin-bottom:0.75rem" {
            "These endpoints require admin authentication and return JSON responses."
        }
        div .table-container {
            table .table {
                thead {
                    tr {
                        th style="width:80px" { "Method" }
                        th { "Endpoint" }
                        th { "Description" }
                    }
                }
                tbody {
                    tr {
                        td { span .badge .badge-success { "GET" } }
                        td { code { "/admin/legalpages/documents" } }
                        td { "List all documents (supports " code { "?type=terms|privacy" } " filter)" }
                    }
                    tr {
                        td { span .badge .badge-info { "POST" } }
                        td { code { "/admin/legalpages/documents" } }
                        td { "Create a new document " span .text-muted { "(body: doc_type, title, content)" } }
                    }
                    tr {
                        td { span .badge .badge-warning { "PATCH" } }
                        td { code { "/admin/legalpages/documents/:id" } }
                        td { "Update a document" }
                    }
                    tr {
                        td { span .badge .badge-info { "POST" } }
                        td { code { "/admin/legalpages/documents/:id/publish" } }
                        td { "Publish a document (archives previous published version)" }
                    }
                    tr {
                        td { span .badge .badge-danger { "DELETE" } }
                        td { code { "/admin/legalpages/documents/:id" } }
                        td { "Delete a document" }
                    }
                }
            }
        }

        // Document schema
        h3 style="font-size:1rem;font-weight:600;margin:2rem 0 0.5rem" { "Document Schema" }
        p .text-muted style="font-size:0.875rem;margin-bottom:0.75rem" {
            "Each legal document has the following fields."
        }
        div .table-container {
            table .table {
                thead {
                    tr {
                        th { "Field" }
                        th { "Type" }
                        th { "Description" }
                    }
                }
                tbody {
                    tr {
                        td { code { "doc_type" } }
                        td { "string" }
                        td { "Document type: " code { "terms" } " or " code { "privacy" } }
                    }
                    tr {
                        td { code { "title" } }
                        td { "string" }
                        td { "Document title" }
                    }
                    tr {
                        td { code { "content" } }
                        td { "text" }
                        td { "HTML content of the document" }
                    }
                    tr {
                        td { code { "status" } }
                        td { "string" }
                        td {
                            "Document status: "
                            span .badge .badge-warning { "draft" }
                            " "
                            span .badge .badge-success { "published" }
                            " "
                            span .badge { "archived" }
                        }
                    }
                    tr {
                        td { code { "version" } }
                        td { "int" }
                        td { "Version number" }
                    }
                    tr {
                        td { code { "published_at" } }
                        td { "datetime" }
                        td { "When the document was last published" }
                    }
                }
            }
        }
    };

    legalpages_page(
        "Endpoints",
        &config,
        "/b/legalpages/admin/endpoints",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Save / Publish handlers
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct SaveRequest {
    doc_type: String,
    title: String,
    content: String,
    #[serde(default)]
    doc_id: String,
}

/// Save a draft document. If the current doc is published, creates a new draft
/// so the live version stays untouched until the admin explicitly publishes.
pub async fn handle_save(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body: SaveRequest = match msg.decode() {
        Ok(b) => b,
        Err(e) => {
            return json_respond(
                msg,
                &serde_json::json!({"error": format!("Invalid request: {e}")}),
            )
        }
    };

    let now = helpers::now_rfc3339();

    // If editing a published document, create a new draft instead of modifying the live version
    let should_create_new = if body.doc_id.is_empty() {
        true
    } else {
        match db::get(ctx, COLLECTION, &body.doc_id).await {
            Ok(doc) => {
                doc.data
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    == "published"
            }
            Err(_) => true,
        }
    };

    if should_create_new {
        let data = json_map(serde_json::json!({
            "doc_type": body.doc_type,
            "title": body.title,
            "content": body.content,
            "status": "draft",
            "version": 1,
            "created_at": now,
            "updated_at": now,
            "created_by": msg.user_id()
        }));
        match db::create(ctx, COLLECTION, data).await {
            Ok(record) => json_respond(
                msg,
                &serde_json::json!({
                    "doc_id": record.id,
                    "status": "draft",
                    "message": "Draft saved"
                }),
            ),
            Err(e) => json_respond(
                msg,
                &serde_json::json!({"error": format!("Failed to save: {e}")}),
            ),
        }
    } else {
        let data = json_map(serde_json::json!({
            "title": body.title,
            "content": body.content,
            "updated_at": now
        }));
        match db::update(ctx, COLLECTION, &body.doc_id, data).await {
            Ok(_) => json_respond(
                msg,
                &serde_json::json!({
                    "doc_id": body.doc_id,
                    "status": "draft",
                    "message": "Draft saved"
                }),
            ),
            Err(e) => json_respond(
                msg,
                &serde_json::json!({"error": format!("Failed to save: {e}")}),
            ),
        }
    }
}

/// Save and publish a document. Archives any previously published document
/// of the same type.
pub async fn handle_publish(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body: SaveRequest = match msg.decode() {
        Ok(b) => b,
        Err(e) => {
            return json_respond(
                msg,
                &serde_json::json!({"error": format!("Invalid request: {e}")}),
            )
        }
    };

    let now = helpers::now_rfc3339();
    let doc_id_for_archive = if body.doc_id.is_empty() {
        ""
    } else {
        &body.doc_id
    };

    // Archive existing published documents of this type (except the one we're about to publish)
    archive_published(ctx, &body.doc_type, doc_id_for_archive).await;

    if !body.doc_id.is_empty() {
        // Update existing document and publish
        let data = json_map(serde_json::json!({
            "title": body.title,
            "content": body.content,
            "status": "published",
            "published_at": now,
            "updated_at": now
        }));
        match db::update(ctx, COLLECTION, &body.doc_id, data).await {
            Ok(_) => json_respond(
                msg,
                &serde_json::json!({
                    "doc_id": body.doc_id,
                    "status": "published",
                    "message": "Document published"
                }),
            ),
            Err(e) => json_respond(
                msg,
                &serde_json::json!({"error": format!("Failed to publish: {e}")}),
            ),
        }
    } else {
        // Create new published document
        let data = json_map(serde_json::json!({
            "doc_type": body.doc_type,
            "title": body.title,
            "content": body.content,
            "status": "published",
            "version": 1,
            "created_at": now,
            "updated_at": now,
            "published_at": now,
            "created_by": msg.user_id()
        }));
        match db::create(ctx, COLLECTION, data).await {
            Ok(record) => json_respond(
                msg,
                &serde_json::json!({
                    "doc_id": record.id,
                    "status": "published",
                    "message": "Document published"
                }),
            ),
            Err(e) => json_respond(
                msg,
                &serde_json::json!({"error": format!("Failed to publish: {e}")}),
            ),
        }
    }
}

/// Archive all published documents of a given type, except the specified ID.
async fn archive_published(ctx: &dyn Context, doc_type: &str, except_id: &str) {
    let existing = db::list_all(
        ctx,
        COLLECTION,
        vec![
            Filter {
                field: "doc_type".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!(doc_type),
            },
            Filter {
                field: "status".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!("published"),
            },
        ],
    )
    .await;
    if let Ok(records) = existing {
        for r in records {
            if r.id == except_id {
                continue;
            }
            let upd = json_map(serde_json::json!({"status": "archived"}));
            let _ = db::update(ctx, COLLECTION, &r.id, upd).await;
        }
    }
}
