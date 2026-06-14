pub mod admin;
pub mod auth;
pub mod auth_ui;
pub mod crud;
pub mod email;
pub mod errors;
#[macro_use]
pub mod feature_block;
// `native-embedding` always implies `block-fastembed` (see Cargo.toml), so
// the native build still gets this module. wafer-site / wasm32 builds with
// neither feature drop the ONNX-runtime dep entirely.
#[cfg(feature = "block-fastembed")]
pub mod fastembed;
#[cfg(feature = "block-files")]
pub mod files;
pub mod helpers;
#[cfg(feature = "block-legalpages")]
pub mod legalpages;
// The LLM feature block compiles on every target that enables `block-llm`,
// including wasm32. `LlmBlock` holds `Arc<dyn ProviderAdmin>` (the
// provider-management seam), not the concrete reqwest/tokio
// `ProviderLlmService`, so the block module no longer drags the native HTTP
// stack. The `llm` feature is now just "native provider backend": it gates
// `providers::ProviderLlmService` (reqwest/stream + tokio) and is implied by
// nothing the block module itself needs. Browser/CF builds enable `block-llm`
// without `llm` and supply their own backend via
// `SolobaseBuilder::llm_service` (e.g. `BrowserLlmService` in solobase-web);
// the block holds a `NoopProviderAdmin` there.
#[cfg(feature = "block-llm")]
pub mod llm;
#[cfg(feature = "block-messages")]
pub mod messages;
#[cfg(feature = "block-products")]
pub mod products;
pub mod rate_limit;
pub mod router;
pub mod storage;
pub mod system;
#[cfg(target_arch = "wasm32")]
pub mod transformers_embed;
#[cfg(feature = "block-userportal")]
pub mod userportal;
#[cfg(feature = "block-vector")]
pub mod vector;

