//! Service layer for the messages block.
//!
//! Plain async functions — no HTTP awareness. Both REST and A2A handlers
//! call these. All database interactions live here.

use wafer_core::clients::{
    database as db,
    database::{Filter, FilterOp, ListOptions, SortField},
};
use wafer_run::{context::Context, WaferError};

use crate::blocks::helpers::{self, json_map};

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
    let metadata = metadata.unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

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

pub async fn get_context(ctx: &dyn Context, id: &str) -> Result<db::Record, WaferError> {
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
) -> Result<db::RecordList, String> {
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
) -> Result<db::Record, WaferError> {
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

pub async fn delete_context(ctx: &dyn Context, id: &str) -> Result<(), String> {
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
    let metadata = metadata.unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
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

pub async fn get_entry(ctx: &dyn Context, id: &str) -> Result<db::Record, WaferError> {
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
) -> Result<db::RecordList, String> {
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

pub async fn delete_entry(ctx: &dyn Context, id: &str) -> Result<(), WaferError> {
    db::delete(ctx, ENTRIES_COLLECTION, id).await
}
