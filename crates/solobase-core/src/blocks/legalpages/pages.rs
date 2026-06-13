//! SSR admin pages for the legal pages block.
//!
//! Provides a tabbed admin UI with:
//! - Privacy Policy editor (Quill rich text editor)
//! - Terms of Service editor (Quill rich text editor)
//! - API endpoints reference

use maud::{html, Markup, PreEscaped};
use wafer_block::db::{Filter, FilterOp, ListOptions, SortField};
use wafer_core::clients::database as db;
use wafer_run::{context::Context, ErrorCode, InputStream, Message, OutputStream};

use super::service;
use crate::{
    blocks::helpers::{
        self, err_bad_request, err_internal, err_not_found, json_map, ok_json, RecordExt,
        ResponseBuilder,
    },
    ui::{
        components, icons, nav_groups,
        shell::{Crumb, Topbar},
        SiteConfig, UserInfo,
    },
};

const COLLECTION: &str = super::COLLECTION;

/// Wrap content in the legalpages portal shell (sidebar + layout).
fn legalpages_page<'a>(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    crumb_label: &'a str,
    content: Markup,
    msg: &Message,
) -> OutputStream {
    let groups = nav_groups::portal();
    let topbar = Topbar {
        crumbs: vec![Crumb {
            label: crumb_label,
            href: None,
        }],
        primary_action: None,
        subtitle: None,
        show_palette: true,
    };
    crate::ui::Page {
        config,
        title,
        nav: &groups,
        user,
        current_path: path,
        topbar,
        body: content,
    }
    .response(msg)
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

pub async fn editor_page(ctx: &dyn Context, msg: &Message, doc_type: &str) -> OutputStream {
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
            let ver = super::service::doc_version(d).unwrap_or(1);
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

    let page_content = editor_markup_for_test(
        doc_type, doc_id, title, content, status, updated_at, version,
    );

    legalpages_page(
        default_title,
        &config,
        &format!("/b/legalpages/admin/{doc_type}"),
        user.as_ref(),
        default_title,
        page_content,
        msg,
    )
}

/// Build the editor markup. Split out from `editor_page` so it can be
/// unit-tested without a `Context`.
pub(super) fn editor_markup_for_test(
    doc_type: &str,
    doc_id: &str,
    title: &str,
    content: &str,
    status: &str,
    updated_at: &str,
    version: i64,
) -> Markup {
    let default_title = if doc_type == "privacy" {
        "Privacy Policy"
    } else {
        "Terms of Service"
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

    html! {
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
                    "Open public page"
                }
                button #btn-save .btn .btn-sm .btn-secondary onclick="saveDocument(false)" {
                    "Save Draft"
                }
                button #btn-publish .btn .btn-sm .btn-primary onclick="saveDocument(true)" {
                    "Publish"
                }
            }
        }

        // Title input
        input #title-input .form-input
            type="text"
            name="title"
            value=(title)
            placeholder="Document title"
            style="font-size:1.1rem;font-weight:600;margin-bottom:0.5rem";

        // Hidden fields used by save handler JS
        input #doc-type type="hidden" value=(doc_type);
        input #doc-id type="hidden" value=(doc_id);
        input #doc-version type="hidden" value=(version);

        // Tab strip
        style { (PreEscaped(EDITOR_CSS)) }
        div .editor-tabs {
            button .editor-tab .editor-tab--active type="button"
                data-tab="edit"
                onclick="setEditorTab('edit')"
            { "Edit" }
            button .editor-tab type="button"
                data-tab="preview"
                onclick="setEditorTab('preview')"
            { "Preview" }
        }

        // Edit pane (textarea)
        div #editor-edit-pane .editor-pane {
            textarea #editor .form-input .editor-textarea
                name="content"
                placeholder="Write your legal document in Markdown..."
            { (content) }
        }

        // Preview pane (vanilla JS fetch target)
        div #editor-preview-pane .editor-pane style="display:none" {
            div #editor-preview .preview-content {
                p .text-muted { "Click Preview above to render." }
            }
        }

        script { (PreEscaped(EDITOR_JS)) }
    }
}

