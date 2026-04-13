# Unified Message + Context System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign `suppers-ai/messages` from thread/message CRUD into a protocol-agnostic context + entry system with A2A JSON-RPC support.

**Architecture:** Two tables (`contexts`, `entries`), a shared service layer for business logic, REST and A2A handlers as thin API surfaces on top. The LLM block is updated to use new endpoints.

**Tech Stack:** Rust, wafer-run/wafer-core framework, SQLite via database service, maud for SSR, serde_json for A2A JSON-RPC.

**Spec:** `solobase/docs/superpowers/specs/2026-04-13-unified-messages-context-design.md`

---

## File Structure

```
solobase/crates/solobase-core/src/blocks/messages/
├── mod.rs          # Block struct, BlockInfo, collection schemas, endpoint declarations, routing
├── service.rs      # Core service functions (create_context, add_entry, etc.)
├── rest.rs         # REST endpoint handlers
├── a2a.rs          # A2A JSON-RPC handler
└── pages.rs        # SSR admin pages (maud templates)
```

**Also modified:**
- `solobase/crates/solobase-core/src/blocks/llm/mod.rs` — update inter-block call helpers
- `solobase/crates/solobase-core/src/blocks/llm/pages.rs` — update collection constants + queries
- `solobase/crates/solobase-core/src/pipeline.rs` — add `/a2a` route
- `solobase/crates/solobase-core/src/routing.rs` — add `/a2a` route test case

---

### Task 1: Service Layer — Context Operations

**Files:**
- Create: `solobase/crates/solobase-core/src/blocks/messages/service.rs`
- Modify: `solobase/crates/solobase-core/src/blocks/messages/mod.rs:1` (add `pub mod service;`)

This is the foundation. All other tasks depend on it.

- [ ] **Step 1: Create `service.rs` with collection constants and context functions**

```rust
//! Service layer for the messages block.
//!
//! Plain async functions — no HTTP awareness. Both REST and A2A handlers
//! call these. All database interactions live here.

use crate::blocks::helpers::{self, json_map};
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;

pub const CONTEXTS_COLLECTION: &str = "suppers_ai__messages__contexts";
pub const ENTRIES_COLLECTION: &str = "suppers_ai__messages__entries";

// ---------------------------------------------------------------------------
// Context operations
// ---------------------------------------------------------------------------

pub async fn create_context(
    ctx: &dyn Context,
    context_type: &str,
    title: &str,
    sender_id: &str,
    recipient_id: &str,
    parent_id: Option<&str>,
    metadata: Option<serde_json::Value>,
) -> Result<db::Record, String> {
    let metadata = metadata
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

    let mut data = json_map(serde_json::json!({
        "type": context_type,
        "status": "active",
        "title": title,
        "sender_id": sender_id,
        "recipient_id": recipient_id,
        "metadata": metadata,
    }));

    if let Some(pid) = parent_id {
        data.insert("parent_id".to_string(), serde_json::json!(pid));
    }

    helpers::stamp_created(&mut data);

    db::create(ctx, CONTEXTS_COLLECTION, data)
        .await
        .map_err(|e| format!("Database error: {e}"))
}

pub async fn get_context(
    ctx: &dyn Context,
    id: &str,
) -> Result<db::Record, db::DatabaseError> {
    db::get(ctx, CONTEXTS_COLLECTION, id).await
}

pub struct ListContextsParams {
    pub context_type: Option<String>,
    pub status: Option<String>,
    pub sender_id: Option<String>,
    pub parent_id: Option<String>,
    pub page_size: i64,
    pub offset: i64,
}

pub async fn list_contexts(
    ctx: &dyn Context,
    params: &ListContextsParams,
) -> Result<db::ListResult, String> {
    let mut filters = Vec::new();

    if let Some(ref t) = params.context_type {
        filters.push(Filter {
            field: "type".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(t.clone()),
        });
    }
    if let Some(ref s) = params.status {
        filters.push(Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(s.clone()),
        });
    }
    if let Some(ref sid) = params.sender_id {
        filters.push(Filter {
            field: "sender_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(sid.clone()),
        });
    }
    if let Some(ref pid) = params.parent_id {
        filters.push(Filter {
            field: "parent_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(pid.clone()),
        });
    }

    let opts = ListOptions {
        filters,
        sort: vec![SortField {
            field: "updated_at".to_string(),
            desc: true,
        }],
        limit: params.page_size,
        offset: params.offset,
    };

    db::list(ctx, CONTEXTS_COLLECTION, &opts)
        .await
        .map_err(|e| format!("Database error: {e}"))
}

pub async fn update_context(
    ctx: &dyn Context,
    id: &str,
    updates: std::collections::HashMap<String, serde_json::Value>,
) -> Result<db::Record, db::DatabaseError> {
    let allowed = ["status", "title", "metadata"];
    let mut data = std::collections::HashMap::new();
    for key in &allowed {
        if let Some(v) = updates.get(*key) {
            data.insert(key.to_string(), v.clone());
        }
    }
    helpers::stamp_updated(&mut data);

    db::update(ctx, CONTEXTS_COLLECTION, id, data).await
}

pub async fn delete_context(
    ctx: &dyn Context,
    id: &str,
) -> Result<(), String> {
    // Cascade delete entries first
    let filters = vec![Filter {
        field: "context_id".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(id.to_string()),
    }];
    if let Err(e) = db::delete_by_filters(ctx, ENTRIES_COLLECTION, filters).await {
        tracing::warn!("Failed to cascade delete entries for context {id}: {e}");
    }

    db::delete(ctx, CONTEXTS_COLLECTION, id)
        .await
        .map_err(|e| format!("Database error: {e}"))
}
```

- [ ] **Step 2: Add entry operations to `service.rs`**

Append to the file:

