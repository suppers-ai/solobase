# Solobase Web Phase 1: Browser WASM Runtime

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compile Solobase to WASM and run the full platform in the browser via a Service Worker, with sql.js + OPFS replacing SQLite and the filesystem.

**Architecture:** New `solobase-web` crate (`solobase/crates/solobase-web/`) targets `wasm32-unknown-unknown`. A JavaScript bridge (`bridge.js`) wraps sql.js and OPFS for Rust to call via `wasm-bindgen`. A Service Worker (`sw.js`) loads the WASM module, intercepts all fetch events, converts them to WAFER Messages, dispatches through the site-main flow, and returns HTML/JSON responses. `solobase-core` is unchanged — all existing blocks work because they talk to service traits, not implementations.

**Tech Stack:** Rust (wasm-bindgen, wasm-bindgen-futures, web-sys, js-sys), wasm-pack, sql.js, OPFS API, Service Worker API

**Spec:** `docs/superpowers/specs/2026-04-11-solobase-web-browser-wasm-design.md`

---

## File Structure

```
solobase/crates/solobase-web/
├── Cargo.toml
├── src/
│   ├── lib.rs              # wasm-bindgen entry points: initialize() + handle_request()
│   ├── bridge.rs           # #[wasm_bindgen] extern declarations for JS bridge functions
│   ├── database.rs         # BrowserDatabaseService implementing DatabaseService trait
│   ├── storage.rs          # BrowserStorageService implementing StorageService trait
│   ├── network.rs          # BrowserNetworkService implementing NetworkService trait
│   ├── logger.rs           # ConsoleLogger implementing LoggerService trait
│   ├── convert.rs          # HTTP Request ↔ WAFER Message conversion (browser equivalent of http_to_message)
│   └── config.rs           # Browser-specific config loading from sql.js
└── js/
    ├── bridge.js           # JS functions called by Rust: sql.js ops, OPFS ops, fetch wrapper
    ├── sw.js               # Service Worker: loads WASM, intercepts fetch, calls handle_request()
    ├── loader.js           # Main page script: registers SW, shows loading state
    └── index.html          # Shell HTML page loaded on first visit
```

---

### Task 1: Crate Scaffold + Verify WASM Compilation

**Files:**
- Create: `crates/solobase-web/Cargo.toml`
- Create: `crates/solobase-web/src/lib.rs`
- Modify: `Cargo.toml` (workspace root — add member)

- [ ] **Step 1: Add solobase-web to workspace members**

In `solobase/Cargo.toml`, add the new crate to the workspace:

```toml
members = ["crates/solobase", "crates/solobase-core", "crates/solobase-plans", "crates/solobase-web"]
```

- [ ] **Step 2: Create Cargo.toml**

```toml
[package]
name = "solobase-web"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Solobase compiled to WASM for running in the browser via Service Worker"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# WASM bindings
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Request", "Response", "ResponseInit",
    "Headers", "ReadableStream",
    "console",
] }
js-sys = "0.3"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# WAFER runtime (no tokio, no wasmi, no reqwest)
wafer-run = { path = "../../../wafer-run/crates/wafer-run", default-features = false }
wafer-core = { path = "../../../wafer-run/crates/wafer-core" }
wafer-block = { path = "../../../wafer-run/crates/wafer-block" }

# Solobase blocks
solobase-core = { path = "../solobase-core", default-features = false }

# Middleware blocks
wafer-block-auth-validator = { path = "../../../wafer-run/crates/wafer-block-auth-validator" }
wafer-block-cors = { path = "../../../wafer-run/crates/wafer-block-cors" }
wafer-block-iam-guard = { path = "../../../wafer-run/crates/wafer-block-iam-guard" }
wafer-block-inspector = { path = "../../../wafer-run/crates/wafer-block-inspector" }
wafer-block-readonly-guard = { path = "../../../wafer-run/crates/wafer-block-readonly-guard" }
wafer-block-router = { path = "../../../wafer-run/crates/wafer-block-router" }
wafer-block-security-headers = { path = "../../../wafer-run/crates/wafer-block-security-headers" }

# Config service
wafer-block-config = { path = "../../../wafer-run/crates/wafer-block-config" }

# Crypto service (Argon2 + JWT — pure Rust, works on wasm32)
wafer-block-crypto = { path = "../../../wafer-run/crates/wafer-block-crypto" }

# Async traits
async-trait = "0.1"

# Utils
hex = "0.4"

[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.2", features = ["js"] }
```

- [ ] **Step 3: Create minimal lib.rs**

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn ping() -> String {
    "solobase-web".to_string()
}
```

- [ ] **Step 4: Verify WASM compilation**

Run: `cd solobase && wasm-pack build crates/solobase-web --target web --dev`

Expected: Build succeeds. If any dependency fails to compile for wasm32, fix it before proceeding. Common issues:
- Missing `getrandom` js feature — already added in Cargo.toml
- `parking_lot` — should work on wasm32 (uses spin locks)
- Middleware blocks pulling in native deps — check each one

If a middleware block doesn't compile for wasm32, add `#[cfg(not(target_arch = "wasm32"))]` guards or skip registering it (note this for Task 7).

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-web/ Cargo.toml
git commit -m "feat(solobase-web): scaffold crate with wasm-pack build"
```

---

### Task 2: JavaScript Bridge — sql.js + OPFS + fetch

**Files:**
- Create: `crates/solobase-web/js/bridge.js`
- Create: `crates/solobase-web/src/bridge.rs`

The bridge layer is the boundary between Rust and browser APIs. Rust declares extern functions via `wasm-bindgen`; JS implements them using sql.js, OPFS, and fetch.

- [ ] **Step 1: Create bridge.js**

```javascript
// bridge.js — JS functions called from Rust via wasm-bindgen
//
// sql.js provides a synchronous in-memory SQLite database.
// OPFS provides persistent file storage.
// fetch() provides network requests.

import initSqlJs from 'sql.js';

let db = null;
let sqlJsReady = false;

// ── Database (sql.js) ──────────────────────────────────────────────

export async function dbInit() {
    const SQL = await initSqlJs({
        locateFile: file => `/sql-wasm.wasm`
    });

    // Try to load existing database from OPFS
    let data = null;
    try {
        const root = await navigator.storage.getDirectory();
        const fileHandle = await root.getFileHandle('solobase.db');
        const file = await fileHandle.getFile();
        const buffer = await file.arrayBuffer();
        data = new Uint8Array(buffer);
    } catch (e) {
        // No existing database — first run
    }

    db = data ? new SQL.Database(data) : new SQL.Database();
    db.run("PRAGMA journal_mode=WAL");
    db.run("PRAGMA foreign_keys=ON");
    sqlJsReady = true;
}

export function dbExecRaw(sql, paramsJson) {
    const params = JSON.parse(paramsJson);
    db.run(sql, params);
    return db.getRowsModified().toString();
}

export function dbQueryRaw(sql, paramsJson) {
    const params = JSON.parse(paramsJson);
    const stmt = db.prepare(sql);
    if (params.length > 0) stmt.bind(params);
    const rows = [];
    while (stmt.step()) {
        rows.push(stmt.getAsObject());
    }
    stmt.free();
    return JSON.stringify(rows);
}

export async function dbFlush() {
    if (!db) return;
    const data = db.export();
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle('solobase.db', { create: true });
    const writable = await fileHandle.createWritable();
    await writable.write(data);
    await writable.close();
}

// ── Storage (OPFS) ────────────────────────────────────────────────

async function getStorageDir() {
    const root = await navigator.storage.getDirectory();
    try {
        return await root.getDirectoryHandle('storage', { create: true });
    } catch {
        return await root.getDirectoryHandle('storage', { create: true });
    }
}

async function getFolderDir(folder) {
    const storageDir = await getStorageDir();
    return await storageDir.getDirectoryHandle(folder, { create: true });
}

export async function storagePut(folder, key, data, contentType) {
    const dir = await getFolderDir(folder);
    const fileHandle = await dir.getFileHandle(key, { create: true });
    const writable = await fileHandle.createWritable();
    await writable.write(data);
    await writable.close();
    // Store metadata alongside the file
    const metaHandle = await dir.getFileHandle(key + '.__meta__', { create: true });
    const metaWritable = await metaHandle.createWritable();
    await metaWritable.write(JSON.stringify({ content_type: contentType, size: data.byteLength }));
    await metaWritable.close();
}

