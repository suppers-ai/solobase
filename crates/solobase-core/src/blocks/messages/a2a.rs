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

    let content = extract_text_from_parts(message);

    let context_id = params
        .get("contextId")
        .and_then(|c| c.as_str());

    let task_context = if let Some(cid) = context_id {
        match service::get_context(ctx, cid).await {
            Ok(record) => record,
            Err(e) if e.code == ErrorCode::NotFound => {
                return Err((-32001, format!("Task not found: {cid}")));
            }
            Err(e) => return Err((-32000, format!("Database error: {e}"))),
        }
    } else {
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
        .map(context_to_task)
        .collect();

    Ok(serde_json::json!({
        "tasks": tasks,
        "totalSize": result.total_count,
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