```rust
// ---------------------------------------------------------------------------
// Entry operations
// ---------------------------------------------------------------------------

pub async fn add_entry(
    ctx: &dyn Context,
    context_id: &str,
    kind: &str,
    role: &str,
    sender_id: &str,
    content: &str,
    content_type: Option<&str>,
    metadata: Option<serde_json::Value>,
) -> Result<db::Record, String> {
    let metadata = metadata
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
    let content_type = content_type.unwrap_or("text/plain");

    let now = helpers::now_rfc3339();
    let data = json_map(serde_json::json!({
        "context_id": context_id,
        "kind": kind,
        "role": role,
        "status": "",
        "sender_id": sender_id,
        "content": content,
        "content_type": content_type,
        "metadata": metadata,
        "created_at": now,
    }));

    let record = db::create(ctx, ENTRIES_COLLECTION, data)
        .await
        .map_err(|e| format!("Database error: {e}"))?;

    // Bump parent context's updated_at
    let mut context_update = std::collections::HashMap::new();
    helpers::stamp_updated(&mut context_update);
    if let Err(e) = db::update(ctx, CONTEXTS_COLLECTION, context_id, context_update).await {
        tracing::warn!("Failed to update context updated_at after add_entry: {e}");
    }

    Ok(record)
}

pub async fn get_entry(
    ctx: &dyn Context,
    id: &str,
) -> Result<db::Record, db::DatabaseError> {
    db::get(ctx, ENTRIES_COLLECTION, id).await
}

pub struct ListEntriesParams {
    pub kind: Option<String>,
    pub role: Option<String>,
    pub page_size: i64,
    pub offset: i64,
}

pub async fn list_entries(
    ctx: &dyn Context,
    context_id: &str,
    params: &ListEntriesParams,
) -> Result<db::ListResult, String> {
    let mut filters = vec![Filter {
        field: "context_id".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(context_id.to_string()),
    }];

    if let Some(ref k) = params.kind {
        filters.push(Filter {
            field: "kind".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(k.clone()),
        });
    }
    if let Some(ref r) = params.role {
        filters.push(Filter {
            field: "role".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(r.clone()),
        });
    }

    let opts = ListOptions {
        filters,
        sort: vec![SortField {
            field: "created_at".to_string(),
            desc: false,
        }],
        limit: params.page_size,
        offset: params.offset,
    };

    db::list(ctx, ENTRIES_COLLECTION, &opts)
        .await
        .map_err(|e| format!("Database error: {e}"))
}

pub async fn delete_entry(
    ctx: &dyn Context,
    id: &str,
) -> Result<(), db::DatabaseError> {
    db::delete(ctx, ENTRIES_COLLECTION, id).await
}
```

- [ ] **Step 3: Add `pub mod service;` to `mod.rs`**

Add as the first line of `solobase/crates/solobase-core/src/blocks/messages/mod.rs`:

```rust
pub mod service;
```

(Below the existing `pub mod pages;` line.)

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p solobase-core 2>&1 | head -20`

Expected: should compile (service.rs has no dependents yet so unused warnings are fine).

- [ ] **Step 5: Commit**

```bash
git add solobase/crates/solobase-core/src/blocks/messages/service.rs solobase/crates/solobase-core/src/blocks/messages/mod.rs
git commit -m "feat(messages): add service layer with context and entry operations"
```

---

### Task 2: REST Endpoint Handlers

**Files:**
- Create: `solobase/crates/solobase-core/src/blocks/messages/rest.rs`

- [ ] **Step 1: Create `rest.rs` with context handlers**

```rust
//! REST endpoint handlers for the messages block.
//!
//! Thin layer: parse HTTP request → call service → format JSON response.

use super::service::{self, ListContextsParams, ListEntriesParams};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

/// Convert empty string to None (msg.query() returns "" for missing params).
fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}

// ---------------------------------------------------------------------------
// Path extraction helpers
// ---------------------------------------------------------------------------

/// Extract context ID from paths like `/b/messages/api/contexts/{id}`
/// or `/b/messages/api/contexts/{id}/entries`.
fn extract_context_id(msg: &Message) -> &str {
    let var = msg.var("id");
    if !var.is_empty() {
        return var;
    }
    let path = msg.path();
    let suffix = path
        .strip_prefix("/b/messages/api/contexts/")
        .unwrap_or("");
    suffix.split('/').next().unwrap_or("")
}

/// Extract entry ID from paths like `/b/messages/api/entries/{id}`.
fn extract_entry_id(msg: &Message) -> &str {
    let var = msg.var("id");
    if !var.is_empty() {
        return var;
    }
    let path = msg.path();
    let suffix = path
        .strip_prefix("/b/messages/api/entries/")
        .unwrap_or("");
    suffix.split('/').next().unwrap_or("")
}

// ---------------------------------------------------------------------------
// Context endpoints
// ---------------------------------------------------------------------------

pub async fn list_contexts(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let (_, page_size, offset) = msg.pagination_params(20);
    let params = ListContextsParams {
        context_type: non_empty(msg.query("type")),
        status: non_empty(msg.query("status")),
        sender_id: non_empty(msg.query("sender_id")),
        parent_id: non_empty(msg.query("parent_id")),
        page_size: page_size as i64,
        offset: offset as i64,
    };
    match service::list_contexts(ctx, &params).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &e),
    }
}

pub async fn create_context(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    #[derive(serde::Deserialize)]
    struct Body {
        #[serde(rename = "type")]
        context_type: String,
        #[serde(default)]
        title: String,
        #[serde(default)]
        sender_id: String,
        #[serde(default)]
        recipient_id: String,
        parent_id: Option<String>,
        metadata: Option<serde_json::Value>,
    }
    let body: Body = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    match service::create_context(
        ctx,
        &body.context_type,
        &body.title,
        &body.sender_id,
        &body.recipient_id,
        body.parent_id.as_deref(),
        body.metadata,
    )
    .await
    {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &e),
    }
}

pub async fn get_context(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let id = extract_context_id(msg);
    if id.is_empty() {
        return err_bad_request(msg, "Missing context ID");
    }
    match service::get_context(ctx, id).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Context not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub async fn update_context(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let id = extract_context_id(msg);
    if id.is_empty() {
        return err_bad_request(msg, "Missing context ID");
    }
    let body: std::collections::HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    match service::update_context(ctx, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Context not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub async fn delete_context(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let id = extract_context_id(msg);
    if id.is_empty() {
        return err_bad_request(msg, "Missing context ID");
    }
    match service::delete_context(ctx, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => err_internal(msg, &e),
    }
}

// ---------------------------------------------------------------------------
// Entry endpoints
// ---------------------------------------------------------------------------

pub async fn list_entries(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let context_id = extract_context_id(msg);
    if context_id.is_empty() {
        return err_bad_request(msg, "Missing context ID");
    }
    let (_, page_size, offset) = msg.pagination_params(100);
    let params = ListEntriesParams {
        kind: non_empty(msg.query("kind")),
        role: non_empty(msg.query("role")),
        page_size: page_size as i64,
        offset: offset as i64,
    };
    match service::list_entries(ctx, context_id, &params).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &e),
    }
}

pub async fn add_entry(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let context_id = extract_context_id(msg);
    if context_id.is_empty() {
        return err_bad_request(msg, "Missing context ID");
    }
    #[derive(serde::Deserialize)]
    struct Body {
        #[serde(default = "default_kind")]
        kind: String,
        #[serde(default)]
        role: String,
        #[serde(default)]
        sender_id: String,
        #[serde(default)]
        content: String,
        content_type: Option<String>,
        metadata: Option<serde_json::Value>,
    }
    fn default_kind() -> String {
        "message".to_string()
    }
    let body: Body = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };
    match service::add_entry(
        ctx,
        context_id,
        &body.kind,
        &body.role,
        &body.sender_id,
        &body.content,
        body.content_type.as_deref(),
        body.metadata,
    )
    .await
    {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &e),
    }
}

