//! Block enablement — which blocks are active.
//!
//! Uses a generic HashMap approach backed by the `block_settings` table.
//! Each block's enabled/disabled state is keyed by full block name.

use std::collections::HashMap;

/// Trait for querying which solobase blocks are enabled.
pub trait FeatureConfig: wafer_run::MaybeSend + wafer_run::MaybeSync {
    /// Check if a block is enabled by its full name (e.g., "suppers-ai/products").
    fn is_block_enabled(&self, full_name: &str) -> bool;
}

/// Generic block settings backed by a HashMap.
/// Blocks default to enabled unless explicitly disabled.
pub struct BlockSettings {
    enabled: HashMap<String, bool>,
}

impl BlockSettings {
    pub fn from_map(enabled: HashMap<String, bool>) -> Self {
        Self { enabled }
    }

    /// Check if a block is enabled by its short name (e.g., "products").
    pub fn is_enabled(&self, short_name: &str) -> bool {
        let full = format!("suppers-ai/{short_name}");
        self.enabled.get(&full).copied().unwrap_or(true)
    }
}

impl FeatureConfig for BlockSettings {
    fn is_block_enabled(&self, full_name: &str) -> bool {
        self.enabled.get(full_name).copied().unwrap_or(true)
    }
}

/// All features enabled (for testing).
pub struct AllEnabled;

impl FeatureConfig for AllEnabled {
    fn is_block_enabled(&self, _: &str) -> bool { true }
}
