mod pages;

use std::collections::HashMap;

use maud::{html, Markup, PreEscaped};
use wafer_core::clients::{
    database as db,
    database::{Filter, FilterOp, ListOptions, SortField},
};
use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use crate::{
    blocks::helpers::{
        self, err_bad_request, err_internal, err_not_found, json_map, ok_json, ResponseBuilder,
    },
    ui::{self, templates, SiteConfig},
};

pub struct LegalPagesBlock;

impl LegalPagesBlock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LegalPagesBlock {
    fn default() -> Self {
        Self::new()
    }
}

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
    async fn handle_get_public(&self, ctx: &dyn Context, doc_type: &str) -> OutputStream {
        use wafer_core::clients::config;

        let site = SiteConfig::load(ctx).await;
        let bg_color = config::get_default(ctx, "SUPPERS_AI__LEGALPAGES__BG_COLOR", "").await;
        let back_url = config::get_default(ctx, "SUPPERS_AI__LEGALPAGES__BACK_URL", "/").await;
        let custom_footer = config::get_default(ctx, "SUPPERS_AI__LEGALPAGES__FOOTER", "").await;
        let primary_color = config::get_default(ctx, "SOLOBASE_SHARED__PRIMARY_COLOR", "").await;

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
            Err(e) => {
                tracing::warn!(error = %e, "legalpages: db list failed");
                return err_internal(&format!("Database error: {e}"));
            }
        };

        let (title, content, version, meta) = if result.records.is_empty() {
            (
                type_label.to_string(),
                "<p>No document has been published yet.</p>".to_string(),
                1_i64,
                String::new(),
            )
        } else {
            let record = &result.records[0];
            let title = record
                .data
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(type_label)
                .to_string();
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
            (title, content, version, meta)
        };

        let markup = render_legal_page(LegalPageInputs {
            site: &site,
            title: &title,
            content: &content,
            version,
            meta: &meta,
            back_url: &back_url,
            bg_color: &bg_color,
            primary_color: &primary_color,
            custom_footer: &custom_footer,
        });
        ResponseBuilder::new().body(
            markup.into_string().into_bytes(),
            "text/html; charset=utf-8",
        )
    }

    async fn handle_admin_list(&self, ctx: &dyn Context, msg: &Message) -> OutputStream {
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
            Ok(result) => ok_json(&result),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    async fn handle_admin_get(&self, ctx: &dyn Context, msg: &Message) -> OutputStream {
        let id = extract_doc_id(msg);
        if id.is_empty() {
            return err_bad_request("Missing document ID");
        }
        match db::get(ctx, COLLECTION, id).await {
            Ok(record) => ok_json(&record),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found("Document not found"),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    async fn handle_admin_create(
        &self,
        ctx: &dyn Context,
        msg: &Message,
        input: InputStream,
    ) -> OutputStream {
        #[derive(serde::Deserialize)]
        struct CreateDoc {
            doc_type: String,
            title: String,
            content: String,
        }
        let raw = input.collect_to_bytes().await;
        let body: CreateDoc = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
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
            Ok(record) => ok_json(&record),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    async fn handle_admin_update(
        &self,
        ctx: &dyn Context,
        msg: &Message,
        input: InputStream,
    ) -> OutputStream {
        let id = extract_doc_id(msg);
        if id.is_empty() {
            return err_bad_request("Missing document ID");
        }

        let raw = input.collect_to_bytes().await;
        let body: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        let mut data = body;
        helpers::stamp_updated(&mut data);

        match db::update(ctx, COLLECTION, id, data).await {
            Ok(record) => ok_json(&record),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found("Document not found"),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    async fn handle_admin_publish(&self, ctx: &dyn Context, msg: &Message) -> OutputStream {
        let id = extract_doc_id(msg);
        if id.is_empty() {
            return err_bad_request("Missing document ID");
        }

        // Get current document
        let doc = match db::get(ctx, COLLECTION, id).await {
            Ok(r) => r,
            Err(e) if e.code == ErrorCode::NotFound => return err_not_found("Document not found"),
            Err(e) => return err_internal(&format!("Database error: {e}")),
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
            Ok(record) => ok_json(&record),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    async fn handle_admin_delete(&self, ctx: &dyn Context, msg: &Message) -> OutputStream {
        let id = extract_doc_id(msg);
        if id.is_empty() {
            return err_bad_request("Missing document ID");
        }
        match db::delete(ctx, COLLECTION, id).await {
            Ok(()) => ok_json(&serde_json::json!({"deleted": true})),
            Err(e) if e.code == ErrorCode::NotFound => err_not_found("Document not found"),
            Err(e) => err_internal(&format!("Database error: {e}")),
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

struct LegalPageInputs<'a> {
    site: &'a SiteConfig,
    title: &'a str,
    content: &'a str, // pre-sanitized HTML
    version: i64,
    meta: &'a str,
    back_url: &'a str,
    bg_color: &'a str,      // empty string = use template default
    primary_color: &'a str, // empty string = use template default
    custom_footer: &'a str, // empty string = auto "© YEAR APP_NAME"
}

/// Render the legal-document body (title + meta + content) and delegate to
/// `templates::public_page` for the chrome.
fn render_legal_page(inputs: LegalPageInputs<'_>) -> Markup {
    let body = html! {
        div .public-page__head {
            h1 { (inputs.title) }
            @if !inputs.meta.is_empty() || inputs.version > 0 {
                div .public-page__meta {
                    @if !inputs.meta.is_empty() { span { (inputs.meta) } }
                    @if inputs.version > 0 {
                        span .public-page__version { "v" (inputs.version) }
                    }
                }
            }
        }
        div .public-page__content { (PreEscaped(inputs.content)) }
    };

    let footer_text = if !inputs.custom_footer.is_empty() {
        inputs.custom_footer.to_string()
    } else if !inputs.site.app_name.is_empty() {
        let year = chrono::Utc::now().format("%Y");
        format!(
            "\u{00a9} {} {}. All rights reserved.",
            year, inputs.site.app_name
        )
    } else {
        String::new()
    };
    let footer = if footer_text.is_empty() {
        None
    } else {
        // `custom_footer` allows admin-authored HTML; rendered with PreEscaped
        // here matches prior behavior. No user input on this path beyond what
        // the admin set.
        Some(html! { (PreEscaped(footer_text)) })
    };

    let bg_color = if inputs.bg_color.is_empty() {
        None
    } else {
        Some(inputs.bg_color)
    };
    let accent_color = if inputs.primary_color.is_empty() {
        None
    } else {
        Some(inputs.primary_color)
    };
    let back_url = if inputs.back_url.is_empty() {
        None
    } else {
        Some(inputs.back_url)
    };

    templates::public_page(
        templates::PublicPage {
            title: inputs.title,
            config: inputs.site,
            meta_description: None,
            back_url,
            bg_color,
            accent_color,
            footer,
        },
        body,
    )
}

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
        use wafer_run::{types::CollectionSchema, AuthLevel};

        BlockInfo::new("suppers-ai/legalpages", "0.0.1", "http-handler@v1", "Legal pages management with versioning and publishing")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into()])
            .collections(vec![
                CollectionSchema::new(COLLECTION)
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
                BlockEndpoint::get("/b/legalpages/terms").summary("Published terms of service"),
                BlockEndpoint::get("/b/legalpages/privacy").summary("Published privacy policy"),
                BlockEndpoint::get("/b/legalpages/admin").summary("Admin editor").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/legalpages/api/documents").summary("List documents").auth(AuthLevel::Admin),
                BlockEndpoint::post("/b/legalpages/api/documents").summary("Create document").auth(AuthLevel::Admin),
                BlockEndpoint::patch("/b/legalpages/api/documents/{id}").summary("Update document").auth(AuthLevel::Admin),
                BlockEndpoint::post("/b/legalpages/api/documents/{id}/publish").summary("Publish document").auth(AuthLevel::Admin),
            ])
            .config_keys(vec![
                ConfigVar::new("SUPPERS_AI__LEGALPAGES__BG_COLOR", "Background color for public legal pages (empty = use design token default)", "")
                    .name("Background Color")
                    .input_type(InputType::Color)
                    .optional(),
                ConfigVar::new("SUPPERS_AI__LEGALPAGES__BACK_URL", "Back button URL in the header (e.g., your website homepage)", "/")
                    .name("Back Button URL")
                    .input_type(InputType::Url),
                ConfigVar::new("SUPPERS_AI__LEGALPAGES__FOOTER", "Custom footer text (HTML allowed)", "")
                    .name("Footer Text")
                    .optional(),
            ])
            .admin_url("/b/legalpages/admin")
            .can_disable(true)
            .default_enabled(false)
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let action = msg.action().to_string();
        let path = msg.path().to_string();

        match (action.as_str(), path.as_str()) {
            // Public endpoints
            ("retrieve", "/b/legalpages/terms") => self.handle_get_public(ctx, "terms").await,
            ("retrieve", "/b/legalpages/privacy") => self.handle_get_public(ctx, "privacy").await,

            // Admin UI pages (SSR)
            ("retrieve", "/b/legalpages/admin") | ("retrieve", "/b/legalpages/admin/privacy") => {
                pages::editor_page(ctx, &msg, "privacy").await
            }
            ("retrieve", "/b/legalpages/admin/terms") => {
                pages::editor_page(ctx, &msg, "terms").await
            }
            ("retrieve", "/b/legalpages/admin/settings") => pages::settings_page(ctx, &msg).await,
            ("retrieve", "/b/legalpages/admin/endpoints") => pages::endpoints_page(ctx, &msg).await,

            // Admin UI mutations (from editor save/publish)
            ("create", "/b/legalpages/admin/save") => pages::handle_save(ctx, &msg, input).await,
            ("create", "/b/legalpages/admin/publish") => {
                pages::handle_publish(ctx, &msg, input).await
            }
            ("create", "/b/legalpages/admin/settings") => {
                pages::handle_save_settings(ctx, input).await
            }

            // JSON API at /b/legalpages/api/documents/...
            ("retrieve", "/b/legalpages/api/documents") => self.handle_admin_list(ctx, &msg).await,
            ("retrieve", _) if path.starts_with("/b/legalpages/api/documents/") => {
                self.handle_admin_get(ctx, &msg).await
            }
            ("create", "/b/legalpages/api/documents") => {
                self.handle_admin_create(ctx, &msg, input).await
            }
            ("update", _)
                if path.starts_with("/b/legalpages/api/documents/")
                    && path.ends_with("/publish") =>
            {
                self.handle_admin_publish(ctx, &msg).await
            }
            ("update", _) if path.starts_with("/b/legalpages/api/documents/") => {
                self.handle_admin_update(ctx, &msg, input).await
            }
            ("delete", _) if path.starts_with("/b/legalpages/api/documents/") => {
                self.handle_admin_delete(ctx, &msg).await
            }
            _ => ui::not_found_response(&msg),
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

#[cfg(not(target_arch = "wasm32"))]
::wafer_run::register_static_block!("suppers-ai/legalpages", LegalPagesBlock);

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> SiteConfig {
        SiteConfig {
            app_name: "Acme".to_string(),
            logo_url: String::new(),
            logo_icon_url: String::new(),
            favicon_url: "/favicon.ico".to_string(),
            embedded_scripts: Vec::new(),
        }
    }

    #[test]
    fn render_legal_page_uses_public_page_template() {
        let site_cfg = site();
        let html = render_legal_page(LegalPageInputs {
            site: &site_cfg,
            title: "Terms of Service",
            content: "<p>The terms.</p>",
            version: 3,
            meta: "Last updated: 2026-04-01",
            back_url: "/",
            bg_color: "#fafafa",
            primary_color: "#6366f1",
            custom_footer: "",
        })
        .into_string();

        // Came from the shared template, not bare page chrome in this file.
        // grep-guard-html.sh forbids the page-chrome literals here, so we
        // assert on the public_page wrapper class instead.
        assert!(html.contains(r#"<main class="public-page">"#));
        assert!(html.contains("public-page__head"));
        assert!(html.contains("public-page__content"));
        assert!(html.contains("public-page__version"));
        assert!(html.contains(">v3<"));
        assert!(html.contains("Last updated: 2026-04-01"));
        assert!(html.contains("Terms of Service"));
        assert!(html.contains("The terms."));
        // Color overrides applied as inline custom properties.
        assert!(html.contains("--public-page-bg:#fafafa"));
        assert!(html.contains("--public-page-accent:#6366f1"));
        // Auto footer (year + app name).
        assert!(html.contains("public-page__footer"));
        assert!(html.contains("Acme"));
        assert!(html.contains("All rights reserved"));
        // Standard CSS bundle (not bespoke inline blob).
        assert!(html.contains(r#"<link rel="stylesheet" href="/b/static/app-"#));
    }

    #[test]
    fn render_legal_page_omits_color_inline_when_empty() {
        let site_cfg = site();
        let html = render_legal_page(LegalPageInputs {
            site: &site_cfg,
            title: "Privacy Policy",
            content: "<p>x</p>",
            version: 1,
            meta: "",
            back_url: "/",
            bg_color: "",
            primary_color: "",
            custom_footer: "Custom <strong>footer</strong>",
        })
        .into_string();

        assert!(!html.contains("--public-page-bg"));
        assert!(!html.contains("--public-page-accent"));
        // Custom footer renders verbatim (PreEscaped).
        assert!(html.contains("Custom <strong>footer</strong>"));
    }

    #[test]
    fn render_legal_page_no_meta_section_when_meta_empty_and_version_zero() {
        let site_cfg = site();
        let html = render_legal_page(LegalPageInputs {
            site: &site_cfg,
            title: "x",
            content: "",
            version: 0,
            meta: "",
            back_url: "/",
            bg_color: "",
            primary_color: "",
            custom_footer: "",
        })
        .into_string();
        assert!(!html.contains("public-page__meta"));
    }
}
