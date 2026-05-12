//! Default image-model catalog for the browser backend.

use wafer_core::interfaces::image::service::{ModelCapabilities, ModelInfo};

pub const BACKEND_ID: &str = "transformers-image";

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
        Self::new(default_models())
    }
}

pub fn default_catalog() -> ModelCatalog {
    ModelCatalog::default()
}

fn sd_turbo_caps() -> ModelCapabilities {
    // `ModelCapabilities` is `#[non_exhaustive]`; construct via Default + field assignment.
    let mut c = ModelCapabilities::default();
    c.max_width = Some(512);
    c.max_height = Some(512);
    c.supports_negative_prompt = true;
    c.max_steps = Some(4);
    c
}

fn default_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(BACKEND_ID, "Xenova/sd-turbo", "SD-Turbo (≈500 MB)")
            .with_capabilities(sd_turbo_caps()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_catalog_has_sd_turbo() {
        let cat = default_catalog();
        assert_eq!(cat.models().len(), 1);
        assert_eq!(cat.models()[0].model_id, "Xenova/sd-turbo");
        assert_eq!(cat.models()[0].backend_id, BACKEND_ID);
        assert_eq!(cat.models()[0].capabilities.max_width, Some(512));
        assert!(cat.models()[0].capabilities.supports_negative_prompt);
    }
}
