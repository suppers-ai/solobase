//! Compile-time check that the public make_* surface exists and is callable.
//! No runtime assertions — D1/R2 require a worker runtime to instantiate.
//!
//! If any of the 6 symbols are renamed or removed, this test stops compiling.

#[allow(dead_code)]
fn _signatures_compile() {
    // Services that take no parameters: reference as function items to verify
    // the symbol exists with the expected name and zero-argument arity.
    let _: fn() -> _ = solobase_cloudflare::make_fetch_network_service;
    let _: fn() -> _ = solobase_cloudflare::make_console_logger;

    // Services that take parameters: reference the function item by name
    // (not called) — enough to verify the symbol exists with the right name.
    let _ = solobase_cloudflare::make_d1_database_service;
    let _ = solobase_cloudflare::make_r2_storage_service;
    let _ = solobase_cloudflare::make_jwt_crypto_service;
    let _ = solobase_cloudflare::make_config_service;
}
