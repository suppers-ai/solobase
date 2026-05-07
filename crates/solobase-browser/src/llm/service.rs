//! `BrowserLlmService` — `wafer_core::LlmService` impl backed by WebLLM
//! running in the page via the SW↔page postMessage bridge.

use std::{cell::RefCell, rc::Rc};

use futures::{channel::mpsc, sink::SinkExt, stream::BoxStream};
use tokio_util::sync::CancellationToken;
use wafer_core::interfaces::llm::service::{
    ChatChunk, ChatRequest, LlmError, LlmService, LoadProgress, ModelInfo, ModelStatus,
};

use crate::llm::{
    bridge::{self, StreamFrame},
    catalog::{default_catalog, ModelCatalog},
    openai_codec::{encode_request_body, StreamingDecoder},
};

const WEBLLM_BACKEND: &str = "webllm";

/// `LlmService` impl backed by WebLLM running in the page.
///
/// Single-engine — WebLLM keeps one model in memory at a time. The loaded
/// model id is tracked via `Rc<RefCell<Option<String>>>` so spawn_local
/// tasks (which require `'static` futures) can share it.
pub struct BrowserLlmService {
    catalog: ModelCatalog,
    loaded_model: Rc<RefCell<Option<String>>>,
}

impl BrowserLlmService {
    pub fn new() -> Self {
        Self {
            catalog: default_catalog(),
            loaded_model: Rc::new(RefCell::new(None)),
        }
    }

    pub fn with_catalog(catalog: ModelCatalog) -> Self {
        Self {
            catalog,
            loaded_model: Rc::new(RefCell::new(None)),
        }
    }
}

impl Default for BrowserLlmService {
    fn default() -> Self {
        Self::new()
    }
}

fn one_shot_err<T: 'static>(err: LlmError) -> BoxStream<'static, Result<T, LlmError>> {
    Box::pin(futures::stream::once(async move { Err(err) }))
}

