//! Embed × Cloudflare flow: cross-compile a consumer crate to wasm32,
//! generate wrangler.toml + stage assets, optionally deploy via wrangler.

use std::path::Path;

use anyhow::{bail, Result};

use crate::cli::helpers::cloudflare::{assets, build as cf_build, env, wrangler};

pub async fn build(repo_root: &Path, release: bool) -> Result<()> {
    let cfg = env::load(repo_root)?;

    let out_dir = repo_root.join("target/solobase-cloudflare");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir)?;
    }
    std::fs::create_dir_all(&out_dir)?;

    let wrangler_path = wrangler::generate(&cfg, repo_root, &out_dir)?;
    println!("-> {}", wrangler_path.display());

    cf_build::run(repo_root, release)?;

    let report = assets::stage(repo_root, &out_dir)?;
    println!(
        "-> staged {} files ({:.1} KB) into {}/assets/",
        report.files_copied,
        report.bytes_copied as f64 / 1024.0,
        out_dir.display(),
    );
    if !report.dirs_skipped.is_empty() {
        println!("  (skipped missing dirs: {:?})", report.dirs_skipped);
    }

    println!();
    println!("Next step: solobase deploy --target cloudflare");
    Ok(())
}

pub async fn serve(_repo_root: &Path, _release: bool, _port: Option<u16>) -> Result<()> {
    bail!("embed_cloudflare::serve not yet implemented")
}

pub async fn deploy(_repo_root: &Path, _release: bool) -> Result<()> {
    bail!("embed_cloudflare::deploy not yet implemented")
}