export async function storageGet(folder, key) {
    const dir = await getFolderDir(folder);
    const fileHandle = await dir.getFileHandle(key);
    const file = await fileHandle.getFile();
    const data = new Uint8Array(await file.arrayBuffer());

    let meta = { content_type: 'application/octet-stream', size: data.byteLength };
    try {
        const metaHandle = await dir.getFileHandle(key + '.__meta__');
        const metaFile = await metaHandle.getFile();
        meta = JSON.parse(await metaFile.text());
    } catch (e) { /* no meta file */ }

    return JSON.stringify({ data: Array.from(data), meta });
}

export async function storageDelete(folder, key) {
    const dir = await getFolderDir(folder);
    await dir.removeEntry(key);
    try { await dir.removeEntry(key + '.__meta__'); } catch (e) { /* ok */ }
}

export async function storageList(folder, prefix, limit, offset) {
    const dir = await getFolderDir(folder);
    const entries = [];
    for await (const [name, handle] of dir) {
        if (name.endsWith('.__meta__')) continue;
        if (prefix && !name.startsWith(prefix)) continue;
        let meta = { content_type: 'application/octet-stream', size: 0 };
        try {
            const metaHandle = await dir.getFileHandle(name + '.__meta__');
            const metaFile = await metaHandle.getFile();
            meta = JSON.parse(await metaFile.text());
        } catch (e) { /* no meta */ }
        entries.push({ key: name, content_type: meta.content_type, size: meta.size });
    }
    return JSON.stringify(entries.slice(offset, offset + limit));
}

export async function storageCreateFolder(name) {
    const storageDir = await getStorageDir();
    await storageDir.getDirectoryHandle(name, { create: true });
}

export async function storageDeleteFolder(name) {
    const storageDir = await getStorageDir();
    await storageDir.removeEntry(name, { recursive: true });
}

export async function storageListFolders() {
    const storageDir = await getStorageDir();
    const folders = [];
    for await (const [name, handle] of storageDir) {
        if (handle.kind === 'directory') {
            folders.push(name);
        }
    }
    return JSON.stringify(folders);
}

// ── Network (fetch) ───────────────────────────────────────────────

export async function httpFetch(method, url, headersJson, body) {
    const headers = JSON.parse(headersJson);
    const init = { method, headers };
    if (body && body.byteLength > 0) {
        init.body = body;
    }
    const resp = await fetch(url, init);
    const respBody = new Uint8Array(await resp.arrayBuffer());
    const respHeaders = {};
    resp.headers.forEach((value, key) => {
        respHeaders[key] = respHeaders[key] ? [...respHeaders[key], value] : [value];
    });
    return JSON.stringify({
        status: resp.status,
        headers: respHeaders,
        body: Array.from(respBody),
    });
}
```

- [ ] **Step 2: Create bridge.rs — Rust extern declarations**

```rust
//! Rust declarations for JS bridge functions.
//! These map to exports in js/bridge.js.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/bridge.js")]
extern "C" {
    // ── Database ──
    #[wasm_bindgen(js_name = "dbInit")]
    pub async fn db_init() -> JsValue;

    /// Execute SQL that modifies data. Returns rows-modified count as string.
    #[wasm_bindgen(js_name = "dbExecRaw")]
    pub fn db_exec_raw(sql: &str, params_json: &str) -> String;

    /// Execute SQL query. Returns JSON array of row objects.
    #[wasm_bindgen(js_name = "dbQueryRaw")]
    pub fn db_query_raw(sql: &str, params_json: &str) -> String;

    /// Flush sql.js database to OPFS for persistence.
    #[wasm_bindgen(js_name = "dbFlush")]
    pub async fn db_flush() -> JsValue;

    // ── Storage ──
    #[wasm_bindgen(js_name = "storagePut")]
    pub async fn storage_put(folder: &str, key: &str, data: &[u8], content_type: &str) -> JsValue;

    #[wasm_bindgen(js_name = "storageGet")]
    pub async fn storage_get(folder: &str, key: &str) -> JsValue;

    #[wasm_bindgen(js_name = "storageDelete")]
    pub async fn storage_delete(folder: &str, key: &str) -> JsValue;

    #[wasm_bindgen(js_name = "storageList")]
    pub async fn storage_list(folder: &str, prefix: &str, limit: u32, offset: u32) -> JsValue;

    #[wasm_bindgen(js_name = "storageCreateFolder")]
    pub async fn storage_create_folder(name: &str) -> JsValue;

    #[wasm_bindgen(js_name = "storageDeleteFolder")]
    pub async fn storage_delete_folder(name: &str) -> JsValue;

    #[wasm_bindgen(js_name = "storageListFolders")]
    pub async fn storage_list_folders() -> JsValue;

    // ── Network ──
    #[wasm_bindgen(js_name = "httpFetch")]
    pub async fn http_fetch(method: &str, url: &str, headers_json: &str, body: &[u8]) -> JsValue;
}
```

- [ ] **Step 3: Add bridge module to lib.rs**

```rust
use wasm_bindgen::prelude::*;

mod bridge;

#[wasm_bindgen]
pub fn ping() -> String {
    "solobase-web".to_string()
}
```

- [ ] **Step 4: Verify compilation**

Run: `cd solobase && wasm-pack build crates/solobase-web --target web --dev`

Expected: Build succeeds. The `#[wasm_bindgen(module = "/js/bridge.js")]` declarations compile even though bridge.js exists outside `src/` — wasm-bindgen resolves this at link time.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-web/js/bridge.js crates/solobase-web/src/bridge.rs crates/solobase-web/src/lib.rs
git commit -m "feat(solobase-web): JS bridge layer for sql.js, OPFS, and fetch"
```

---

### Task 3: BrowserDatabaseService

**Files:**
- Create: `crates/solobase-web/src/database.rs`

Implements the `DatabaseService` trait from `wafer-core` by calling into sql.js via the JS bridge. sql.js executes SQL synchronously in memory; the async trait methods just wrap the sync calls. After each write operation, we flush to OPFS for persistence.

- [ ] **Step 1: Write the BrowserDatabaseService implementation**

```rust
use std::collections::HashMap;

use async_trait::async_trait;
use wafer_core::interfaces::database::service::{DatabaseError, DatabaseService};
use wafer_core::interfaces::database::types::*;

use crate::bridge;

pub struct BrowserDatabaseService;

// Safety: wasm32-unknown-unknown is single-threaded.
unsafe impl Send for BrowserDatabaseService {}
unsafe impl Sync for BrowserDatabaseService {}

impl BrowserDatabaseService {
    pub fn new() -> Self {
        Self
    }

