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
/// Both `enabled`, `migration`, and `seed_defaults_hash` live on the same
/// row, loaded together by the per-isolate cache.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BlockState {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub migration: MigrationState,
    /// SHA-256 hex of the deterministic seed payload last applied by
    /// `admin::settings::seed_defaults`. Empty = never seeded (or pre-PR3
    /// row). When this matches the current hash of `shared_config_vars()`,
    /// `seed_defaults` short-circuits before issuing any D1 query. Only the
    /// `suppers-ai/admin` row uses this field today — other blocks leave
    /// it empty.
    #[serde(default)]
    pub seed_defaults_hash: String,
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
                        seed_defaults_hash: String::new(),
                    },
                )
            })
            .collect();
        Self { blocks }
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
                seed_defaults_hash: String::new(),
            })
    }

    /// Serialize all block state to JSON for transport through the wafer
    /// config snapshot under [`BLOCK_SETTINGS_CONFIG_KEY`]. Empty map → `"{}"`.
    ///
    /// # Panics
    ///
    /// `HashMap<String, BlockState>` has no custom `Serialize` impl that can
    /// fail (the field types are `String` / `bool`), so an error here
    /// indicates either OOM during string growth or a future schema change
    /// that broke this invariant — both should be loud rather than silently
    /// emitting `"{}"` and losing migration-gate state on transport.
    pub fn to_config_json(&self) -> String {
        serde_json::to_string(&self.blocks)
            .expect("BlockState serialization is infallible for current schema")
    }

    /// Parse a `BlockSettings` from the JSON shape produced by
    /// [`Self::to_config_json`]. Falls back to empty on parse error so
    /// `is_block_enabled` retains "default enabled" semantics.
    pub fn from_config_json(json: &str) -> Self {
        let blocks: HashMap<String, BlockState> = serde_json::from_str(json).unwrap_or_default();
        Self::from_blocks(blocks)
    }

    /// Look up a single block's [`BlockState`] from the JSON produced by
    /// [`Self::to_config_json`] without materializing every entry.
    ///
    /// Used by `migration_helper::apply_if_blessed` — called once per block
    /// per startup, on a payload that grows linearly with installed blocks.
    /// Walks the JSON's top-level object until the key matches, then
    /// deserializes only that entry. Returns the default when the key is
    /// absent or the JSON is malformed (same "default enabled" semantics
    /// as `from_config_json`).
    pub fn state_for(json: &str, block_name: &str) -> BlockState {
        let missing = || BlockState {
            enabled: true,
            migration: MigrationState::default(),
            seed_defaults_hash: String::new(),
        };
        let value: serde_json::Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(_) => return missing(),
        };
        let Some(obj) = value.as_object() else {
            return missing();
        };
        let Some(entry) = obj.get(block_name) else {
            return missing();
        };
        serde_json::from_value(entry.clone()).unwrap_or_else(|_| missing())
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

/// `FeatureConfig` for the shared+mutable form `Arc<RwLock<BlockSettings>>`.
///
/// `SolobaseBuilder` stores its block_settings as `Arc<RwLock<BlockSettings>>`
/// so consumers can mutate the live snapshot post-build (the OPFS-backed
/// `solobase-web` flow needs this — it can't load block_settings until after
/// `init_block(admin)` has created the `suppers_ai__admin__block_settings`
/// table, but the runtime's `FeatureConfig` Arc has to exist at build time).
/// The router holds an `Arc<dyn FeatureConfig>` cloned off the same lock, so
/// post-`build()` writes are visible on subsequent route checks without any
/// SolobaseBuilder API gymnastics.
///
/// `read()` panics only if a previous holder panicked while holding the
/// write lock, which would leave the snapshot in an indeterminate state —
/// surfacing that immediately is preferable to handing the router a stale
/// "all-enabled" fallback.
impl FeatureConfig for std::sync::RwLock<BlockSettings> {
    fn is_block_enabled(&self, full_name: &str) -> bool {
        self.read()
            .expect("BlockSettings RwLock poisoned")
            .is_block_enabled(full_name)
    }
}

/// Canonical defaults for `suppers_ai__admin__block_settings.enabled`.
/// Consumed by [`plan_seed_decisions`] on every cold start.
///
/// Adding a block here: bump the list, ship — every existing row gets the
/// INSERT path (no row yet → write the new default at the current hash).
///
/// Changing an existing default: just edit the bool — the hash gate detects
/// the change and re-seeds rows still at the old default. Admin-UI edits
/// (marked [`USER_EDITED_SENTINEL`]) are preserved.
///
/// Excluded for now: `suppers-ai/llm` and `suppers-ai/vector`. The LLM
/// block module is gated on `feature = "llm"` (wasm32-incompatible) so
/// the router would dispatch into a void on wasm32 if either was enabled
/// here. Restored when the LlmService trait refactor lands.
pub const ENABLED_DEFAULTS: &[(&str, bool)] = &[
    ("suppers-ai/auth", true),
    ("suppers-ai/admin", true),
    ("suppers-ai/files", true),
    ("suppers-ai/legalpages", true),
    ("suppers-ai/messages", true),
    ("suppers-ai/products", true),
    ("suppers-ai/system", true),
    ("suppers-ai/userportal", true),
];

/// Stored in `seed_defaults_hash` to mark a row that was last written by
/// the admin UI's toggle. Such rows are never overwritten by the seed.
pub const USER_EDITED_SENTINEL: &str = "user-edited";

/// Compute the canonical `"seed:<sha256_hex>"` marker for a default value.
///
/// The body of the hash is `sha256_hex(b"true")` or `sha256_hex(b"false")`
/// — short, deterministic, and stable across builds. The `"seed:"` prefix
/// distinguishes seed-managed rows from admin-managed rows
/// ([`USER_EDITED_SENTINEL`]) and from legacy empty-string state.
pub fn seed_hash_for(default: bool) -> String {
    let hex = crate::migration_helper::sha256_hex_bytes(default.to_string().as_bytes());
    format!("seed:{hex}")
}

/// One row of `suppers_ai__admin__block_settings` as seen by the seed
/// planner. Decoupled from `BlockState` so the planner stays pure (no
/// migration-helper dependency, no FeatureConfig trait conversion).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingRow {
    pub enabled: bool,
    pub hash: String,
}

/// What the planner decided about a given block name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedDecision {
    /// Static block name from [`ENABLED_DEFAULTS`].
    pub block_name: &'static str,
    /// Value to write.
    pub enabled: bool,
    /// `seed_defaults_hash` value to write (always `"seed:<hex>"`).
    pub hash: String,
    pub op: SeedOp,
}

/// INSERT vs UPDATE. Lets the caller pick the right SQL shape (some
/// callers can collapse both into a single UPSERT statement; others
/// prefer two distinct paths for logging clarity).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedOp {
    Insert,
    Update,
}

/// All features enabled (for testing).
pub struct AllEnabled;

impl FeatureConfig for AllEnabled {
    fn is_block_enabled(&self, _: &str) -> bool {
        true
    }
}
