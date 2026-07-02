//! Deploy subprocess orchestration: atomic versioned `wrangler` deploys
//! (`versions upload` → `/_deploy/init` gate → `versions deploy` promote),
//! plus R2 asset upload. The version-upload/promote helpers inherit
//! stdio for interactive progress except `wrangler_versions_upload`,
//! which captures stdout to parse the version id and preview URL.

use std::{path::Path, process::Command};

use anyhow::{bail, Context, Result};

use super::assets::mime_for_path;

/// Output of `wrangler versions upload` (unpromoted deployment).
pub struct VersionUpload {
    pub version_id: String,
    pub preview_url: String,
}

/// Upload a new worker version WITHOUT routing traffic to it. Captures
/// stdout (unlike the inherit-stdio helpers) to parse the version id and
/// preview URL.
pub fn wrangler_versions_upload(wrangler_toml: &Path) -> Result<VersionUpload> {
    let output = Command::new("wrangler")
        .args(["versions", "upload", "--config"])
        .arg(wrangler_toml)
        .output()
        .context("run wrangler versions upload")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        bail!(
            "wrangler versions upload failed (exit {:?})",
            output.status.code()
        );
    }
    // wrangler prints lines like:
    //   Worker Version ID: 8e3c...-....
    //   Version Preview URL: https://<hash>-<worker>.<subdomain>.workers.dev
    let version_id = parse_labeled_line(&stdout, "Version ID:")
        .context("parse Version ID from wrangler versions upload output")?;
    let preview_url = parse_labeled_line(&stdout, "Preview URL:")
        .context("parse Preview URL (enable preview_urls in wrangler.toml)")?;
    Ok(VersionUpload {
        version_id,
        preview_url,
    })
}

fn parse_labeled_line(stdout: &str, label: &str) -> Option<String> {
    stdout
        .lines()
        .find_map(|l| l.split(label).nth(1))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Promote an uploaded version to 100% of traffic.
pub fn wrangler_versions_promote(version_id: &str, wrangler_toml: &Path) -> Result<()> {
    let status = Command::new("wrangler")
        .args([
            "versions",
            "deploy",
            &format!("{version_id}@100%"),
            "--yes",
            "--config",
        ])
        .arg(wrangler_toml)
        .status()
        .context("run wrangler versions deploy")?;
    if !status.success() {
        bail!("wrangler versions deploy failed (exit {:?})", status.code());
    }
    Ok(())
}

/// POST /_deploy/init on the preview URL. Returns (ok, report_body).
pub async fn call_deploy_init(preview_url: &str, token: &str) -> Result<(bool, String)> {
    let url = format!("{}/_deploy/init", preview_url.trim_end_matches('/'));
    let resp = reqwest::Client::new()
        .post(&url)
        .header("x-deploy-token", token)
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    let ok = resp.status().is_success();
    let body = resp.text().await.unwrap_or_default();
    Ok((ok, body))
}

pub fn r2_upload_dir(bucket: &str, assets_root: &Path) -> Result<usize> {
    if !assets_root.is_dir() {
        return Ok(0);
    }
    let mut uploaded = 0;
    walk_files(assets_root, &mut |abs| {
        let rel = abs.strip_prefix(assets_root).unwrap_or(abs);
        let key = rel.to_string_lossy().replace('\\', "/");
        let mime = mime_for_path(abs);
        let status = Command::new("wrangler")
            .args(["r2", "object", "put", &format!("{bucket}/{key}"), "--file"])
            .arg(abs)
            .args(["--content-type", mime, "--remote"])
            .status()
            .context("run wrangler r2 object put")?;
        if !status.success() {
            bail!("upload {} failed (exit {:?})", key, status.code());
        }
        uploaded += 1;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(uploaded)
}

fn walk_files<F>(root: &Path, f: &mut F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    for entry in std::fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walk_files(&path, f)?;
        } else if ft.is_file() {
            f(&path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_labeled_line;

    #[test]
    fn parses_wrangler_version_lines() {
        let out = "Total Upload: 4210 KiB\nWorker Version ID: abc-123\nVersion Preview URL: https://x-y.z.workers.dev\n";
        assert_eq!(
            parse_labeled_line(out, "Version ID:").as_deref(),
            Some("abc-123")
        );
        assert_eq!(
            parse_labeled_line(out, "Preview URL:").as_deref(),
            Some("https://x-y.z.workers.dev")
        );
    }

    #[test]
    fn missing_label_returns_none() {
        assert_eq!(parse_labeled_line("no labels here", "Version ID:"), None);
    }
}
