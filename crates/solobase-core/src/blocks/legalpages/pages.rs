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
            label: "Settings".into(),
            href: "/b/legalpages/admin/settings".into(),
            icon: "settings",
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
    let markup = ui::layout::block_shell(title, config, &nav(), user, path, content, is_fragment);
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

    let (doc_id, title, content, status, updated_at, version) = match &doc {
        Some(d) => {
            let t = d.str_field("title");
            let title = if t.is_empty() { default_title } else { t };
            let ver = d
                .data
                .get("version")
                .and_then(|v| {
                    v.as_i64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
                .unwrap_or(1);
            (
                d.id.as_str(),
                title,
                d.str_field("content"),
                d.str_field("status"),
                d.str_field("updated_at"),
                ver,
            )
        }
        None => ("", default_title, "", "none", "", 1),
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
        // Status bar (compact, top of page)
        div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:0.75rem" {
            div style="display:flex;align-items:center;gap:0.5rem" {
                h2 style="font-size:1.25rem;font-weight:600;margin:0" { (default_title) }
                span #status-badge .badge .(badge_class) { (badge_text) }
                span .badge style="font-size:0.7rem;background:#f1f5f9;color:#64748b;cursor:pointer"
                    title="Click to change version"
                    onclick="promptVersion()"
                { "v" span #version-display { (version) } }
                @if !updated_at.is_empty() {
                    span .text-muted style="font-size:0.8rem" {
                        " \u{00b7} " (updated_at.get(..10).unwrap_or(updated_at))
                    }
                }
            }
            div style="display:flex;gap:0.5rem" {
                a .btn .btn-sm .btn-ghost
                    href={"/b/legalpages/" (doc_type)}
                    target="_blank"
                {
                    (icons::eye()) " Preview"
                }
                button #btn-save .btn .btn-sm .btn-secondary onclick="saveDocument(false)" {
                    "Save Draft"
                }
                button #btn-publish .btn .btn-sm .btn-primary onclick="saveDocument(true)" {
                    "Publish"
                }
            }
        }

        // Title input (streamlined)
        input #title-input .form-input
            type="text"
            name="title"
            value=(title)
            placeholder="Document title"
            style="font-size:1.1rem;font-weight:600;margin-bottom:0.5rem";

        // Hidden fields
        input #doc-type type="hidden" value=(doc_type);
        input #doc-id type="hidden" value=(doc_id);
        input #doc-version type="hidden" value=(version);

        // Editor with built-in toolbar
        style { (PreEscaped(EDITOR_CSS)) }
        div .card .editor-card {
            // Toolbar
            div .editor-toolbar {
                // Block format
                select #format-block .toolbar-select title="Paragraph format"
                    onchange="editorCmd('formatBlock', this.value); this.value=''"
                {
                    option value="" disabled selected { "Format" }
                    option value="p" { "Paragraph" }
                    option value="h1" { "Heading 1" }
                    option value="h2" { "Heading 2" }
                    option value="h3" { "Heading 3" }
                    option value="blockquote" { "Quote" }
                    option value="pre" { "Code" }
                }
                span .toolbar-sep {}

                // Inline formatting
                button .toolbar-btn title="Bold (Ctrl+B)" onclick="editorCmd('bold')" { "B" }
                button .toolbar-btn title="Italic (Ctrl+I)" onclick="editorCmd('italic')" style="font-style:italic" { "I" }
                button .toolbar-btn title="Underline (Ctrl+U)" onclick="editorCmd('underline')" style="text-decoration:underline" { "U" }
                button .toolbar-btn title="Strikethrough" onclick="editorCmd('strikeThrough')" style="text-decoration:line-through" { "S" }
                span .toolbar-sep {}

                // Lists
                button .toolbar-btn title="Ordered list" onclick="editorCmd('insertOrderedList')" { "1." }
                button .toolbar-btn title="Bullet list" onclick="editorCmd('insertUnorderedList')" { "\u{2022}" }
                button .toolbar-btn title="Indent" onclick="editorCmd('indent')" { "\u{2192}" }
                button .toolbar-btn title="Outdent" onclick="editorCmd('outdent')" { "\u{2190}" }
                span .toolbar-sep {}

                // Insert
                button .toolbar-btn title="Insert link" onclick="insertLink()" { "\u{1f517}" }
                button .toolbar-btn title="Horizontal rule" onclick="editorCmd('insertHorizontalRule')" { "\u{2015}" }
                span .toolbar-sep {}

                // Undo/redo
                button .toolbar-btn title="Undo (Ctrl+Z)" onclick="editorCmd('undo')" { "\u{21a9}" }
                button .toolbar-btn title="Redo (Ctrl+Y)" onclick="editorCmd('redo')" { "\u{21aa}" }
                button .toolbar-btn title="Clear formatting" onclick="editorCmd('removeFormat')" { "\u{2718}" }
            }

            // Editable content area
            div #editor .editor-content contenteditable="true" { (PreEscaped(content)) }
        }

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
.editor-card { padding: 0; overflow: hidden; display: flex; flex-direction: column; }
.editor-toolbar {
    display: flex; flex-wrap: wrap; align-items: center; gap: 2px;
    padding: 6px 8px; border-bottom: 1px solid var(--border-color, #e2e8f0);
    background: var(--bg-secondary, #f8fafc); position: sticky; top: 0; z-index: 5;
}
.toolbar-btn {
    background: none; border: 1px solid transparent; border-radius: 4px;
    padding: 4px 8px; cursor: pointer; font-size: 14px; font-weight: 600;
    color: #475569; min-width: 30px; text-align: center; line-height: 1.2;
}
.toolbar-btn:hover { background: #e2e8f0; border-color: #cbd5e1; }
.toolbar-select {
    background: none; border: 1px solid transparent; border-radius: 4px;
    padding: 4px 6px; cursor: pointer; font-size: 13px; color: #475569;
    appearance: auto;
}
.toolbar-select:hover { background: #e2e8f0; border-color: #cbd5e1; }
.toolbar-sep { width: 1px; height: 20px; background: #e2e8f0; margin: 0 4px; }
.editor-content {
    min-height: 500px; padding: 1.5rem; font-size: 1rem; line-height: 1.8;
    outline: none; font-family: Georgia, 'Times New Roman', serif;
    overflow-y: auto; flex: 1;
}
.editor-content:focus { box-shadow: inset 0 0 0 2px rgba(99, 102, 241, 0.15); }
.editor-content h1 { font-size: 1.75rem; margin: 0 0 0.5rem; font-weight: 700; }
.editor-content h2 { font-size: 1.4rem; margin: 1.5rem 0 0.5rem; font-weight: 600; }
.editor-content h3 { font-size: 1.15rem; margin: 1rem 0 0.5rem; font-weight: 600; }
.editor-content p { margin-bottom: 0.75rem; }
.editor-content blockquote { border-left: 3px solid #e2e8f0; padding-left: 1rem; color: #64748b; margin: 1rem 0; }
.editor-content pre { background: #f1f5f9; padding: 1rem; border-radius: 6px; font-family: monospace; font-size: 0.9rem; overflow-x: auto; margin: 1rem 0; }
.editor-content ul, .editor-content ol { margin: 0.5rem 0 1rem 1.5rem; }
.editor-content li { margin-bottom: 0.25rem; }
.editor-content a { color: #6366f1; }
.editor-content hr { border: none; border-top: 1px solid #e2e8f0; margin: 1.5rem 0; }
.editor-content table { width: 100%; border-collapse: collapse; margin: 1rem 0; }
.editor-content th, .editor-content td { padding: 0.5rem; text-align: left; border: 1px solid #e2e8f0; }
.editor-content [data-placeholder]:empty::before {
    content: attr(data-placeholder); color: #94a3b8; pointer-events: none;
}
"#;

const EDITOR_JS: &str = r#"
(function() {
    var editor = document.getElementById('editor');

    // Placeholder support
    if (!editor.innerHTML.trim()) {
        editor.setAttribute('data-placeholder', 'Start writing your legal document...');
    }
    editor.addEventListener('input', function() {
        if (editor.innerHTML.trim()) editor.removeAttribute('data-placeholder');
        else editor.setAttribute('data-placeholder', 'Start writing your legal document...');
    });

    // Ensure default paragraph on Enter (not <div>)
    document.execCommand('defaultParagraphSeparator', false, 'p');

    // Editor commands
    window.editorCmd = function(cmd, val) {
        editor.focus();
        if (cmd === 'formatBlock') {
            document.execCommand('formatBlock', false, '<' + val + '>');
        } else {
            document.execCommand(cmd, false, val || null);
        }
    };

    window.insertLink = function() {
        var sel = window.getSelection();
        var text = sel.toString();
        var url = prompt('Enter URL:', 'https://');
        if (url) {
            if (!text) {
                document.execCommand('insertHTML', false, '<a href="' + url + '">' + url + '</a>');
            } else {
                document.execCommand('createLink', false, url);
            }
        }
    };

    window.promptVersion = function() {
        var current = document.getElementById('doc-version').value;
        var v = prompt('Set version number:', current);
        if (v !== null && v.trim() !== '') {
            var num = parseInt(v, 10);
            if (num > 0) {
                document.getElementById('doc-version').value = num;
                document.getElementById('version-display').textContent = num;
            }
        }
    };

    // Keyboard shortcuts
    editor.addEventListener('keydown', function(e) {
        if (e.ctrlKey || e.metaKey) {
            switch (e.key) {
                case 's': e.preventDefault(); saveDocument(false); break;
            }
        }
    });

    // Save handler
    window.saveDocument = function(publish) {
        var title = document.getElementById('title-input').value;
        var content = editor.innerHTML;
        var docType = document.getElementById('doc-type').value;
        var docId = document.getElementById('doc-id').value;
        var version = parseInt(document.getElementById('doc-version').value, 10) || 0;
        var url = publish ? '/b/legalpages/admin/publish' : '/b/legalpages/admin/save';

        var btn = document.getElementById(publish ? 'btn-publish' : 'btn-save');
        var origText = btn.textContent;
        btn.disabled = true;
        btn.textContent = publish ? 'Publishing...' : 'Saving...';

        fetch(url, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ doc_type: docType, title: title, content: content, doc_id: docId, version: version })
        })
        .then(function(r) { return r.json(); })
        .then(function(data) {
            document.body.dispatchEvent(new CustomEvent('showToast', {
                detail: { message: data.message || (data.error || 'Done'), type: data.error ? 'error' : 'success' }
            }));
            if (data.doc_id) document.getElementById('doc-id').value = data.doc_id;
            if (data.version) {
                document.getElementById('doc-version').value = data.version;
                document.getElementById('version-display').textContent = data.version;
            }
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
                        td { code { "/b/legalpages/api/documents" } }
                        td { "List all documents (supports " code { "?type=terms|privacy" } " filter)" }
                    }
                    tr {
                        td { span .badge .badge-info { "POST" } }
                        td { code { "/b/legalpages/api/documents" } }
                        td { "Create a new document " span .text-muted { "(body: doc_type, title, content)" } }
                    }
                    tr {
                        td { span .badge .badge-warning { "PATCH" } }
                        td { code { "/b/legalpages/api/documents/:id" } }
                        td { "Update a document" }
                    }
                    tr {
                        td { span .badge .badge-info { "POST" } }
                        td { code { "/b/legalpages/api/documents/:id/publish" } }
                        td { "Publish a document (archives previous published version)" }
                    }
                    tr {
                        td { span .badge .badge-danger { "DELETE" } }
                        td { code { "/b/legalpages/api/documents/:id" } }
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
    #[serde(default)]
    version: i64,
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

    // Use user-provided version if > 0, otherwise auto-increment
    let next_version = if body.version > 0 {
        body.version
    } else {
        let opts = ListOptions {
            filters: vec![Filter {
                field: "doc_type".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!(&body.doc_type),
            }],
            sort: vec![SortField {
                field: "version".into(),
                desc: true,
            }],
            limit: 1,
            ..Default::default()
        };
        db::list(ctx, COLLECTION, &opts)
            .await
            .ok()
            .and_then(|r| {
                r.records.first().and_then(|r| {
                    let v = r.data.get("version")?;
                    v.as_i64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
            })
            .unwrap_or(0)
            + 1
    };

    // Archive existing published documents of this type (except the one we're about to publish)
    archive_published(ctx, &body.doc_type, doc_id_for_archive).await;

    if !body.doc_id.is_empty() {
        // Update existing document and publish
        let data = json_map(serde_json::json!({
            "title": body.title,
            "content": body.content,
            "status": "published",
            "version": next_version,
            "published_at": now,
            "updated_at": now
        }));
        match db::update(ctx, COLLECTION, &body.doc_id, data).await {
            Ok(_) => json_respond(
                msg,
                &serde_json::json!({
                    "doc_id": body.doc_id,
                    "status": "published",
                    "version": next_version,
                    "message": format!("Published as v{}", next_version)
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
            "version": next_version,
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
                    "version": next_version,
                    "message": format!("Published as v{}", next_version)
                }),
            ),
            Err(e) => json_respond(
                msg,
                &serde_json::json!({"error": format!("Failed to publish: {e}")}),
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Settings page
// ---------------------------------------------------------------------------

const SETTINGS_KEYS: &[(&str, &str, &str, &str)] = &[
    (
        "SUPPERS_AI__LEGALPAGES__BG_COLOR",
        "Background Color",
        "Background color for the public legal pages.",
        "#f8fafc",
    ),
    (
        "SUPPERS_AI__LEGALPAGES__BACK_URL",
        "Back Button URL",
        "Where the back arrow in the header links to (e.g., your website homepage).",
        "/",
    ),
    (
        "SUPPERS_AI__LEGALPAGES__FOOTER",
        "Footer Text",
        "Custom footer text (HTML allowed). Leave empty for default copyright.",
        "",
    ),
];

pub async fn settings_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    use wafer_core::clients::config;

    let site_config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    // Load current values
    let mut values = Vec::new();
    for &(key, label, help, default) in SETTINGS_KEYS {
        let value = config::get_default(ctx, key, default).await;
        values.push((key, label, help, default, value));
    }

    let saved = msg.query("saved") == "1";

    let content = html! {
        (components::page_header("Settings", Some("Customize the public legal pages appearance"), None))

        @if saved {
            div .alert .alert-success style="margin-bottom:1rem" {
                "Settings saved successfully."
            }
        }

        form #settings-form onsubmit="return submitSettings(event)" {
            @for (key, label, help, _default, value) in &values {
                div .form-group style="margin-bottom:1.25rem" {
                    label .form-label for=(key) { (label) }
                    @if *key == "SUPPERS_AI__LEGALPAGES__FOOTER" {
                        textarea .form-input #(key) name=(key)
                            rows="3"
                            placeholder="Leave empty for default copyright text"
                            style="font-family:monospace;font-size:0.9rem"
                        { (value) }
                    } @else if *key == "SUPPERS_AI__LEGALPAGES__BG_COLOR" {
                        div style="display:flex;align-items:center;gap:0.75rem" {
                            input .form-input #(key) name=(key)
                                type="text"
                                value=(value)
                                style="flex:1";
                            input type="color" value=(value)
                                style="width:40px;height:36px;border:1px solid #e2e8f0;border-radius:6px;cursor:pointer;padding:2px"
                                onchange={"document.getElementById('" (key) "').value=this.value"};
                        }
                    } @else {
                        input .form-input #(key) name=(key)
                            type="text"
                            value=(value)
                            placeholder=(*_default);
                    }
                    p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (help) }
                }
            }

            // Preview section
            div .card style="margin-bottom:1.25rem;padding:1rem" {
                h4 style="font-size:0.9rem;font-weight:600;margin-bottom:0.5rem" { "Preview" }
                p .text-muted style="font-size:0.8rem;margin-bottom:0.75rem" {
                    "See how your changes look on the public pages."
                }
                div style="display:flex;gap:0.5rem" {
                    a .btn .btn-sm .btn-ghost href="/b/legalpages/privacy" target="_blank" {
                        (icons::eye()) " Privacy Policy"
                    }
                    a .btn .btn-sm .btn-ghost href="/b/legalpages/terms" target="_blank" {
                        (icons::eye()) " Terms of Service"
                    }
                }
            }

            button .btn .btn-primary type="submit" { "Save Settings" }
        }

        script { (PreEscaped(r#"
        function submitSettings(e) {
            e.preventDefault();
            var form = document.getElementById('settings-form');
            var data = {};
            var inputs = form.querySelectorAll('input[name], textarea[name]');
            inputs.forEach(function(el) { data[el.name] = el.value; });
            var btn = form.querySelector('button[type="submit"]');
            btn.disabled = true; btn.textContent = 'Saving...';
            fetch('/b/legalpages/admin/settings', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(data)
            })
            .then(function(r) { return r.json(); })
            .then(function(d) {
                document.body.dispatchEvent(new CustomEvent('showToast', {
                    detail: { message: d.message || 'Saved', type: d.error ? 'error' : 'success' }
                }));
            })
            .catch(function(err) {
                document.body.dispatchEvent(new CustomEvent('showToast', {
                    detail: { message: 'Error: ' + err.message, type: 'error' }
                }));
            })
            .finally(function() { btn.disabled = false; btn.textContent = 'Save Settings'; });
            return false;
        }
        "#)) }
    };

    legalpages_page(
        "Settings",
        &site_config,
        "/b/legalpages/admin/settings",
        user.as_ref(),
        content,
        msg,
    )
}

pub async fn handle_save_settings(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    use wafer_core::clients::config;

    let body: std::collections::HashMap<String, String> = match msg.decode() {
        Ok(b) => b,
        Err(e) => {
            return json_respond(
                msg,
                &serde_json::json!({"error": format!("Invalid request: {e}")}),
            )
        }
    };

    for &(key, _, _, _) in SETTINGS_KEYS {
        if let Some(value) = body.get(key) {
            let _ = config::set(ctx, key, value).await;
        }
    }

    json_respond(msg, &serde_json::json!({"message": "Settings saved"}))
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
