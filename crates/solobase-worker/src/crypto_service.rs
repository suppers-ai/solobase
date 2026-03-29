use std::collections::HashMap;
use std::time::Duration;
use wafer_core::interfaces::crypto::service::{CryptoError, CryptoService};

/// CryptoService using solobase_core::crypto (works on both native and WASM).
pub struct SolobaseCryptoService {
    jwt_secret: String,
}

// Safety: wasm32-unknown-unknown is single-threaded.
unsafe impl Send for SolobaseCryptoService {}
unsafe impl Sync for SolobaseCryptoService {}

impl SolobaseCryptoService {
    pub fn new(jwt_secret: String) -> Self {
        Self { jwt_secret }
    }
}

impl CryptoService for SolobaseCryptoService {
    fn hash(&self, password: &str) -> Result<String, CryptoError> {
        solobase_core::crypto::hash_password(password)
            .map_err(CryptoError::HashError)
    }

    fn compare_hash(&self, password: &str, hash: &str) -> Result<(), CryptoError> {
        if solobase_core::crypto::verify_password(password, hash) {
            Ok(())
        } else {
            Err(CryptoError::PasswordMismatch)
        }
    }

    fn sign(
        &self,
        claims: HashMap<String, serde_json::Value>,
        expiry: Duration,
    ) -> Result<String, CryptoError> {
        Ok(solobase_core::crypto::jwt_sign(&claims, expiry, &self.jwt_secret))
    }

    fn verify(
        &self,
        token: &str,
    ) -> Result<HashMap<String, serde_json::Value>, CryptoError> {
        solobase_core::crypto::jwt_verify(token, &self.jwt_secret)
            .map_err(CryptoError::VerifyError)
    }

    fn random_bytes(&self, n: usize) -> Result<Vec<u8>, CryptoError> {
        solobase_core::crypto::random_bytes(n)
            .map_err(CryptoError::Other)
    }
}
