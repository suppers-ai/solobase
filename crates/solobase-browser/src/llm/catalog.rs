//! Default browser model catalog. Populated in Task 3.

use wafer_core::interfaces::llm::service::ModelInfo;

/// Browser-side model catalog. Wraps a `Vec<ModelInfo>` and can be
/// overridden by consumers via `BrowserLlmService::with_catalog`.
#[derive(Debug, Clone)]
pub struct ModelCatalog {
    models: Vec<ModelInfo>,
}

impl ModelCatalog {
    pub fn new(models: Vec<ModelInfo>) -> Self {
        Self { models }
    }
    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }
}

impl Default for ModelCatalog {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

pub fn default_catalog() -> ModelCatalog {
    ModelCatalog::new(Vec::new())
}