pub async fn get_entry(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let id = extract_entry_id(msg);
    if id.is_empty() {
        return err_bad_request(msg, "Missing entry ID");
    }
    match service::get_entry(ctx, id).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Entry not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub async fn delete_entry(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let id = extract_entry_id(msg);
    if id.is_empty() {
        return err_bad_request(msg, "Missing entry ID");
    }
    match service::delete_entry(ctx, id).await {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Entry not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}
```

- [ ] **Step 2: Add `pub mod rest;` to `mod.rs`**

Add below the `pub mod service;` line:

```rust
pub mod rest;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p solobase-core 2>&1 | head -20`

The code uses `msg.query("name")` which returns `&str` (empty if missing). The `non_empty()` helper converts to `Option<String>` for the filter params.

- [ ] **Step 4: Commit**

```bash
git add solobase/crates/solobase-core/src/blocks/messages/rest.rs solobase/crates/solobase-core/src/blocks/messages/mod.rs
git commit -m "feat(messages): add REST endpoint handlers for contexts and entries"
```

---

### Task 3: Rewrite `mod.rs` — BlockInfo + Routing

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/messages/mod.rs`

Replace the entire contents of `mod.rs`. The old thread/message handlers, collection constants, and path extractors are all replaced.

- [ ] **Step 1: Rewrite `mod.rs` with new BlockInfo and routing**

Replace the full content of `solobase/crates/solobase-core/src/blocks/messages/mod.rs` with:

```rust
pub mod pages;
pub mod rest;
pub mod service;

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub struct MessagesBlock;

// ---------------------------------------------------------------------------
// Block trait implementation
// ---------------------------------------------------------------------------

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for MessagesBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::types::CollectionSchema;
        use wafer_run::AuthLevel;

        BlockInfo::new(
            "suppers-ai/messages",
            "0.0.1",
            "http-handler@v1",
            "Unified message and context system",
        )
        .instance_mode(InstanceMode::Singleton)
        .requires(vec!["wafer-run/database".into()])
        .collections(vec![
            CollectionSchema::new(service::CONTEXTS_COLLECTION)
                .field("type", "string")
                .field_default("status", "string", "active")
                .field_default("title", "string", "")
                .field_default("sender_id", "string", "")
                .field_default("recipient_id", "string", "")
                .field_optional("parent_id", "string")
                .field_default("metadata", "text", "{}")
                .index(&["type"])
                .index(&["status"])
                .index(&["parent_id"])
                .index(&["updated_at"])
                .index(&["sender_id"]),
            CollectionSchema::new(service::ENTRIES_COLLECTION)
                .field_ref(
                    "context_id",
                    "string",
                    &format!("{}.id", service::CONTEXTS_COLLECTION),
                )
                .field_default("kind", "string", "message")
                .field_default("role", "string", "")
                .field_default("status", "string", "")
                .field_default("sender_id", "string", "")
                .field_default("content", "text", "")
                .field_default("content_type", "string", "text/plain")
                .field_default("metadata", "text", "{}")
                .index(&["context_id"])
                .index(&["context_id", "created_at"])
                .index(&["kind"])
                .index(&["context_id", "kind"]),
        ])
        .category(wafer_run::BlockCategory::Feature)
        .description(
            "Protocol-agnostic context + entry system. Supports chat conversations, \
             A2A task lifecycle, notifications, and future protocols. Contexts are \
             containers (conversations, tasks, channels). Entries are the universal \
             primitive (messages, artifacts, notifications, status changes).",
        )
        .endpoints(vec![
            // Contexts
            BlockEndpoint::get("/b/messages/api/contexts")
                .summary("List contexts")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/messages/api/contexts")
                .summary("Create context")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/messages/api/contexts/{id}")
                .summary("Get context")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::patch("/b/messages/api/contexts/{id}")
                .summary("Update context")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::delete("/b/messages/api/contexts/{id}")
                .summary("Delete context and its entries")
                .auth(AuthLevel::Authenticated),
            // Entries
            BlockEndpoint::get("/b/messages/api/contexts/{id}/entries")
                .summary("List entries in context")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/messages/api/contexts/{id}/entries")
                .summary("Add entry to context")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/messages/api/entries/{id}")
                .summary("Get entry")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::delete("/b/messages/api/entries/{id}")
                .summary("Delete entry")
                .auth(AuthLevel::Authenticated),
        ])
        .can_disable(true)
        .default_enabled(true)
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![wafer_run::UiRoute::authenticated("/")]
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let action = msg.action();
        let path = msg.path();
        let is_api = path.contains("/api/");
        let user_id = msg.user_id().to_string();

        // All endpoints require authentication
        if user_id.is_empty() {
            return crate::ui::forbidden_response(msg);
        }

        // UI pages require admin role
        if !is_api {
            let is_admin = msg
                .get_meta("auth.user_roles")
                .split(',')
                .any(|r| r.trim() == "admin");
            if !is_admin {
                return crate::ui::forbidden_response(msg);
            }
        }

        match (action, path) {
            // UI pages
            ("retrieve", "/b/messages/") => pages::context_list_page(ctx, msg).await,
            ("retrieve", _)
                if path.starts_with("/b/messages/contexts/") && !path.contains("/api/") =>
            {
                pages::context_detail_page(ctx, msg).await
            }

            // Context CRUD
            ("retrieve", "/b/messages/api/contexts") => rest::list_contexts(ctx, msg).await,
            ("create", "/b/messages/api/contexts") => rest::create_context(ctx, msg).await,
            ("retrieve", _)
                if path.starts_with("/b/messages/api/contexts/")
                    && !path["/b/messages/api/contexts/".len()..].contains('/') =>
            {
                rest::get_context(ctx, msg).await
            }
            ("update", _)
                if path.starts_with("/b/messages/api/contexts/")
                    && !path["/b/messages/api/contexts/".len()..].contains('/') =>
            {
                rest::update_context(ctx, msg).await
            }
            ("delete", _)
                if path.starts_with("/b/messages/api/contexts/")
                    && !path["/b/messages/api/contexts/".len()..].contains('/') =>
            {
                rest::delete_context(ctx, msg).await
            }

            // Entries within a context
            ("retrieve", _)
                if path.starts_with("/b/messages/api/contexts/")
                    && path.ends_with("/entries") =>
            {
                rest::list_entries(ctx, msg).await
            }
            ("create", _)
                if path.starts_with("/b/messages/api/contexts/")
                    && path.ends_with("/entries") =>
            {
                rest::add_entry(ctx, msg).await
            }

            // Direct entry access
            ("retrieve", _) if path.starts_with("/b/messages/api/entries/") => {
                rest::get_entry(ctx, msg).await
            }
            ("delete", _) if path.starts_with("/b/messages/api/entries/") => {
                rest::delete_entry(ctx, msg).await
            }

            _ => err_not_found(msg, "not found"),
        }
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p solobase-core 2>&1 | head -20`

Expected: Compile errors in `pages.rs` (still references old constants). That's expected — we fix pages in Task 5.

- [ ] **Step 3: Commit**

```bash
git add solobase/crates/solobase-core/src/blocks/messages/mod.rs
git commit -m "feat(messages): rewrite BlockInfo with contexts/entries schema and new routing"
```

---

### Task 4: A2A JSON-RPC Handler

**Files:**
- Create: `solobase/crates/solobase-core/src/blocks/messages/a2a.rs`
- Modify: `solobase/crates/solobase-core/src/pipeline.rs:32-51` (add `/a2a` route)
- Modify: `solobase/crates/solobase-core/src/blocks/messages/mod.rs` (add `pub mod a2a;`)

- [ ] **Step 1: Create `a2a.rs` with JSON-RPC dispatcher**

```rust
//! A2A JSON-RPC handler for the messages block.
//!
//! Handles `POST /a2a` — dispatches by JSON-RPC method field.
//! Maps A2A Task/Message/Artifact concepts to internal contexts/entries.

use super::service::{self, ListContextsParams, ListEntriesParams};
use crate::blocks::helpers::RecordExt;
use wafer_core::clients::database as db;
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

// ---------------------------------------------------------------------------
// JSON-RPC types
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

fn jsonrpc_response(id: Option<serde_json::Value>, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "result": result,
        "id": id,
    })
}

fn jsonrpc_error(
    id: Option<serde_json::Value>,
    code: i64,
    message: &str,
) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": code,
            "message": message,
        },
        "id": id,
    })
}

// ---------------------------------------------------------------------------
// Main dispatcher
// ---------------------------------------------------------------------------

pub async fn handle_a2a(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let req: JsonRpcRequest = match msg.decode() {
        Ok(r) => r,
        Err(e) => {
            let body = jsonrpc_error(None, -32700, &format!("Parse error: {e}"));
            return json_respond(msg, &body);
        }
    };

    if req.jsonrpc != "2.0" {
        let body = jsonrpc_error(req.id, -32600, "Invalid JSON-RPC version");
        return json_respond(msg, &body);
    }

    let params = req.params.unwrap_or(serde_json::Value::Null);

    let result = match req.method.as_str() {
        "SendMessage" => handle_send_message(ctx, &params).await,
        "GetTask" => handle_get_task(ctx, &params).await,
        "ListTasks" => handle_list_tasks(ctx, &params).await,
        "CancelTask" => handle_cancel_task(ctx, &params).await,
        _ => Err((-32601, format!("Method not found: {}", req.method))),
    };

    let body = match result {
        Ok(value) => jsonrpc_response(req.id, value),
        Err((code, message)) => jsonrpc_error(req.id, code, &message),
    };
    json_respond(msg, &body)
}

// ---------------------------------------------------------------------------
// A2A method handlers
// ---------------------------------------------------------------------------

async fn handle_send_message(
    ctx: &dyn Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, (i64, String)> {
    let message = params
        .get("message")
        .ok_or((-32602, "Missing 'message' parameter".to_string()))?;

    let role = message
        .get("role")
        .and_then(|r| r.as_str())
        .unwrap_or("user");

    // Extract text content from parts
    let content = extract_text_from_parts(message);

    // Check if we're adding to an existing context via contextId
    let context_id = params
        .get("contextId")
        .and_then(|c| c.as_str());

    let task_context = if let Some(cid) = context_id {
        // Add message to existing context
        match service::get_context(ctx, cid).await {
            Ok(record) => record,
            Err(e) if e.code == ErrorCode::NotFound => {
                return Err((-32001, format!("Task not found: {cid}")));
            }
            Err(e) => return Err((-32000, format!("Database error: {e}"))),
        }
    } else {
        // Create new task context
        let title = message
            .get("parts")
            .and_then(|p| p.as_array())
            .and_then(|parts| parts.first())
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("A2A Task")
            .chars()
            .take(100)
            .collect::<String>();

        service::create_context(ctx, "task", &title, "", "", None, None)
            .await
            .map_err(|e| (-32000, e))?
    };

    // Add the message as an entry
    let parts_meta = message.get("parts").cloned();
    service::add_entry(
        ctx,
        &task_context.id,
        "message",
        role,
        "",
        &content,
        Some("text/plain"),
        parts_meta.map(|p| serde_json::json!({"parts": p})),
    )
    .await
    .map_err(|e| (-32000, e))?;

    // Update status to "submitted" if newly created
    if context_id.is_none() {
        let mut updates = std::collections::HashMap::new();
        updates.insert(
            "status".to_string(),
            serde_json::json!("submitted"),
        );
        let _ = service::update_context(ctx, &task_context.id, updates).await;
    }

    // Return the task
    build_task_response(ctx, &task_context.id).await
}

async fn handle_get_task(
    ctx: &dyn Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, (i64, String)> {
    let id = params
        .get("id")
        .and_then(|i| i.as_str())
        .ok_or((-32602, "Missing 'id' parameter".to_string()))?;

    let history_length = params
        .get("historyLength")
        .and_then(|h| h.as_i64());

    build_task_response_with_history(ctx, id, history_length).await
}

async fn handle_list_tasks(
    ctx: &dyn Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, (i64, String)> {
    let status = params
        .get("status")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let context_id = params
        .get("contextId")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());
    let page_size = params
        .get("pageSize")
        .and_then(|p| p.as_i64())
        .unwrap_or(50)
        .min(100);

    let list_params = ListContextsParams {
        context_type: Some("task".to_string()),
        status,
        sender_id: None,
        parent_id: context_id,
        page_size,
        offset: 0,
    };

    let result = service::list_contexts(ctx, &list_params)
        .await
        .map_err(|e| (-32000, e))?;

    let tasks: Vec<serde_json::Value> = result
        .records
        .iter()
        .map(|r| context_to_task(r))
        .collect();

    Ok(serde_json::json!({
        "tasks": tasks,
        "totalSize": result.total,
    }))
}

async fn handle_cancel_task(
    ctx: &dyn Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, (i64, String)> {
    let id = params
        .get("id")
        .and_then(|i| i.as_str())
        .ok_or((-32602, "Missing 'id' parameter".to_string()))?;

    // Check current status
    let context = service::get_context(ctx, id)
        .await
        .map_err(|e| {
            if e.code == ErrorCode::NotFound {
                (-32001, format!("Task not found: {id}"))
            } else {
                (-32000, format!("Database error: {e}"))
            }
        })?;

    let current_status = context.str_field("status");
    let terminal = ["completed", "failed", "canceled", "rejected"];
    if terminal.contains(&current_status) {
        return Err((-32002, format!("Task is already in terminal state: {current_status}")));
    }

    let mut updates = std::collections::HashMap::new();
    updates.insert("status".to_string(), serde_json::json!("canceled"));
    service::update_context(ctx, id, updates)
        .await
        .map_err(|e| (-32000, format!("Database error: {e}")))?;

    build_task_response(ctx, id).await
}

// ---------------------------------------------------------------------------
// Response builders
// ---------------------------------------------------------------------------

fn context_to_task(record: &db::Record) -> serde_json::Value {
    serde_json::json!({
        "id": record.id,
        "status": {
            "state": record.str_field("status"),
            "timestamp": record.str_field("updated_at"),
        },
        "contextId": record.data.get("parent_id").cloned().unwrap_or(serde_json::Value::Null),
        "metadata": record.data.get("metadata").cloned().unwrap_or(serde_json::json!({})),
    })
}

fn entry_to_message(record: &db::Record) -> serde_json::Value {
    let content = record.str_field("content");
    serde_json::json!({
        "role": record.str_field("role"),
        "parts": [{"text": content}],
        "metadata": record.data.get("metadata").cloned().unwrap_or(serde_json::json!({})),
    })
}

fn entry_to_artifact(record: &db::Record) -> serde_json::Value {
    let content = record.str_field("content");
    let content_type = record.str_field("content_type");
    serde_json::json!({
        "id": record.id,
        "mimeType": content_type,
        "parts": [{"text": content}],
        "metadata": record.data.get("metadata").cloned().unwrap_or(serde_json::json!({})),
    })
}

async fn build_task_response(
    ctx: &dyn Context,
    context_id: &str,
) -> Result<serde_json::Value, (i64, String)> {
    build_task_response_with_history(ctx, context_id, None).await
}

async fn build_task_response_with_history(
    ctx: &dyn Context,
    context_id: &str,
    history_length: Option<i64>,
) -> Result<serde_json::Value, (i64, String)> {
    let context = service::get_context(ctx, context_id)
        .await
        .map_err(|e| {
            if e.code == ErrorCode::NotFound {
                (-32001, format!("Task not found: {context_id}"))
            } else {
                (-32000, format!("Database error: {e}"))
            }
        })?;

    let mut task = context_to_task(&context);

    // Load entries unless historyLength == 0
    if history_length != Some(0) {
        let limit = history_length.unwrap_or(200);
        let entries_params = ListEntriesParams {
            kind: None,
            role: None,
            page_size: limit,
            offset: 0,
        };
        if let Ok(entries) = service::list_entries(ctx, context_id, &entries_params).await {
            let mut messages = Vec::new();
            let mut artifacts = Vec::new();

            for entry in &entries.records {
                match entry.str_field("kind") {
                    "artifact" => artifacts.push(entry_to_artifact(entry)),
                    _ => messages.push(entry_to_message(entry)),
                }
            }

            task["messages"] = serde_json::json!(messages);
            task["artifacts"] = serde_json::json!(artifacts);
        }
    }

    Ok(task)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_text_from_parts(message: &serde_json::Value) -> String {
    message
        .get("parts")
        .and_then(|p| p.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}
```

- [ ] **Step 2: Add `pub mod a2a;` to `mod.rs`**

Add below the `pub mod rest;` line in `solobase/crates/solobase-core/src/blocks/messages/mod.rs`:

```rust
pub mod a2a;
```

- [ ] **Step 3: Add `/a2a` route to pipeline.rs**

In `solobase/crates/solobase-core/src/pipeline.rs`, add a new route check after the discovery endpoints block (after line 51, before the `// 1. Strip /api prefix` comment). The A2A endpoint requires auth, so it goes after the JWT validation step. Insert this block after the auth validation (after line 67) and before the request info capture (before line 70):

```rust
    // A2A JSON-RPC endpoint
    let path = msg.path().to_string();
    if path == "/a2a" {
        return crate::blocks::messages::a2a::handle_a2a(ctx, msg).await;
    }
```

Note: The exact insertion point is after JWT/auth validation so the A2A handler can read auth metadata from the message. Place it just before the `// Capture request info before routing` comment (line 70).

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p solobase-core 2>&1 | head -20`

- [ ] **Step 5: Commit**

```bash
git add solobase/crates/solobase-core/src/blocks/messages/a2a.rs solobase/crates/solobase-core/src/blocks/messages/mod.rs solobase/crates/solobase-core/src/pipeline.rs
git commit -m "feat(messages): add A2A JSON-RPC handler with SendMessage, GetTask, ListTasks, CancelTask"
```

---

### Task 5: Update SSR Pages

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/messages/pages.rs`

Rewrite the pages to use contexts/entries terminology and the service layer.

- [ ] **Step 1: Rewrite `pages.rs`**

Replace the full content of `solobase/crates/solobase-core/src/blocks/messages/pages.rs`:

```rust
//! SSR pages for the messages block.
//!
//! Provides:
//! - Context list page (`GET /b/messages/`)
//! - Context detail page (`GET /b/messages/contexts/{id}`)

use crate::blocks::helpers::RecordExt;
use crate::ui::{self, components, NavItem, SiteConfig, UserInfo};
use maud::{html, Markup};
use wafer_core::clients::database as db;
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

use super::service::{self, ListContextsParams, ListEntriesParams};

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

fn nav() -> Vec<NavItem> {
    vec![NavItem {
        label: "Contexts".into(),
        href: "/b/messages/".into(),
        icon: "message-square",
    }]
}

fn messages_page(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    content: Markup,
    msg: &mut Message,
) -> Result_ {
    let is_fragment = ui::is_htmx(msg);
    let markup =
        ui::layout::block_shell(title, config, &nav(), user, path, content, is_fragment);
    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// Entry card fragment
// ---------------------------------------------------------------------------

pub fn entry_card(record: &db::Record) -> Markup {
    let kind = record.str_field("kind");
    let role = record.str_field("role");
    let content = record.str_field("content");
    let content_type = record.str_field("content_type");
    let created_at = record.str_field("created_at");
    let date = created_at.get(..10).unwrap_or(created_at);

    let (bg_style, badge_class) = match kind {
        "artifact" => (
            "background:#fdf4ff;border-left:3px solid #a855f7",
            "badge-warning",
        ),
        "notification" => (
            "background:#fefce8;border-left:3px solid #eab308",
            "badge-warning",
        ),
        "status" => (
            "background:#f0f9ff;border-left:3px solid #0ea5e9",
            "badge-info",
        ),
        _ => match role {
            "user" => (
                "background:#eff6ff;border-left:3px solid #3b82f6",
                "badge-info",
            ),
            "agent" | "assistant" => (
                "background:#f8fafc;border-left:3px solid #94a3b8",
                "badge",
            ),
            "system" => (
                "background:#fefce8;border-left:3px solid #eab308",
                "badge-warning",
            ),
            _ => (
                "background:#f0fdf4;border-left:3px solid #22c55e",
                "badge-success",
            ),
        },
    };

    html! {
        div .card style={"margin-bottom:0.75rem;" (bg_style)} {
            div style="display:flex;align-items:center;gap:0.5rem;margin-bottom:0.5rem" {
                span .badge .(badge_class) style="text-transform:capitalize" { (kind) }
                @if !role.is_empty() {
                    span .badge style="text-transform:capitalize" { (role) }
                }
                @if kind == "artifact" && !content_type.is_empty() && content_type != "text/plain" {
                    span .text-muted style="font-size:0.7rem" { (content_type) }
                }
                @if !date.is_empty() {
                    span .text-muted style="font-size:0.75rem;margin-left:auto" { (date) }
                }
            }
            p style="margin:0;white-space:pre-wrap;word-break:break-word" { (content) }
        }
    }
}

// ---------------------------------------------------------------------------
// Context list page
// ---------------------------------------------------------------------------

pub async fn context_list_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    let params = ListContextsParams {
        context_type: None,
        status: None,
        sender_id: None,
        parent_id: None,
        page_size: 50,
        offset: 0,
    };

    let contexts = match service::list_contexts(ctx, &params).await {
        Ok(r) => r.records,
        Err(_) => vec![],
    };

    let content = html! {
        (components::page_header(
            "Messages",
            Some("Manage contexts and entries"),
            None,
        ))

        // New context form
        div .card style="margin-bottom:1.5rem" {
            h3 style="font-size:1rem;font-weight:600;margin:0 0 0.75rem" { "New Context" }
            form
                hx-post="/b/messages/api/contexts"
                hx-target="#context-list"
                hx-swap="afterbegin"
                hx-on--after-request="if(event.detail.successful){this.reset()}"
            {
                div style="display:flex;gap:0.5rem" {
                    select .form-input name="type" style="width:auto" {
                        option value="conversation" { "Conversation" }
                        option value="task" { "Task" }
                        option value="notification" { "Notification" }
                    }
                    input .form-input
                        type="text"
                        name="title"
                        placeholder="Title"
                        required
                        style="flex:1"
                    ;
                    button .btn .btn-primary type="submit" { "Create" }
                }
            }
        }

        // Context list
        div #context-list {
            @if contexts.is_empty() {
                div .text-center .text-muted style="padding:2rem" {
                    "No contexts yet. Create one above."
                }
            } @else {
                @for context in &contexts {
                    @let id = context.id.as_str();
                    @let title = context.str_field("title");
                    @let context_type = context.str_field("type");
                    @let status = context.str_field("status");
                    @let updated_at = context.str_field("updated_at");
                    @let date = updated_at.get(..10).unwrap_or(updated_at);
                    a .card href={"/b/messages/contexts/" (id)}
                        style="display:block;text-decoration:none;margin-bottom:0.5rem;transition:box-shadow 0.15s"
                        onmouseover="this.style.boxShadow='0 2px 8px rgba(0,0,0,0.1)'"
                        onmouseout="this.style.boxShadow=''"
                    {
                        div style="display:flex;align-items:center;justify-content:space-between" {
                            div style="display:flex;align-items:center;gap:0.5rem" {
                                span .badge style="text-transform:capitalize" { (context_type) }
                                span style="font-weight:500;color:var(--text-primary)" {
                                    @if title.is_empty() { "Untitled" } @else { (title) }
                                }
                            }
                            div style="display:flex;align-items:center;gap:0.5rem" {
                                span .badge { (status) }
                                @if !date.is_empty() {
                                    span .text-muted style="font-size:0.8rem" { (date) }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    messages_page("Messages", &config, &path, user.as_ref(), content, msg)
}

// ---------------------------------------------------------------------------
// Context detail page
// ---------------------------------------------------------------------------

pub async fn context_detail_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    let context_id = path
        .strip_prefix("/b/messages/contexts/")
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("");

    if context_id.is_empty() {
        return ui::not_found_response(msg);
    }

    let context = match service::get_context(ctx, context_id).await {
        Ok(r) => r,
        Err(e) if e.code == ErrorCode::NotFound => return ui::not_found_response(msg),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

    let entries_params = ListEntriesParams {
        kind: None,
        role: None,
        page_size: 200,
        offset: 0,
    };

    let entries = match service::list_entries(ctx, context_id, &entries_params).await {
        Ok(r) => r.records,
        Err(_) => vec![],
    };

    let context_title = context.str_field("title");
    let context_type = context.str_field("type");
    let context_status = context.str_field("status");
    let display_title = if context_title.is_empty() {
        "Untitled"
    } else {
        context_title
    };

    let post_url = format!("/b/messages/api/contexts/{context_id}/entries");

    let content = html! {
        // Header with back button
        div style="display:flex;align-items:center;gap:0.75rem;margin-bottom:1.5rem" {
            a .btn .btn-ghost .btn-sm href="/b/messages/" { "\u{2190} Back" }
            h1 .page-title style="margin:0" { (display_title) }
            span .badge style="text-transform:capitalize" { (context_type) }
            span .badge { (context_status) }
        }

        // Entries area
        div #entries-list style="margin-bottom:1.5rem;max-height:60vh;overflow-y:auto" {
            @if entries.is_empty() {
                div .text-center .text-muted style="padding:2rem" {
                    "No entries yet. Add one below."
                }
            } @else {
                @for e in &entries {
                    (entry_card(e))
                }
            }
        }

        // New entry form
        div .card {
            h3 style="font-size:0.9rem;font-weight:600;margin:0 0 0.75rem;color:var(--text-muted)" {
                "Add Entry"
            }
            form
                hx-post=(post_url)
                hx-target="#entries-list"
                hx-swap="beforeend"
                hx-on--after-request="if(event.detail.successful){this.reset();var list=document.getElementById('entries-list');list.scrollTop=list.scrollHeight;}"
            {
                div style="display:flex;gap:0.5rem;margin-bottom:0.5rem" {
                    select .form-input name="kind" style="width:auto" {
                        option value="message" { "message" }
                        option value="artifact" { "artifact" }
                        option value="notification" { "notification" }
                        option value="status" { "status" }
                    }
                    select .form-input name="role" style="width:auto" {
                        option value="user" { "user" }
                        option value="agent" { "agent" }
                        option value="system" { "system" }
                    }
                }
                div style="display:flex;gap:0.5rem;align-items:flex-end" {
                    textarea .form-input
                        name="content"
                        placeholder="Entry content"
                        rows="3"
                        required
                        style="flex:1;resize:vertical"
                    {}
                    button .btn .btn-primary type="submit" { "Add" }
                }
            }
        }
    };

    messages_page(display_title, &config, &path, user.as_ref(), content, msg)
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p solobase-core 2>&1 | head -20`

Expected: May still have compile errors from LLM block pages — that's fine, fixed in next task.

- [ ] **Step 3: Commit**

```bash
git add solobase/crates/solobase-core/src/blocks/messages/pages.rs
git commit -m "feat(messages): rewrite SSR pages for contexts and entries"
```

---

### Task 6: Update LLM Block

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/llm/mod.rs:25-103` (inter-block call helpers)
- Modify: `solobase/crates/solobase-core/src/blocks/llm/pages.rs:24-25` (collection constants)

- [ ] **Step 1: Update LLM `mod.rs` inter-block call helpers**

In `solobase/crates/solobase-core/src/blocks/llm/mod.rs`, replace the `messages_create` function (lines 24-69):

```rust
/// Call the messages block to create an entry in a context.
async fn messages_create(
    ctx: &dyn Context,
    original_msg: &Message,
    context_id: &str,
    role: &str,
    content: &str,
) -> Option<serde_json::Value> {
    let body = serde_json::to_vec(&serde_json::json!({
        "kind": "message",
        "role": role,
        "content": content,
    }))
    .unwrap_or_default();

    let resource = format!("/b/messages/api/contexts/{context_id}/entries");
    let mut msg = Message::new(
        format!("create:{resource}"),
        body,
    );
    msg.set_meta("req.action", "create");
    msg.set_meta("req.resource", &resource);
    msg.set_meta("http.method", "POST");
    msg.set_meta("http.path", &resource);
    msg.set_meta("req.content_type", "application/json");
    // Forward auth from original request
    let user_id = original_msg.user_id().to_string();
    if !user_id.is_empty() {
        msg.set_meta("auth.user_id", &user_id);
    }
    let user_email = original_msg.get_meta("auth.user_email").to_string();
    if !user_email.is_empty() {
        msg.set_meta("auth.user_email", &user_email);
    }
    let user_roles = original_msg.get_meta("auth.user_roles").to_string();
    if !user_roles.is_empty() {
        msg.set_meta("auth.user_roles", &user_roles);
    }

    let result = ctx.call_block("suppers-ai/messages", &mut msg).await;
    if matches!(result.action, Action::Respond) {
        if let Some(ref resp) = result.response {
            return serde_json::from_slice::<serde_json::Value>(&resp.data).ok();
        }
    }
    None
}
```

Replace the `messages_list` function (lines 71-103):

```rust
/// Call the messages block to list entries in a context.
async fn messages_list(
    ctx: &dyn Context,
    original_msg: &Message,
    context_id: &str,
) -> Vec<serde_json::Value> {
    let resource = format!("/b/messages/api/contexts/{context_id}/entries?kind=message");
    let mut msg = Message::new(format!("retrieve:{resource}"), vec![]);
    msg.set_meta("req.action", "retrieve");
    msg.set_meta("req.resource", &resource);
    msg.set_meta("http.method", "GET");
    msg.set_meta("http.path", &resource);
    let user_id = original_msg.user_id().to_string();
    if !user_id.is_empty() {
        msg.set_meta("auth.user_id", &user_id);
    }
    let user_roles = original_msg.get_meta("auth.user_roles").to_string();
    if !user_roles.is_empty() {
        msg.set_meta("auth.user_roles", &user_roles);
    }

    let result = ctx.call_block("suppers-ai/messages", &mut msg).await;
    if matches!(result.action, Action::Respond) {
        if let Some(ref resp) = result.response {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&resp.data) {
                if let Some(records) = v.get("records").and_then(|r| r.as_array()) {
                    return records.clone();
                }
            }
        }
    }
    vec![]
}
```

Note: The parameter name changes from `thread_id` to `context_id` in both functions. All call sites in `handle_chat` (lines 180, 183, 223) already pass a variable called `thread_id` — these still work since it's just a string ID. The variable name in the calling code can stay as `thread_id` for now (it's the LLM block's internal naming — the messages block doesn't care what the caller calls it).

- [ ] **Step 2: Update LLM `pages.rs` collection constants**

In `solobase/crates/solobase-core/src/blocks/llm/pages.rs`, replace lines 24-25:

```rust
const THREADS_COLLECTION: &str = "suppers_ai__messages__threads";
const MESSAGES_COLLECTION: &str = "suppers_ai__messages__messages";
```

With:

```rust
const CONTEXTS_COLLECTION: &str = "suppers_ai__messages__contexts";
const ENTRIES_COLLECTION: &str = "suppers_ai__messages__entries";
```

Then update all references in the file:
- Replace all `THREADS_COLLECTION` with `CONTEXTS_COLLECTION`
- Replace all `MESSAGES_COLLECTION` with `ENTRIES_COLLECTION`
- Replace `"thread_id"` filter field with `"context_id"` in the `ListOptions` filters (the `messages_opts` in `thread_page` around line 291)

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p solobase-core 2>&1 | head -30`

Expected: Clean compile. If there are issues with field names in the LLM pages (e.g., accessing `"role"` or `"content"` on entries), fix them — the field names in the new entries table are the same (`role`, `content`), so these should work without changes.

- [ ] **Step 4: Commit**

```bash
git add solobase/crates/solobase-core/src/blocks/llm/mod.rs solobase/crates/solobase-core/src/blocks/llm/pages.rs
git commit -m "feat(llm): update to use new contexts/entries endpoints and collections"
```

---

### Task 7: Add Routing Test + Final Verification

**Files:**
- Modify: `solobase/crates/solobase-core/src/routing.rs` (update test cases)

- [ ] **Step 1: Update routing tests**

In `solobase/crates/solobase-core/src/routing.rs`, the existing test `route_table_maps_expected_paths` (line 243) should already pass since the `/b/messages` route hasn't changed. Verify by running:

Run: `cargo test -p solobase-core -- routing 2>&1 | tail -20`

Expected: All routing tests pass.

- [ ] **Step 2: Full compile check**

Run: `cargo check -p solobase-core 2>&1 | tail -20`

Expected: Clean compile, no errors.

- [ ] **Step 3: Run all solobase-core tests**

Run: `cargo test -p solobase-core 2>&1 | tail -30`

Expected: All tests pass. Fix any failures.

- [ ] **Step 4: Commit any test fixes**

Only if fixes were needed:

```bash
git add -u
git commit -m "fix: resolve test failures from messages block redesign"
```

---

### Task 8: Add JSON Schemas to Endpoints

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/messages/mod.rs` (BlockInfo endpoint declarations)

Add JSON Schemas to the endpoint declarations so they appear in OpenAPI and A2A AgentCard discovery.

- [ ] **Step 1: Add schemas to context endpoints in `mod.rs`**

In the `info()` method of `mod.rs`, replace the plain endpoint declarations with schema-annotated versions. Update the `.endpoints(vec![...])` section:

```rust
        .endpoints(vec![
            // Contexts
            BlockEndpoint::get("/b/messages/api/contexts")
                .summary("List contexts")
                .description("List contexts with optional filters by type, status, sender_id, parent_id")
                .auth(AuthLevel::Authenticated)
                .query_params_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "type": {"type": "string", "description": "Filter by context type (conversation, task, notification)"},
                        "status": {"type": "string", "description": "Filter by status"},
                        "sender_id": {"type": "string", "description": "Filter by sender"},
                        "parent_id": {"type": "string", "description": "Filter by parent context"},
                        "page": {"type": "integer", "default": 1},
                        "page_size": {"type": "integer", "default": 20}
                    }
                }))
                .output_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "records": {"type": "array", "items": {"$ref": "#/components/schemas/Context"}},
                        "total": {"type": "integer"}
                    }
                }))
                .tags(&["contexts"]),
            BlockEndpoint::post("/b/messages/api/contexts")
                .summary("Create context")
                .auth(AuthLevel::Authenticated)
                .input_schema(serde_json::json!({
                    "type": "object",
                    "required": ["type"],
                    "properties": {
                        "type": {"type": "string", "description": "Context type: conversation, task, notification, etc."},
                        "title": {"type": "string", "default": ""},
                        "sender_id": {"type": "string", "default": ""},
                        "recipient_id": {"type": "string", "default": ""},
                        "parent_id": {"type": "string", "description": "Parent context ID for sub-tasks/threads"},
                        "metadata": {"type": "object", "default": {}}
                    }
                }))
                .tags(&["contexts"]),
            BlockEndpoint::get("/b/messages/api/contexts/{id}")
                .summary("Get context")
                .auth(AuthLevel::Authenticated)
                .path_params_schema(serde_json::json!({
                    "type": "object",
                    "required": ["id"],
                    "properties": {
                        "id": {"type": "string", "description": "Context ID"}
                    }
                }))
                .tags(&["contexts"]),
            BlockEndpoint::patch("/b/messages/api/contexts/{id}")
                .summary("Update context")
                .auth(AuthLevel::Authenticated)
                .input_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "status": {"type": "string"},
                        "title": {"type": "string"},
                        "metadata": {"type": "object"}
                    }
                }))
                .tags(&["contexts"]),
            BlockEndpoint::delete("/b/messages/api/contexts/{id}")
                .summary("Delete context and its entries")
                .auth(AuthLevel::Authenticated)
                .tags(&["contexts"]),
            // Entries
            BlockEndpoint::get("/b/messages/api/contexts/{id}/entries")
                .summary("List entries in context")
                .auth(AuthLevel::Authenticated)
                .query_params_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "kind": {"type": "string", "description": "Filter by kind (message, artifact, notification, status)"},
                        "role": {"type": "string", "description": "Filter by role (user, agent, system)"},
                        "page": {"type": "integer", "default": 1},
                        "page_size": {"type": "integer", "default": 100}
                    }
                }))
                .tags(&["entries"]),
            BlockEndpoint::post("/b/messages/api/contexts/{id}/entries")
                .summary("Add entry to context")
                .auth(AuthLevel::Authenticated)
                .input_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "kind": {"type": "string", "default": "message", "description": "Entry kind: message, artifact, notification, status"},
                        "role": {"type": "string", "default": "", "description": "Sender role: user, agent, system"},
                        "sender_id": {"type": "string", "default": ""},
                        "content": {"type": "string", "default": ""},
                        "content_type": {"type": "string", "default": "text/plain"},
                        "metadata": {"type": "object", "default": {}}
                    }
                }))
                .tags(&["entries"]),
            BlockEndpoint::get("/b/messages/api/entries/{id}")
                .summary("Get entry")
                .auth(AuthLevel::Authenticated)
                .tags(&["entries"]),
            BlockEndpoint::delete("/b/messages/api/entries/{id}")
                .summary("Delete entry")
                .auth(AuthLevel::Authenticated)
                .tags(&["entries"]),
        ])
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p solobase-core 2>&1 | head -20`

- [ ] **Step 3: Commit**

```bash
git add solobase/crates/solobase-core/src/blocks/messages/mod.rs
git commit -m "feat(messages): add JSON schemas to all context and entry endpoints"
```

---

### Task 9: End-to-End Smoke Test

**Files:** None (testing only)

- [ ] **Step 1: Build and run the server**

Run: `cargo build -p solobase 2>&1 | tail -10`

Expected: Clean build.

- [ ] **Step 2: Start the server and test REST endpoints**

Start the server, then test from another terminal:

```bash
# Create a context
curl -s -X POST http://localhost:8080/b/messages/api/contexts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"type":"conversation","title":"Test conversation"}' | jq .

