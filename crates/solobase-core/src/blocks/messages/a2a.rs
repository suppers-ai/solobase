//! A2A JSON-RPC handler for the messages block.
//!
//! Handles `POST /a2a` — dispatches by JSON-RPC method field.
//! Maps A2A Task/Message/Artifact concepts to internal contexts/entries.

use wafer_core::clients::database as db;
use wafer_run::{context::Context, InputStream, OutputStream, ErrorCode, Message};

use super::service::{self, ListContextsParams, ListEntriesParams};
use crate::blocks::helpers::{ok_json, RecordExt};

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

fn jsonrpc_error(id: Option<serde_json::Value>, code: i64, message: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": code,
            "message": message,
        },
        "id": id,
    })
}

pub async fn handle_a2a(ctx: &dyn Context, _msg: Message, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let req: JsonRpcRequest = match serde_json::from_slice(&raw) {
        Ok(r) => r,
        Err(e) => {
            let body = jsonrpc_error(None, -32700, &format!("Parse error: {e}"));
            return ok_json(&body);
        }
    };

    if req.jsonrpc != "2.0" {
        let body = jsonrpc_error(req.id, -32600, "Invalid JSON-RPC version");
        return ok_json(&body);
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
    ok_json(&body)
}

// ---------- Typed per-method params ----------
//
// JSON-RPC dispatch keeps the raw `params: Option<Value>`, but every method
// handler decodes into its own typed struct rather than fishing fields out
// of the `Value` by string keys. Saves the per-call `.get(...).and_then(...)`
// dance and surfaces a missing/wrong-typed field as `-32602 Invalid params`
// in one place per method.

#[derive(serde::Deserialize)]
struct SendMessageParams {
    message: A2aMessage,
    #[serde(rename = "contextId", default)]
    context_id: Option<String>,
}

#[derive(serde::Deserialize)]
struct A2aMessage {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    parts: Option<Vec<A2aMessagePart>>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct A2aMessagePart {
    #[serde(default)]
    text: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct GetTaskParams {
    id: String,
    #[serde(rename = "historyLength", default)]
    history_length: Option<i64>,
}

#[derive(serde::Deserialize, Default)]
struct ListTasksParams {
    #[serde(default)]
    status: Option<String>,
    #[serde(rename = "contextId", default)]
    context_id: Option<String>,
    #[serde(rename = "pageSize", default)]
    page_size: Option<i64>,
}

#[derive(serde::Deserialize)]
struct CancelTaskParams {
    id: String,
}

fn parse_params<T: serde::de::DeserializeOwned>(
    params: &serde_json::Value,
) -> Result<T, (i64, String)> {
    // `-32602` is the JSON-RPC code for "Invalid params". We use it for both
    // missing fields and type-mismatched fields — callers care that their
    // request was rejected, not the precise serde error class.
    serde_json::from_value(params.clone()).map_err(|e| (-32602, format!("Invalid params: {e}")))
}

async fn handle_send_message(
    ctx: &dyn Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, (i64, String)> {
    let SendMessageParams {
        message,
        context_id,
    } = parse_params(params)?;

    let role = message.role.as_deref().unwrap_or("user");
    let parts = message.parts.unwrap_or_default();
    let content = parts
        .iter()
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n");

    let task_context = if let Some(cid) = context_id.as_deref() {
        match service::get_context(ctx, cid).await {
            Ok(record) => record,
            Err(e) if e.code == ErrorCode::NotFound => {
                return Err((-32001, format!("Task not found: {cid}")));
            }
            Err(e) => return Err((-32000, format!("Database error: {e}"))),
        }
    } else {
        let title: String = parts
            .first()
            .and_then(|p| p.text.as_deref())
            .unwrap_or("A2A Task")
            .chars()
            .take(100)
            .collect();

        service::create_context(ctx, "task", &title, "", "", None, None)
            .await
            .map_err(|e| (-32000, e))?
    };

    let parts_meta = if parts.is_empty() {
        None
    } else {
        Some(serde_json::json!({ "parts": parts }))
    };
    service::add_entry(
        ctx,
        &task_context.id,
        "message",
        role,
        "",
        &content,
        Some("text/plain"),
        parts_meta,
    )
    .await
    .map_err(|e| (-32000, e))?;

    if context_id.is_none() {
        let mut updates = std::collections::HashMap::new();
        updates.insert("status".to_string(), serde_json::json!("submitted"));
        let _ = service::update_context(ctx, &task_context.id, updates).await;
    }

    build_task_response(ctx, &task_context.id).await
}

async fn handle_get_task(
    ctx: &dyn Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, (i64, String)> {
    let GetTaskParams { id, history_length } = parse_params(params)?;
    build_task_response_with_history(ctx, &id, history_length).await
}

async fn handle_list_tasks(
    ctx: &dyn Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, (i64, String)> {
    let ListTasksParams {
        status,
        context_id,
        page_size,
    } = parse_params(params)?;
    let page_size = page_size.unwrap_or(50).min(100);

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

    let total_count = result.total_count;
    // Consume the records — each task carries metadata/parent_id we'd
    // otherwise clone out of a borrowed Record.
    let tasks: Vec<serde_json::Value> = result.records.into_iter().map(context_to_task).collect();

    Ok(serde_json::json!({
        "tasks": tasks,
        "totalSize": total_count,
    }))
}

async fn handle_cancel_task(
    ctx: &dyn Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, (i64, String)> {
    let CancelTaskParams { id } = parse_params(params)?;

    let context = service::get_context(ctx, &id).await.map_err(|e| {
        if e.code == ErrorCode::NotFound {
            (-32001, format!("Task not found: {id}"))
        } else {
            (-32000, format!("Database error: {e}"))
        }
    })?;

    let current_status = context.str_field("status");
    let terminal = ["completed", "failed", "canceled", "rejected"];
    if terminal.contains(&current_status) {
        return Err((
            -32002,
            format!("Task is already in terminal state: {current_status}"),
        ));
    }

    let mut updates = std::collections::HashMap::new();
    updates.insert("status".to_string(), serde_json::json!("canceled"));
    service::update_context(ctx, &id, updates)
        .await
        .map_err(|e| (-32000, format!("Database error: {e}")))?;

    build_task_response(ctx, &id).await
}

fn context_to_task(mut record: db::Record) -> serde_json::Value {
    let status = record.str_field("status").to_string();
    let updated_at = record.str_field("updated_at").to_string();
    let parent_id = record
        .data
        .remove("parent_id")
        .unwrap_or(serde_json::Value::Null);
    let metadata = record
        .data
        .remove("metadata")
        .unwrap_or(serde_json::json!({}));
    serde_json::json!({
        "id": record.id,
        "status": {
            "state": status,
            "timestamp": updated_at,
        },
        "contextId": parent_id,
        "metadata": metadata,
    })
}

fn entry_to_message(mut record: db::Record) -> serde_json::Value {
    let role = record.str_field("role").to_string();
    let content = record.str_field("content").to_string();
    let metadata = record
        .data
        .remove("metadata")
        .unwrap_or(serde_json::json!({}));
    serde_json::json!({
        "role": role,
        "parts": [{"text": content}],
        "metadata": metadata,
    })
}

fn entry_to_artifact(mut record: db::Record) -> serde_json::Value {
    let content = record.str_field("content").to_string();
    let content_type = record.str_field("content_type").to_string();
    let metadata = record
        .data
        .remove("metadata")
        .unwrap_or(serde_json::json!({}));
    serde_json::json!({
        "id": record.id,
        "mimeType": content_type,
        "parts": [{"text": content}],
        "metadata": metadata,
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
    let context = service::get_context(ctx, context_id).await.map_err(|e| {
        if e.code == ErrorCode::NotFound {
            (-32001, format!("Task not found: {context_id}"))
        } else {
            (-32000, format!("Database error: {e}"))
        }
    })?;

    let mut task = context_to_task(context);

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

            for entry in entries.records {
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