/// The single `(feature-cfg, name, constructor)` manifest of solobase feature
/// blocks whose constructors take **no arguments** (every `suppers-ai/*` block
/// except the three special cases below).
///
/// This macro is the one place enumerating that block set. It generates, from
/// the same entries:
///
/// - [`all_block_infos`] — `.info()` over every entry (config-var discovery,
///   inspector route granularity, the route/auth policy table);
/// - [`register_feature_blocks`] — `register_block(name, Arc::new(Ctor::new()))`
///   over every entry, called from `SolobaseBuilder::build` on **both** native
///   and wasm32.
///
/// Replaces the three formerly hand-synced lists (per-block `register_static_block!`
/// linkme sites on native, the `register_all_static_blocks` wasm32 list, and
/// the `all_block_infos` push list) — audit findings #12/#13. A block is now
/// added in exactly one place.
///
/// Each entry's `cfg` gates the block on its `block-*` Cargo feature; the
/// dual-target blocks compile with no `cfg` (always on). `fastembed` carries
/// `feature = "block-fastembed"`, which is never enabled on wasm32 (it pulls
/// ONNX Runtime — see the `pub mod fastembed` cfg above), so the same gate
/// covers "native-only" without a redundant `not(target_arch = "wasm32")`.
///
/// Special cases stay **out** of the manifest and are registered explicitly by
/// `SolobaseBuilder::build`, because their constructors are not zero-argument:
/// `suppers-ai/llm` (`Arc<dyn ProviderAdmin>`, via [`register_llm`]),
/// `suppers-ai/auth` (framework `AuthBlock` wrapping `AuthServiceImpl`, via
/// [`register_auth`]), and `suppers-ai/transformers-embed` (wasm32-only,
/// injected `Arc<dyn EmbeddingService>`). `llm`'s `BlockInfo` is still added to
/// [`all_block_infos`] below via a `NoopProviderAdmin` handle (info is
/// declarative and never drives the provider surface).
macro_rules! feature_block_manifest {
    ( $( $(#[$cfg:meta])? $ctor:path ),+ $(,)? ) => {
        /// `BlockInfo` for every zero-arg solobase feature block, plus the
        /// `suppers-ai/llm` block (constructed with a `NoopProviderAdmin`).
        ///
        /// Used by `collect_all_config_vars()` to discover declared config
        /// variables, by the inspector route table, and by the routing/auth
        /// policy, before block registration runs.
        pub fn all_block_infos() -> Vec<wafer_run::BlockInfo> {
            use wafer_run::Block as _;
            #[allow(unused_mut)]
            let mut infos: Vec<wafer_run::BlockInfo> = Vec::new();
            $(
                $(#[$cfg])?
                infos.push(<$ctor>::new().info());
            )+

            // `suppers-ai/llm` is registered separately (its ctor takes
            // `Arc<dyn ProviderAdmin>`), but its declarative `info()` belongs
            // in the discovery set. A no-op provider-admin handle suffices.
            #[cfg(feature = "block-llm")]
            infos.push(
                llm::LlmBlock::new(std::sync::Arc::new(llm::provider_admin::NoopProviderAdmin))
                    .info(),
            );

            infos
        }

        /// Register every zero-arg solobase feature block on the runtime.
        ///
        /// Called from `SolobaseBuilder::build` on **both** native and wasm32 —
        /// there is no longer a native (linkme) / wasm32 (manual list) split.
        /// The `suppers-ai/llm`, `suppers-ai/auth`, and (wasm32)
        /// `suppers-ai/transformers-embed` blocks are registered explicitly by
        /// the builder afterwards (non-zero-arg constructors).
        pub fn register_feature_blocks(
            wafer: &mut wafer_run::Wafer,
        ) -> Result<(), wafer_run::RuntimeError> {
            use std::sync::Arc;
            $(
                $(#[$cfg])?
                wafer.register_block(
                    <$ctor>::BLOCK_NAME,
                    Arc::new(<$ctor>::new()),
                )?;
            )+
            Ok(())
        }
    };
}

feature_block_manifest! {
    admin::AdminBlock,
    auth_ui::AuthUiBlock,
    email::EmailBlock,
    system::SystemBlock,
    #[cfg(feature = "block-files")]
    files::FilesBlock,
    #[cfg(feature = "block-legalpages")]
    legalpages::LegalPagesBlock,
    #[cfg(feature = "block-messages")]
    messages::MessagesBlock,
    #[cfg(feature = "block-products")]
    products::ProductsBlock,
    #[cfg(feature = "block-userportal")]
    userportal::UserPortalBlock,
    #[cfg(feature = "block-vector")]
    vector::VectorBlock,
    // Native-only: fastembed pulls ONNX Runtime; `block-fastembed` is never
    // enabled on wasm32, so this gate doubles as "not wasm32".
    #[cfg(feature = "block-fastembed")]
    fastembed::FastembedBlock,
}

/// Collect every compiled block's ordered SQLite migration scripts for the
/// Cloudflare D1 build.
///
/// This is the single schema source for `solobase build --target cloudflare`:
/// the same per-block `migrations::SQLITE_MIGRATIONS` consts that the runtime
/// `apply()` paths execute at `lifecycle(Init)` are written verbatim into the
/// generated D1 migration directory (`embed_cloudflare.rs`). There is no
/// separate DDL generator and no second schema declaration — the hand-authored
/// `*.sqlite.sql` files own the schema for both native/in-Worker boot and the
/// `wrangler d1 migrations apply` deploy path.
///
/// Returns `(filename, content)` pairs already prefixed with a zero-padded
/// global sequence number so `wrangler d1 migrations apply` runs them in this
/// exact order. Auth comes first because other blocks' tables carry foreign
/// keys onto `suppers_ai__auth__users`. The per-block source order (the order
/// inside each `SQLITE_MIGRATIONS` slice) is preserved.
///
/// Feature-gated blocks contribute only when their `block-*` feature is on,
/// matching the block set that actually registers at runtime. The native
/// `solobase` binary that runs the Cloudflare build enables every default
/// `block-*` feature, so the generated migration set is complete.
pub fn all_sqlite_migrations() -> Vec<(String, &'static str)> {
    // `(block-slug, &SQLITE_MIGRATIONS)` in apply order. Auth first (FK
    // targets), then admin (block_settings / variables that the migration
    // gate itself writes), then the feature blocks.
    #[allow(unused_mut)]
    let mut blocks: Vec<(&'static str, &'static [(&'static str, &'static str)])> = vec![
        ("auth", auth::migrations::SQLITE_MIGRATIONS),
        ("admin", admin::migrations::SQLITE_MIGRATIONS),
    ];

    #[cfg(feature = "block-files")]
    blocks.push(("files", files::migrations::SQLITE_MIGRATIONS));
    #[cfg(feature = "block-legalpages")]
    blocks.push(("legalpages", legalpages::migrations::SQLITE_MIGRATIONS));
    #[cfg(feature = "block-messages")]
    blocks.push(("messages", messages::migrations::SQLITE_MIGRATIONS));
    #[cfg(feature = "block-products")]
    blocks.push(("products", products::migrations::SQLITE_MIGRATIONS));
    #[cfg(feature = "block-userportal")]
    blocks.push(("userportal", userportal::migrations::SQLITE_MIGRATIONS));
    #[cfg(feature = "block-vector")]
    blocks.push(("vector", vector::migrations::SQLITE_MIGRATIONS));
    #[cfg(feature = "block-llm")]
    blocks.push(("llm", llm::migrations::SQLITE_MIGRATIONS));

    let mut out = Vec::new();
    let mut seq = 1u32;
    for (slug, migrations) in blocks {
        for (basename, content) in migrations {
            out.push((format!("{seq:04}_{slug}__{basename}.sql"), *content));
            seq += 1;
        }
    }
    out
}

/// Register the LLM feature block with the WAFER runtime.
///
/// LlmBlock is not in the feature-block manifest because its constructor takes
/// `Arc<dyn ProviderAdmin>`. Call this after the LLM service router is
/// registered in `SolobaseBuilder::build()`.
///
/// `provider_admin` is the provider-management seam: the concrete
/// `ProviderLlmService` on native (`feature = "llm"`) or a `NoopProviderAdmin`
/// on wasm32 (where the browser configures providers inside its own
/// `BrowserLlmService`).
#[cfg(feature = "block-llm")]
pub fn register_llm(
    w: &mut wafer_run::Wafer,
    provider_admin: std::sync::Arc<dyn llm::provider_admin::ProviderAdmin>,
) -> Result<(), wafer_run::RuntimeError> {
    w.register_block(
        "suppers-ai/llm".to_string(),
        std::sync::Arc::new(llm::LlmBlock::new(provider_admin)),
    )
}

/// Register the framework `suppers-ai/auth` block — wafer-core's `AuthBlock`
/// wrapping solobase's `AuthServiceImpl`.
///
/// Cannot self-register via the feature-block manifest because the framework
/// `AuthBlock::new` takes `Arc<dyn AuthService>`. Called explicitly from
/// `SolobaseBuilder::build` (both targets) to install both the block and the
/// service.
///
/// The `AuthServiceImpl`'s context cell starts empty here; it gets populated
/// when the runtime fires the framework AuthBlock's `lifecycle(Init)` event,
/// which calls into `AuthService::init` and stashes `ctx.clone_arc()` for
/// later `require_*` dispatches.
pub fn register_auth(wafer: &mut wafer_run::Wafer) -> Result<(), wafer_run::RuntimeError> {
    use std::sync::Arc;
    let state = auth::service::BlockState::new();
    let svc = Arc::new(auth::service::AuthServiceImpl::new(state));
    wafer_core::service_blocks::auth::register_with(wafer, svc)
}

#[cfg(test)]
mod migration_registry_tests {
    use super::all_sqlite_migrations;

    /// The D1 build registry is the single schema source for Cloudflare:
    /// it must carry every block's full migration set, including the auth
    /// scripts the old CollectionSchema generator silently dropped (auth
    /// only had 001/002 wired through `extra_migrations`; 003-007 plus
    /// llm/vector/legalpages never reached the D1 migration set).
    #[test]
    fn registry_covers_auth_and_feature_block_schemas() {
        let migrations = all_sqlite_migrations();
        let names: Vec<&str> = migrations.iter().map(|(n, _)| n.as_str()).collect();

        // Auth first (FK targets) — all seven scripts, not just 001/002.
        assert!(names.iter().any(|n| n.contains("auth__001_auth_schema")));
        assert!(names.iter().any(|n| n.contains("auth__002_reserved_orgs")));
        assert!(names.iter().any(|n| n.contains("auth__007_api_keys")));

        // Admin's migration-state tables must be present (the gate writes
        // block_settings / variables) and ordered after auth.
        assert!(names.iter().any(|n| n.contains("admin__001_admin_schema")));

        // The blocks the old generator never emitted for the D1 build,
        // gated by their feature flags.
        #[cfg(feature = "block-llm")]
        assert!(names.iter().any(|n| n.contains("llm__001_llm_schema")));
        #[cfg(feature = "block-vector")]
        assert!(names
            .iter()
            .any(|n| n.contains("vector__001_vector_schema")));
        #[cfg(feature = "block-legalpages")]
        assert!(names
            .iter()
            .any(|n| n.contains("legalpages__001_legalpages_schema")));
    }

    /// Wrangler applies migrations in lexical order, so filenames must be
    /// unique and carry a strictly increasing zero-padded sequence prefix
    /// with auth first.
    #[test]
    fn registry_filenames_are_unique_sequenced_and_nonempty() {
        let migrations = all_sqlite_migrations();
        assert!(!migrations.is_empty());

        let mut prev_seq = 0u32;
        let mut seen = std::collections::HashSet::new();
        for (i, (name, content)) in migrations.iter().enumerate() {
            assert!(name.ends_with(".sql"), "{name} must end with .sql");
            assert!(seen.insert(name.clone()), "duplicate migration file {name}");
            assert!(
                !content.trim().is_empty(),
                "migration {name} has empty SQL content"
            );
            let seq: u32 = name[..4]
                .parse()
                .unwrap_or_else(|_| panic!("migration {name} must start with a 4-digit sequence"));
            assert_eq!(seq as usize, i + 1, "sequence must be dense and 1-based");
            assert!(seq > prev_seq, "sequence must strictly increase");
            prev_seq = seq;
        }

        // Auth schema is the very first applied migration.
        assert!(migrations[0].0.contains("auth__001_auth_schema"));
    }
}
