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

    // Write the D1 migrations from the per-block SQL files — the single
    // schema source. `all_sqlite_migrations()` returns the same
    // `migrations::SQLITE_MIGRATIONS` scripts the runtime `apply()` paths
    // execute at `lifecycle(Init)`, sequenced as `NNNN_<block>__<name>.sql`.
    // Wrangler picks these up via `migrations_dir` in wrangler.toml when
    // `wrangler d1 migrations apply` runs at deploy time.
    let migrations_dir = out_dir.join("migrations");
    std::fs::create_dir_all(&migrations_dir)?;
    for (name, content) in solobase_core::blocks::all_sqlite_migrations() {
        std::fs::write(migrations_dir.join(&name), content)?;
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

pub async fn deploy(repo_root: &Path, release: bool) -> Result<()> {
    let cfg = env::load(repo_root)?;
    let _ = env::require_api_token()?; // account_id already validated by load()
    let token_key = solobase_core::config_vars::DEPLOY_TOKEN_KEY;
    let deploy_token = std::env::var(token_key).map_err(|_| {
        anyhow::anyhow!(
            "{token_key} is not set. Provision it with `solobase deploy secret` \
             (or `wrangler secret put {token_key}`) and export it for deploys."
        )
    })?;

    build(repo_root, release).await?;

    let out_dir = repo_root.join("target/solobase-cloudflare");
    let wrangler_toml = out_dir.join("wrangler.toml");

    // 1. Upload an unpromoted version (no traffic routed yet).
    let upload = cf_deploy::wrangler_versions_upload(&wrangler_toml)?;
    println!(
        "-> uploaded version {} (preview {})",
        upload.version_id, upload.preview_url
    );

    // 2. Run migrations + seeds through the new version's own code, against
    //    the shared production D1 (additive migrations keep the still-live
    //    old version safe). Abort pre-promote on failure.
    let (ok, report) = cf_deploy::call_deploy_init(&upload.preview_url, &deploy_token).await?;
    println!("{report}");
    if !ok {
        bail!(
            "deploy init failed — version {} NOT promoted",
            upload.version_id
        );
    }

    // 3. Promote.
    cf_deploy::wrangler_versions_promote(&upload.version_id, &wrangler_toml)?;
    println!("-> promoted {}", upload.version_id);

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

/// `solobase deploy secret`: provision the one-time-per-environment worker
/// secrets (`SOLOBASE_DEPLOY_TOKEN` + the auth JWT secret) via
/// `wrangler secret put`. Each value is taken from the same-named env var when
/// set, otherwise a fresh 32-byte hex token is generated. Requires the
/// generated `wrangler.toml` (run `solobase build --target cloudflare` first).
pub async fn deploy_secret(repo_root: &Path) -> Result<()> {
    let out_dir = repo_root.join("target/solobase-cloudflare");
    let wrangler_toml = out_dir.join("wrangler.toml");
    if !wrangler_toml.exists() {
        bail!(
            "wrangler.toml not found at {}. Run `solobase build --target cloudflare` first.",
            wrangler_toml.display()
        );
    }

    let deploy_token_key = solobase_core::config_vars::DEPLOY_TOKEN_KEY;
    for name in [
        deploy_token_key,
        solobase_core::blocks::auth::JWT_SECRET_KEY,
    ] {
        // 32 random bytes → 64 hex chars. getrandom is already a dependency
        // (used for variable seeding); no new crate for randomness.
        let mut buf = [0u8; 32];
        getrandom::getrandom(&mut buf).map_err(|e| anyhow::anyhow!("getrandom: {e}"))?;
        let (value, generated) = cf_deploy::resolve_secret(std::env::var(name).ok(), &buf);

        cf_deploy::wrangler_secret_put(&wrangler_toml, name, &value)?;

        if generated {
            println!("-> generated and set worker secret {name}");
            if name == deploy_token_key {
                println!(
                    "   IMPORTANT: export this for future `solobase deploy` runs:\n     \
                     export {name}={value}"
                );
            }
        } else {
            println!("-> set worker secret {name} (from env {name})");
        }
    }
    Ok(())
}