    fn query(&self, sql: &str, params: &[serde_json::Value]) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let params_json = serde_json::to_string(params).map_err(|e| DatabaseError::Internal(e.to_string()))?;
        let result = bridge::db_query_raw(sql, &params_json);
        let rows: Vec<serde_json::Value> = serde_json::from_str(&result)
            .map_err(|e| DatabaseError::Internal(format!("failed to parse query result: {e}")))?;
        Ok(rows)
    }

    fn exec(&self, sql: &str, params: &[serde_json::Value]) -> Result<i64, DatabaseError> {
        let params_json = serde_json::to_string(params).map_err(|e| DatabaseError::Internal(e.to_string()))?;
        let result = bridge::db_exec_raw(sql, &params_json);
        let count: i64 = result.parse().unwrap_or(0);
        Ok(count)
    }

    async fn flush(&self) {
        let _ = bridge::db_flush().await;
    }

    fn row_to_record(row: serde_json::Value) -> Result<Record, DatabaseError> {
        let obj = row.as_object().ok_or_else(|| DatabaseError::Internal("row is not an object".into()))?;
        let id = obj.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mut data = HashMap::new();
        for (key, value) in obj {
            data.insert(key.clone(), value.clone());
        }
        Ok(Record { id, data })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl DatabaseService for BrowserDatabaseService {
    async fn get(&self, collection: &str, id: &str) -> Result<Record, DatabaseError> {
        let table = sanitize_ident(collection);
        let rows = self.query(
            &format!("SELECT * FROM {table} WHERE id = ?1"),
            &[serde_json::Value::String(id.to_string())],
        )?;
        let row = rows.into_iter().next().ok_or(DatabaseError::NotFound)?;
        Self::row_to_record(row)
    }

    async fn list(&self, collection: &str, opts: &ListOptions) -> Result<RecordList, DatabaseError> {
        let table = sanitize_ident(collection);
        let mut sql = format!("SELECT * FROM {table}");
        let mut params: Vec<serde_json::Value> = Vec::new();
        let mut param_idx = 1;

        // Build WHERE clause from filters
        if !opts.filters.is_empty() {
            let mut conditions = Vec::new();
            for filter in &opts.filters {
                let col = sanitize_ident(&filter.field);
                let op = match filter.op {
                    FilterOp::Eq => "=",
                    FilterOp::Ne => "!=",
                    FilterOp::Gt => ">",
                    FilterOp::Gte => ">=",
                    FilterOp::Lt => "<",
                    FilterOp::Lte => "<=",
                    FilterOp::Like => "LIKE",
                };
                conditions.push(format!("{col} {op} ?{param_idx}"));
                params.push(filter.value.clone());
                param_idx += 1;
            }
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        // ORDER BY
        if let Some(ref sort) = opts.sort {
            let col = sanitize_ident(&sort.field);
            let dir = if sort.desc { "DESC" } else { "ASC" };
            sql.push_str(&format!(" ORDER BY {col} {dir}"));
        }

        // LIMIT + OFFSET
        let limit = opts.limit.unwrap_or(100).min(1000);
        let offset = opts.offset.unwrap_or(0);
        sql.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));

        let rows = self.query(&sql, &params)?;
        let items: Vec<Record> = rows.into_iter()
            .filter_map(|r| Self::row_to_record(r).ok())
            .collect();

        // Get total count
        let count_sql = if opts.filters.is_empty() {
            format!("SELECT COUNT(*) as cnt FROM {table}")
        } else {
            // Reuse WHERE clause
            let where_clause = sql.split(" WHERE ").nth(1)
                .and_then(|s| s.split(" ORDER BY").next())
                .unwrap_or("");
            format!("SELECT COUNT(*) as cnt FROM {table} WHERE {where_clause}")
        };
        let count_rows = self.query(&count_sql, &params)?;
        let total = count_rows.first()
            .and_then(|r| r.get("cnt"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(RecordList { items, total, page: offset / limit, per_page: limit })
    }

    async fn create(&self, collection: &str, data: HashMap<String, serde_json::Value>) -> Result<Record, DatabaseError> {
        let table = sanitize_ident(collection);
        let id = data.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| generate_id());

        let mut columns = vec!["id".to_string()];
        let mut placeholders = vec!["?1".to_string()];
        let mut params: Vec<serde_json::Value> = vec![serde_json::Value::String(id.clone())];
        let mut idx = 2;

        for (key, value) in &data {
            if key == "id" { continue; }
            columns.push(sanitize_ident(key));
            placeholders.push(format!("?{idx}"));
            params.push(value.clone());
            idx += 1;
        }

        let sql = format!(
            "INSERT INTO {table} ({}) VALUES ({})",
            columns.join(", "),
            placeholders.join(", ")
        );
        self.exec(&sql, &params)?;
        self.flush().await;

        self.get(collection, &id).await
    }

    async fn update(&self, collection: &str, id: &str, data: HashMap<String, serde_json::Value>) -> Result<Record, DatabaseError> {
        let table = sanitize_ident(collection);
        let mut sets = Vec::new();
        let mut params: Vec<serde_json::Value> = Vec::new();
        let mut idx = 1;

        for (key, value) in &data {
            if key == "id" { continue; }
            sets.push(format!("{} = ?{idx}", sanitize_ident(key)));
            params.push(value.clone());
            idx += 1;
        }

        if sets.is_empty() {
            return self.get(collection, id).await;
        }

        params.push(serde_json::Value::String(id.to_string()));
        let sql = format!("UPDATE {table} SET {} WHERE id = ?{idx}", sets.join(", "));
        self.exec(&sql, &params)?;
        self.flush().await;

        self.get(collection, id).await
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<(), DatabaseError> {
        let table = sanitize_ident(collection);
        self.exec(
            &format!("DELETE FROM {table} WHERE id = ?1"),
            &[serde_json::Value::String(id.to_string())],
        )?;
        self.flush().await;
        Ok(())
    }

    async fn count(&self, collection: &str, filters: &[Filter]) -> Result<i64, DatabaseError> {
        let table = sanitize_ident(collection);
        let mut sql = format!("SELECT COUNT(*) as cnt FROM {table}");
        let mut params: Vec<serde_json::Value> = Vec::new();

        if !filters.is_empty() {
            let mut conditions = Vec::new();
            for (i, f) in filters.iter().enumerate() {
                let col = sanitize_ident(&f.field);
                let op = filter_op_sql(&f.op);
                conditions.push(format!("{col} {op} ?{}", i + 1));
                params.push(f.value.clone());
            }
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        let rows = self.query(&sql, &params)?;
        Ok(rows.first().and_then(|r| r.get("cnt")).and_then(|v| v.as_i64()).unwrap_or(0))
    }

    async fn sum(&self, collection: &str, field: &str, filters: &[Filter]) -> Result<f64, DatabaseError> {
        let table = sanitize_ident(collection);
        let col = sanitize_ident(field);
        let mut sql = format!("SELECT COALESCE(SUM({col}), 0) as total FROM {table}");
        let mut params: Vec<serde_json::Value> = Vec::new();

        if !filters.is_empty() {
            let mut conditions = Vec::new();
            for (i, f) in filters.iter().enumerate() {
                let fcol = sanitize_ident(&f.field);
                let op = filter_op_sql(&f.op);
                conditions.push(format!("{fcol} {op} ?{}", i + 1));
                params.push(f.value.clone());
            }
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        let rows = self.query(&sql, &params)?;
        Ok(rows.first().and_then(|r| r.get("total")).and_then(|v| v.as_f64()).unwrap_or(0.0))
    }

    async fn query_raw(&self, query: &str, args: &[serde_json::Value]) -> Result<Vec<Record>, DatabaseError> {
        let rows = self.query(query, args)?;
        rows.into_iter().map(Self::row_to_record).collect()
    }

    async fn exec_raw(&self, query: &str, args: &[serde_json::Value]) -> Result<i64, DatabaseError> {
        let count = self.exec(query, args)?;
        self.flush().await;
        Ok(count)
    }

    async fn ensure_schema_table(&self, table: &Table) -> Result<(), DatabaseError> {
        let name = sanitize_ident(&table.name);
        let mut col_defs = Vec::new();
        for col in &table.columns {
            let col_name = sanitize_ident(&col.name);
            let col_type = column_type_sql(&col.col_type);
            let mut def = format!("{col_name} {col_type}");
            if col.primary_key { def.push_str(" PRIMARY KEY"); }
            if !col.nullable { def.push_str(" NOT NULL"); }
            if let Some(ref default) = col.default {
                def.push_str(&format!(" DEFAULT {default}"));
            }
            if col.unique { def.push_str(" UNIQUE"); }
            col_defs.push(def);
        }
        let sql = format!("CREATE TABLE IF NOT EXISTS {name} ({})", col_defs.join(", "));
        self.exec(&sql, &[])?;
        self.flush().await;
        Ok(())
    }

    async fn schema_table_exists(&self, name: &str) -> Result<bool, DatabaseError> {
        let rows = self.query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
            &[serde_json::Value::String(name.to_string())],
        )?;
        Ok(!rows.is_empty())
    }

    async fn schema_drop_table(&self, name: &str) -> Result<(), DatabaseError> {
        let table = sanitize_ident(name);
        self.exec(&format!("DROP TABLE IF EXISTS {table}"), &[])?;
        self.flush().await;
        Ok(())
    }

    async fn schema_add_column(&self, table: &str, column: &Column) -> Result<(), DatabaseError> {
        let tbl = sanitize_ident(table);
        let col_name = sanitize_ident(&column.name);
        let col_type = column_type_sql(&column.col_type);
        let mut sql = format!("ALTER TABLE {tbl} ADD COLUMN {col_name} {col_type}");
        if let Some(ref default) = column.default {
            sql.push_str(&format!(" DEFAULT {default}"));
        }
        // ALTER TABLE ADD COLUMN may fail if column exists; that's OK
        let _ = self.exec(&sql, &[]);
        self.flush().await;
        Ok(())
    }
}

fn sanitize_ident(name: &str) -> String {
    // Only allow alphanumeric + underscore to prevent SQL injection
    let sanitized: String = name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    sanitized
}

fn filter_op_sql(op: &FilterOp) -> &'static str {
    match op {
        FilterOp::Eq => "=",
        FilterOp::Ne => "!=",
        FilterOp::Gt => ">",
        FilterOp::Gte => ">=",
        FilterOp::Lt => "<",
        FilterOp::Lte => "<=",
        FilterOp::Like => "LIKE",
    }
}

fn column_type_sql(col_type: &ColumnType) -> &'static str {
    match col_type {
        ColumnType::Text => "TEXT",
        ColumnType::Integer => "INTEGER",
        ColumnType::Real => "REAL",
        ColumnType::Blob => "BLOB",
        ColumnType::Boolean => "INTEGER",
    }
}

fn generate_id() -> String {
    // Use a simple random ID. getrandom with js feature provides crypto randomness.
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf).expect("getrandom failed");
    hex::encode(buf)
}
```

Note: Check the exact types (`Record`, `RecordList`, `ListOptions`, `Filter`, `FilterOp`, `Table`, `Column`, `ColumnType`) from `wafer_core::interfaces::database::types`. The field names and enum variants above are based on the trait signatures found in the codebase — adjust if the actual types differ. If `hex` crate is not available, use a manual hex encoding or the `uuid` crate for ID generation.

- [ ] **Step 2: Add module to lib.rs**

```rust
mod bridge;
mod database;
```

- [ ] **Step 3: Verify compilation**

Run: `cd solobase && wasm-pack build crates/solobase-web --target web --dev`

Expected: Build succeeds. If types from `wafer_core::interfaces::database` don't match, read the actual type definitions and adjust.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-web/src/database.rs crates/solobase-web/src/lib.rs
git commit -m "feat(solobase-web): BrowserDatabaseService wrapping sql.js"
```

