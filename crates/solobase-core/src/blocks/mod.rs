pub mod admin;
pub mod auth;
pub mod auth_ui;
pub mod crud;
pub mod email;
pub mod errors;
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

/// Return `BlockInfo` for every solobase block.
///
/// This is the single canonical source of truth for both native and wasm32.
/// Previously, native used `linkme`/`STATIC_BLOCK_REGISTRATIONS` iteration
/// and wasm32 had a separate manual list — see audit finding #13.
///
/// The two lists had diverged: the native linkme sweep also picked up
/// `wafer-run/*` framework blocks (cors, inspector, etc.) that were never
/// relevant to `collect_all_config_vars`, and the wasm32 list included the
/// framework `AuthBlock` whose config vars are declared via
/// `shared_config_vars()` → `auth_config_vars()` rather than
/// `BlockInfo::config_keys`, making it redundant there too. This function
/// enumerates only the solobase feature blocks, consistently on both targets.
///
/// Used by `collect_all_config_vars()` to discover declared config
/// variables before block registration runs.
pub fn all_block_infos() -> Vec<wafer_run::BlockInfo> {
    use wafer_run::Block as _;

    // `unused_mut` fires when every optional feature is off and no later
    // `.push(...)` exists to mutate the vec.
    #[allow(unused_mut)]
    let mut infos: Vec<wafer_run::BlockInfo> = vec![
        admin::AdminBlock::new().info(),
        auth_ui::AuthUiBlock::default().info(),
        email::EmailBlock::new().info(),
        system::SystemBlock::new().info(),
    ];

    #[cfg(feature = "block-files")]
    infos.push(files::FilesBlock::new().info());
    #[cfg(feature = "block-legalpages")]
    infos.push(legalpages::LegalPagesBlock::new().info());
    #[cfg(feature = "block-messages")]
    infos.push(messages::MessagesBlock::new().info());
    #[cfg(feature = "block-products")]
    infos.push(products::ProductsBlock::new().info());
    #[cfg(feature = "block-userportal")]
    infos.push(userportal::UserPortalBlock::new().info());
    #[cfg(feature = "block-vector")]
    infos.push(vector::VectorBlock::new().info());

    // fastembed is native-only: it requires ONNX Runtime which is not
    // available on wasm32.
    #[cfg(feature = "block-fastembed")]
    infos.push(fastembed::FastembedBlock::new().info());

    // LlmBlock cannot self-register because its constructor takes
    // Arc<dyn ProviderAdmin>. A no-op provider-admin handle is enough here
    // since info() is declarative and never drives the provider surface.
    #[cfg(feature = "block-llm")]
    {
        use std::sync::Arc;
        let provider_admin = Arc::new(llm::provider_admin::NoopProviderAdmin);
        infos.push(llm::LlmBlock::new(provider_admin).info());
    }

    infos
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
/// LlmBlock cannot self-register via `register_static_block!` because its
/// constructor takes `Arc<dyn ProviderAdmin>`. Call this after the LLM
/// service router is registered in `SolobaseBuilder::build()`.
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
/// Cannot self-register via `register_static_block!` because the framework
/// `AuthBlock::new` takes `Arc<dyn AuthService>`. Call this from
/// `SolobaseBuilder::build` (native) or `register_all_static_blocks` (wasm32)
/// to install both the block and the service.
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

/// Register every solobase feature block on wasm32 builds.
///
/// On native, each block self-registers via `register_static_block!` (gated
/// `cfg(not(target_arch = "wasm32"))` because linkme's distributed_slice
/// only emits on ELF/Mach-O/PE — see `wafer_run::builder::WaferBuilder::build`).
/// On wasm32 that path is a no-op, so the runtime starts with zero
/// `suppers-ai/*` blocks and the SolobaseRouter dispatches into a void —
/// every feature route returns `block 'suppers-ai/<name>' not found`.
///
/// This helper mirrors the linkme manifest so wasm builds get the same
/// block set. Keep this list in sync with the `register_static_block!`
/// invocations across `crate::blocks::*` and with the `all_block_infos`
/// wasm32 fallback above.
///
/// Excludes `suppers-ai/fastembed` (native-only, requires
/// `feature = "native-embedding"`). `suppers-ai/llm` IS registered here when
/// `block-llm` is on: the block now holds `Arc<dyn ProviderAdmin>`, so the
/// wasm32 build installs it with a `NoopProviderAdmin` (the browser's
/// `BrowserLlmService` on the shared router serves chat; provider CRUD /
/// discovery are admin-only and degrade to no-ops).
#[cfg(target_arch = "wasm32")]
pub fn register_all_static_blocks(
    wafer: &mut wafer_run::Wafer,
) -> Result<(), wafer_run::RuntimeError> {
    use std::sync::Arc;

    wafer.register_block("suppers-ai/admin", Arc::new(admin::AdminBlock::new()))?;
    // Framework `suppers-ai/auth` is registered unconditionally by
    // `SolobaseBuilder::build` (after this fn returns) — don't duplicate
    // here, the second register_block would fail with "block already
    // registered" and abort the wasm boot.
    wafer.register_block(
        "suppers-ai/auth-ui",
        Arc::new(auth_ui::AuthUiBlock::default()),
    )?;
    wafer.register_block("suppers-ai/email", Arc::new(email::EmailBlock::new()))?;
    wafer.register_block("suppers-ai/system", Arc::new(system::SystemBlock::new()))?;

    #[cfg(feature = "block-files")]
    wafer.register_block("suppers-ai/files", Arc::new(files::FilesBlock::new()))?;
    #[cfg(feature = "block-legalpages")]
    wafer.register_block(
        "suppers-ai/legalpages",
        Arc::new(legalpages::LegalPagesBlock::new()),
    )?;
    #[cfg(feature = "block-messages")]
    wafer.register_block(
        "suppers-ai/messages",
        Arc::new(messages::MessagesBlock::new()),
    )?;
    #[cfg(feature = "block-products")]
    wafer.register_block(
        "suppers-ai/products",
        Arc::new(products::ProductsBlock::new()),
    )?;
    #[cfg(feature = "block-userportal")]
    wafer.register_block(
        "suppers-ai/userportal",
        Arc::new(userportal::UserPortalBlock::new()),
    )?;
    #[cfg(feature = "block-vector")]
    wafer.register_block("suppers-ai/vector", Arc::new(vector::VectorBlock::new()))?;

    // The LLM block runs on wasm32 against a browser-supplied `LlmService`
    // (registered on the router via `SolobaseBuilder::llm_service`). Provider
    // CRUD / discovery have no browser surface, so a `NoopProviderAdmin`
    // stands in for the native HTTP `ProviderLlmService`.
    #[cfg(feature = "block-llm")]
    register_llm(wafer, Arc::new(llm::provider_admin::NoopProviderAdmin))?;

    Ok(())
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
