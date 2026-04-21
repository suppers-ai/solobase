use crate::config::Config;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildProfile {
    Dev,
    Release,
}

/// Construct the `wasm-pack build` arg vector.
///
/// Example: ["build", "--target", "web", "--release", "--out-dir", "pkg"]
pub fn wasm_pack_args(cfg: &Config, profile: BuildProfile) -> Vec<String> {
    let mut out = vec!["build".into(), "--target".into(), "web".into()];
    match profile {
        BuildProfile::Release => out.push("--release".into()),
        BuildProfile::Dev => out.push("--dev".into()),
    }
    out.push("--out-dir".into());
    out.push(cfg.wasm.out_dir.clone());
    out
}

/// Construct the arg vector that goes AFTER `cargo run -p solobase-browser
/// --release --bin export-assets --`. `dist_dir` is usually the same as
/// `repo_root.join(&cfg.wasm.out_dir)`. `repo_root` is the absolute path
/// to the consumer repo root (dir containing `solobase.toml`).
pub fn export_assets_args(
    cfg: &Config,
    repo_root: &Path,
    dist_dir: &Path,
    profile: BuildProfile,
) -> Vec<String> {
    let mut out = vec![
        format!("{}/", dist_dir.display()),
        "--repo-dir".into(),
        repo_root.display().to_string(),
        "--app-name".into(),
        cfg.app.name.clone(),
        "--app-title".into(),
        cfg.app.title.clone(),
        "--boot-redirect".into(),
        cfg.app.boot_redirect.clone(),
    ];
    if !cfg.assets.extra_bypass_prefix.is_empty() {
        out.push("--extra-bypass-prefix".into());
        out.push(cfg.assets.extra_bypass_prefix.join(","));
    }
    if profile == BuildProfile::Dev {
        out.push("--dev".into());
    }
    out
}

use std::path::PathBuf;
use std::process::Command;

/// Run the full build pipeline for `cfg`. `repo_root` is the directory that
/// contains `solobase.toml`.
///
/// Steps:
/// 1. Skill-block auto-discovery — runs `wafer build` in each `blocks/*/`.
/// 2. `wasm-pack build ...`.
/// 3. `cargo run -p solobase-browser --release --bin export-assets -- ...`.
/// 4. Apply `[[assets.overlay]]` — copy each `from` → `<dist>/<to>`.
///
/// On success prints a one-line summary to stdout.
pub fn run(cfg: &Config, repo_root: &PathBuf, profile: BuildProfile) -> anyhow::Result<()> {
    if std::env::var("SOLOBASE_CLI_DRY_RUN").as_deref() == Ok("1") {
        // Emit a machine-readable summary instead of spawning children.
        // Used by integration tests that can't assume wasm-pack / cargo /
        // wafer are available in the sandbox.
        let skills = crate::skills::discover(repo_root)?;
        let wp = wasm_pack_args(cfg, profile);
        let dist_dir = repo_root.join(&cfg.wasm.out_dir);
        let ea = export_assets_args(cfg, repo_root, &dist_dir, profile);
        let overlays: Vec<String> = cfg
            .assets
            .overlay
            .iter()
            .map(|o| format!("{}->{}", o.from, o.to))
            .collect();
        println!(
            "DRY_RUN\napp={}\nprofile={:?}\nskills={}\nwasm_pack={:?}\nexport_assets={:?}\noverlays={:?}",
            cfg.app.name, profile, skills.len(), wp, ea, overlays
        );
        return Ok(());
    }

    // 1. Skill blocks.
    crate::skills::build_all(repo_root)?;

    // 2. wasm-pack.
    let mut wp = Command::new("wasm-pack");
    wp.args(wasm_pack_args(cfg, profile)).current_dir(repo_root);
    crate::cmd::run("wasm-pack build", wp)?;

    // 3. export-assets.
    let dist_dir = repo_root.join(&cfg.wasm.out_dir);
    let mut ea = Command::new("cargo");
    ea.args([
        "run",
        "-p",
        "solobase-browser",
        "--release",
        "--bin",
        "export-assets",
        "--",
    ])
    .args(export_assets_args(cfg, repo_root, &dist_dir, profile))
    .current_dir(repo_root);
    crate::cmd::run("export-assets", ea)?;

    // 4. Overlays.
    for overlay in &cfg.assets.overlay {
        let src = repo_root.join(&overlay.from);
        let dst = dist_dir.join(&overlay.to);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("create dir {parent:?}: {e}"))?;
        }
        std::fs::copy(&src, &dst)
            .map_err(|e| anyhow::anyhow!("overlay {src:?} → {dst:?}: {e}"))?;
    }

    // 5. Summary.
    let profile_label = match profile {
        BuildProfile::Dev => "dev",
        BuildProfile::Release => "release",
    };
    println!(
        "built {} ({}) → {}",
        cfg.app.name,
        profile_label,
        cfg.wasm.out_dir
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn minimal_cfg() -> Config {
        crate::config::parse(
            r#"
[app]
name = "solobase-web"
title = "Solobase"
boot_redirect = "/b/system/"
"#,
        )
        .unwrap()
    }

    #[test]
    fn wasm_pack_release_args() {
        let cfg = minimal_cfg();
        let args = wasm_pack_args(&cfg, BuildProfile::Release);
        assert_eq!(
            args,
            vec![
                "build".to_string(),
                "--target".into(),
                "web".into(),
                "--release".into(),
                "--out-dir".into(),
                "pkg".into(),
            ]
        );
    }

    #[test]
    fn wasm_pack_dev_args() {
        let cfg = minimal_cfg();
        let args = wasm_pack_args(&cfg, BuildProfile::Dev);
        assert!(args.contains(&"--dev".to_string()));
        assert!(!args.contains(&"--release".to_string()));
    }

    #[test]
    fn export_assets_minimal() {
        let cfg = minimal_cfg();
        let repo = PathBuf::from("/repo");
        let dist = PathBuf::from("pkg");
        let args = export_assets_args(&cfg, &repo, &dist, BuildProfile::Release);
        assert_eq!(args[0], "pkg/");
        assert!(args.iter().any(|a| a == "--repo-dir"));
        assert!(args.iter().any(|a| a == "/repo"));
        assert!(args.iter().any(|a| a == "--app-name"));
        assert!(args.iter().any(|a| a == "solobase-web"));
        assert!(args.iter().any(|a| a == "--boot-redirect"));
        assert!(args.iter().any(|a| a == "/b/system/"));
        assert!(!args.iter().any(|a| a == "--extra-bypass-prefix"));
        assert!(!args.iter().any(|a| a == "--dev"));
    }

    #[test]
    fn export_assets_with_bypass_and_dev() {
        let cfg: Config = crate::config::parse(
            r#"
[app]
name = "gizza-ai"
title = "Gizza AI"
boot_redirect = "/"

[assets]
extra_bypass_prefix = ["/gizza-app.js", "/gizza.css"]
"#,
        )
        .unwrap();
        let args = export_assets_args(&cfg, Path::new("/repo"), Path::new("dist"), BuildProfile::Dev);
        assert_eq!(args[0], "dist/");
        let bp_ix = args.iter().position(|a| a == "--extra-bypass-prefix").unwrap();
        assert_eq!(args[bp_ix + 1], "/gizza-app.js,/gizza.css");
        assert!(args.contains(&"--dev".to_string()));
    }
}
