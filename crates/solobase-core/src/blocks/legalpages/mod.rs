mod pages;

use crate::blocks::helpers::{self, json_map};
use crate::ui::SiteConfig;
use maud::{html, Markup, PreEscaped, DOCTYPE};
use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub struct LegalPagesBlock;

pub(crate) const COLLECTION: &str = "suppers_ai__legalpages__documents";

/// Extract document ID from path like `/b/legalpages/api/documents/{id}` or
/// `/b/legalpages/api/documents/{id}/publish`.
fn extract_doc_id(msg: &Message) -> &str {
    // Try router-extracted var first (native axum), fall back to path parsing (CF)
    let var = msg.var("id");
    if !var.is_empty() {
        return var;
    }
    let path = msg.path();
    let suffix = path
        .strip_prefix("/b/legalpages/api/documents/")
        .unwrap_or("");
    // Strip trailing /publish or /
    suffix.split('/').next().unwrap_or("")
}

impl LegalPagesBlock {
    async fn handle_get_public(
        &self,
        ctx: &dyn Context,
        msg: &mut Message,
        doc_type: &str,
    ) -> Result_ {
        use wafer_core::clients::config;

        let site = SiteConfig::load(ctx).await;
        let bg_color =
            config::get_default(ctx, "SUPPERS_AI__LEGALPAGES__BG_COLOR", "#f8fafc").await;
        let back_url = config::get_default(ctx, "SUPPERS_AI__LEGALPAGES__BACK_URL", "/").await;
        let custom_footer = config::get_default(ctx, "SUPPERS_AI__LEGALPAGES__FOOTER", "").await;
        let primary_color =
            config::get_default(ctx, "SOLOBASE_SHARED__PRIMARY_COLOR", "#6366f1").await;

        let type_label = if doc_type == "terms" {
            "Terms of Service"
        } else {
            "Privacy Policy"
        };

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
            sort: vec![SortField {
                field: "version".to_string(),
                desc: true,
            }],
            limit: 1,
            ..Default::default()
        };

        let result = match db::list(ctx, COLLECTION, &opts).await {
            Ok(r) => r,
            Err(e) => return err_internal(msg, &format!("Database error: {e}")),
        };

        let page_config = PublicPageConfig {
            app_name: &site.app_name,
            favicon_url: &site.favicon_url,
            bg_color: &bg_color,
            back_url: &back_url,
            custom_footer: &custom_footer,
            primary_color: &primary_color,
        };

        if result.records.is_empty() {
            let markup = public_page(
                &page_config,
                type_label,
                "",
                1,
                "<p>No document has been published yet.</p>",
            );
            return respond(
                msg,
                markup.into_string().into_bytes(),
                "text/html; charset=utf-8",
            );
        }

        let record = &result.records[0];
        let title = record
            .data
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(type_label);
        let raw_content = record
            .data
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = sanitize_html(raw_content);
        let published_at = record
            .data
            .get("published_at")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let version = record
            .data
            .get("version")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(1);
        let meta = if !published_at.is_empty() {
            format!(
                "Last updated: {}",
                published_at.get(..10).unwrap_or(published_at),
            )
        } else {
            String::new()
        };

        let markup = public_page(&page_config, title, &meta, version, &content);
        respond(
            msg,
            markup.into_string().into_bytes(),
            "text/html; charset=utf-8",
        )
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
            sort: vec![SortField {
                field: "updated_at".to_string(),
                desc: true,
            }],
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
            Err(e) if e.code == ErrorCode::NotFound => {
                return err_not_found(msg, "Document not found")
            }
            Err(e) => return err_internal(msg, &format!("Database error: {e}")),
        };

        let doc_type = doc
            .data
            .get("doc_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Unpublish other documents of same type
        let existing = db::list_all(
            ctx,
            COLLECTION,
            vec![
                Filter {
                    field: "doc_type".to_string(),
                    operator: FilterOp::Equal,
                    value: serde_json::Value::String(doc_type),
                },
                Filter {
                    field: "status".to_string(),
                    operator: FilterOp::Equal,
                    value: serde_json::Value::String("published".to_string()),
                },
            ],
        )
        .await;
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

// ---------------------------------------------------------------------------
// Public page rendering
// ---------------------------------------------------------------------------

struct PublicPageConfig<'a> {
    app_name: &'a str,
    favicon_url: &'a str,
    bg_color: &'a str,
    back_url: &'a str,
    custom_footer: &'a str,
    primary_color: &'a str,
}

