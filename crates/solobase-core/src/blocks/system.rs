use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use crate::{
    blocks::helpers::{err_not_found, ok_json, ResponseBuilder},
    ui,
};

pub struct SystemBlock;

impl SystemBlock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SystemBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new("suppers-ai/system", "0.0.1", "http-handler@v1", "System health and embedded static assets")
            .instance_mode(InstanceMode::Singleton)
            .category(wafer_run::BlockCategory::Infrastructure)
            .description("Core system services including health checks and embedded static assets (CSS, JavaScript).")
            .endpoints(vec![
                BlockEndpoint::get("/health").summary("Health check"),
                BlockEndpoint::get("/b/static/app-{hash}.css").summary("Embedded CSS"),
                BlockEndpoint::get("/b/static/htmx-{hash}.min.js").summary("Embedded htmx JS"),
                BlockEndpoint::get("/b/static/llm-chat-{hash}.js").summary("Embedded LLM chat JS"),
                BlockEndpoint::get("/b/static/files-browser-{hash}.js").summary("Embedded files-browser JS"),
                BlockEndpoint::get("/b/static/itim-latin-{hash}.woff2").summary("Embedded Itim font (latin)"),
                BlockEndpoint::get("/b/static/itim-latin-ext-{hash}.woff2").summary("Embedded Itim font (latin-ext)"),
            ])
    }

    async fn handle(&self, _ctx: &dyn Context, msg: Message, _input: InputStream) -> OutputStream {
        let path = msg.path();

        if path == "/health" {
            return ok_json(&serde_json::json!({"status": "ok"}));
        }

        // Embedded static assets (CSS, JS, fonts) with content-hash URLs for
        // cache busting. The dispatch table replaces a stack of
        // `_ if path.starts_with(...) && path.ends_with(...)` arms — order
        // matters in that form (`latin-ext` must precede `latin`), and a
        // table makes the order explicit and lookup uniform.
        // Each entry: (prefix, suffix, content_type, bytes-fn).
        type Bytes = std::borrow::Cow<'static, [u8]>;
        let table: &[(&str, &str, &str, fn() -> Bytes)] = &[
            ("/b/static/app-", ".css", "text/css; charset=utf-8", || {
                Bytes::Owned(ui::assets::css().as_bytes().to_vec())
            }),
            (
                "/b/static/htmx-",
                ".min.js",
                "application/javascript; charset=utf-8",
                || Bytes::Owned(ui::assets::htmx_js().as_bytes().to_vec()),
            ),
            (
                "/b/static/llm-chat-",
                ".js",
                "application/javascript; charset=utf-8",
                || Bytes::Owned(ui::assets::llm_chat_js().as_bytes().to_vec()),
            ),
            (
                "/b/static/files-browser-",
                ".js",
                "application/javascript; charset=utf-8",
                || Bytes::Owned(ui::assets::files_browser_js().as_bytes().to_vec()),
            ),
            // `latin-ext` must come before `latin` so the longer prefix
            // wins. The table is scanned in order.
            ("/b/static/itim-latin-ext-", ".woff2", "font/woff2", || {
                Bytes::Owned(ui::assets::itim_latin_ext_woff2().to_vec())
            }),
            ("/b/static/itim-latin-", ".woff2", "font/woff2", || {
                Bytes::Owned(ui::assets::itim_latin_woff2().to_vec())
            }),
        ];

        for (prefix, suffix, content_type, bytes_fn) in table {
            if path.starts_with(prefix) && path.ends_with(suffix) {
                return ResponseBuilder::new()
                    .set_header("Cache-Control", "public, max-age=31536000, immutable")
                    .body(bytes_fn().into_owned(), content_type);
            }
        }

        err_not_found("not found")
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
::wafer_block::register_static_block!("suppers-ai/system", SystemBlock);

#[cfg(test)]
mod tests {
    use wafer_run::meta::META_RESP_CONTENT_TYPE;

    use super::*;
    use crate::ui::assets;

    #[derive(Clone)]
    struct NopCtx;
    #[async_trait::async_trait]
    impl Context for NopCtx {
        async fn call_block(
            &self,
            _block_name: &str,
            _msg: Message,
            _input: InputStream,
        ) -> OutputStream {
            panic!("call_block not used");
        }
        fn is_cancelled(&self) -> bool {
            false
        }
        fn config_get(&self, _key: &str) -> Option<&str> {
            None
        }
        fn clone_arc(&self) -> std::sync::Arc<dyn Context> {
            std::sync::Arc::new(self.clone())
        }
    }

    #[tokio::test]
    async fn system_handle_serves_llm_chat_js() {
        let block = SystemBlock;
        let url = assets::llm_chat_js_url();
        let mut msg = Message::new(format!("retrieve:{url}"));
        msg.set_meta(wafer_run::meta::META_REQ_ACTION, "retrieve");
        msg.set_meta(wafer_run::meta::META_REQ_RESOURCE, url);

        let out = block.handle(&NopCtx, msg, InputStream::empty()).await;
        let buffered = out.collect_buffered().await.expect("response");
        let content_type = buffered
            .meta
            .iter()
            .find(|m| m.key == META_RESP_CONTENT_TYPE)
            .map(|m| m.value.as_str());
        assert_eq!(
            content_type,
            Some("application/javascript; charset=utf-8"),
            "wrong content type"
        );
        let body = std::str::from_utf8(&buffered.body).unwrap();
        assert!(
            body.contains("solobaseLlmChat"),
            "body should contain the JS module"
        );
    }

    #[tokio::test]
    async fn system_handle_serves_files_browser_js() {
        let block = SystemBlock;
        let url = assets::files_browser_js_url();
        let mut msg = Message::new(format!("retrieve:{url}"));
        msg.set_meta(wafer_run::meta::META_REQ_ACTION, "retrieve");
        msg.set_meta(wafer_run::meta::META_REQ_RESOURCE, url);

        let out = block.handle(&NopCtx, msg, InputStream::empty()).await;
        let buffered = out.collect_buffered().await.expect("response");
        let content_type = buffered
            .meta
            .iter()
            .find(|m| m.key == META_RESP_CONTENT_TYPE)
            .map(|m| m.value.as_str());
        assert_eq!(
            content_type,
            Some("application/javascript; charset=utf-8"),
            "wrong content type"
        );
        let body = std::str::from_utf8(&buffered.body).unwrap();
        assert!(
            body.starts_with("// solobase files-browser"),
            "body should start with the placeholder comment"
        );
    }
}