---

### Task 4: BrowserStorageService

**Files:**
- Create: `crates/solobase-web/src/storage.rs`

Implements `StorageService` trait using OPFS via the JS bridge.

- [ ] **Step 1: Write the BrowserStorageService**

```rust
use async_trait::async_trait;
use wafer_core::interfaces::storage::service::{StorageError, StorageService};
use wafer_core::interfaces::storage::types::*;

use crate::bridge;

pub struct BrowserStorageService;

unsafe impl Send for BrowserStorageService {}
unsafe impl Sync for BrowserStorageService {}

impl BrowserStorageService {
    pub fn new() -> Self {
        Self
    }

    fn parse_js_error(val: wasm_bindgen::JsValue) -> StorageError {
        let msg = val.as_string().unwrap_or_else(|| format!("{:?}", val));
        if msg.contains("NotFoundError") || msg.contains("not found") {
            StorageError::NotFound
        } else {
            StorageError::Internal(msg)
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl StorageService for BrowserStorageService {
    async fn put(&self, folder: &str, key: &str, data: &[u8], content_type: &str) -> Result<(), StorageError> {
        bridge::storage_put(folder, key, data, content_type).await;
        Ok(())
    }

    async fn get(&self, folder: &str, key: &str) -> Result<(Vec<u8>, ObjectInfo), StorageError> {
        let result_js = bridge::storage_get(folder, key).await;
        let result_str = result_js.as_string()
            .ok_or_else(|| StorageError::Internal("storage_get returned non-string".into()))?;
        let parsed: serde_json::Value = serde_json::from_str(&result_str)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let data_arr = parsed.get("data")
            .and_then(|v| v.as_array())
            .ok_or(StorageError::NotFound)?;
        let data: Vec<u8> = data_arr.iter()
            .filter_map(|v| v.as_u64().map(|n| n as u8))
            .collect();

        let meta = parsed.get("meta").cloned().unwrap_or_default();
        let content_type = meta.get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or("application/octet-stream")
            .to_string();
        let size = meta.get("size")
            .and_then(|v| v.as_u64())
            .unwrap_or(data.len() as u64);

        let info = ObjectInfo {
            key: key.to_string(),
            content_type,
            size,
        };

        Ok((data, info))
    }

    async fn delete(&self, folder: &str, key: &str) -> Result<(), StorageError> {
        bridge::storage_delete(folder, key).await;
        Ok(())
    }

    async fn list(&self, folder: &str, opts: &ListOptions) -> Result<ObjectList, StorageError> {
        let prefix = opts.prefix.as_deref().unwrap_or("");
        let limit = opts.limit.unwrap_or(100) as u32;
        let offset = opts.offset.unwrap_or(0) as u32;

        let result_js = bridge::storage_list(folder, prefix, limit, offset).await;
        let result_str = result_js.as_string()
            .ok_or_else(|| StorageError::Internal("storage_list returned non-string".into()))?;
        let entries: Vec<serde_json::Value> = serde_json::from_str(&result_str)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let items: Vec<ObjectInfo> = entries.iter().filter_map(|e| {
            Some(ObjectInfo {
                key: e.get("key")?.as_str()?.to_string(),
                content_type: e.get("content_type").and_then(|v| v.as_str()).unwrap_or("application/octet-stream").to_string(),
                size: e.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
            })
        }).collect();

        Ok(ObjectList { items, total: items.len() as i64 })
    }

    async fn create_folder(&self, name: &str, _public: bool) -> Result<(), StorageError> {
        bridge::storage_create_folder(name).await;
        Ok(())
    }

    async fn delete_folder(&self, name: &str) -> Result<(), StorageError> {
        bridge::storage_delete_folder(name).await;
        Ok(())
    }

    async fn list_folders(&self) -> Result<Vec<FolderInfo>, StorageError> {
        let result_js = bridge::storage_list_folders().await;
        let result_str = result_js.as_string()
            .ok_or_else(|| StorageError::Internal("storage_list_folders returned non-string".into()))?;
        let names: Vec<String> = serde_json::from_str(&result_str)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(names.into_iter().map(|name| FolderInfo { name, public: false }).collect())
    }
}
```

Note: Check exact types (`ObjectInfo`, `ObjectList`, `FolderInfo`, `ListOptions`) from `wafer_core::interfaces::storage::types`. Adjust field names if they differ.

- [ ] **Step 2: Add module to lib.rs**

- [ ] **Step 3: Verify compilation**

Run: `cd solobase && wasm-pack build crates/solobase-web --target web --dev`

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-web/src/storage.rs crates/solobase-web/src/lib.rs
git commit -m "feat(solobase-web): BrowserStorageService wrapping OPFS"
```

---

### Task 5: BrowserNetworkService + ConsoleLogger

**Files:**
- Create: `crates/solobase-web/src/network.rs`
- Create: `crates/solobase-web/src/logger.rs`

- [ ] **Step 1: Write BrowserNetworkService**

```rust
use std::collections::HashMap;

use async_trait::async_trait;
use wafer_core::interfaces::network::service::{
    NetworkError, NetworkService, Request, Response,
};

use crate::bridge;

pub struct BrowserNetworkService;

unsafe impl Send for BrowserNetworkService {}
unsafe impl Sync for BrowserNetworkService {}

