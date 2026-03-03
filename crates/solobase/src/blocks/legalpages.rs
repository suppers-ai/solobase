use std::collections::HashMap;
use std::sync::Arc;
use wafer_run::block::{Block, BlockInfo, AdminUIInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{self, DatabaseService, Filter, FilterOp, ListOptions, SortField};
use super::helpers::get_db;

pub struct LegalPagesBlock;

const COLLECTION: &str = "ext_legalpages_legal_documents";

impl LegalPagesBlock {
    fn handle_get_public(&self, ctx: &dyn Context, msg: &mut Message, doc_type: &str) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };

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

        let result = match db.list(COLLECTION, &opts) {
            Ok(r) => r,
            Err(e) => return err_internal(msg.clone(), &format!("Database error: {e}")),
        };

        if result.records.is_empty() {
            let html = format!(
                "<html><body><h1>{}</h1><p>No {} document has been published yet.</p></body></html>",
                if doc_type == "terms" { "Terms of Service" } else { "Privacy Policy" },
                doc_type
            );
            return respond(msg.clone(), 200, html.into_bytes(), "text/html; charset=utf-8");
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
        respond(msg.clone(), 200, html.into_bytes(), "text/html; charset=utf-8")
    }

    fn handle_admin_ui(&self, msg: &mut Message) -> Result_ {
        respond(msg.clone(), 200, ADMIN_HTML.as_bytes().to_vec(), "text/html; charset=utf-8")
    }

    fn handle_admin_list(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };
        let (_, page_size, offset) = msg.pagination_params(20);
        let doc_type = msg.query("type");
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
        match db.list(COLLECTION, &opts) {
            Ok(result) => json_respond(msg.clone(), 200, &result),
            Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
        }
    }

    fn handle_admin_get(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };
        let id = msg.var("id");
        if id.is_empty() {
            return err_bad_request(msg.clone(), "Missing document ID");
        }
        match db.get(COLLECTION, id) {
            Ok(record) => json_respond(msg.clone(), 200, &record),
            Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "Document not found"),
            Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
        }
    }

    fn handle_admin_create(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };

        #[derive(serde::Deserialize)]
        struct CreateDoc {
            doc_type: String,
            title: String,
            content: String,
        }
        let body: CreateDoc = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
        };

        let now = chrono::Utc::now().to_rfc3339();
        let mut data = HashMap::new();
        data.insert("doc_type".to_string(), serde_json::Value::String(body.doc_type));
        data.insert("title".to_string(), serde_json::Value::String(body.title));
        data.insert("content".to_string(), serde_json::Value::String(body.content));
        data.insert("status".to_string(), serde_json::Value::String("draft".to_string()));
        data.insert("version".to_string(), serde_json::Value::Number(1.into()));
        data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
        data.insert("updated_at".to_string(), serde_json::Value::String(now));
        data.insert("created_by".to_string(), serde_json::Value::String(msg.user_id().to_string()));

        match db.create(COLLECTION, data) {
            Ok(record) => json_respond(msg.clone(), 201, &record),
            Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
        }
    }

    fn handle_admin_update(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };
        let id = msg.var("id");
        if id.is_empty() {
            return err_bad_request(msg.clone(), "Missing document ID");
        }

        let body: HashMap<String, serde_json::Value> = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
        };

        let mut data = body;
        data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

        match db.update(COLLECTION, id, data) {
            Ok(record) => json_respond(msg.clone(), 200, &record),
            Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "Document not found"),
            Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
        }
    }

    fn handle_admin_publish(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };
        let id = msg.var("id");
        if id.is_empty() {
            return err_bad_request(msg.clone(), "Missing document ID");
        }

        // Get current document
        let doc = match db.get(COLLECTION, id) {
            Ok(r) => r,
            Err(database::DatabaseError::NotFound) => return err_not_found(msg.clone(), "Document not found"),
            Err(e) => return err_internal(msg.clone(), &format!("Database error: {e}")),
        };

        let doc_type = doc.data.get("doc_type").and_then(|v| v.as_str()).unwrap_or("").to_string();

        // Unpublish other documents of same type
        let existing = database::list_all(
            db.as_ref(),
            COLLECTION,
            vec![
                Filter { field: "doc_type".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(doc_type) },
                Filter { field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("published".to_string()) },
            ],
        );
        if let Ok(records) = existing {
            for r in records {
                let mut upd = HashMap::new();
                upd.insert("status".to_string(), serde_json::Value::String("archived".to_string()));
                if let Err(e) = db.update(COLLECTION, &r.id, upd) {
                    tracing::warn!("Failed to archive previous legal page version: {e}");
                }
            }
        }

        // Publish this one
        let mut data = HashMap::new();
        data.insert("status".to_string(), serde_json::Value::String("published".to_string()));
        data.insert("published_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

        match db.update(COLLECTION, id, data) {
            Ok(record) => json_respond(msg.clone(), 200, &record),
            Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
        }
    }

    fn handle_admin_delete(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };
        let id = msg.var("id");
        if id.is_empty() {
            return err_bad_request(msg.clone(), "Missing document ID");
        }
        match db.delete(COLLECTION, id) {
            Ok(()) => json_respond(msg.clone(), 200, &serde_json::json!({"deleted": true})),
            Err(database::DatabaseError::NotFound) => err_not_found(msg.clone(), "Document not found"),
            Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
        }
    }

    fn seed_defaults(&self, db: &Arc<dyn DatabaseService>) {
        let count = db.count(COLLECTION, &[]).unwrap_or(0);
        if count > 0 {
            return;
        }

        let now = chrono::Utc::now().to_rfc3339();
        for (doc_type, title, content) in &[
            ("terms", "Terms of Service", "<p>These are the default terms of service. Please update them in the admin panel.</p>"),
            ("privacy", "Privacy Policy", "<p>This is the default privacy policy. Please update it in the admin panel.</p>"),
        ] {
            let mut data = HashMap::new();
            data.insert("doc_type".to_string(), serde_json::Value::String(doc_type.to_string()));
            data.insert("title".to_string(), serde_json::Value::String(title.to_string()));
            data.insert("content".to_string(), serde_json::Value::String(content.to_string()));
            data.insert("status".to_string(), serde_json::Value::String("published".to_string()));
            data.insert("version".to_string(), serde_json::Value::Number(1.into()));
            data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
            data.insert("updated_at".to_string(), serde_json::Value::String(now.clone()));
            data.insert("published_at".to_string(), serde_json::Value::String(now.clone()));
            data.insert("created_by".to_string(), serde_json::Value::String("system".to_string()));
            if let Err(e) = db.create(COLLECTION, data) {
                tracing::warn!("Failed to seed default legal page '{doc_type}': {e}");
            }
        }
    }
}

