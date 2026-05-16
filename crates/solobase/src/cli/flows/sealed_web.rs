//! Sealed × web: assemble a static dist/ from the bundled solobase-web
//! wasm + the user's frontend overlays + any blocks/.

use std::path::Path;

use anyhow::{anyhow, Result};

use crate::cli::{
    config,
    helpers::{blocks, http_server, overlays, wasm},
};

pub async fn build(repo_root: &Path, release: bool) -> Result<()> {
    // 1. Discover and build user blocks (if any).
    blocks::build_all(repo_root).await?;

    // 2. Prepare dist directory.
    let dist = repo_root.join("dist");
    if dist.exists() {
        std::fs::remove_dir_all(&dist).map_err(|e| anyhow!("clean dist/: {e}"))?;
    }
    std::fs::create_dir_all(&dist).map_err(|e| anyhow!("create dist/: {e}"))?;

    // 3. Resolve and write the solobase-web wasm.
    let wasm_bytes = wasm::resolve_solobase_web_wasm()?;
    let wasm_path = dist.join("solobase-web.wasm");
    std::fs::write(&wasm_path, &*wasm_bytes).map_err(|e| anyhow!("write {wasm_path:?}: {e}"))?;

    // 4. Run the bundler — content-hash assets + render templates.
    //    This calls solobase_browser::tools::bundle::run, which writes the
    //    static shell (index.html, sw.js, loader.js) into dist/.
    let cfg = config::find_and_load(repo_root).ok();
    let app = match cfg.as_ref() {
        Some((c, _)) => solobase_browser::tools::bundle::AppConfig {
            app_name: Some(c.app.name.clone()),
            app_title: Some(c.app.title.clone()),
            boot_redirect: Some(c.app.boot_redirect.clone()),
            extra_bypass_prefix: c.assets.extra_bypass_prefix.clone(),
        },
        None => solobase_browser::tools::bundle::AppConfig {
            app_name: None,
            app_title: None,
            boot_redirect: None,
            extra_bypass_prefix: vec![],
        },
    };

    solobase_browser::assets::write_to(&dist)?;
    solobase_browser::tools::bundle::run(&dist, repo_root, app)?;

    // 5. Apply overlays from solobase.toml if present.
    if let Some((cfg, root)) = cfg {
        overlays::apply_overlays(&cfg, &root, &dist)?;
    }

    let profile = if release { "release" } else { "dev" };
    println!("built sealed × web ({profile}) → dist/");
    Ok(())
}

pub async fn serve(
    repo_root: &Path,
    release: bool,
    port: Option<u16>,
    _run_migrations: bool,
) -> Result<()> {
    // Web serve runs a static-file server over the wasm bundle; the
    // wasm itself owns its own runtime-side migration state. The flag is
    // accepted for CLI-symmetry but has nothing to do at this layer.
    build(repo_root, release).await?;
    let port = port.unwrap_or(8080);
    let dist = repo_root.join("dist");
    eprintln!("serving {} on http://127.0.0.1:{port}", dist.display());
    http_server::serve_static(&dist, port).await
}