/// Render a professionally styled public legal page.
fn public_page(
    config: &PublicPageConfig,
    title: &str,
    meta: &str,
    version: i64,
    content: &str,
) -> Markup {
    let year = chrono::Utc::now().format("%Y");
    let footer_text = if !config.custom_footer.is_empty() {
        config.custom_footer.to_string()
    } else if !config.app_name.is_empty() {
        format!(
            "\u{00a9} {} {}. All rights reserved.",
            year, config.app_name
        )
    } else {
        String::new()
    };

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width,initial-scale=1";
                @if config.app_name.is_empty() {
                    title { (title) }
                } @else {
                    title { (title) " \u{2014} " (config.app_name) }
                }
                @if !config.favicon_url.is_empty() {
                    link rel="icon" href=(config.favicon_url);
                }
                style {
                    (PreEscaped(format!(
                        ":root{{--bg-page:{};--accent:{};}}",
                        config.bg_color, config.primary_color
                    )))
                    (PreEscaped(PUBLIC_PAGE_CSS))
                }
            }
            body {
                header .legal-header {
                    div .legal-header-inner {
                        a .legal-back href=(config.back_url) title="Go back" {
                            "\u{2190}"
                        }
                    }
                }
                main .legal-outer {
                    div .legal-card {
                        // Title + meta header
                        div .legal-card-header {
                            h1 { (title) }
                            @if !meta.is_empty() || version > 0 {
                                div .legal-meta {
                                    @if !meta.is_empty() {
                                        span { (meta) }
                                    }
                                    @if version > 0 {
                                        span .legal-version { "v" (version) }
                                    }
                                }
                            }
                        }
                        // Content
                        div .legal-content { (PreEscaped(content)) }
                    }
                }
                @if !footer_text.is_empty() {
                    footer .legal-footer {
                        (PreEscaped(footer_text))
                    }
                }
            }
        }
    }
}

