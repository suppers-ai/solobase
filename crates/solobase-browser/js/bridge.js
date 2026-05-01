// sql.js ESM wrapper is statically imported. Dynamic import() is forbidden in
// Service Workers, so this must be a static import. The wrapper is vendored
// inside solobase-browser and written to `/vendor/sql-wasm-esm.js` by the
// framework's `export-assets` bin; `/vendor/sql-wasm.wasm` is the matching
// binary loaded by sql.js at runtime via its `locateFile` callback.
import initSqlJs from '/vendor/sql-wasm-esm.js';

// Module-level state
let _db = null;
const SQL_WASM_PATH = '/vendor/sql-wasm.wasm';
const DB_FILENAME = 'solobase.db';

// ─── Database (sql.js) ────────────────────────────────────────────────────────

/**
 * Load sql.js WASM, try to load existing DB from OPFS, create new if none exists.
 * Sets PRAGMA foreign_keys=ON.
 */
export async function dbInit() {
    const SQL = await initSqlJs({
        locateFile: () => SQL_WASM_PATH,
    });

    const root = await navigator.storage.getDirectory();
    let existingData = null;
    try {
        const fileHandle = await root.getFileHandle(DB_FILENAME);
        const file = await fileHandle.getFile();
        const buffer = await file.arrayBuffer();
        if (buffer.byteLength > 0) {
            existingData = new Uint8Array(buffer);
        }
    } catch (_e) {
        // File does not exist yet — start fresh
    }

    if (existingData) {
        _db = new SQL.Database(existingData);
    } else {
        _db = new SQL.Database();
    }

    _db.run('PRAGMA foreign_keys = ON;');
}

/**
 * Execute SQL that modifies data (INSERT/UPDATE/DELETE/DDL).
 * @param {string} sql
 * @param {string} paramsJson - JSON array of parameters
 * @returns {string} rows-modified count as string
 */
export function dbExecRaw(sql, paramsJson) {
    const params = JSON.parse(paramsJson);
    _db.run(sql, params);
    const rowsModified = _db.getRowsModified();
    return String(rowsModified);
}

/**
 * Execute a SELECT SQL query.
 * @param {string} sql
 * @param {string} paramsJson - JSON array of parameters
 * @returns {string} JSON array of row objects
 */
export function dbQueryRaw(sql, paramsJson) {
    const params = JSON.parse(paramsJson);
    const results = _db.exec(sql, params);
    if (!results || results.length === 0) {
        return '[]';
    }
    const { columns, values } = results[0];
    const rows = values.map((row) => {
        const obj = {};
        columns.forEach((col, i) => {
            obj[col] = row[i];
        });
        return obj;
    });
    return JSON.stringify(rows);
}

/**
 * Export the sql.js DB to a Uint8Array and write it to OPFS at `solobase.db`.
 */
export async function dbFlush() {
    if (!_db) return;
    const data = _db.export();
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle(DB_FILENAME, { create: true });
    const writable = await fileHandle.createWritable();
    await writable.write(data);
    await writable.close();
}

// ─── Storage (OPFS) ──────────────────────────────────────────────────────────

const STORAGE_DIR = 'storage';

async function getStorageRoot() {
    const root = await navigator.storage.getDirectory();
    return root.getDirectoryHandle(STORAGE_DIR, { create: true });
}

async function getFolderHandle(storageRoot, folder, create = false) {
    return storageRoot.getDirectoryHandle(folder, { create });
}

/**
 * Write file + metadata to OPFS.
 * @param {string} folder
 * @param {string} key
 * @param {Uint8Array} data
 * @param {string} contentType
 */
export async function storagePut(folder, key, data, contentType) {
    const storageRoot = await getStorageRoot();
    const folderHandle = await getFolderHandle(storageRoot, folder, true);

    // Write file data
    const fileHandle = await folderHandle.getFileHandle(key, { create: true });
    const writable = await fileHandle.createWritable();
    await writable.write(data);
    await writable.close();

    // Write metadata
    const meta = { content_type: contentType, size: data.length };
    const metaHandle = await folderHandle.getFileHandle(`${key}.__meta__`, { create: true });
    const metaWritable = await metaHandle.createWritable();
    await metaWritable.write(JSON.stringify(meta));
    await metaWritable.close();
}

/**
 * Read file + metadata from OPFS.
 * @param {string} folder
 * @param {string} key
 * @returns {string} JSON string: { data: number[], meta: { content_type, size } }
 */
