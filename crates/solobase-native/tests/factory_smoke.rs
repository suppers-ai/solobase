//! Smoke tests: each `make_*_service` factory returns a non-null
//! `Arc<dyn ...>`. These don't exercise the underlying service in
//! depth — they just catch compile-time type-signature regressions
//! and confirm the factory hands back something usable.

#[test]
fn sqlite_factory_returns_service() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("smoke.db");
    let _svc = solobase_native::make_sqlite_database_service(path.to_str().unwrap());
    // If we got here, the factory worked.
}

#[test]
fn local_storage_factory_returns_service() {
    let tmp = tempfile::tempdir().unwrap();
    let _svc = solobase_native::make_local_storage_service(tmp.path().to_str().unwrap());
}

#[test]
fn network_factory_returns_service() {
    let _svc = solobase_native::make_fetch_network_service();
}

#[test]
fn crypto_factory_returns_service() {
    let _svc = solobase_native::make_jwt_crypto_service("smoke-test-secret".to_string());
}

#[test]
fn logger_factory_returns_service() {
    let _svc = solobase_native::make_tracing_logger();
}

#[test]
fn infra_config_reads_defaults_when_env_unset() {
    let _cfg = solobase_native::InfraConfig::from_env();
}

#[test]
fn collect_app_env_vars_excludes_solobase_prefix() {
    // Use a unique test-key prefix so parallel test runs don't collide.
    std::env::set_var("SOLOBASE_NATIVE_FACTORY_SMOKE_INFRA_X", "1");
    std::env::set_var("NATIVE_FACTORY_SMOKE_APP_Y", "2");

    let vars = solobase_native::collect_app_env_vars();
    assert!(
        !vars.contains_key("SOLOBASE_NATIVE_FACTORY_SMOKE_INFRA_X"),
        "SOLOBASE_-prefixed var leaked into collect_app_env_vars output"
    );
    assert_eq!(
        vars.get("NATIVE_FACTORY_SMOKE_APP_Y").map(String::as_str),
        Some("2"),
        "non-SOLOBASE_ var missing from collect_app_env_vars output"
    );

    std::env::remove_var("SOLOBASE_NATIVE_FACTORY_SMOKE_INFRA_X");
    std::env::remove_var("NATIVE_FACTORY_SMOKE_APP_Y");
}
