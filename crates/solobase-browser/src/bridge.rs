use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/bridge.js")]
extern "C" {
    // ─── Database (sql.js) ────────────────────────────────────────────────────

    /// Load sql.js WASM, try to load existing DB from OPFS, create new if none.
    /// Sets PRAGMA foreign_keys=ON.
    pub async fn dbInit() -> JsValue;

    /// Execute SQL that modifies data (INSERT/UPDATE/DELETE/DDL).
    /// `params_json` is a JSON array of parameters.
    /// Returns rows-modified count as a string. Throws on SQL error.
    #[wasm_bindgen(catch, js_name = dbExecRaw)]
    pub fn db_exec_raw(sql: &str, params_json: &str) -> Result<String, JsValue>;

    /// Execute a SELECT SQL query.
    /// `params_json` is a JSON array of parameters.
    /// Returns JSON array of row objects as a string. Throws on SQL error.
    #[wasm_bindgen(catch, js_name = dbQueryRaw)]
    pub fn db_query_raw(sql: &str, params_json: &str) -> Result<String, JsValue>;

    /// Export the sql.js DB to OPFS at `solobase.db`.
    pub async fn dbFlush() -> JsValue;

    // ─── Storage (OPFS) ───────────────────────────────────────────────────────

    /// Write file + metadata to OPFS.
    #[wasm_bindgen(js_name = storagePut)]
    pub async fn storage_put(folder: &str, key: &str, data: &[u8], content_type: &str) -> JsValue;

    /// Read file + metadata from OPFS.
    /// Returns JSON string: `{ data: number[], meta: { content_type, size } }`.
    #[wasm_bindgen(js_name = storageGet)]
    pub async fn storage_get(folder: &str, key: &str) -> JsValue;

    /// Delete file + metadata from OPFS.
    #[wasm_bindgen(js_name = storageDelete)]
    pub async fn storage_delete(folder: &str, key: &str) -> JsValue;

    /// List files in a folder matching a prefix, with pagination.
    /// Returns JSON array of key strings.
    #[wasm_bindgen(js_name = storageList)]
    pub async fn storage_list(folder: &str, prefix: &str, limit: u32, offset: u32) -> JsValue;

    /// Create an OPFS directory under the storage root.
    #[wasm_bindgen(js_name = storageCreateFolder)]
    pub async fn storage_create_folder(name: &str) -> JsValue;

    /// Remove an OPFS directory recursively.
    #[wasm_bindgen(js_name = storageDeleteFolder)]
    pub async fn storage_delete_folder(name: &str) -> JsValue;

    /// List top-level storage directories.
    /// Returns JSON array of folder name strings.
    #[wasm_bindgen(js_name = storageListFolders)]
    pub async fn storage_list_folders() -> JsValue;

    // ─── Cookies (read from SW via CookieStore API) ──────────────────────────

    /// Read all cookies via `self.cookieStore.getAll()` and return them
    /// formatted as a `Cookie` header value (e.g., `auth_token=xyz; foo=bar`).
    /// Empty string if CookieStore isn't available or no cookies exist.
    ///
    /// Workaround for the SW spec filtering the `Cookie` header out of
    /// `FetchEvent.request.headers`. See `bridge.js::readCookieHeader`.
    #[wasm_bindgen(js_name = readCookieHeader)]
    pub async fn read_cookie_header() -> JsValue;

    // ─── Network (fetch) ──────────────────────────────────────────────────────

    /// Execute an HTTP fetch request.
    /// `headers_json` is a JSON object of header key/value pairs.
    /// `body` is the request body bytes (pass empty slice for no body).
    /// Returns a plain JS object `{ status, headers, body: Uint8Array }` —
    /// NOT a JSON string. Decode directly with `serde_wasm_bindgen::from_value`.
    #[wasm_bindgen(js_name = httpFetch)]
    pub async fn http_fetch(method: &str, url: &str, headers_json: &str, body: &[u8]) -> JsValue;

    // ─── Asset loader bridge (SW → main thread) ───────────────────────────────

    /// Load an external asset by id. Returns a Promise that resolves to an
    /// `AssetLoadStatus`-shaped JS object: `{ status: "ready" | "failed",
    /// error?: string }`.
    ///
    /// Called from `SwAssetLoader::load`. The id is the block's manifest
    /// asset id; `manifest_json` is a serialised `ExternalAsset` (`{id,
    /// loader, version, url, sha256}`) — the JS side does the actual fetch +
    /// sha256 verification + named-loader init by postMessaging the main
    /// thread (only place where `fetch`, `crypto.subtle`, and JS-level
    /// loaders like ffmpeg.wasm are reachable).
    #[wasm_bindgen(js_name = loadAsset)]
    pub async fn load_asset(asset_id: &str, manifest_json: &str) -> JsValue;

    // ─── LLM bridge ───────────────────────────────────────────────────────────

    /// Unload the engine on the page.
    #[wasm_bindgen(js_name = llmUnloadEngine, catch)]
    pub async fn llm_unload_engine(model_id: &str) -> Result<JsValue, JsValue>;

    /// Start a streaming chat completion. Returns the stream id as a JS string.
    #[wasm_bindgen(js_name = llmChatStream, catch)]
    pub async fn llm_chat_stream(body_json: &str) -> Result<JsValue, JsValue>;

    /// Pull the next frame from an LLM chat stream.
    /// Frame JSON: `{kind:'chunk', payload:<openai chunk json>}` |
    ///             `{kind:'done'}` | `{kind:'error', payload:<string>}`.
    #[wasm_bindgen(js_name = llmNextStreamFrame, catch)]
    pub async fn llm_next_stream_frame(stream_id: &str) -> Result<JsValue, JsValue>;

    /// Cancel an in-flight stream.
    #[wasm_bindgen(js_name = llmCancelStream, catch)]
    pub async fn llm_cancel_stream(stream_id: &str) -> Result<JsValue, JsValue>;

    // ─── Image bridge ─────────────────────────────────────────────────────────

    /// Load the page-side T2I engine for `model_id`. One-shot — resolves when
    /// the model is fully loaded onto the WebGPU device.
    #[wasm_bindgen(js_name = imageLoadEngine, catch)]
    pub async fn image_load_engine(model_id: &str) -> Result<JsValue, JsValue>;

    /// Unload the page-side T2I engine.
    #[wasm_bindgen(js_name = imageUnloadEngine, catch)]
    pub async fn image_unload_engine() -> Result<JsValue, JsValue>;

    /// Start an image generation. Returns the request id as a JS string. Pump
    /// frames with `imageNextFrame`.
    #[wasm_bindgen(js_name = imageStartGenerate, catch)]
    pub async fn image_start_generate(body_json: &str) -> Result<JsValue, JsValue>;

    /// Pull the next frame from an image generation. Frame JSON:
    ///   `{kind:'progress', payload:{stage, bytes_downloaded?, bytes_total?}}` |
    ///   `{kind:'done', payload:{data:<base64>, mime_type}}` |
    ///   `{kind:'error', payload:<string>}`.
    #[wasm_bindgen(js_name = imageNextFrame, catch)]
    pub async fn image_next_frame(request_id: &str) -> Result<JsValue, JsValue>;

    /// Cancel an in-flight generation.
    #[wasm_bindgen(js_name = imageCancelStream, catch)]
    pub async fn image_cancel_stream(request_id: &str) -> Result<JsValue, JsValue>;

    // ─── Embed bridge ─────────────────────────────────────────────────────────

    /// Embed `texts` using the page-resident Transformers.js pipeline for
    /// `model_id`. Resolves to a JSON string `{"vectors":[[...]],"dims":<n>}`.
    #[wasm_bindgen(catch, js_name = embedRun)]
    pub async fn embed_run(model_id: &str, texts_json: &str) -> Result<JsValue, JsValue>;

    /// Eagerly load the pipeline for `model_id`. Optional — `embedRun` will
    /// lazy-load if needed.
    #[wasm_bindgen(catch, js_name = embedCreatePipeline)]
    pub async fn embed_create_pipeline(model_id: &str) -> Result<JsValue, JsValue>;

    /// Free the page-resident pipeline for `model_id`. Optional.
    #[wasm_bindgen(catch, js_name = embedUnload)]
    pub async fn embed_unload(model_id: &str) -> Result<JsValue, JsValue>;
}