const PUBLIC_PAGE_CSS: &str = r#"
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:system-ui,-apple-system,sans-serif;color:#1e293b;background:var(--bg-page,#f8fafc);line-height:1.7;min-height:100vh;display:flex;flex-direction:column}
.legal-header{background:transparent;padding:0.75rem 1.5rem;position:sticky;top:0;z-index:10}
.legal-header-inner{max-width:800px;margin:0 auto;display:flex;align-items:center}
.legal-back{display:flex;align-items:center;justify-content:center;width:32px;height:32px;border-radius:8px;text-decoration:none;color:#64748b;font-size:1.1rem;transition:all 0.15s}
.legal-back:hover{color:var(--accent,#6366f1);background:rgba(0,0,0,0.05)}
.legal-outer{flex:1;max-width:800px;width:100%;margin:2rem auto;padding:0 1.5rem}
.legal-card{background:#fff;border-radius:12px;box-shadow:0 1px 3px rgba(0,0,0,.06),0 1px 2px rgba(0,0,0,.04);overflow:hidden}
.legal-card-header{padding:2.5rem 3rem 0;border-bottom:none}
.legal-card-header h1{font-size:1.85rem;font-weight:700;color:#0f172a;margin-bottom:0.5rem;line-height:1.3}
.legal-meta{display:flex;align-items:center;gap:0.75rem;color:#94a3b8;font-size:0.85rem;padding-bottom:1.5rem;border-bottom:1px solid #f1f5f9}
.legal-version{background:#f1f5f9;color:#64748b;padding:1px 8px;border-radius:10px;font-size:0.75rem;font-weight:600}
.legal-content{padding:2rem 3rem 2.5rem;font-family:Georgia,'Times New Roman',serif;font-size:1.05rem;line-height:1.85;color:#334155}
.legal-content h1{font-size:1.6rem;font-weight:700;margin:1.5rem 0 0.75rem;color:#0f172a;font-family:system-ui,sans-serif}
.legal-content h2{font-size:1.35rem;font-weight:600;margin:2rem 0 0.75rem;color:#0f172a;font-family:system-ui,sans-serif}
.legal-content h3{font-size:1.1rem;font-weight:600;margin:1.5rem 0 0.5rem;color:#1e293b;font-family:system-ui,sans-serif}
.legal-content p{margin-bottom:1rem}
.legal-content ul,.legal-content ol{margin:.5rem 0 1rem 1.5rem}
.legal-content li{margin-bottom:.35rem}
.legal-content a{color:var(--accent,#6366f1);text-decoration:underline;text-underline-offset:2px}
.legal-content a:hover{opacity:.8}
.legal-content blockquote{border-left:3px solid var(--accent,#6366f1);padding:0.75rem 1rem;margin:1rem 0;background:#f8fafc;border-radius:0 6px 6px 0;color:#475569;font-style:italic}
.legal-content pre{background:#f1f5f9;padding:1rem;border-radius:6px;overflow-x:auto;font-family:monospace;font-size:.9rem;margin:1rem 0}
.legal-content code{background:#f1f5f9;padding:2px 5px;border-radius:3px;font-size:.9em}
.legal-content pre code{background:none;padding:0}
.legal-content table{width:100%;border-collapse:collapse;margin:1rem 0}
.legal-content th,.legal-content td{padding:.6rem .75rem;text-align:left;border-bottom:1px solid #e2e8f0}
.legal-content th{font-weight:600;background:#f8fafc}
.legal-content hr{border:none;border-top:1px solid #e2e8f0;margin:1.5rem 0}
.legal-footer{text-align:center;color:#94a3b8;font-size:.8rem;padding:1.5rem 2rem 2rem}
.legal-footer a{color:var(--accent,#6366f1);text-decoration:none}
@media print{.legal-header,.legal-footer{display:none}.legal-outer{margin:0;padding:0}.legal-card{box-shadow:none;border-radius:0}}
@media(max-width:640px){.legal-card-header{padding:1.5rem 1.25rem 0}.legal-content{padding:1.25rem 1.25rem 1.5rem}.legal-outer{padding:0 0.75rem;margin:1rem auto}}
"#;

/// Sanitize admin-authored HTML content to prevent XSS.
fn sanitize_html(input: &str) -> String {
    ammonia::Builder::default()
        .add_tags(&["h1", "h2", "h3", "h4", "h5", "h6"])
        .add_tags(&["p", "br", "hr", "blockquote", "pre", "code"])
        .add_tags(&["ul", "ol", "li", "dl", "dt", "dd"])
        .add_tags(&["table", "thead", "tbody", "tr", "th", "td"])
        .add_tags(&[
            "a", "strong", "em", "b", "i", "u", "s", "sub", "sup", "small",
        ])
        .add_tags(&["img", "figure", "figcaption"])
        .add_tags(&[
            "div", "span", "section", "article", "header", "footer", "nav", "aside",
        ])
        .add_tag_attributes("a", &["href", "title", "target"])
        .add_tag_attributes("img", &["src", "alt", "title", "width", "height"])
        .add_tag_attributes("td", &["colspan", "rowspan"])
        .add_tag_attributes("th", &["colspan", "rowspan"])
        .link_rel(Some("noopener noreferrer"))
        .clean(input)
        .to_string()
}

// ---------------------------------------------------------------------------
// Block trait implementation
// ---------------------------------------------------------------------------

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
                CollectionSchema::new("suppers_ai__legalpages__documents")
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
                BlockEndpoint::get("/b/legalpages/admin", "Admin editor", AuthLevel::Admin),
                BlockEndpoint::get("/b/legalpages/api/documents", "List documents", AuthLevel::Admin),
                BlockEndpoint::post("/b/legalpages/api/documents", "Create document", AuthLevel::Admin),
                BlockEndpoint::patch("/b/legalpages/api/documents/{id}", "Update document", AuthLevel::Admin),
                BlockEndpoint::post("/b/legalpages/api/documents/{id}/publish", "Publish document", AuthLevel::Admin),
            ])
            .config_keys(vec![
                ConfigVar::new("SUPPERS_AI__LEGALPAGES__BG_COLOR", "Background color for public legal pages", "#f8fafc")
                    .name("Background Color")
                    .input_type(InputType::Color),
                ConfigVar::new("SUPPERS_AI__LEGALPAGES__BACK_URL", "Back button URL in the header (e.g., your website homepage)", "/")
                    .name("Back Button URL")
                    .input_type(InputType::Url),
                ConfigVar::new("SUPPERS_AI__LEGALPAGES__FOOTER", "Custom footer text (HTML allowed)", "")
                    .name("Footer Text"),
            ])
            .admin_url("/b/legalpages/admin")
            .can_disable(true)
            .default_enabled(false)
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let action = msg.action();
        let path = msg.path();

        match (action, path) {
            // Public endpoints
            ("retrieve", "/b/legalpages/terms") => self.handle_get_public(ctx, msg, "terms").await,
            ("retrieve", "/b/legalpages/privacy") => {
                self.handle_get_public(ctx, msg, "privacy").await
            }

            // Admin UI pages (SSR)
            ("retrieve", "/b/legalpages/admin") | ("retrieve", "/b/legalpages/admin/privacy") => {
                pages::editor_page(ctx, msg, "privacy").await
            }
            ("retrieve", "/b/legalpages/admin/terms") => {
                pages::editor_page(ctx, msg, "terms").await
            }
            ("retrieve", "/b/legalpages/admin/settings") => pages::settings_page(ctx, msg).await,
            ("retrieve", "/b/legalpages/admin/endpoints") => pages::endpoints_page(ctx, msg).await,

            // Admin UI mutations (from editor save/publish)
            ("create", "/b/legalpages/admin/save") => pages::handle_save(ctx, msg).await,
            ("create", "/b/legalpages/admin/publish") => pages::handle_publish(ctx, msg).await,
            ("create", "/b/legalpages/admin/settings") => {
                pages::handle_save_settings(ctx, msg).await
            }

            // JSON API at /b/legalpages/api/documents/...
            ("retrieve", "/b/legalpages/api/documents") => self.handle_admin_list(ctx, msg).await,
            ("retrieve", _) if path.starts_with("/b/legalpages/api/documents/") => {
                self.handle_admin_get(ctx, msg).await
            }
            ("create", "/b/legalpages/api/documents") => self.handle_admin_create(ctx, msg).await,
            ("update", _)
                if path.starts_with("/b/legalpages/api/documents/")
                    && path.ends_with("/publish") =>
            {
                self.handle_admin_publish(ctx, msg).await
            }
            ("update", _) if path.starts_with("/b/legalpages/api/documents/") => {
                self.handle_admin_update(ctx, msg).await
            }
            ("delete", _) if path.starts_with("/b/legalpages/api/documents/") => {
                self.handle_admin_delete(ctx, msg).await
            }
            _ => err_not_found(msg, "not found"),
        }
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            self.seed_defaults(ctx).await;
        }
        Ok(())
    }
}