/// Remove dangerous HTML tags and their contents from admin-authored content.
/// Strips `<script>`, `<iframe>`, `<object>`, and `<embed>` tags to prevent stored XSS.
fn sanitize_html(input: &str) -> String {
    let mut s = input.to_string();
    for tag in &["script", "iframe", "object", "embed"] {
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
    s
}

impl Block for LegalPagesBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "legalpages-feature".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Legal pages management with versioning and publishing".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: Some(AdminUIInfo {
                path: "/ext/legalpages/admin".to_string(),
                icon: "Scale".to_string(),
                title: "Legal Pages".to_string(),
            }),
        }
    }

    fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let action = msg.action();
        let path = msg.path();

        match (action, path) {
            // Public endpoints
            ("retrieve", "/ext/legalpages/terms") => self.handle_get_public(ctx, msg, "terms"),
            ("retrieve", "/ext/legalpages/privacy") => self.handle_get_public(ctx, msg, "privacy"),
            // Admin UI
            ("retrieve", "/ext/legalpages/admin") => self.handle_admin_ui(msg),
            // Admin API
            ("retrieve", "/admin/legalpages/documents") => self.handle_admin_list(ctx, msg),
            ("retrieve", _) if path.starts_with("/admin/legalpages/documents/") => self.handle_admin_get(ctx, msg),
            ("create", "/admin/legalpages/documents") => self.handle_admin_create(ctx, msg),
            ("update", _) if path.starts_with("/admin/legalpages/documents/") && path.ends_with("/publish") => {
                self.handle_admin_publish(ctx, msg)
            }
            ("update", _) if path.starts_with("/admin/legalpages/documents/") => self.handle_admin_update(ctx, msg),
            ("delete", _) if path.starts_with("/admin/legalpages/documents/") => self.handle_admin_delete(ctx, msg),
            // ext API aliases (same as admin, but routed through admin-pipe)
            ("retrieve", "/ext/legalpages/documents") => self.handle_admin_list(ctx, msg),
            ("create", "/ext/legalpages/documents") => self.handle_admin_create(ctx, msg),
            _ => err_not_found(msg.clone(), "not found"),
        }
    }

    fn lifecycle(&self, ctx: &dyn Context, event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            if let Some(db) = ctx.services().and_then(|s| s.database.as_ref()) {
                self.seed_defaults(db);
            }
        }
        Ok(())
    }
}

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
