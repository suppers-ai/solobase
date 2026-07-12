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

pub async fn serve(repo_root: &Path, release: bool, port: Option<u16>) -> Result<()> {
    build(repo_root, release).await?;

    let out_dir = repo_root.join("target/solobase-cloudflare");
    let wrangler_toml = out_dir.join("wrangler.toml");

    // Ephemeral deploy token for this serve session: lets us drive the same
    // /_deploy/init funnel a production deploy uses (migrations + seeds,
    // auto_generate vars included) against the local D1. `wrangler dev`
    // resolves `--var` bindings through `env.secret()`.
    let mut buf = [0u8; 32];
    getrandom::getrandom(&mut buf).map_err(|e| anyhow::anyhow!("getrandom: {e}"))?;
    let dev_token = solobase_core::util::hex_encode(&buf);

    // `wrangler dev` is a long-running child: spawn (not status) so we can
    // POST the init funnel once it is up, then wait for it.
    let mut dev = tokio::process::Command::new("wrangler");
    dev.args(["dev", "--config"]).arg(&wrangler_toml);
    let local_port = port.unwrap_or(8787);
    dev.args(["--port", &local_port.to_string()]);
    dev.args([
        "--var",
        &format!(
            "{}:{dev_token}",
            solobase_core::config_vars::DEPLOY_TOKEN_KEY
        ),
    ]);
    let mut child = dev.spawn()?;

    let local_url = format!("http://localhost:{local_port}");
    match wait_and_run_local_init(&mut child, &local_url, &dev_token).await {
        Ok((ok, report)) => {
            if ok {
                println!("-> local /_deploy/init ok (migrations + seeds applied)");
            } else {
                eprintln!("-> local /_deploy/init reported failures:\n{report}");
                eprintln!(
                    "-> retry manually: POST {local_url}/_deploy/init with header \
                     x-deploy-token: {dev_token}"
                );
            }
        }
        Err(e) => eprintln!(
            "-> local /_deploy/init not reachable ({e}); serving anyway — \
             POST {local_url}/_deploy/init with header x-deploy-token: {dev_token} to seed manually"
        ),
    }

    let status = child.wait().await?;
    if !status.success() {
        bail!("wrangler dev failed (exit {:?})", status.code());
    }
    Ok(())
}

/// Poll until the local worker answers, then run the deploy-init funnel
/// against it. Bounded: ~60s of connect retries.
///
/// Checks `child` for an early exit before each retry sleep so a wrangler
/// crash surfaces as "wrangler dev exited" instead of masquerading as 60s
/// of generic "not reachable" connect failures.
async fn wait_and_run_local_init(
    child: &mut tokio::process::Child,
    local_url: &str,
    token: &str,
) -> Result<(bool, String)> {
    const ATTEMPTS: u32 = 120;
    const INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);
    let mut last_err = None;
    for _ in 0..ATTEMPTS {
        match cf_deploy::call_deploy_init(local_url, token).await {
            Ok(out) => return Ok(out),
            Err(e) => {
                last_err = Some(e);
                if let Some(status) = child.try_wait()? {
                    return Err(anyhow::anyhow!(
                        "wrangler dev exited ({status}) before /_deploy/init became \
                         reachable — see wrangler output above"
                    ));
                }
                tokio::time::sleep(INTERVAL).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("wrangler dev never became reachable")))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A child that exits immediately (no wrangler binary needed) must be
    /// detected between connect retries: `wait_and_run_local_init` should
    /// report "wrangler dev exited" rather than exhausting all ~60s of
    /// retries against a port nothing is listening on.
    #[tokio::test]
    async fn wait_and_run_local_init_detects_dead_child() {
        let mut child = tokio::process::Command::new("true")
            .spawn()
            .expect("spawn `true`");

        // Port 1 is unassigned on loopback — connections are refused near-
        // instantly rather than timing out, so a dead child is what makes
        // the loop exit quickly instead of the 60s retry budget.
        let err = wait_and_run_local_init(&mut child, "http://127.0.0.1:1", "token")
            .await
            .expect_err("dead child must surface as an error, not a 60s hang");

        assert!(
            err.to_string().contains("wrangler dev exited"),
            "expected a wrangler-death error, got: {err}"
        );
    }
}