export async function storageGet(folder, key) {
    const storageRoot = await getStorageRoot();
    const folderHandle = await getFolderHandle(storageRoot, folder, false);

    // Read file data
    const fileHandle = await folderHandle.getFileHandle(key);
    const file = await fileHandle.getFile();
    const buffer = await file.arrayBuffer();
    const dataArray = Array.from(new Uint8Array(buffer));

    // Read metadata
    let meta = { content_type: 'application/octet-stream', size: dataArray.length };
    try {
        const metaHandle = await folderHandle.getFileHandle(`${key}.__meta__`);
        const metaFile = await metaHandle.getFile();
        const metaText = await metaFile.text();
        meta = JSON.parse(metaText);
    } catch (_e) {
        // No metadata file — use defaults
    }

    return JSON.stringify({ data: dataArray, meta });
}

/**
 * Delete file + metadata from OPFS.
 * @param {string} folder
 * @param {string} key
 */
export async function storageDelete(folder, key) {
    const storageRoot = await getStorageRoot();
    const folderHandle = await getFolderHandle(storageRoot, folder, false);
    await folderHandle.removeEntry(key);
    try {
        await folderHandle.removeEntry(`${key}.__meta__`);
    } catch (_e) {
        // Metadata may not exist
    }
}

/**
 * List files in a folder.
 * @param {string} folder
 * @param {string} prefix
 * @param {number} limit
 * @param {number} offset
 * @returns {string} JSON array of key strings
 */
export async function storageList(folder, prefix, limit, offset) {
    const storageRoot = await getStorageRoot();
    const folderHandle = await getFolderHandle(storageRoot, folder, false);

    const keys = [];
    for await (const [name] of folderHandle.entries()) {
        // Skip metadata files
        if (name.endsWith('.__meta__')) continue;
        if (!prefix || name.startsWith(prefix)) {
            keys.push(name);
        }
    }

    keys.sort();
    const page = keys.slice(offset, limit > 0 ? offset + limit : undefined);
    return JSON.stringify(page);
}

/**
 * Create OPFS directory under storage root.
 * @param {string} name
 */
export async function storageCreateFolder(name) {
    const storageRoot = await getStorageRoot();
    await storageRoot.getDirectoryHandle(name, { create: true });
}

/**
 * Remove OPFS directory recursively.
 * @param {string} name
 */
export async function storageDeleteFolder(name) {
    const storageRoot = await getStorageRoot();
    await storageRoot.removeEntry(name, { recursive: true });
}

/**
 * List top-level storage directories.
 * @returns {string} JSON array of folder name strings
 */
export async function storageListFolders() {
    const storageRoot = await getStorageRoot();
    const folders = [];
    for await (const [name, handle] of storageRoot.entries()) {
        if (handle.kind === 'directory') {
            folders.push(name);
        }
    }
    folders.sort();
    return JSON.stringify(folders);
}

// ─── Asset loader bridge (SW → main thread) ─────────────────────────────────
//
// The Rust SwAssetLoader (running inside this SW) calls loadAsset() to ask the
// main thread to fetch + verify + init an external asset (ffmpeg.wasm, etc).
// We postMessage a 'load-asset-request' to the first window client, then wait
// for the matching 'load-asset-response' to arrive at sw.js's message listener.
// sw.js routes the response back here via globalThis.__solobaseCompleteAssetLoad.

const _pendingAssetLoads = new Map(); // correlationId -> resolve fn

/**
 * Load an external asset by id by postMessaging the main thread.
 * @param {string} assetId
 * @param {string} manifestJson - JSON-serialised ExternalAsset {id, loader, version, url, sha256}
 * @returns {Promise<{status: 'ready'|'pending'|'failed', error?: string}>}
 */
