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

/// Per-block runtime state stored in `suppers_ai__admin__block_settings`.
/// Both `enabled` and `migration` live on the same row, loaded together by
/// the per-isolate cache.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BlockState {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub migration: MigrationState,
}

/// Hashes that gate `migration_helper::apply_if_blessed`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MigrationState {
    /// SHA-256 hex of the SQL bytes that have been applied. Empty = never.
    #[serde(default)]
    pub current_hash: String,
    /// SHA-256 hex of the SQL bytes the operator has blessed. Empty = never.
    #[serde(default)]
    pub blessed_hash: String,
}

fn default_true() -> bool {
    true
}

/// Generic block settings backed by a HashMap.
/// Blocks default to enabled unless explicitly disabled.
#[derive(Clone, Debug, Default)]
pub struct BlockSettings {
    blocks: HashMap<String, BlockState>,
}

impl BlockSettings {
    pub fn from_blocks(blocks: HashMap<String, BlockState>) -> Self {
        Self { blocks }
    }

    /// Construct from a legacy `enabled` map. Migration state defaults to empty.
    /// Used by callers that haven't been updated to the new shape yet.
    pub fn from_map(enabled: HashMap<String, bool>) -> Self {
        let blocks = enabled
            .into_iter()
            .map(|(name, enabled)| {
                (
                    name,
                    BlockState {
                        enabled,
                        migration: MigrationState::default(),
                    },
                )
            })
            .collect();
        Self { blocks }
    }

    /// Check if a block is enabled by its short name (e.g., "products").
    pub fn is_enabled(&self, short_name: &str) -> bool {
        let full = format!("suppers-ai/{short_name}");
        self.blocks.get(&full).map(|s| s.enabled).unwrap_or(true)
    }

    /// Look up the full `BlockState` for a block by full name.
    /// Returns a default (enabled + empty migration state) when the block has
    /// no row in `block_settings` yet.
    pub fn state(&self, full_name: &str) -> BlockState {
        self.blocks
            .get(full_name)
            .cloned()
            .unwrap_or_else(|| BlockState {
                enabled: true,
                migration: MigrationState::default(),
            })
    }

    /// Serialize all block state to JSON for transport through the wafer
    /// config snapshot under [`BLOCK_SETTINGS_CONFIG_KEY`]. Empty map → `"{}"`.
    pub fn to_config_json(&self) -> String {
        serde_json::to_string(&self.blocks).unwrap_or_else(|_| "{}".to_string())
    }

    /// Parse a `BlockSettings` from the JSON shape produced by
    /// [`Self::to_config_json`]. Falls back to empty on parse error so
    /// `is_block_enabled` retains "default enabled" semantics.
    pub fn from_config_json(json: &str) -> Self {
        let blocks: HashMap<String, BlockState> = serde_json::from_str(json).unwrap_or_default();
        Self::from_blocks(blocks)
    }
}

impl FeatureConfig for BlockSettings {
    fn is_block_enabled(&self, full_name: &str) -> bool {
        self.blocks
            .get(full_name)
            .map(|s| s.enabled)
            .unwrap_or(true)
    }
}

/// All features enabled (for testing).
pub struct AllEnabled;

impl FeatureConfig for AllEnabled {
    fn is_block_enabled(&self, _: &str) -> bool {
        true
    }
}
