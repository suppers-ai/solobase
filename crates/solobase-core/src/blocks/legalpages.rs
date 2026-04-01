use std::collections::HashMap;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use super::helpers::{self, json_map};

pub struct LegalPagesBlock;

const COLLECTION: &str = "block_legalpages_legal_documents";

/// Extract document ID from path like `/admin/legalpages/documents/{id}` or
/// `/admin/legalpages/documents/{id}/publish`.
fn extract_doc_id(msg: &Message) -> &str {
    // Try router-extracted var first (native axum), fall back to path parsing (CF)
    let var = msg.var("id");
    if !var.is_empty() {
        return var;
    }
    let path = msg.path();
    let suffix = path.strip_prefix("/admin/legalpages/documents/")
        .or_else(|| path.strip_prefix("/b/legalpages/documents/"))
        .unwrap_or("");
    // Strip trailing /publish or /
    suffix.split('/').next().unwrap_or("")
}

impl LegalPagesBlock {
    async fn handle_get_public(&self, ctx: &dyn Context, msg: &mut Message, doc_type: &str) -> Result_ {
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

        let result = match db::list(ctx, COLLECTION, &opts).await {
            Ok(r) => r,
            Err(e) => return err_internal(msg, &format!("Database error: {e}")),
        };

        if result.records.is_empty() {
            let html = format!(
                "<html><body><h1>{}</h1><p>No {} document has been published yet.</p></body></html>",
                if doc_type == "terms" { "Terms of Service" } else { "Privacy Policy" },
                doc_type
            );
            return respond(msg, html.into_bytes(), "text/html; charset=utf-8");
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
        respond(msg, html.into_bytes(), "text/html; charset=utf-8")
    }

    fn handle_admin_ui(&self, msg: &mut Message) -> Result_ {
        respond(msg, ADMIN_HTML.as_bytes().to_vec(), "text/html; charset=utf-8")
    }

    async fn handle_admin_list(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
        match db::list(ctx, COLLECTION, &opts).await {
            Ok(result) => json_respond(msg, &result),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn handle_admin_get(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let id = extract_doc_id(msg);
        if id.is_empty() {
            return err_bad_request(msg, "Missing document ID");
        }
        match db::get(ctx, COLLECTION, id).await {
            Ok(record) => json_respond(msg, &record),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Document not found"),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn handle_admin_create(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        #[derive(serde::Deserialize)]
        struct CreateDoc {
            doc_type: String,
            title: String,
            content: String,
        }
        let body: CreateDoc = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        let mut data = json_map(serde_json::json!({
            "doc_type": body.doc_type,
            "title": body.title,
            "content": body.content,
            "status": "draft",
            "version": 1,
            "created_by": msg.user_id()
        }));
        helpers::stamp_created(&mut data);

        match db::create(ctx, COLLECTION, data).await {
            Ok(record) => json_respond(msg, &record),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn handle_admin_update(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let id = extract_doc_id(msg);
        if id.is_empty() {
            return err_bad_request(msg, "Missing document ID");
        }

        let body: HashMap<String, serde_json::Value> = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        let mut data = body;
        helpers::stamp_updated(&mut data);

        match db::update(ctx, COLLECTION, id, data).await {
            Ok(record) => json_respond(msg, &record),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Document not found"),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn handle_admin_publish(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let id = extract_doc_id(msg);
        if id.is_empty() {
            return err_bad_request(msg, "Missing document ID");
        }

        // Get current document
        let doc = match db::get(ctx, COLLECTION, id).await {
            Ok(r) => r,
            Err(e) if e.code == ErrorCode::NotFound => return err_not_found(msg, "Document not found"),
            Err(e) => return err_internal(msg, &format!("Database error: {e}")),
        };

        let doc_type = doc.data.get("doc_type").and_then(|v| v.as_str()).unwrap_or("").to_string();

        // Unpublish other documents of same type
        let existing = db::list_all(
            ctx,
            COLLECTION,
            vec![
                Filter { field: "doc_type".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(doc_type) },
                Filter { field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("published".to_string()) },
            ],
        ).await;
        if let Ok(records) = existing {
            for r in records {
                let upd = json_map(serde_json::json!({"status": "archived"}));
                if let Err(e) = db::update(ctx, COLLECTION, &r.id, upd).await {
                    tracing::warn!("Failed to archive previous legal page version: {e}");
                }
            }
        }

        // Publish this one
        let now = helpers::now_rfc3339();
        let data = json_map(serde_json::json!({
            "status": "published",
            "published_at": now,
            "updated_at": now
        }));

        match db::update(ctx, COLLECTION, id, data).await {
            Ok(record) => json_respond(msg, &record),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn handle_admin_delete(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let id = extract_doc_id(msg);
        if id.is_empty() {
            return err_bad_request(msg, "Missing document ID");
        }
        match db::delete(ctx, COLLECTION, id).await {
            Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Document not found"),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn seed_defaults(&self, ctx: &dyn Context) {
        let count = db::count(ctx, COLLECTION, &[]).await.unwrap_or(0);
        if count > 0 {
            return;
        }

        let now = helpers::now_rfc3339();
        for (doc_type, title, content) in &[
            ("terms", "Terms of Service", "<p>These are the default terms of service. Please update them in the admin panel.</p>"),
            ("privacy", "Privacy Policy", "<p>This is the default privacy policy. Please update it in the admin panel.</p>"),
        ] {
            let data = json_map(serde_json::json!({
                "doc_type": doc_type,
                "title": title,
                "content": content,
                "status": "published",
                "version": 1,
                "created_at": now,
                "updated_at": now,
                "published_at": now,
                "created_by": "system"
            }));
            if let Err(e) = db::create(ctx, COLLECTION, data).await {
                tracing::warn!("Failed to seed default legal page '{doc_type}': {e}");
            }
        }
    }
}

/// Sanitize admin-authored HTML content to prevent XSS.
/// Uses the `ammonia` crate for battle-tested HTML sanitization.
fn sanitize_html(input: &str) -> String {
    ammonia::Builder::default()
        .add_tags(&["h1", "h2", "h3", "h4", "h5", "h6"])
        .add_tags(&["p", "br", "hr", "blockquote", "pre", "code"])
        .add_tags(&["ul", "ol", "li", "dl", "dt", "dd"])
        .add_tags(&["table", "thead", "tbody", "tr", "th", "td"])
        .add_tags(&["a", "strong", "em", "b", "i", "u", "s", "sub", "sup", "small"])
        .add_tags(&["img", "figure", "figcaption"])
        .add_tags(&["div", "span", "section", "article", "header", "footer", "nav", "aside"])
        .add_tag_attributes("a", &["href", "title", "target", "rel"])
        .add_tag_attributes("img", &["src", "alt", "title", "width", "height"])
        .add_tag_attributes("td", &["colspan", "rowspan"])
        .add_tag_attributes("th", &["colspan", "rowspan"])
        .link_rel(Some("noopener noreferrer"))
        .clean(input)
        .to_string()
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for LegalPagesBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::types::CollectionSchema;
        use wafer_run::AuthLevel;

        BlockInfo::new("suppers-ai/legalpages", "0.0.1", "http-handler@v1", "Legal pages management with versioning and publishing")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into()])
            .collections(vec![
                CollectionSchema::new("block_legalpages_legal_documents")
                    .field("doc_type", "string")
                    .field("title", "string")
                    .field_default("content", "text", "")
                    .field_default("status", "string", "draft")
                    .field_default("version", "int", "1")
                    .field_default("created_by", "string", "")
                    .field_optional("published_at", "datetime")
                    .index(&["doc_type", "status"]),
            ])
            .category(wafer_run::BlockCategory::Feature)
            .description("Legal document management with versioning and publishing. Create and manage terms of service, privacy policies, and other legal documents. Supports draft/published workflow with version tracking.")
            .endpoints(vec![
                BlockEndpoint::get("/b/legalpages/terms", "Published terms of service", AuthLevel::Public),
                BlockEndpoint::get("/b/legalpages/privacy", "Published privacy policy", AuthLevel::Public),
                BlockEndpoint::get("/admin/legalpages/documents", "List documents", AuthLevel::Admin),
                BlockEndpoint::post("/admin/legalpages/documents", "Create document", AuthLevel::Admin),
                BlockEndpoint::patch("/admin/legalpages/documents/{id}", "Update document", AuthLevel::Admin),
                BlockEndpoint::post("/admin/legalpages/documents/{id}/publish", "Publish document", AuthLevel::Admin),
            ])
            .can_disable(true)
            .default_enabled(false)
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let action = msg.action();
        let path = msg.path();

        match (action, path) {
            // Public endpoints
            ("retrieve", "/b/legalpages/terms") => self.handle_get_public(ctx, msg, "terms").await,
            ("retrieve", "/b/legalpages/privacy") => self.handle_get_public(ctx, msg, "privacy").await,
            // Admin UI
            ("retrieve", "/b/legalpages/admin") => self.handle_admin_ui(msg),
            // Admin API
            ("retrieve", "/admin/legalpages/documents") => self.handle_admin_list(ctx, msg).await,
            ("retrieve", _) if path.starts_with("/admin/legalpages/documents/") => self.handle_admin_get(ctx, msg).await,
            ("create", "/admin/legalpages/documents") => self.handle_admin_create(ctx, msg).await,
            ("update", _) if path.starts_with("/admin/legalpages/documents/") && path.ends_with("/publish") => {
                self.handle_admin_publish(ctx, msg).await
            }
            ("update", _) if path.starts_with("/admin/legalpages/documents/") => self.handle_admin_update(ctx, msg).await,
            ("delete", _) if path.starts_with("/admin/legalpages/documents/") => self.handle_admin_delete(ctx, msg).await,
            // ext API aliases (same as admin, but routed through admin-pipe)
            ("retrieve", "/b/legalpages/documents") => self.handle_admin_list(ctx, msg).await,
            ("create", "/b/legalpages/documents") => self.handle_admin_create(ctx, msg).await,
            _ => err_not_found(msg, "not found"),
        }
    }

    async fn lifecycle(&self, ctx: &dyn Context, event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            self.seed_defaults(ctx).await;
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