#[async_trait::async_trait(?Send)]
impl LlmService for BrowserLlmService {
    async fn chat_stream(
        &self,
        req: ChatRequest,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<ChatChunk, LlmError>> {
        if !self.claims_backend(&req.backend_id) {
            return one_shot_err(LlmError::BackendError(format!(
                "backend '{}' not claimed by webllm",
                req.backend_id
            )));
        }

        let body_json = match encode_request_body(&req.messages, &req.tools) {
            Ok(s) => s,
            Err(e) => return one_shot_err(e),
        };

        let (mut tx, rx) = mpsc::channel::<Result<ChatChunk, LlmError>>(16);

        wasm_bindgen_futures::spawn_local(async move {
            let stream_id = match bridge::start_chat_stream(&body_json).await {
                Ok(id) => id,
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
            };

            let mut decoder = StreamingDecoder::new();
            loop {
                if cancel.is_cancelled() {
                    let _ = bridge::cancel_stream(&stream_id).await;
                    break;
                }
                let frame = match bridge::next_chunk(&stream_id).await {
                    Ok(f) => f,
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                };
                match frame {
                    StreamFrame::Chunk(s) => match decoder.feed(&s) {
                        Ok(chunks) => {
                            for chunk in chunks {
                                if tx.send(Ok(chunk)).await.is_err() {
                                    // consumer dropped
                                    return;
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e)).await;
                            break;
                        }
                    },
                    StreamFrame::Progress(_) => {
                        // Progress frames belong to create-engine streams, not chat.
                        // The page-side handlers don't cross-emit, so this branch
                        // only fires on a wire bug — surface as a backend error so
                        // we don't silently drop frames.
                        let _ = tx
                            .send(Err(LlmError::BackendError(
                                "webllm: unexpected progress frame on chat stream".into(),
                            )))
                            .await;
                        break;
                    }
                    StreamFrame::Done => break,
                    StreamFrame::Error(msg) => {
                        let _ = tx
                            .send(Err(LlmError::BackendError(format!("webllm: {msg}"))))
                            .await;
                        break;
                    }
                }
            }
        });

        Box::pin(rx)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        Ok(self.catalog.models().to_vec())
    }

    async fn status(&self, backend_id: &str, model_id: &str) -> Result<ModelStatus, LlmError> {
        if backend_id != WEBLLM_BACKEND {
            return Err(LlmError::BackendError(format!(
                "backend '{backend_id}' not claimed by webllm"
            )));
        }
        let borrow = self.loaded_model.borrow();
        match borrow.as_deref() {
            Some(m) if m == model_id => Ok(ModelStatus::ready()),
            _ => Ok(ModelStatus::unloaded()),
        }
    }

    fn load_model(
        &self,
        backend_id: &str,
        model_id: &str,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<LoadProgress, LlmError>> {
        if backend_id != WEBLLM_BACKEND {
            return Box::pin(futures::stream::once({
                let backend_id = backend_id.to_string();
                async move {
                    Err(LlmError::BackendError(format!(
                        "backend '{backend_id}' not claimed by webllm"
                    )))
                }
            }));
        }
        // Channel buffer must be large enough that a burst of progress frames
        // from WebLLM (1/sec during shard fetch) doesn't backpressure the
        // spawn_local task. 32 is comfortably above WebLLM's tick rate.
        let (mut tx, rx) = mpsc::channel::<Result<LoadProgress, LlmError>>(32);
        let loaded_model = Rc::clone(&self.loaded_model);
        let model_id = model_id.to_string();

        wasm_bindgen_futures::spawn_local(async move {
            if cancel.is_cancelled() {
                return;
            }
            // Start the page-side load. The returned stream will emit one
            // `Progress` frame per WebLLM `initProgressCallback` tick (~1/sec
            // during a cold download) and terminate with `Done` or `Error`.
            // This is what keeps the SW fetch handler producing SSE bytes —
            // a one-shot await over a multi-minute download lets Chrome's
            // idle keep-alive drop the request mid-flight.
            let stream_id = match bridge::start_create_engine_stream(&model_id).await {
                Ok(id) => id,
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
            };

            loop {
                if cancel.is_cancelled() {
                    // WebLLM has no cancel-load primitive — the page-side
                    // CreateMLCEngine call will run to completion. We
                    // best-effort unload after the fact so the page doesn't
                    // sit on an engine nobody asked for. Errors here are
                    // intentionally swallowed.
                    let _ = bridge::cancel_stream(&stream_id).await;
                    let _ = bridge::unload_engine(&model_id).await;
                    return;
                }
                let frame = match bridge::next_chunk(&stream_id).await {
                    Ok(f) => f,
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        return;
                    }
                };
                match frame {
                    StreamFrame::Progress(stage) => {
                        if tx.send(Ok(LoadProgress::new(stage))).await.is_err() {
                            // Consumer dropped — best-effort unload (the
                            // download is still in flight on the page).
                            let _ = bridge::cancel_stream(&stream_id).await;
                            let _ = bridge::unload_engine(&model_id).await;
                            return;
                        }
                    }
                    StreamFrame::Done => {
                        *loaded_model.borrow_mut() = Some(model_id.clone());
                        let _ = tx.send(Ok(LoadProgress::new("ready"))).await;
                        return;
                    }
                    StreamFrame::Error(msg) => {
                        let _ = tx
                            .send(Err(LlmError::BackendError(format!("webllm: {msg}"))))
                            .await;
                        return;
                    }
                    StreamFrame::Chunk(_) => {
                        // Chunk frames belong to chat streams. As with the
                        // mirrored arm in `chat_stream`, a wire bug is the
                        // only way to land here — surface as a backend error.
                        let _ = tx
                            .send(Err(LlmError::BackendError(
                                "webllm: unexpected chunk frame on create-engine stream".into(),
                            )))
                            .await;
                        return;
                    }
                }
            }
        });
        Box::pin(rx)
    }

    async fn unload_model(&self, backend_id: &str, model_id: &str) -> Result<(), LlmError> {
        if backend_id != WEBLLM_BACKEND {
            return Err(LlmError::BackendError(format!(
                "backend '{backend_id}' not claimed by webllm"
            )));
        }
        bridge::unload_engine(model_id).await?;
        *self.loaded_model.borrow_mut() = None;
        Ok(())
    }

    fn claims_backend(&self, backend_id: &str) -> bool {
        backend_id == WEBLLM_BACKEND
    }
}