# List contexts
curl -s http://localhost:8080/b/messages/api/contexts \
  -H "Authorization: Bearer <token>" | jq .

# Add an entry
curl -s -X POST http://localhost:8080/b/messages/api/contexts/<id>/entries \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"kind":"message","role":"user","content":"Hello world"}' | jq .

# List entries
curl -s http://localhost:8080/b/messages/api/contexts/<id>/entries \
  -H "Authorization: Bearer <token>" | jq .
```

- [ ] **Step 3: Test A2A endpoint**

```bash
# SendMessage
curl -s -X POST http://localhost:8080/a2a \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"jsonrpc":"2.0","method":"SendMessage","params":{"message":{"role":"user","parts":[{"text":"Hello from A2A"}]}},"id":"1"}' | jq .

# GetTask (use id from above response)
curl -s -X POST http://localhost:8080/a2a \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"jsonrpc":"2.0","method":"GetTask","params":{"id":"<task_id>"},"id":"2"}' | jq .

# ListTasks
curl -s -X POST http://localhost:8080/a2a \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"jsonrpc":"2.0","method":"ListTasks","params":{},"id":"3"}' | jq .
```

- [ ] **Step 4: Test discovery endpoints include new schemas**

```bash
curl -s http://localhost:8080/openapi.json | jq '.paths | keys'
curl -s http://localhost:8080/.well-known/agent.json | jq '.skills | length'
```

Expected: The OpenAPI doc includes paths for `/b/messages/api/contexts`, `/b/messages/api/contexts/{id}`, etc. The agent card includes skills for the messages block endpoints that have schemas.

- [ ] **Step 5: Test the admin UI**

Visit `http://localhost:8080/b/messages/` in a browser. Verify:
- Context list page renders with type/status badges
- "New Context" form works (create conversation/task/notification)
- Click into a context → detail page shows entries
- "Add Entry" form works with kind selector

- [ ] **Step 6: Commit any fixes**

```bash
git add -u
git commit -m "fix: resolve issues found during smoke testing"
```
