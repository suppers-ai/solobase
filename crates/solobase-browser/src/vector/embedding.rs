//! `BrowserEmbeddingService` — Transformers.js feature-extraction pipeline,
//! invoked over the SW↔page postMessage bridge.

use wafer_core::interfaces::vector::{
    catalog::get_model,
    service::{EmbeddingService, Result as VResult, VectorError},
};

use crate::vector::embedding_bridge;

pub struct BrowserEmbeddingService {
    model_id: String,
    dimensions: u32,
}

unsafe impl Send for BrowserEmbeddingService {}
unsafe impl Sync for BrowserEmbeddingService {}

impl BrowserEmbeddingService {
    pub fn new() -> Result<Self, String> {
        Self::with_model("multilingual-e5-small")
    }

    pub fn with_model(model_id: &str) -> Result<Self, String> {
        let info = get_model(model_id).ok_or_else(|| format!("unknown model: {model_id}"))?;
        if !info.runtimes.browser_transformers {
            return Err(format!(
                "model '{model_id}' is not browser-runnable (browser_transformers=false)"
            ));
        }
        Ok(Self {
            model_id: model_id.to_string(),
            dimensions: info.dimensions,
        })
    }
}

impl Default for BrowserEmbeddingService {
    fn default() -> Self {
        Self::new().expect("default model is always valid")
    }
}

#[async_trait::async_trait(?Send)]
impl EmbeddingService for BrowserEmbeddingService {
    fn model(&self) -> &str {
        &self.model_id
    }

    fn dimensions(&self) -> u32 {
        self.dimensions
    }

    async fn embed(&self, texts: Vec<String>) -> VResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let vectors = embedding_bridge::run(&self.model_id, &texts).await?;
        for (i, v) in vectors.iter().enumerate() {
            if v.len() as u32 != self.dimensions {
                return Err(VectorError::Internal(format!(
                    "embed[{}]: model returned {} dims, expected {}",
                    i,
                    v.len(),
                    self.dimensions
                )));
            }
        }
        Ok(vectors)
    }
}
