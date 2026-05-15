//! Browser-optimized CryptoService using PBKDF2 for password hashing.
//!
//! The native Solobase binary uses Argon2id (via wafer-block-crypto) which is
//! too slow in WASM (~3+ minutes with default params). This service uses
//! PBKDF2-HMAC-SHA256 instead — fast, sync, pure Rust, and a well-established
//! standard (NIST SP 800-132).
//!
//! JWT signing reuses the pure-Rust HS256 implementation from wafer-block-crypto.
//!
//! Since browser data is local-only (no sync to native), hash format
//! differences don't matter.

use std::{collections::HashMap, time::Duration};

use wafer_core::interfaces::crypto::service::{CryptoError, CryptoService};

/// PBKDF2-HMAC-SHA256 iteration count.
///
/// OWASP's 2023 Password Storage Cheat Sheet recommends 600,000 iterations
/// for PBKDF2-SHA256. NIST SP 800-132 sets 10,000 as the absolute floor.
/// The previous value here (1,000) was below NIST's minimum.
///
/// 600,000 iterations runs in ~1-2 seconds in single-threaded WASM on a
/// modern laptop. That's acceptable because PBKDF2 is invoked only on
/// user-visible actions (login / password change) where the browser tab
/// is already blocking for input — not on every request.
const PBKDF2_ITERATIONS: u32 = 600_000;
const PBKDF2_HASH_LEN: usize = 32;
const PBKDF2_SALT_LEN: usize = 16;

pub struct BrowserCryptoService {
    jwt_secret: String,
}

// SAFETY: `BrowserCryptoService` only holds owned data (`String`).
// wasm32-unknown-unknown has no threads, so the `Send`/`Sync` bounds
// required by `Arc<dyn CryptoService>` are satisfied trivially — no
// cross-thread aliasing or data races are possible.
unsafe impl Send for BrowserCryptoService {}
unsafe impl Sync for BrowserCryptoService {}

impl BrowserCryptoService {
    pub fn new(jwt_secret: String) -> Self {
        Self { jwt_secret }
    }

    fn derive_block_key(&self, block_id: &str) -> Result<String, CryptoError> {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let hk = Hkdf::<Sha256>::new(None, self.jwt_secret.as_bytes());
        let info = format!("wafer-jwt|{block_id}");
        let mut okm = [0u8; 32];
        hk.expand(info.as_bytes(), &mut okm)
            .map_err(|_| CryptoError::Other("HKDF expand failed".to_string()))?;
        Ok(okm.iter().map(|b| format!("{b:02x}")).collect())
    }
}

impl CryptoService for BrowserCryptoService {
    fn hash(&self, password: &str) -> Result<String, CryptoError> {
        use base64ct::{Base64, Encoding};
        use hmac::Hmac;
        use sha2::Sha256;

        // Generate random salt
        let mut salt = [0u8; PBKDF2_SALT_LEN];
        getrandom::getrandom(&mut salt).map_err(|e| CryptoError::HashError(e.to_string()))?;

        // Derive key
        let mut hash = [0u8; PBKDF2_HASH_LEN];
        pbkdf2::pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &salt, PBKDF2_ITERATIONS, &mut hash)
            .map_err(|e| CryptoError::HashError(e.to_string()))?;

        // Encode as PHC-like string: $pbkdf2-sha256$i=100000$<base64-salt>$<base64-hash>
        let salt_b64 = Base64::encode_string(&salt);
        let hash_b64 = Base64::encode_string(&hash);
        Ok(format!(
            "$pbkdf2-sha256$i={PBKDF2_ITERATIONS}${salt_b64}${hash_b64}"
        ))
    }

    fn compare_hash(&self, password: &str, hash_str: &str) -> Result<(), CryptoError> {
        use base64ct::{Base64, Encoding};
        use hmac::Hmac;
        use sha2::Sha256;
        use subtle::ConstantTimeEq;

        // Parse the PHC string
        let parts: Vec<&str> = hash_str.split('$').collect();
        // Format: ["", "pbkdf2-sha256", "i=100000", "<salt>", "<hash>"]
        if parts.len() != 5 || parts[1] != "pbkdf2-sha256" {
            return Err(CryptoError::VerifyError(
                "unsupported hash format".to_string(),
            ));
        }

        let iterations: u32 = parts[2]
            .strip_prefix("i=")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| CryptoError::VerifyError("invalid iteration count".to_string()))?;

        let salt = Base64::decode_vec(parts[3])
            .map_err(|e| CryptoError::VerifyError(format!("invalid salt: {e}")))?;
        let expected = Base64::decode_vec(parts[4])
            .map_err(|e| CryptoError::VerifyError(format!("invalid hash: {e}")))?;

        // Derive
        let mut computed = vec![0u8; expected.len()];
        pbkdf2::pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &salt, iterations, &mut computed)
            .map_err(|e| CryptoError::VerifyError(e.to_string()))?;

        // Constant-time comparison via the dedicated `subtle` crate.
        // (Previous implementation went through HMAC finalize + `CtOutput`
        // equality, which is also constant-time but obscured the intent.)
        if computed.ct_eq(&expected).into() {
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
        // Delegate to the same HS256 implementation used by the native crypto
        // service. `new` now enforces the 32-byte minimum JWT secret length;
        // a too-short secret here is a deployment misconfig and the error
        // bubbles up to the caller.
        wafer_block_crypto::service::Argon2JwtCryptoService::new(self.jwt_secret.clone())?
            .sign(claims, expiry)
    }

    fn verify(&self, token: &str) -> Result<HashMap<String, serde_json::Value>, CryptoError> {
        wafer_block_crypto::service::Argon2JwtCryptoService::new(self.jwt_secret.clone())?
            .verify(token)
    }

    fn sign_for(
        &self,
        block_id: &str,
        claims: HashMap<String, serde_json::Value>,
        expiry: Duration,
    ) -> Result<String, CryptoError> {
        let derived = self.derive_block_key(block_id)?;
        let temp = BrowserCryptoService::new(derived);
        temp.sign(claims, expiry)
    }

    fn verify_for(
        &self,
        block_id: &str,
        token: &str,
    ) -> Result<HashMap<String, serde_json::Value>, CryptoError> {
        let derived = self.derive_block_key(block_id)?;
        let temp = BrowserCryptoService::new(derived);
        temp.verify(token)
    }

    fn random_bytes(&self, n: usize) -> Result<Vec<u8>, CryptoError> {
        let mut buf = vec![0u8; n];
        getrandom::getrandom(&mut buf).map_err(|e| CryptoError::Other(e.to_string()))?;
        Ok(buf)
    }
}

/// Factory: returns an `Arc<dyn CryptoService>` seeded with `jwt_secret`.
/// The secret is used for HMAC-based JWT signing. It is the caller's
/// responsibility to source this secret (typically from an
/// `SUPPERS_AI__AUTH__JWT_SECRET` config var).
pub fn make_crypto_service(
    jwt_secret: String,
) -> std::sync::Arc<dyn wafer_core::interfaces::crypto::service::CryptoService> {
    std::sync::Arc::new(BrowserCryptoService::new(jwt_secret))
}
