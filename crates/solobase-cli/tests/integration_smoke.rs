use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn cli_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_solobase"))
}

fn run_in(fixture_dir: &Path, args: &[&str]) -> (String, String, Option<i32>) {
    let bin = cli_path();
    let output = Command::new(&bin)
        .args(args)
        .current_dir(fixture_dir)
        .env("SOLOBASE_CLI_DRY_RUN", "1")
        .output()
        .expect("spawn solobase");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code(),
    )
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn solobase_web_style_dry_run() {
    let (stdout, _stderr, code) = run_in(&fixture("solobase-web-style"), &["build"]);
    assert_eq!(code, Some(0), "expected success, got {code:?}");
    assert!(stdout.contains("DRY_RUN"));
    assert!(stdout.contains("app=solobase-web"));
    assert!(stdout.contains("skills=0"));
    assert!(stdout.contains("overlays=[]"));
}

#[test]
fn gizza_ai_style_dry_run() {
    let (stdout, stderr, code) = run_in(&fixture("gizza-ai-style"), &["dev"]);
    assert_eq!(
        code,
        Some(0),
        "expected success, got {code:?}\nstderr={stderr}"
    );
    assert!(stdout.contains("DRY_RUN"));
    assert!(stdout.contains("app=gizza-ai"));
    assert!(stdout.contains("skills=1"));
    assert!(stdout.contains("site/index.html->index.html"));
    assert!(stdout.contains("site/gizza-app.js->gizza-app.js"));
    assert!(stdout.contains("extra-bypass-prefix"));
}

#[test]
fn missing_config_error() {
    let tmp = tempfile::tempdir().unwrap();
    let (_stdout, stderr, code) = run_in(tmp.path(), &["build"]);
    assert_ne!(code, Some(0));
    assert!(stderr.contains("no solobase.toml found"));
}
