//! Crypto platform-service factory for native targets.

use std::sync::Arc;

use wafer_core::interfaces::crypto::service::CryptoService;

/// Construct a CryptoService seeded with `jwt_secret`. Argon2-backed on
/// native (see `wafer_block_crypto::service::Argon2JwtCryptoService`).
pub fn make_jwt_crypto_service(jwt_secret: String) -> Arc<dyn CryptoService> {
    Arc::new(wafer_block_crypto::service::Argon2JwtCryptoService::new(
        jwt_secret,
    ))
}
