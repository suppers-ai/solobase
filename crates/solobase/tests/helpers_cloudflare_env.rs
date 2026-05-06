use std::fs;

use solobase::cli::helpers::cloudflare::env::{load, require_deploy_env};
use tempfile::tempdir;

const VALID_TOML: &str = r#"
[cloudflare]
worker_name = "x"
compatibility_date = "2026-05-01"

[cloudflare.d1]
binding = "DB"
database_name = "x"
database_id = "00000000-0000-0000-0000-000000000000"

[cloudflare.r2]
binding = "STORAGE"
bucket_name = "x-bucket"
"#;

#[test]
fn load_returns_cfg_when_section_present() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("solobase.toml"), VALID_TOML).unwrap();
    let cfg = load(tmp.path()).unwrap();
    assert_eq!(cfg.worker_name, "x");
    assert_eq!(cfg.d1.binding, "DB");
}

#[test]
fn load_errors_when_section_missing() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("solobase.toml"), "# empty\n").unwrap();
    let err = load(tmp.path()).unwrap_err();
    assert!(
        err.to_string().contains("missing a [cloudflare] section"),
        "expected 'missing a [cloudflare] section' in: {err}"
    );
}

#[test]
fn require_deploy_env_errors_on_missing_token() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("solobase.toml"), VALID_TOML).unwrap();
    let mut cfg = load(tmp.path()).unwrap();
    cfg.account_id = Some("acct123".into());
    // SAFETY: tests within this binary may be parallel; this could flake
    // if another test sets CLOUDFLARE_API_TOKEN concurrently. Acceptable
    // for v1; use `temp_env` crate later if it flakes.
    std::env::remove_var("CLOUDFLARE_API_TOKEN");
    let err = require_deploy_env(&cfg).unwrap_err();
    assert!(
        err.to_string().contains("CLOUDFLARE_API_TOKEN"),
        "expected 'CLOUDFLARE_API_TOKEN' in: {err}"
    );
}
