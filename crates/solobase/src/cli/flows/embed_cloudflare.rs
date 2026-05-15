//! Embed × Cloudflare flow: cross-compile a consumer crate to wasm32,
//! generate wrangler.toml + stage assets, optionally deploy via wrangler.

use std::path::Path;

use anyhow::{bail, Result};

use crate::cli::helpers::cloudflare::{
    assets, build as cf_build, deploy as cf_deploy, env, profile_check, wrangler,
};

pub async fn build(repo_root: &Path, release: bool) -> Result<()> {
    let cfg = env::load(repo_root)?;

    // Inspect [profile.release] before we kick off the long cargo build.
    // Warns only — doesn't block — but surfaces the most common cause of
    // the Workers 400ms startup-CPU 1102 cliff.
    if release {
        profile_check::check_release_profile(repo_root)?;
    }

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

    // Hand-authored migrations for blocks whose schema isn't declared via
    // CollectionSchema (e.g. wafer-core's AuthBlock). Embedded at compile
    // time; written alongside the auto-generated initial schema.
    for (name, content) in solobase_core::migrations::extra_migrations() {
        std::fs::write(migrations_dir.join(name), content)?;
    }

    let migration_count = std::fs::read_dir(&migrations_dir)?.count();
    println!(
        "-> wrote {} migration files to {}",
        migration_count,
        migrations_dir.display()
    );

    cf_build::run(repo_root, release).await?;

    // Post-build: measure the produced WASM. Warns if it's likely to
    // exceed the Workers startup-CPU cap on cold-start.
    if release {
        let wasm_path = repo_root.join("build/index_bg.wasm");
        profile_check::check_wasm_size(&wasm_path)?;
    }

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

pub async fn serve(
    repo_root: &Path,
    release: bool,
    port: Option<u16>,
    run_migrations: bool,
) -> Result<()> {
    build(repo_root, release).await?;

    let out_dir = repo_root.join("target/solobase-cloudflare");
    let wrangler_toml = out_dir.join("wrangler.toml");
    let cfg = env::load(repo_root)?;

    // D1 local migrations: best-effort. Skip cleanly if no migrations dir.
    // Use `tokio::process::Command` because both `wrangler d1 migrations
    // apply --local` and especially `wrangler dev` (below) are long-running
    // subprocesses — running them under `std::process::Command` from an
    // async fn would block the tokio worker for the lifetime of the child.
    if repo_root.join("migrations").is_dir() {
        let mut m = tokio::process::Command::new("wrangler");
        m.args([
            "d1",
            "migrations",
            "apply",
            cfg.d1.database_name.as_str(),
            "--local",
            "--config",
        ])
        .arg(&wrangler_toml);
        let status = m.status().await?;
        if !status.success() {
            bail!("wrangler d1 migrations apply --local failed");
        }
    }

    let mut dev = tokio::process::Command::new("wrangler");
    dev.args(["dev", "--config"]).arg(&wrangler_toml);
    if let Some(p) = port {
        dev.args(["--port", &p.to_string()]);
    }
    // Pass the flag to the Worker via wrangler's `--var` so the deployed
    // bundle's `apply_if_blessed` sees it through `ctx.config_get`.
    if run_migrations {
        dev.args(["--var", "SOLOBASE_RUN_MIGRATIONS:1"]);
    }
    let status = dev.status().await?;
    if !status.success() {
        bail!("wrangler dev failed (exit {:?})", status.code());
    }
    Ok(())
}

pub async fn deploy(repo_root: &Path, release: bool, run_migrations: bool) -> Result<()> {
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

    // Block-SQL migrations (the in-Worker hash-gated `apply_if_blessed`
    // sweep) gate on `SOLOBASE_RUN_MIGRATIONS=1` in the Worker env. The
    // D1-side schema migrations above run during `wrangler d1 migrations
    // apply` and are independent of this flag.
    cf_deploy::wrangler_deploy_with_vars(
        &wrangler_toml,
        if run_migrations {
            &[("SOLOBASE_RUN_MIGRATIONS", "1")]
        } else {
            &[]
        },
    )?;

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
