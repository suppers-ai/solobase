//! Embed × Cloudflare flow: cross-compile a consumer crate to wasm32,
//! generate wrangler.toml + stage assets, optionally deploy via wrangler.

use std::{path::Path, process::Command};

use anyhow::{bail, Result};

use crate::cli::helpers::cloudflare::{
    assets, build as cf_build, deploy as cf_deploy, env, wrangler,
};

pub async fn build(repo_root: &Path, release: bool) -> Result<()> {
    let cfg = env::load(repo_root)?;

    let out_dir = repo_root.join("target/solobase-cloudflare");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir)?;
    }
    std::fs::create_dir_all(&out_dir)?;

    let wrangler_path = wrangler::generate(&cfg, repo_root, &out_dir)?;
    println!("-> {}", wrangler_path.display());

    // Generate D1 migrations from registered blocks' CollectionSchema.
    // Wrangler picks these up via `migrations_dir` in wrangler.toml when
    // `wrangler d1 migrations apply` runs at deploy time.
    let migrations_dir = out_dir.join("migrations");
    std::fs::create_dir_all(&migrations_dir)?;
    let block_infos = solobase_core::blocks::all_block_infos();
    let collections: Vec<_> = block_infos
        .iter()
        .flat_map(|b| b.collections.iter().cloned())
        .collect();
    let sql = solobase_core::migrations::generate_initial_schema(&collections);
    std::fs::write(migrations_dir.join("0001_initial_schema.sql"), &sql)?;
    println!(
        "-> wrote {} bytes of migrations to {}",
        sql.len(),
        migrations_dir.display()
    );

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

pub async fn serve(repo_root: &Path, release: bool, port: Option<u16>) -> Result<()> {
    build(repo_root, release).await?;

    let out_dir = repo_root.join("target/solobase-cloudflare");
    let wrangler_toml = out_dir.join("wrangler.toml");
    let cfg = env::load(repo_root)?;

    // D1 local migrations: best-effort. Skip cleanly if no migrations dir.
    if repo_root.join("migrations").is_dir() {
        let mut m = Command::new("wrangler");
        m.args([
            "d1",
            "migrations",
            "apply",
            cfg.d1.database_name.as_str(),
            "--local",
            "--config",
        ])
        .arg(&wrangler_toml);
        let status = m.status()?;
        if !status.success() {
            bail!("wrangler d1 migrations apply --local failed");
        }
    }

    let mut dev = Command::new("wrangler");
    dev.args(["dev", "--config"]).arg(&wrangler_toml);
    if let Some(p) = port {
        dev.args(["--port", &p.to_string()]);
    }
    let status = dev.status()?;
    if !status.success() {
        bail!("wrangler dev failed (exit {:?})", status.code());
    }
    Ok(())
}

pub async fn deploy(repo_root: &Path, release: bool) -> Result<()> {
    let cfg = env::load(repo_root)?;
    let _ = env::require_api_token()?; // account_id already validated by load()

    build(repo_root, release).await?;

    let out_dir = repo_root.join("target/solobase-cloudflare");
    let wrangler_toml = out_dir.join("wrangler.toml");

    // Apply D1 migrations BEFORE pushing the worker bundle. The new code
    // expects tables to exist (no more lazy ensure_table()), so migrations
    // must land first.
    let migrations_dir = out_dir.join("migrations");
    if migrations_dir.is_dir() {
        cf_deploy::d1_migrate_remote(&cfg.d1.database_name, &wrangler_toml)?;
        println!("-> D1 migrations applied to {}", cfg.d1.database_name);
    }

    cf_deploy::wrangler_deploy(&wrangler_toml)?;

    let assets_root = out_dir.join("assets");
    let n = cf_deploy::r2_upload_dir(&cfg.r2.bucket_name, &assets_root)?;
    println!(
        "-> uploaded {} R2 objects to bucket {}",
        n, cfg.r2.bucket_name
    );

    println!();
    println!("deploy complete");
    Ok(())
}
