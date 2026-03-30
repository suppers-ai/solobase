wit_bindgen::generate!({
    world: "wafer-block",
    path: "../../../wafer-run/wit/wit",
    additional_derives: [serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash],
    export_macro_name: "export_block",
});

use exports::wafer::block_world::block::Guest;
use wafer::block_world::types::*;

// wafer-core clients (use WASM sync variants via WIT call-block import)
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};

#[allow(dead_code)]
mod helpers;
use helpers::*;

struct LegalPagesBlockWasm;

const COLLECTION: &str = "block_legalpages_legal_documents";

impl Guest for LegalPagesBlockWasm {
    fn info() -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/legalpages".to_string(),
            version: "1.0.0".to_string(),
            interface: "http-handler@v1".to_string(),
            summary: "Legal pages management with versioning and publishing".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: Vec::new(),
            collections: Vec::new(),
            config_schema: None,
        }
    }

    fn handle(msg: Message) -> BlockResult {
        let action = msg_get_meta(&msg, "req.action").to_string();
        let path = msg_get_meta(&msg, "req.resource").to_string();

        match (action.as_str(), path.as_str()) {
            // Public endpoints
            ("retrieve", "/b/legalpages/terms") => handle_get_public(&msg, "terms"),
            ("retrieve", "/b/legalpages/privacy") => handle_get_public(&msg, "privacy"),
            // Admin UI
            ("retrieve", "/b/legalpages/admin") => handle_admin_ui(&msg),
            // Admin API
            ("retrieve", "/admin/legalpages/documents") => handle_admin_list(&msg),
            ("retrieve", p) if p.starts_with("/admin/legalpages/documents/") => handle_admin_get(&msg, p),
            ("create", "/admin/legalpages/documents") => handle_admin_create(&msg),
            ("update", p) if p.starts_with("/admin/legalpages/documents/") && p.ends_with("/publish") => {
                handle_admin_publish(&msg, p)
            }
            ("update", p) if p.starts_with("/admin/legalpages/documents/") => handle_admin_update(&msg, p),
            ("delete", p) if p.starts_with("/admin/legalpages/documents/") => handle_admin_delete(&msg, p),
            // ext API aliases (same as admin, but routed through admin-pipe)
            ("retrieve", "/b/legalpages/documents") => handle_admin_list(&msg),
            ("create", "/b/legalpages/documents") => handle_admin_create(&msg),
            _ => err_not_found(&msg, "not found"),
        }
    }

    fn lifecycle(_event: LifecycleEvent) -> Result<(), WaferError> {
        // Lifecycle (admin seeding) is handled by the native runtime.
        Ok(())
    }
}

export_block!(LegalPagesBlockWasm);

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Extract document ID from path like `/admin/legalpages/documents/{id}` or
/// `/admin/legalpages/documents/{id}/publish`.
fn extract_doc_id<'a>(msg: &'a Message, path: &'a str) -> &'a str {
    // Try router-extracted var first (native axum), fall back to path parsing (CF)
    let var = msg_get_meta(msg, "req.param.id");
    if !var.is_empty() {
        return var;
    }
    let suffix = path
        .strip_prefix("/admin/legalpages/documents/")
        .or_else(|| path.strip_prefix("/b/legalpages/documents/"))
        .unwrap_or("");
    // Strip trailing /publish or /
    suffix.split('/').next().unwrap_or("")
}

/// Extract pagination params from message query string.
fn pagination_params(msg: &Message) -> (usize, usize) {
    let page_str = msg_get_meta(msg, "req.query.page");
    let size_str = msg_get_meta(msg, "req.query.page_size");
    let page: usize = page_str.parse().unwrap_or(1).max(1);
    let page_size: usize = size_str.parse().unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;
    (page_size, offset)
}

// ---------------------------------------------------------------------------
// Handlers (sync — use wafer-core WASM client shims)
// ---------------------------------------------------------------------------