impl BrowserNetworkService {
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl NetworkService for BrowserNetworkService {
    async fn do_request(&self, req: &Request) -> Result<Response, NetworkError> {
        let headers_json = serde_json::to_string(&req.headers)
            .map_err(|e| NetworkError::RequestError(e.to_string()))?;
        let body = req.body.as_deref().unwrap_or(&[]);

        let result_js = bridge::http_fetch(&req.method, &req.url, &headers_json, body).await;
        let result_str = result_js.as_string()
            .ok_or_else(|| NetworkError::RequestError("http_fetch returned non-string".into()))?;

        #[derive(serde::Deserialize)]
        struct FetchResult {
            status: u16,
            headers: HashMap<String, Vec<String>>,
            body: Vec<u8>,
        }

        let parsed: FetchResult = serde_json::from_str(&result_str)
            .map_err(|e| NetworkError::RequestError(e.to_string()))?;

        Ok(Response {
            status_code: parsed.status,
            headers: parsed.headers,
            body: parsed.body,
        })
    }
}
```

- [ ] **Step 2: Write ConsoleLogger**

Check the `LoggerService` trait in `wafer-core` (likely in `wafer_core::interfaces::logger::service`). Implement it using `web_sys::console`:

```rust
use wafer_core::interfaces::logger::service::LoggerService;

pub struct ConsoleLogger;

unsafe impl Send for ConsoleLogger {}
unsafe impl Sync for ConsoleLogger {}

impl LoggerService for ConsoleLogger {
    fn log(&self, level: &str, message: &str) {
        match level {
            "error" => web_sys::console::error_1(&message.into()),
            "warn" => web_sys::console::warn_1(&message.into()),
            "debug" => web_sys::console::debug_1(&message.into()),
            _ => web_sys::console::log_1(&message.into()),
        }
    }
}
```

Note: The exact `LoggerService` trait may have different methods (e.g., `debug()`, `info()`, `warn()`, `error()` separately, or a single `log()` with a level parameter). Read `wafer-block-logger` crate to see what the trait looks like and implement accordingly.

- [ ] **Step 3: Add modules to lib.rs**

- [ ] **Step 4: Verify compilation + commit**

```bash
git add crates/solobase-web/src/network.rs crates/solobase-web/src/logger.rs crates/solobase-web/src/lib.rs
git commit -m "feat(solobase-web): BrowserNetworkService + ConsoleLogger"
```

---

### Task 6: Request/Response Conversion

**Files:**
- Create: `crates/solobase-web/src/convert.rs`

Browser equivalent of `http_to_message()` and `wafer_result_to_response()` from `wafer-block-http-listener`. These convert between `web_sys::Request`/`web_sys::Response` and WAFER `Message`/`Result_`.

- [ ] **Step 1: Write conversion functions**

```rust
use std::collections::HashMap;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use wafer_block::core_types::{Action, Message, MetaEntry, Result_};
use wafer_block::meta::*; // META_REQ_ACTION, META_REQ_RESOURCE, etc.

/// Convert a browser Request into a WAFER Message.
///
/// Mirrors the logic in wafer-block-http-listener::http_to_message()
/// but takes web_sys types instead of hyper types.
pub async fn request_to_message(request: &web_sys::Request) -> Result<Message, JsValue> {
    let method = request.method();
    let url = web_sys::Url::new(&request.url())?;
    let path = url.pathname();
    let raw_query = url.search();
    let raw_query = raw_query.strip_prefix('?').unwrap_or(&raw_query);

    // Read body
    let body = if method == "GET" || method == "HEAD" {
        Vec::new()
    } else {
        let array_buffer = JsFuture::from(request.array_buffer()?).await?;
        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
        uint8_array.to_vec()
    };

    let mut msg = Message::new(format!("{method}:{path}"), body);

    // HTTP-specific meta
    msg.set_meta("http.method", &method);
    msg.set_meta("http.path", &path);
    msg.set_meta("http.raw_query", raw_query);
    msg.set_meta("http.remote_addr", "127.0.0.1"); // browser is always local

    // Normalized request meta
    let action = http_method_to_action(&method);
    msg.set_meta(META_REQ_ACTION, &action);
    msg.set_meta(META_REQ_RESOURCE, &path);
    msg.set_meta(META_REQ_CLIENT_IP, "127.0.0.1");

    // Copy headers
    let headers = request.headers();
    // web_sys::Headers doesn't have a direct iterator; use js_sys to iterate
    let entries = js_sys::try_iter(&headers.entries()?)?.ok_or_else(|| JsValue::from_str("headers not iterable"))?;
    for entry in entries {
        let entry = entry?;
        let pair = js_sys::Array::from(&entry);
        let key = pair.get(0).as_string().unwrap_or_default();
        let value = pair.get(1).as_string().unwrap_or_default();
        msg.set_meta(format!("http.header.{key}"), &value);

        if key == "content-type" {
            msg.set_meta("http.content_type", &value);
            msg.set_meta(META_REQ_CONTENT_TYPE, &value);
        }
        if key == "host" {
            msg.set_meta("http.host", &value);
        }
    }

    // Copy query params
    if !raw_query.is_empty() {
        for pair in raw_query.split('&') {
            let mut parts = pair.splitn(2, '=');
            if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                let decoded = urlencoding::decode(val).unwrap_or_else(|_| val.into());
                msg.set_meta(format!("http.query.{key}"), &decoded);
                msg.set_meta(format!("{META_REQ_QUERY_PREFIX}{key}"), &decoded);
            }
        }
    }

    Ok(msg)
}

/// Convert a WAFER Result_ into a browser Response.
///
/// Mirrors wafer-block-http-listener::wafer_result_to_response().
pub fn result_to_response(result: Result_) -> Result<web_sys::Response, JsValue> {
    match result.action {
        Action::Respond => {
            let resp_meta = result.response.as_ref()
                .map(|r| r.meta.as_slice())
                .unwrap_or(&[]);
            let status = get_status_code(resp_meta, 200);
            let body = result.response.map(|r| r.data).unwrap_or_default();

            let init = web_sys::ResponseInit::new();
            init.set_status(status);

            let headers = web_sys::Headers::new()?;
            apply_meta_to_headers(resp_meta, &headers)?;
            if let Some(ref msg) = result.message {
                apply_meta_to_headers(&msg.meta, &headers)?;
            }
            // Default content-type
            if headers.get("Content-Type")?.is_none() {
                headers.set("Content-Type", "application/json")?;
            }
            init.set_headers(&headers.into());

            let body_js = js_sys::Uint8Array::from(body.as_slice());
            web_sys::Response::new_with_opt_buffer_source_and_init(
                Some(&body_js.into()),
                &init,
            )
        }

        Action::Error => {
            let err_meta = result.error.as_ref()
                .map(|e| e.meta.as_slice())
                .unwrap_or(&[]);
            let status = get_error_status_code(result.error.as_ref(), err_meta);

            let body = if let Some(ref err) = result.error {
                serde_json::json!({ "error": err.code, "message": err.message }).to_string()
            } else {
                "{}".to_string()
            };

            let init = web_sys::ResponseInit::new();
            init.set_status(status);
            let headers = web_sys::Headers::new()?;
            headers.set("Content-Type", "application/json")?;
            apply_meta_to_headers(err_meta, &headers)?;
            init.set_headers(&headers.into());

            web_sys::Response::new_with_opt_str_and_init(Some(&body), &init)
        }

        Action::Drop => {
            let init = web_sys::ResponseInit::new();
            init.set_status(204);
            web_sys::Response::new_with_opt_str_and_init(None, &init)
        }

        Action::Continue => {
            let body = result.message.map(|m| m.data).unwrap_or_default();
            let init = web_sys::ResponseInit::new();
            init.set_status(200);
            let headers = web_sys::Headers::new()?;
            headers.set("Content-Type", "application/json")?;
            init.set_headers(&headers.into());

            let body_js = js_sys::Uint8Array::from(body.as_slice());
            web_sys::Response::new_with_opt_buffer_source_and_init(
                Some(&body_js.into()),
                &init,
            )
        }
    }
}

fn http_method_to_action(method: &str) -> String {
    match method {
        "GET" => "retrieve".to_string(),
        "POST" => "create".to_string(),
        "PUT" | "PATCH" => "update".to_string(),
        "DELETE" => "delete".to_string(),
        _ => method.to_lowercase(),
    }
}

fn get_status_code(meta: &[MetaEntry], default: u16) -> u16 {
    for entry in meta {
        if entry.key == META_RESP_STATUS || entry.key == "http.status" {
            if let Ok(code) = entry.value.parse::<u16>() {
                return code;
            }
        }
    }
    default
}

fn get_error_status_code(error: Option<&wafer_block::core_types::WaferError>, meta: &[MetaEntry]) -> u16 {
    // Check meta first
    let from_meta = get_status_code(meta, 0);
    if from_meta > 0 { return from_meta; }

    // Map error codes to HTTP status
    if let Some(err) = error {
        return match err.code.as_str() {
            "not_found" => 404,
            "unauthorized" | "auth_required" => 401,
            "forbidden" => 403,
            "bad_request" | "validation" => 400,
            "conflict" => 409,
            "rate_limited" => 429,
            _ => 500,
        };
    }
    500
}

