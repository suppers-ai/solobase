//! Integration tests for the legacy CLI build pipeline.
//!
//! Before Task 4 these tests shelled out to the `solobase` binary in
//! dry-run mode and parsed its stdout. After absorbing `solobase-cli`,
//! the binary no longer dispatches to the legacy CLI (Task 7 reinstates
//! that under the new verb-based surface). To preserve the regression
//! baseline for the legacy modules, we now assert directly on the
//! public helpers (`wasm_pack_args`, `export_assets_args`, skill
//! discovery) — the same pieces the dry-run summary used to print.
use std::path::{Path, PathBuf};

use solobase::cli::{legacy_build, legacy_config, skills};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cli/fixtures")
        .join(name)
}

#[test]
fn solobase_web_style_pipeline() {
    let (cfg, repo_root) =
        legacy_config::find_and_load(&fixture("solobase-web-style")).expect("load config");

    assert_eq!(cfg.app.name, "solobase-web");
    assert_eq!(skills::discover(&repo_root).expect("skills").len(), 0);
    assert!(cfg.assets.overlay.is_empty());

    let wp = legacy_build::wasm_pack_args(&cfg, legacy_build::BuildProfile::Release);
    assert!(wp.iter().any(|s| s == "--release"));

    let dist_dir = repo_root.join(&cfg.wasm.out_dir);
    let ea = legacy_build::export_assets_args(
        &cfg,
        &repo_root,
        &dist_dir,
        legacy_build::BuildProfile::Release,
    );
    assert!(ea.iter().any(|s| s == "--app-name"));
    assert!(ea.iter().any(|s| s == "solobase-web"));
}

#[test]
fn gizza_ai_style_pipeline() {
    let (cfg, repo_root) =
        legacy_config::find_and_load(&fixture("gizza-ai-style")).expect("load config");

    assert_eq!(cfg.app.name, "gizza-ai");
    assert_eq!(skills::discover(&repo_root).expect("skills").len(), 1);

    let overlays: Vec<String> = cfg
        .assets
        .overlay
        .iter()
        .map(|o| format!("{}->{}", o.from, o.to))
        .collect();
    assert!(overlays.iter().any(|s| s == "site/index.html->index.html"));
    assert!(overlays
        .iter()
        .any(|s| s == "site/gizza-app.js->gizza-app.js"));

    let wp = legacy_build::wasm_pack_args(&cfg, legacy_build::BuildProfile::Dev);
    assert!(wp.iter().any(|s| s == "--dev"));

    let dist_dir = repo_root.join(&cfg.wasm.out_dir);
    let ea = legacy_build::export_assets_args(
        &cfg,
        &repo_root,
        &dist_dir,
        legacy_build::BuildProfile::Dev,
    );
    // `extra_bypass_prefix` was set in the fixture; check it survives into
    // the `--extra-bypass-prefix` arg pair.
    let idx = ea
        .iter()
        .position(|s| s == "--extra-bypass-prefix")
        .expect("extra-bypass-prefix flag present");
    let value = &ea[idx + 1];
    assert!(value.contains("/gizza-app.js"));
}

#[test]
fn missing_config_error() {
    let tmp = tempfile::tempdir().unwrap();
    let err = legacy_config::find_and_load(tmp.path())
        .err()
        .expect("should fail when no solobase.toml present")
        .to_string();
    assert!(err.contains("no solobase.toml found"));
}
