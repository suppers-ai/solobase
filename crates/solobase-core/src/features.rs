//! Block enablement — which blocks are active.
//!
//! Uses a generic HashMap approach backed by the `block_settings` table.
//! Each block's enabled/disabled state is keyed by full block name.

use std::collections::HashMap;

/// Synthetic config key carrying the block_settings JSON map.
///
/// At boot, loaders fan-out the loaded `BlockSettings` into the wafer config
/// snapshot under this key (in addition to the existing `FeatureConfig` Arc
/// handed to `SolobaseRouterBlock`). Blocks that need to query enablement
/// state without re-reading D1 read this key via `ctx.config_get` and parse
/// the JSON. Double-underscore brackets mark the key as internal — it is
/// never set via env var or the variables table.
pub const BLOCK_SETTINGS_CONFIG_KEY: &str = "__SOLOBASE_BLOCK_SETTINGS_JSON__";

/// Trait for querying which solobase blocks are enabled.
pub trait FeatureConfig: wafer_run::MaybeSend + wafer_run::MaybeSync {
    /// Check if a block is enabled by its full name (e.g., "suppers-ai/products").
    fn is_block_enabled(&self, full_name: &str) -> bool;
}

/// Generic block settings backed by a HashMap.
/// Blocks default to enabled unless explicitly disabled.
#[derive(Clone)]
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

    /// Serialize the enabled map to a JSON string for transport through the
    /// wafer config snapshot under [`BLOCK_SETTINGS_CONFIG_KEY`]. Empty map
    /// serializes to `"{}"`.
    pub fn to_config_json(&self) -> String {
        serde_json::to_string(&self.enabled).unwrap_or_else(|_| "{}".to_string())
    }

    /// Parse a `BlockSettings` from the JSON shape produced by
    /// [`Self::to_config_json`]. Falls back to an empty map on parse error so
    /// `is_block_enabled` retains "default enabled" semantics.
    pub fn from_config_json(json: &str) -> Self {
        let map = serde_json::from_str(json).unwrap_or_default();
        Self::from_map(map)
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
    fn is_block_enabled(&self, _: &str) -> bool {
        true
    }
}
