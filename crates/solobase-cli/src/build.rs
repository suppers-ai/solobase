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
