// sql.js ESM wrapper is statically imported. Dynamic import() is forbidden in
// Service Workers, so this must be a static import. The ESM wrapper is created
// by the build (Makefile) from the UMD sql-wasm.js.
import initSqlJs from '/sql-wasm-esm.js';

// Module-level state
let _db = null;
const SQL_WASM_PATH = '/sql-wasm.wasm';
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
