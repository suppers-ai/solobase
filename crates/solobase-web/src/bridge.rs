use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/bridge.js")]
extern "C" {
    // ─── Database (sql.js) ────────────────────────────────────────────────────

    /// Load sql.js WASM, try to load existing DB from OPFS, create new if none.
    /// Sets PRAGMA foreign_keys=ON.
    pub async fn dbInit() -> JsValue;

    /// Execute SQL that modifies data (INSERT/UPDATE/DELETE/DDL).
    /// `params_json` is a JSON array of parameters.
    /// Returns rows-modified count as a string.
    #[wasm_bindgen(js_name = dbExecRaw)]
    pub fn db_exec_raw(sql: &str, params_json: &str) -> String;

    /// Execute a SELECT SQL query.
    /// `params_json` is a JSON array of parameters.
    /// Returns JSON array of row objects as a string.
    #[wasm_bindgen(js_name = dbQueryRaw)]
    pub fn db_query_raw(sql: &str, params_json: &str) -> String;

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

    // ─── Network (fetch) ──────────────────────────────────────────────────────

    /// Execute an HTTP fetch request.
    /// `headers_json` is a JSON object of header key/value pairs.
    /// `body` is the request body bytes (pass empty slice for no body).
    /// Returns JSON string: `{ status, headers, body: number[] }`.
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
}
