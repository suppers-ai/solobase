//! Rust-side wrapper over the SW↔page LLM postMessage bridge.
//!
//! Glue around the `llm*` functions in `bridge.js`. Turns the async
//! postMessage exchanges into typed Rust calls. Not testable in native —
//! exercised via the `BrowserLlmService` integration and the browser smoke
//! test.

use wafer_core::interfaces::llm::service::LlmError;

use crate::bridge::{
    llm_cancel_stream, llm_chat_stream, llm_create_engine_stream, llm_next_stream_frame,
    llm_unload_engine,
};

fn js_err(e: wasm_bindgen::JsValue) -> LlmError {
    LlmError::BackendError(format!(
        "webllm bridge: {}",
        e.as_string().unwrap_or_else(|| format!("{e:?}"))
    ))
}

/// Start an LLM engine load. Returns the stream id; pump with `next_chunk`
/// to receive `Progress` frames during the cold download and a terminal
/// `Done` (success) or `Error`.
pub async fn start_create_engine_stream(model_id: &str) -> Result<String, LlmError> {
    let v = llm_create_engine_stream(model_id).await.map_err(js_err)?;
    v.as_string()
        .ok_or_else(|| LlmError::BackendError("webllm bridge: stream id not a string".into()))
}

pub async fn unload_engine(model_id: &str) -> Result<(), LlmError> {
    llm_unload_engine(model_id)
        .await
        .map(|_| ())
        .map_err(js_err)
}

pub async fn start_chat_stream(body_json: &str) -> Result<String, LlmError> {
    let v = llm_chat_stream(body_json).await.map_err(js_err)?;
    v.as_string()
        .ok_or_else(|| LlmError::BackendError("webllm bridge: stream id not a string".into()))
}

/// One frame pulled from the page-side stream. Chat streams emit `Chunk`
/// (OpenAI chunk JSON) frames; create-engine streams emit `Progress` (stage
/// text) frames. Both terminate with `Done` or `Error`.
pub enum StreamFrame {
    /// OpenAI chunk JSON string (chat). Pass to `openai_codec::StreamingDecoder::feed`.
    Chunk(String),
    /// Free-form stage text (create-engine). Surface as `LoadProgress::stage`.
    Progress(String),
    Done,
    Error(String),
}

pub async fn next_chunk(stream_id: &str) -> Result<StreamFrame, LlmError> {
    let v = llm_next_stream_frame(stream_id).await.map_err(js_err)?;
    let s = v
        .as_string()
        .ok_or_else(|| LlmError::BackendError("webllm bridge: frame not a string".into()))?;
    let frame: serde_json::Value = serde_json::from_str(&s)
        .map_err(|e| LlmError::BackendError(format!("webllm bridge: frame parse: {e}")))?;
    let kind = frame.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    let payload = || {
        frame
            .get("payload")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    match kind {
        "chunk" => Ok(StreamFrame::Chunk(payload())),
        "progress" => Ok(StreamFrame::Progress(payload())),
        "done" => Ok(StreamFrame::Done),
        "error" => Ok(StreamFrame::Error(if payload().is_empty() {
            "unknown".to_string()
        } else {
            payload()
        })),
        other => Err(LlmError::BackendError(format!(
            "webllm bridge: unknown frame kind '{other}'"
        ))),
    }
}

pub async fn cancel_stream(stream_id: &str) -> Result<(), LlmError> {
    llm_cancel_stream(stream_id)
        .await
        .map(|_| ())
        .map_err(js_err)
}