const EDITOR_CSS: &str = r#"
.editor-tabs {
    display: flex; gap: 4px; margin-bottom: -1px; border-bottom: 1px solid #e2e8f0;
}
.editor-tab {
    background: none; border: 1px solid transparent; border-bottom: none;
    border-radius: 6px 6px 0 0; padding: 6px 14px; cursor: pointer;
    font-size: 0.875rem; color: #64748b;
}
.editor-tab:hover { background: #f1f5f9; color: #1e293b; }
.editor-tab--active {
    background: white; border-color: #e2e8f0; color: #1e293b; font-weight: 600;
}
.editor-pane {
    background: white; border: 1px solid #e2e8f0; border-radius: 0 6px 6px 6px;
    min-height: 500px;
}
.editor-textarea {
    width: 100%; min-height: 500px; padding: 1rem;
    border: none; outline: none; resize: vertical;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.9rem; line-height: 1.6; background: transparent;
}
.preview-content {
    padding: 1.5rem; min-height: 500px; font-family: Georgia, 'Times New Roman', serif;
    line-height: 1.8;
}
"#;

const EDITOR_JS: &str = r#"
(function() {
    // Preview wiring: vanilla JS fetch (no json-enc htmx extension loaded)
    window.setEditorTab = function(name) {
        document.querySelectorAll('.editor-tab').forEach(function(t) {
            t.classList.toggle('editor-tab--active', t.dataset.tab === name);
        });
        document.getElementById('editor-edit-pane').style.display = (name === 'edit') ? '' : 'none';
        document.getElementById('editor-preview-pane').style.display = (name === 'preview') ? '' : 'none';
        if (name === 'preview') {
            var content = document.getElementById('editor').value;
            fetch('/b/legalpages/admin/render-preview', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content: content })
            })
            .then(function(r) {
                if (!r.ok) { throw new Error('HTTP ' + r.status); }
                return r.text();
            })
            .then(function(html) { document.getElementById('editor-preview').innerHTML = html; })
            .catch(function(err) {
                document.getElementById('editor-preview').innerHTML =
                    '<p style="color:#ef4444">Preview failed: ' + err.message + '</p>';
            });
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

    // Ctrl+S / Cmd+S → save draft
    document.addEventListener('keydown', function(e) {
        if ((e.ctrlKey || e.metaKey) && e.key === 's') {
            e.preventDefault();
            saveDocument(false);
        }
    });

    // Save handler (reads textarea .value)
    window.saveDocument = function(publish) {
        var title = document.getElementById('title-input').value;
        var content = document.getElementById('editor').value;
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

pub async fn endpoints_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
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
        "Endpoints",
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
pub async fn handle_save(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: SaveRequest = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        // Previously returned 200 OK with an `error` key — htmx clients
        // would still treat that as success. Use the proper 4xx so the
        // caller can branch on status alone.
        Err(e) => return err_bad_request(&format!("Invalid request: {e}")),
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
            Ok(record) => ok_json(&serde_json::json!({
                "doc_id": record.id,
                "status": "draft",
                "message": "Draft saved"
            })),
            Err(e) => err_internal("Failed to save legal-page draft", e),
        }
    } else {
        let data = json_map(serde_json::json!({
            "title": body.title,
            "content": body.content,
            "updated_at": now
        }));
        match db::update(ctx, COLLECTION, &body.doc_id, data).await {
            Ok(_) => ok_json(&serde_json::json!({
                "doc_id": body.doc_id,
                "status": "draft",
                "message": "Draft saved"
            })),
            Err(e) => err_internal("Failed to save legal-page draft", e),
        }
    }
}

/// Save and publish a document. Archives any previously published document
/// of the same type (publish-then-archive ordering lives in
/// `service::publish_document`).
pub async fn handle_publish(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: SaveRequest = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        // Previously returned 200 OK with an `error` key — clients would
        // still treat that as success. Use the proper 4xx so the caller
        // can branch on status alone (matches `handle_save`).
        Err(e) => return err_bad_request(&format!("Invalid request: {e}")),
    };

    let published = match service::publish_document(
        ctx,
        service::PublishRequest {
            doc_type: &body.doc_type,
            doc_id: &body.doc_id,
            title: Some(&body.title),
            content: Some(&body.content),
            version: body.version,
            created_by: msg.user_id(),
        },
    )
    .await
    {
        Ok(p) => p,
        Err(e) if e.code == ErrorCode::NotFound => return err_not_found("Document not found"),
        Err(e) => return err_internal("Failed to publish legal page", e),
    };

    ok_json(&serde_json::json!({
        "doc_id": published.record.id,
        "status": "published",
        "version": published.version,
        "message": format!("Published as v{}", published.version)
    }))
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

pub async fn settings_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
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
        "Settings",
        content,
        msg,
    )
}

pub async fn handle_save_settings(ctx: &dyn Context, input: InputStream) -> OutputStream {
    use wafer_core::clients::config;

    let raw = input.collect_to_bytes().await;
    let body: std::collections::HashMap<String, String> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid request: {e}")),
    };

    for &(key, _, _, _) in SETTINGS_KEYS {
        if let Some(value) = body.get(key) {
            if let Err(e) = config::set(ctx, key, value).await {
                tracing::warn!(error = %e, key = key, "legalpages: failed to set config value");
            }
        }
    }

    ok_json(&serde_json::json!({"message": "Settings saved"}))
}

// ---------------------------------------------------------------------------
// Preview rendering (used by editor's Preview tab via htmx)
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct PreviewRequest {
    content: String,
}

/// Render Markdown into the same `<div class="public-page__content">`
/// wrapper used by the live `/b/legalpages/{terms,privacy}` pages, so
/// the Preview tab in the editor matches production typography exactly.
pub(super) fn render_preview_fragment(markdown: &str) -> String {
    let rendered = super::markdown_to_html(markdown);
    format!(r#"<div class="public-page__content">{}</div>"#, rendered)
}

/// `POST /b/legalpages/admin/render-preview` — body: `{"content": "<markdown>"}`.
/// Returns the rendered HTML fragment for direct htmx swap into the
/// preview pane.
pub async fn handle_render_preview(_ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: PreviewRequest = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid request: {e}")),
    };
    let fragment = render_preview_fragment(&body.content);
    ResponseBuilder::new().body(fragment.into_bytes(), "text/html; charset=utf-8")
}