fn handle_get_public(msg: &Message, doc_type: &str) -> BlockResult {
    // Find published document of given type
    let opts = ListOptions {
        filters: vec![
            Filter {
                field: "doc_type".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(doc_type.to_string()),
            },
            Filter {
                field: "status".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String("published".to_string()),
            },
        ],
        sort: vec![SortField { field: "version".to_string(), desc: true }],
        limit: 1,
        ..Default::default()
    };

    let result = match db::list(COLLECTION, &opts) {
        Ok(r) => r,
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

    if result.records.is_empty() {
        let html = format!(
            "<html><body><h1>{}</h1><p>No {} document has been published yet.</p></body></html>",
            if doc_type == "terms" { "Terms of Service" } else { "Privacy Policy" },
            doc_type
        );
        return respond_html(msg, html.into_bytes());
    }

    let record = &result.records[0];
    let raw_content = record.data.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let content = sanitize_html(raw_content);
    let title = record.data.get("title").and_then(|v| v.as_str()).unwrap_or(doc_type)
        .replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");

    let html = format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>{title}</title>
<style>body{{font-family:system-ui,sans-serif;max-width:800px;margin:40px auto;padding:0 20px;line-height:1.6;color:#333}}h1{{color:#111}}</style>
</head><body><h1>{title}</h1><div>{content}</div></body></html>"#
    );
    respond_html(msg, html.into_bytes())
}

fn handle_admin_ui(msg: &Message) -> BlockResult {
    respond_html(msg, ADMIN_HTML.as_bytes().to_vec())
}

fn handle_admin_list(msg: &Message) -> BlockResult {
    let (page_size, offset) = pagination_params(msg);
    let doc_type = msg_get_meta(msg, "req.query.type");
    let mut filters = Vec::new();
    if !doc_type.is_empty() {
        filters.push(Filter {
            field: "doc_type".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(doc_type.to_string()),
        });
    }
    let opts = ListOptions {
        filters,
        sort: vec![SortField { field: "updated_at".to_string(), desc: true }],
        limit: page_size as i64,
        offset: offset as i64,
    };
    match db::list(COLLECTION, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_admin_get(msg: &Message, path: &str) -> BlockResult {
    let id = extract_doc_id(msg, path);
    if id.is_empty() {
        return err_bad_request(msg, "Missing document ID");
    }
    match db::get(COLLECTION, id) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) if e.code == wafer_block::ErrorCode::NOT_FOUND => err_not_found(msg, "Document not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_admin_create(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct CreateDoc {
        doc_type: String,
        title: String,
        content: String,
    }
    let body: CreateDoc = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let user_id = msg_get_meta(msg, "auth.user_id");

    let mut data = json_map(serde_json::json!({
        "doc_type": body.doc_type,
        "title": body.title,
        "content": body.content,
        "status": "draft",
        "version": 1,
        "created_by": user_id
    }));
    stamp_created(&mut data);

    match db::create(COLLECTION, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_admin_update(msg: &Message, path: &str) -> BlockResult {
    let id = extract_doc_id(msg, path);
    if id.is_empty() {
        return err_bad_request(msg, "Missing document ID");
    }

    let body: std::collections::HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let mut data = body;
    stamp_updated(&mut data);

    match db::update(COLLECTION, id, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) if e.code == wafer_block::ErrorCode::NOT_FOUND => err_not_found(msg, "Document not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_admin_publish(msg: &Message, path: &str) -> BlockResult {
    let id = extract_doc_id(msg, path);
    if id.is_empty() {
        return err_bad_request(msg, "Missing document ID");
    }

    // Get current document
    let doc = match db::get(COLLECTION, id) {
        Ok(r) => r,
        Err(e) if e.code == wafer_block::ErrorCode::NOT_FOUND => return err_not_found(msg, "Document not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

    let doc_type = doc.data.get("doc_type").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Unpublish other documents of same type
    let existing = db::list_all(
        COLLECTION,
        vec![
            Filter { field: "doc_type".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(doc_type) },
            Filter { field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("published".to_string()) },
        ],
    );
    if let Ok(records) = existing {
        for r in records {
            let upd = json_map(serde_json::json!({"status": "archived"}));
            let _ = db::update(COLLECTION, &r.id, upd);
        }
    }

    // Publish this one
    let now = now_rfc3339();
    let data = json_map(serde_json::json!({
        "status": "published",
        "published_at": now,
        "updated_at": now
    }));

    match db::update(COLLECTION, id, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

fn handle_admin_delete(msg: &Message, path: &str) -> BlockResult {
    let id = extract_doc_id(msg, path);
    if id.is_empty() {
        return err_bad_request(msg, "Missing document ID");
    }
    match db::delete(COLLECTION, id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == wafer_block::ErrorCode::NOT_FOUND => err_not_found(msg, "Document not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// ---------------------------------------------------------------------------
// HTML sanitizer — pure string processing, works the same on WASM
// ---------------------------------------------------------------------------

/// Comprehensive HTML sanitizer to prevent XSS in admin-authored content.
/// Strips dangerous tags, event handlers, and javascript/data URIs.
fn sanitize_html(input: &str) -> String {
    let mut s = input.to_string();

    // Strip dangerous tags and their contents
    for tag in &["script", "iframe", "object", "embed", "style", "form",
                 "input", "textarea", "select", "button", "meta", "link",
                 "base", "svg", "math", "applet"] {
        loop {
            let lower = s.to_lowercase();
            let open = format!("<{}", tag);
            if let Some(start) = lower.find(&open) {
                let close = format!("</{}>", tag);
                let end = if let Some(rel_end) = lower[start..].find(&close) {
                    start + rel_end + close.len()
                } else if let Some(gt) = s[start..].find('>') {
                    start + gt + 1
                } else {
                    s.len()
                };
                s = format!("{}{}", &s[..start], &s[end..]);
            } else {
                break;
            }
        }
    }

    // Strip event handler attributes (on*)
    let bytes = s.as_bytes();
    let lower = s.to_lowercase();
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Inside a tag - find the closing >
            if let Some(end) = s[i..].find('>') {
                let tag_content = &s[i..i + end + 1];
                let tag_lower = &lower[i..i + end + 1];
                let cleaned = remove_dangerous_attrs(tag_content, tag_lower);
                result.push_str(&cleaned);
                i += end + 1;
            } else {
                result.push(bytes[i] as char);
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

/// Remove dangerous attributes from an HTML tag string.
fn remove_dangerous_attrs(tag: &str, _tag_lower: &str) -> String {
    let mut result = String::new();
    let mut chars = tag.chars().peekable();

    // Copy the tag name
    while let Some(&c) = chars.peek() {
        result.push(c);
        chars.next();
        if c == ' ' || c == '>' || c == '/' {
            break;
        }
    }

    if result.ends_with('>') {
        return result;
    }

    // Process attributes
    let rest: String = chars.collect();
    let rest_lower = rest.to_lowercase();
    let mut pos = 0;
    let rest_bytes = rest.as_bytes();

    while pos < rest_bytes.len() {
        // Skip whitespace
        while pos < rest_bytes.len() && rest_bytes[pos].is_ascii_whitespace() {
            result.push(rest_bytes[pos] as char);
            pos += 1;
        }
        if pos >= rest_bytes.len() { break; }
        if rest_bytes[pos] == b'>' || (rest_bytes[pos] == b'/' && pos + 1 < rest_bytes.len() && rest_bytes[pos + 1] == b'>') {
            result.push_str(&rest[pos..]);
            break;
        }

        // Read attribute name
        let attr_start = pos;
        while pos < rest_bytes.len() && rest_bytes[pos] != b'=' && rest_bytes[pos] != b'>' && !rest_bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let attr_name = &rest_lower[attr_start..pos];

        // Check if dangerous
        let is_dangerous = attr_name.starts_with("on")
            || attr_name == "srcdoc"
            || attr_name == "formaction";

        // Read = and value if present
        let mut attr_end = pos;
        if pos < rest_bytes.len() && rest_bytes[pos] == b'=' {
            pos += 1; // skip =
            // Skip optional quotes
            if pos < rest_bytes.len() && (rest_bytes[pos] == b'"' || rest_bytes[pos] == b'\'') {
                let quote = rest_bytes[pos];
                pos += 1;
                while pos < rest_bytes.len() && rest_bytes[pos] != quote {
                    pos += 1;
                }
                if pos < rest_bytes.len() { pos += 1; } // skip closing quote
            } else {
                while pos < rest_bytes.len() && !rest_bytes[pos].is_ascii_whitespace() && rest_bytes[pos] != b'>' {
                    pos += 1;
                }
            }
            attr_end = pos;

            // Check value for javascript:/data: URIs
            let attr_value = &rest_lower[attr_start..attr_end];
            let has_dangerous_uri = attr_value.contains("javascript:") || attr_value.contains("data:text/html") || attr_value.contains("vbscript:");

            if !is_dangerous && !has_dangerous_uri {
                result.push_str(&rest[attr_start..attr_end]);
            }
        } else if !is_dangerous {
            result.push_str(&rest[attr_start..attr_end]);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Admin UI HTML
// ---------------------------------------------------------------------------

const ADMIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Legal Pages Admin</title>
<style>
body{font-family:system-ui,sans-serif;margin:0;padding:20px;background:#f5f5f5}
.container{max-width:900px;margin:0 auto}
h1{color:#333}
.card{background:white;border-radius:8px;padding:20px;margin:10px 0;box-shadow:0 1px 3px rgba(0,0,0,0.1)}
.btn{padding:8px 16px;border:none;border-radius:4px;cursor:pointer;font-size:14px;margin:4px}
.btn-primary{background:#6366f1;color:white}
.btn-success{background:#22c55e;color:white}
.btn-danger{background:#ef4444;color:white}
.badge{display:inline-block;padding:2px 8px;border-radius:12px;font-size:12px}
.badge-published{background:#dcfce7;color:#166534}
.badge-draft{background:#fef3c7;color:#92400e}
.badge-archived{background:#e5e7eb;color:#374151}
table{width:100%;border-collapse:collapse}
th,td{padding:10px;text-align:left;border-bottom:1px solid #eee}
</style>
</head>
<body>
<div class="container">
<h1>Legal Pages</h1>
<p>Manage your Terms of Service and Privacy Policy documents.</p>
<div class="card">
<p>Use the admin API endpoints to manage documents:</p>
<ul>
<li><code>GET /admin/legalpages/documents</code> - List all documents</li>
<li><code>POST /admin/legalpages/documents</code> - Create a document</li>
<li><code>PATCH /admin/legalpages/documents/:id</code> - Update a document</li>
<li><code>PATCH /admin/legalpages/documents/:id/publish</code> - Publish a document</li>
<li><code>DELETE /admin/legalpages/documents/:id</code> - Delete a document</li>
</ul>
</div>
</div>
</body>
</html>"#;