fn apply_meta_to_headers(meta: &[MetaEntry], headers: &web_sys::Headers) -> Result<(), JsValue> {
    for entry in meta {
        // Map WAFER meta keys to HTTP headers
        if entry.key == META_RESP_CONTENT_TYPE || entry.key == "Content-Type" {
            headers.set("Content-Type", &entry.value)?;
        } else if entry.key.starts_with("http.resp.header.") {
            let header_name = entry.key.strip_prefix("http.resp.header.").unwrap();
            headers.set(header_name, &entry.value)?;
        } else if entry.key == "Set-Cookie" || entry.key.starts_with("http.resp.set-cookie") {
            headers.append("Set-Cookie", &entry.value)?;
        }
    }
    Ok(())
}
```

Note: The exact `META_*` constants are in `wafer_block::meta` or similar module. Check the actual constant names (e.g., `META_RESP_STATUS`, `META_RESP_CONTENT_TYPE`, `META_REQ_ACTION`, etc.) from the `wafer-block` crate and adjust imports. Also add `web_sys::Url` to the features list in Cargo.toml: `"Url"`.

Add `urlencoding = "2"` to Cargo.toml dependencies.

- [ ] **Step 2: Update Cargo.toml web-sys features**

Add to the web-sys features list:

```toml
web-sys = { version = "0.3", features = [
    "Request", "Response", "ResponseInit",
    "Headers", "ReadableStream",
    "Url",
    "console",
] }
```

Add:

```toml
urlencoding = "2"
```

- [ ] **Step 3: Add module to lib.rs, verify compilation**

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-web/src/convert.rs crates/solobase-web/Cargo.toml crates/solobase-web/src/lib.rs
git commit -m "feat(solobase-web): Request/Response conversion for Service Worker"
```

---

### Task 7: Runtime Initialization + Fetch Handler

**Files:**
- Modify: `crates/solobase-web/src/lib.rs`
- Create: `crates/solobase-web/src/config.rs`

This is the core task — wire up the WAFER runtime with browser service implementations and expose `initialize()` + `handle_request()` to JavaScript.

- [ ] **Step 1: Create config.rs — load variables from sql.js**

```rust
//! Browser config loading — equivalent of app_config::seed_and_load_variables()
//! but reads from sql.js instead of rusqlite.

use std::collections::HashMap;
use crate::bridge;

/// Create the variables table if it doesn't exist, then load all variables.
pub fn seed_and_load_variables() -> HashMap<String, String> {
    // Create table if needed
    let _ = bridge::db_exec_raw(
        "CREATE TABLE IF NOT EXISTS variables (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL DEFAULT '',
            sensitive INTEGER NOT NULL DEFAULT 0,
            source TEXT NOT NULL DEFAULT 'default'
        )",
        "[]",
    );

    // Generate JWT secret if not exists
    let rows = bridge::db_query_raw(
        "SELECT value FROM variables WHERE key = 'SUPPERS_AI__AUTH__JWT_SECRET'",
        "[]",
    );
    let existing: Vec<serde_json::Value> = serde_json::from_str(&rows).unwrap_or_default();
    if existing.is_empty() {
        let mut buf = [0u8; 32];
        getrandom::getrandom(&mut buf).expect("getrandom");
        let secret = hex::encode(buf);
        let params = serde_json::to_string(&[&secret]).unwrap();
        bridge::db_exec_raw(
            "INSERT INTO variables (key, value, sensitive, source) VALUES ('SUPPERS_AI__AUTH__JWT_SECRET', ?1, 1, 'generated')",
            &params,
        );
    }

    // Load all variables
    let rows = bridge::db_query_raw("SELECT key, value FROM variables", "[]");
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&rows).unwrap_or_default();
    let mut vars = HashMap::new();
    for row in parsed {
        if let (Some(key), Some(value)) = (
            row.get("key").and_then(|v| v.as_str()),
            row.get("value").and_then(|v| v.as_str()),
        ) {
            vars.insert(key.to_string(), value.to_string());
        }
    }
    vars
}

/// Load block settings (enabled/disabled state).
pub fn load_block_settings() -> solobase_core::BlockSettings {
    let rows = bridge::db_query_raw(
        "SELECT key, value FROM suppers_ai__admin__block_settings",
        "[]",
    );
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&rows).unwrap_or_default();
    let mut settings = HashMap::new();
    for row in parsed {
        if let (Some(key), Some(value)) = (
            row.get("key").and_then(|v| v.as_str()),
            row.get("value").and_then(|v| v.as_str()),
        ) {
            settings.insert(key.to_string(), value.to_string());
        }
    }
    solobase_core::BlockSettings::from(settings)
}
```

Note: Check how `BlockSettings` / `FeatureConfig` is actually constructed in `solobase::app_config::load_block_settings()`. The table may not exist on first run — handle gracefully (empty settings = all blocks enabled). Also check if the `hex` crate needs to be added to Cargo.toml.

- [ ] **Step 2: Write lib.rs — full runtime init + fetch handler**

