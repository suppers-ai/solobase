//! `BrowserLlmService` — populated in Task 6.

use crate::llm::catalog::{default_catalog, ModelCatalog};

pub struct BrowserLlmService {
    catalog: ModelCatalog,
}

impl BrowserLlmService {
    pub fn new() -> Self {
        Self { catalog: default_catalog() }
    }

    pub fn with_catalog(catalog: ModelCatalog) -> Self {
        Self { catalog }
    }

    #[allow(dead_code)]
    pub(crate) fn catalog(&self) -> &ModelCatalog {
        &self.catalog
    }
}

impl Default for BrowserLlmService {
    fn default() -> Self {
        Self::new()
    }
}