export async function loadAsset(assetId, manifestJson) {
    const manifest = JSON.parse(manifestJson);

    // Find any window client. If none, fail fast — no point waiting.
    const clients = await self.clients.matchAll({ type: 'window', includeUncontrolled: false });
    if (clients.length === 0) {
        return { status: 'failed', error: 'no active page — open the app in a tab to load assets' };
    }

    const correlationId = `asset-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
    const replyPromise = new Promise((resolve) => {
        _pendingAssetLoads.set(correlationId, resolve);
        // Bound the wait so a misbehaving page can't block the SW forever.
        setTimeout(() => {
            if (_pendingAssetLoads.has(correlationId)) {
                _pendingAssetLoads.delete(correlationId);
                resolve({ status: 'failed', error: 'load-asset timed out' });
            }
        }, manifest.timeout_ms ?? 120_000);
    });

    clients[0].postMessage({
        type: 'load-asset-request',
        id: correlationId,
        manifest,
    });

    return await replyPromise;
}

/**
 * Resolve a pending loadAsset() call. Called from sw.js's message handler
 * when a 'load-asset-response' arrives from the main thread. Exposed on
 * globalThis so sw.js (a separate top-level script) can reach it without
 * importing this module — wasm-bindgen owns the import path here.
 *
 * @param {string} correlationId
 * @param {{status: 'ready'|'pending'|'failed', error?: string}} reply
 */
export function _completeAssetLoad(correlationId, reply) {
    const resolve = _pendingAssetLoads.get(correlationId);
    if (resolve) {
        _pendingAssetLoads.delete(correlationId);
        resolve(reply);
    }
}

globalThis.__solobaseCompleteAssetLoad = _completeAssetLoad;

// ─── LLM (SW → page postMessage bridge) ─────────────────────────────────────
//
// Mirrors the loadAsset pattern: correlation-id keyed postMessage to a window
// client; resolvers kept in a Map; sw.js routes replies via globalThis hook.
// Streams use an async queue so Rust can `await` one chunk at a time while
// many chunks are buffered in flight.

const _pendingLlmRequests = new Map();   // id -> { resolve, reject } (one-shot: create, unload)
const _activeLlmStreams   = new Map();   // id -> { pushChunk, closeOk, closeErr, queue, waiters }

async function _postToWindowClient(payload) {
    const clients = await self.clients.matchAll({ type: 'window', includeUncontrolled: false });
    if (clients.length === 0) {
        throw new Error('no active page — open the app in a tab');
    }
    clients[0].postMessage(payload);
}

function _mkLlmId(prefix) {
    return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

/**
 * Create + initialise a WebLLM engine on the page. Resolves when loaded.
 * @param {string} modelId
 * @returns {Promise<void>}
 */
export async function llmCreateEngine(modelId) {
    const id = _mkLlmId('llm-create');
    const replyPromise = new Promise((resolve, reject) => {
        _pendingLlmRequests.set(id, { resolve, reject });
    });
    await _postToWindowClient({ type: 'llm-create-engine-request', id, modelId });
    return await replyPromise;
}

/**
 * Unload the engine on the page.
 * @param {string} modelId
 * @returns {Promise<void>}
 */
export async function llmUnloadEngine(modelId) {
    const id = _mkLlmId('llm-unload');
    const replyPromise = new Promise((resolve, reject) => {
        _pendingLlmRequests.set(id, { resolve, reject });
    });
    await _postToWindowClient({ type: 'llm-unload-request', id, modelId });
    return await replyPromise;
}

/**
 * Start a streaming chat completion. Returns a stream id the caller pumps
 * with `llmNextChunk`.
 * @param {string} bodyJson - JSON request body as built by Rust encode_request_body
 * @returns {Promise<string>} stream id
 */
export async function llmChatStream(bodyJson) {
    const id = _mkLlmId('llm-stream');
    const queue = [];     // { kind: 'chunk'|'done'|'error', payload? }
    const waiters = [];   // Array<(frame) => void>
    const pushChunk = (frame) => {
        if (waiters.length > 0) {
            waiters.shift()(frame);
        } else {
            queue.push(frame);
        }
    };
    _activeLlmStreams.set(id, {
        pushChunk,
        closeOk: () => pushChunk({ kind: 'done' }),
        closeErr: (err) => pushChunk({ kind: 'error', payload: err }),
        queue,
        waiters,
    });
    await _postToWindowClient({ type: 'llm-chat-stream-request', id, body: bodyJson });
    return id;
}

/**
 * Pull the next frame from a stream. Blocks until a frame arrives.
 * After a terminal frame (done/error) the stream entry is removed.
 * @param {string} id
 * @returns {Promise<string>} JSON-encoded frame: {kind:'chunk',payload}|{kind:'done'}|{kind:'error',payload}
 */
export async function llmNextChunk(id) {
    const stream = _activeLlmStreams.get(id);
    if (!stream) {
        return JSON.stringify({ kind: 'error', payload: 'unknown stream id' });
    }
    let frame;
    if (stream.queue.length > 0) {
        frame = stream.queue.shift();
    } else {
        frame = await new Promise((resolve) => stream.waiters.push(resolve));
    }
    if (frame.kind === 'done' || frame.kind === 'error') {
        _activeLlmStreams.delete(id);
    }
    return JSON.stringify(frame);
}

/**
 * Cancel an in-flight stream.
 * @param {string} id
 */
export async function llmCancelStream(id) {
    const stream = _activeLlmStreams.get(id);
    if (stream) {
        // Terminate any pending awaiter with an error frame (no-op if the
        // Rust side has already broken out of its loop).
        stream.closeErr('cancelled');
        // Remove the entry now rather than waiting for the (possibly never
        // called) next llmNextChunk to notice the terminal frame — the Rust
        // side breaks its loop immediately after calling cancel_stream.
        _activeLlmStreams.delete(id);
    }
    await _postToWindowClient({ type: 'llm-stream-cancel', id });
}

/**
 * Called by sw.js when a page reply arrives. Routes to the pending request
 * or active stream by id.
 *
 * Page → SW message shapes:
 *   { type: 'llm-create-engine-response', id, error? }
 *   { type: 'llm-unload-response', id, error? }
 *   { type: 'llm-chat-stream-chunk', id, chunk }    // chunk = OpenAI chunk JSON string
 *   { type: 'llm-chat-stream-done', id }
 *   { type: 'llm-chat-stream-error', id, error }
 */
export function _completeLlmMessage(msg) {
    if (msg.type === 'llm-create-engine-response' || msg.type === 'llm-unload-response') {
        const pending = _pendingLlmRequests.get(msg.id);
        if (!pending) return;
        _pendingLlmRequests.delete(msg.id);
        if (msg.error) pending.reject(new Error(msg.error));
        else pending.resolve();
        return;
    }
    const stream = _activeLlmStreams.get(msg.id);
    if (!stream) return;
    if (msg.type === 'llm-chat-stream-chunk') {
        stream.pushChunk({ kind: 'chunk', payload: msg.chunk });
    } else if (msg.type === 'llm-chat-stream-done') {
        stream.closeOk();
    } else if (msg.type === 'llm-chat-stream-error') {
        stream.closeErr(msg.error ?? 'unknown error');
    }
}

globalThis.__solobaseCompleteLlmMessage = _completeLlmMessage;

// ─── Cookies (readable from SW via CookieStore API) ─────────────────────────
//
// The Service-Worker spec filters the `Cookie` header out of
// `FetchEvent.request.headers`: the SW cannot read it back from a Request.
// The cookies ARE sent over the wire for same-origin requests and are
// readable via `self.cookieStore.getAll()` (available in Chromium-based
// browsers; Firefox behind a flag). We surface them to Rust so
// `convert::request_to_message` can inject a synthetic `http.header.cookie`
// meta; downstream consumers (e.g. the `suppers-ai/auth` block) then see
// the cookie exactly as they would on a native deployment.

/**
 * Read all cookies from the SW's CookieStore and format as a Cookie header.
 * Returns an empty string if CookieStore isn't available or no cookies exist.
 * @returns {Promise<string>}
 */
export async function readCookieHeader() {
    if (typeof self.cookieStore === 'undefined' || !self.cookieStore.getAll) {
        return '';
    }
    try {
        const cookies = await self.cookieStore.getAll();
        return cookies.map((c) => `${c.name}=${c.value}`).join('; ');
    } catch (_e) {
        return '';
    }
}

// ─── Network (fetch) ─────────────────────────────────────────────────────────

/**
 * Execute an HTTP fetch request.
 * @param {string} method
 * @param {string} url
 * @param {string} headersJson - JSON object of header key/value pairs
 * @param {Uint8Array|null} body
 * @returns {string} JSON string: { status, headers, body: number[] }
 */
export async function httpFetch(method, url, headersJson, body) {
    const headersObj = JSON.parse(headersJson);
    const init = {
        method,
        headers: headersObj,
    };

    if (body && body.length > 0) {
        init.body = body;
    }

    const response = await fetch(url, init);

    const responseHeaders = {};
    response.headers.forEach((value, name) => {
        responseHeaders[name] = value;
    });

    const responseBuffer = await response.arrayBuffer();
    const responseBody = Array.from(new Uint8Array(responseBuffer));

    return JSON.stringify({
        status: response.status,
        headers: responseHeaders,
        body: responseBody,
    });
}