```rust
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use wafer_core::interfaces::config::service::ConfigService;
use wafer_run::Wafer;

mod bridge;
mod config;
mod convert;
mod database;
mod logger;
mod network;
mod storage;

// Store the started runtime in a thread-local (wasm32 is single-threaded).
thread_local! {
    static RUNTIME: RefCell<Option<RuntimeState>> = RefCell::new(None);
}

struct RuntimeState {
    wafer: wafer_run::Wafer, // or whatever start() returns
    // We may need to store flow/block references for dispatch
}

/// Initialize the WAFER runtime. Called once when the Service Worker starts.
#[wasm_bindgen]
pub async fn initialize() -> Result<(), JsValue> {
    // 1. Initialize sql.js from OPFS
    bridge::db_init().await;

    // 2. Seed variables and load config
    let vars = config::seed_and_load_variables();
    let jwt_secret = vars.get("SUPPERS_AI__AUTH__JWT_SECRET")
        .cloned()
        .unwrap_or_default();

    // 3. Create WAFER runtime
    let mut wafer = Wafer::new();
    wafer.set_admin_block("suppers-ai/admin");

    // 4. Register service blocks with browser implementations
    let db_service = Arc::new(database::BrowserDatabaseService::new());
    wafer_core::service_blocks::database::register_with(&mut wafer, db_service)
        .map_err(|e| JsValue::from_str(&e))?;
    wafer.add_alias("db", "wafer-run/database");

    let storage_service = Arc::new(storage::BrowserStorageService::new());
    // Note: solobase wraps storage with WRAP access control via blocks::storage::create().
    // Check if we can reuse that wrapper here. If it compiles for wasm32, use it.
    // Otherwise, register the raw storage service:
    wafer_core::service_blocks::storage::register_with(&mut wafer, storage_service)
        .map_err(|e| JsValue::from_str(&e))?;
    wafer.add_alias("storage", "wafer-run/storage");

    let config_service = wafer_block_config::service::EnvConfigService::new();
    for (key, value) in &vars {
        config_service.set(key, value);
    }
    wafer_core::service_blocks::config::register_with(&mut wafer, Arc::new(config_service))
        .map_err(|e| JsValue::from_str(&e))?;

    let crypto_service = Arc::new(
        wafer_block_crypto::service::Argon2JwtCryptoService::new(jwt_secret.clone())
    );
    wafer_core::service_blocks::crypto::register_with(&mut wafer, crypto_service)
        .map_err(|e| JsValue::from_str(&e))?;

    let network_service = Arc::new(network::BrowserNetworkService::new());
    // If solobase's network wrapper (blocks::network::create) compiles for wasm32, use it.
    // Otherwise register directly:
    wafer_core::service_blocks::network::register_with(&mut wafer, network_service)
        .map_err(|e| JsValue::from_str(&e))?;

    let logger_service = Arc::new(logger::ConsoleLogger);
    wafer_core::service_blocks::logger::register_with(&mut wafer, logger_service)
        .map_err(|e| JsValue::from_str(&e))?;

    // 5. Register middleware blocks
    wafer_block_auth_validator::register(&mut wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer_block_cors::register(&mut wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer_block_iam_guard::register(&mut wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer_block_inspector::register(&mut wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer.add_block_config("wafer-run/inspector", serde_json::json!({ "allow_anonymous": false }));
    wafer_block_readonly_guard::register(&mut wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer_block_router::register(&mut wafer).map_err(|e| JsValue::from_str(&e))?;
    wafer_block_security_headers::register(&mut wafer).map_err(|e| JsValue::from_str(&e))?;
    // Note: wafer_block_web (filesystem serving) is NOT registered — no filesystem in browser.
    // Note: wafer_block_http_listener is NOT registered — we handle fetch events directly.

    // 6. Register solobase feature blocks
    let features = config::load_block_settings();
    let shared_blocks = solobase_core::blocks::create_blocks(|name| features.is_enabled(name));
    solobase_core::blocks::register_shared_blocks(&mut wafer, &shared_blocks);

    // Email block
    wafer.register_block(
        "suppers-ai/email",
        Arc::new(solobase_core::blocks::email::EmailBlock),
    ).map_err(|e| JsValue::from_str(&e))?;

    // 7. Register solobase router
    // Note: NativeBlockFactory and SolobaseRouterBlock may need to be imported
    // from solobase crate (lib.rs). If they're not accessible from solobase-core,
    // check if solobase's lib.rs re-exports them. The solobase crate exports
    // `pub use solobase_core::blocks` and `pub mod flows`.
    // The router/factory types are in solobase (not solobase-core).
    // We may need to either:
    //   a) Move NativeBlockFactory + SolobaseRouterBlock to solobase-core, or
    //   b) Depend on the solobase crate (without server feature), or
    //   c) Duplicate the router setup
    // Investigate during implementation. For now, assume approach (b):
    let feature_config: Arc<dyn solobase_core::FeatureConfig> = Arc::new(features);
    // ... router setup here — match main.rs pattern

    // 8. Register flows
    // Route config — same as native but without the web block fallback
    let routes = serde_json::json!([
        { "path": "/b/**",                   "block": "suppers-ai/router" },
        { "path": "/health",                 "block": "suppers-ai/router" },
        { "path": "/openapi.json",           "block": "suppers-ai/router" },
        { "path": "/.well-known/agent.json", "block": "suppers-ai/router" },
    ]);
    wafer.add_block_config("wafer-run/router", serde_json::json!({ "routes": routes }));

    // Register the site-main flow
    // The flow JSON is in solobase::flows::site_main::JSON. If accessible from
    // solobase-core, use it. Otherwise, inline the flow JSON here.
    let flow_json = r#"{
        "id": "site-main",
        "name": "Site Main",
        "version": "0.1.0",
        "description": "Top-level HTTP dispatch — security + router",
        "steps": [
            { "id": "security-headers", "block": "wafer-run/security-headers" },
            { "id": "cors", "block": "wafer-run/cors" },
            { "id": "readonly-guard", "block": "wafer-run/readonly-guard" },
            { "id": "router", "block": "wafer-run/router" }
        ],
        "config": { "on_error": "stop" }
    }"#;
    wafer.add_flow_json(flow_json).map_err(|e| JsValue::from_str(&e))?;

    // 9. Start runtime (initializes all blocks, runs lifecycle Init)
    let wafer = wafer.start().await
        .map_err(|e| JsValue::from_str(&format!("WAFER start failed: {e}")))?;

    // 10. Store runtime
    RUNTIME.with(|r| {
        *r.borrow_mut() = Some(RuntimeState { wafer });
    });

    web_sys::console::log_1(&"solobase-web initialized".into());
    Ok(())
}

/// Handle a fetch request from the Service Worker.
/// Converts the Request to a WAFER Message, dispatches through the site-main
/// flow, and returns the Response.
#[wasm_bindgen]
pub async fn handle_request(request: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    // Convert browser Request to WAFER Message
    let mut msg = convert::request_to_message(&request).await?;

    // Dispatch through the site-main flow
    // The exact dispatch API depends on what Wafer::start() returns.
    // Options:
    //   a) wafer.run_flow("site-main", &mut msg)
    //   b) wafer.run_block("suppers-ai/router", &mut msg)
    //   c) Create a RuntimeContext and use the flow executor
    //
    // Check the Wafer API at runtime.rs and runtime/runner.rs for the exact method.
    // The http-listener uses RuntimeHandle::run() — find the equivalent for direct access.
    let result = RUNTIME.with(|r| {
        let rt = r.borrow();
        let runtime = rt.as_ref().expect("runtime not initialized");
        // TODO: Call the actual dispatch method.
        // This might be: runtime.wafer.run_flow("site-main", &mut msg).await
        // or: runtime.wafer.dispatch(&mut msg).await
        // Investigate the Wafer started-state API.
        unimplemented!("dispatch through WAFER runtime")
    });

    // Convert WAFER Result to browser Response
    convert::result_to_response(result)
}
```

**Important implementation note:** The `handle_request` function needs access to the WAFER runtime's dispatch method. The exact API depends on what `Wafer::start()` returns. During implementation:

1. Read `wafer-run/src/runtime.rs` to find what `start()` returns
2. Read `wafer-run/src/runtime/runner.rs` for the `run_flow()` / `run_block()` API
3. Check if the RuntimeHandle (used by http-listener) can be obtained directly
4. If the API requires `&self` (not `&mut self`) for dispatch, `RefCell` works. If it requires `&mut self`, use a different pattern.

The `unimplemented!()` above MUST be replaced with the actual dispatch call.

- [ ] **Step 2: Verify compilation**

Run: `cd solobase && wasm-pack build crates/solobase-web --target web --dev`

Compilation may reveal issues with:
- Missing imports (solobase vs solobase-core boundaries)
- The router block factory (may need to be moved to solobase-core)
- Feature flags on solobase-core

Fix each issue as encountered. Document any code moves needed.

- [ ] **Step 3: Commit**

```bash
git add crates/solobase-web/src/
git commit -m "feat(solobase-web): runtime init + fetch handler entry point"
```

---

### Task 8: Service Worker JavaScript Files

**Files:**
- Create: `crates/solobase-web/js/sw.js`
- Create: `crates/solobase-web/js/loader.js`
- Create: `crates/solobase-web/js/index.html`

- [ ] **Step 1: Create sw.js — the Service Worker**

```javascript
// sw.js — Service Worker that runs Solobase via WASM
//
// Intercepts all fetch events and routes them through the WASM
// Solobase runtime. The Service Worker IS the server.

import init, { initialize, handle_request } from './solobase_web.js';

let initialized = false;
let initPromise = null;

async function ensureInitialized() {
    if (initialized) return;
    if (initPromise) return await initPromise;

    initPromise = (async () => {
        console.log('[solobase-web] Loading WASM module...');
        await init();
        console.log('[solobase-web] Initializing runtime...');
        await initialize();
        initialized = true;
        console.log('[solobase-web] Runtime ready.');
    })();

    await initPromise;
}

self.addEventListener('install', (event) => {
    console.log('[solobase-web] Service Worker installing...');
    event.waitUntil(self.skipWaiting());
});

self.addEventListener('activate', (event) => {
    console.log('[solobase-web] Service Worker activating...');
    event.waitUntil(self.clients.claim());
});

self.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);

    // Only intercept same-origin requests
    if (url.origin !== self.location.origin) return;

    // Don't intercept requests for the SW itself or static assets
    if (url.pathname === '/sw.js' ||
        url.pathname === '/loader.js' ||
        url.pathname === '/index.html' ||
        url.pathname === '/' ||
        url.pathname.startsWith('/pkg/') ||
        url.pathname.startsWith('/sql-')) {
        return;
    }

    event.respondWith(handleFetch(event.request));
});

async function handleFetch(request) {
    try {
        await ensureInitialized();
        return await handle_request(request);
    } catch (error) {
        console.error('[solobase-web] Error handling request:', error);
        return new Response(
            JSON.stringify({ error: 'internal_error', message: String(error) }),
            { status: 500, headers: { 'Content-Type': 'application/json' } }
        );
    }
}
```

- [ ] **Step 2: Create loader.js — main page bootstrap**

```javascript
// loader.js — Registers the Service Worker and bootstraps the app

async function boot() {
    const status = document.getElementById('status');

    if (!('serviceWorker' in navigator)) {
        status.textContent = 'Service Workers not supported in this browser.';
        return;
    }

    try {
        status.textContent = 'Registering Service Worker...';
        const registration = await navigator.serviceWorker.register('/sw.js', {
            type: 'module',
            scope: '/',
        });

        // Wait for the SW to be active
        const sw = registration.installing || registration.waiting || registration.active;
        if (sw && sw.state !== 'activated') {
            await new Promise((resolve) => {
                sw.addEventListener('statechange', () => {
                    if (sw.state === 'activated') resolve();
                });
                // If already activated by the time we listen
                if (sw.state === 'activated') resolve();
            });
        }

        // Ensure the SW controls this page
        if (!navigator.serviceWorker.controller) {
            // First install — need to reload so SW intercepts fetches
            status.textContent = 'First-time setup complete. Loading Solobase...';
            window.location.reload();
            return;
        }

        // SW is active and controlling — navigate to dashboard
        status.textContent = 'Loading Solobase...';
        window.location.href = '/b/system/';
    } catch (error) {
        status.textContent = `Error: ${error.message}`;
        console.error('[solobase-web] Boot error:', error);
    }
}

boot();
```

- [ ] **Step 3: Create index.html — shell page**

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Solobase</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            background: #f5f5f5;
            color: #333;
        }
        .loader {
            text-align: center;
        }
        .loader h1 {
            font-size: 1.5rem;
            margin-bottom: 1rem;
        }
        #status {
            color: #666;
            font-size: 0.9rem;
        }
    </style>
</head>
<body>
    <div class="loader">
        <h1>Solobase</h1>
        <p id="status">Loading...</p>
    </div>
    <script src="/loader.js"></script>
</body>
</html>
```

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-web/js/
git commit -m "feat(solobase-web): Service Worker + loader + index.html"
```

---

### Task 9: Build Pipeline + End-to-End Test

**Files:**
- Create: `crates/solobase-web/Makefile` (or add targets to root Makefile)

- [ ] **Step 1: Create build script**

Create `crates/solobase-web/Makefile`:

```makefile
.PHONY: build dev clean serve

# Build for production
build:
	wasm-pack build --target web --release --out-dir pkg
	cp js/sw.js pkg/
	cp js/loader.js pkg/
	cp js/index.html pkg/
	cp js/bridge.js pkg/
	# Download sql.js WASM if not present
	@if [ ! -f pkg/sql-wasm.wasm ]; then \
		echo "Downloading sql.js WASM..."; \
		curl -sL https://sql.js.org/dist/sql-wasm.wasm -o pkg/sql-wasm.wasm; \
	fi

# Build for development
dev:
	wasm-pack build --target web --dev --out-dir pkg
	cp js/sw.js pkg/
	cp js/loader.js pkg/
	cp js/index.html pkg/
	cp js/bridge.js pkg/
	@if [ ! -f pkg/sql-wasm.wasm ]; then \
		echo "Downloading sql.js WASM..."; \
		curl -sL https://sql.js.org/dist/sql-wasm.wasm -o pkg/sql-wasm.wasm; \
	fi

# Serve locally for testing
serve: dev
	@echo "Serving at http://localhost:8080"
	@echo "Note: Service Workers require HTTPS in production, but localhost is exempt."
	python3 -m http.server 8080 -d pkg

clean:
	rm -rf pkg target
```

- [ ] **Step 2: Build the project**

Run: `cd solobase/crates/solobase-web && make dev`

Expected: Build succeeds. Fix any remaining compilation errors.

- [ ] **Step 3: Serve and test in browser**

Run: `cd solobase/crates/solobase-web && make serve`

Open `http://localhost:8080` in Chrome/Edge. Expected behavior:

1. `index.html` loads, shows "Loading..."
2. `loader.js` registers the Service Worker
3. SW loads WASM module, initializes sql.js + WAFER runtime
4. Page redirects to `/b/system/` (dashboard)
5. Dashboard renders via maud HTML served by the SW

**Debug in DevTools:**
- Application tab → Service Workers: should show `sw.js` as active
- Console: should show `[solobase-web] Runtime ready.`
- Network tab: subsequent requests should show "from ServiceWorker"

If the dashboard doesn't render, check:
- Console for WASM errors
- Network tab for failed requests
- Whether the flow dispatch is working (the `unimplemented!()` in Task 7)

- [ ] **Step 4: Test core features**

Navigate through the app and verify:
- `/b/auth/login` — login page renders
- Create an account — database write works (sql.js)
- Login — JWT auth works
- `/b/admin/` — admin panel renders with block list
- Upload a file — OPFS storage works

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-web/Makefile
git commit -m "feat(solobase-web): build pipeline + e2e verified"
```

---

### Task 10: Site Updates + Demo Replacement

**Files:**
- Modify: `packages/solobase-site/src/pages/home.jsx` (or equivalent)
- Modify: `packages/solobase-site/src/data/navigation.js`
- Modify: `packages/solobase-site/public/_redirects`

This task updates the solobase marketing site to include the browser version and replaces the Fly.io demo.

- [ ] **Step 1: Add "Browser (No Install)" to the Get Started section**

Read `packages/solobase-site/src/pages/home.jsx` and find the platform download grid. Add a new entry:

```jsx
{
    platform: "Browser",
    description: "No download. No setup. Runs entirely in your browser.",
    action: "Try Now",
    url: "https://demo.solobase.dev",
    icon: "globe", // or appropriate icon
    badge: "No Install",
}
```

Place it as the first option in the grid (most prominent position).

- [ ] **Step 2: Update Demo link in navigation**

In `packages/solobase-site/src/data/navigation.js`, update the Demo link to point to the hosted browser version:

```javascript
{ name: 'Demo', href: 'https://demo.solobase.dev' },
```

- [ ] **Step 3: Add constraints note**

Below the "Browser" download option, add a small expandable section:

```
Data is local to your browser (no sync between devices).
Storage limited by browser quotas. No background processing.
```

- [ ] **Step 4: Deploy solobase-web to demo.solobase.dev**

Deploy the `pkg/` output from `make build` to Cloudflare Pages (or similar static host) at `demo.solobase.dev`:

```bash
# If using Cloudflare Pages:
cd crates/solobase-web && make build
npx wrangler pages deploy pkg --project-name solobase-demo
```

Or configure the deployment in the project's CI.

- [ ] **Step 5: Retire Fly.io demo**

Once `demo.solobase.dev` is serving the browser version:
- Stop the Fly.io app: `fly apps destroy solobase-demo` (confirm with user first)
- The `deploy/demo/` directory (Dockerfile, fly.toml) can be kept for reference or removed

- [ ] **Step 6: Commit site changes**

```bash
git add packages/solobase-site/
git commit -m "feat(site): add Browser option to Get Started, replace demo with WASM version"
```

---

## Notes for Implementation

### Dependency Resolution Order

If blocks or middleware don't compile for wasm32, investigate in this order:
1. Check if the dependency uses OS-specific features behind cfg gates
2. Check if `default-features = false` removes the problematic dependency
3. If a crate is fundamentally native-only, skip registering that block on wasm32

### Router Block Factory

The `NativeBlockFactory` and `SolobaseRouterBlock` are in the `solobase` crate (not `solobase-core`). For `solobase-web` to use them:
- **Option A (recommended):** Move `NativeBlockFactory` and `SolobaseRouterBlock` to `solobase-core` so both `solobase` and `solobase-web` can use them
- **Option B:** Add `solobase` as a dependency of `solobase-web` with `default-features = false` (no `server` feature)
- **Option C:** Create a `BrowserBlockFactory` in `solobase-web` that mirrors `NativeBlockFactory`

### WASM Size Optimization

The release build uses the workspace profile: `opt-level = "z"`, LTO, `codegen-units = 1`, strip. This should produce a reasonably sized WASM binary. If the binary exceeds 10MB gzipped:
1. Check if `wasm-opt` is installed (wasm-pack uses it automatically for release builds)
2. Consider splitting: load blocks lazily if possible
3. Profile with `twiggy` to find large contributors

### sql.js Version

Pin sql.js to a specific version in bridge.js. The `sql-wasm.wasm` binary must match the JS version. Use the CDN or bundle the specific version.
