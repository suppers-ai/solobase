//! PKCE (RFC 7636) S256 helpers — code_verifier generation + code_challenge
//! derivation. Kept synchronous and context-free so the OAuth login handler
//! can build a verifier on the fly without reaching into the crypto block
//! for every value.

use base64ct::{Base64UrlUnpadded, Encoding};
use sha2::{Digest, Sha256};

/// Generates a fresh PKCE code verifier: 32 random bytes encoded as
/// base64url (unpadded) — 43 chars, within RFC 7636's 43–128 range and
/// using only the [A-Za-z0-9\-_] alphabet.
pub(super) fn new_verifier() -> String {
    let mut bytes = [0u8; 32];
    // `getrandom` is already a dep of solobase-core and used by the auth
    // block elsewhere; it pulls from the OS CSPRNG on native and
    // `crypto.getRandomValues` on wasm32.
    getrandom::getrandom(&mut bytes).expect("OS CSPRNG");
    Base64UrlUnpadded::encode_string(&bytes)
}

/// S256 challenge: base64url(unpadded)(sha256(verifier)). The verifier is
/// hashed as raw bytes of its ASCII representation per RFC 7636 §4.2.
pub(super) fn challenge_for(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    Base64UrlUnpadded::encode_string(&digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_is_base64url_43_to_128_chars() {
        let v = new_verifier();
        assert!(v.len() >= 43 && v.len() <= 128, "len was {}", v.len());
        assert!(
            v.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "verifier has non-base64url char: {v}"
        );
    }

    #[test]
    fn challenge_is_sha256_base64url_of_verifier() {
        // RFC 7636 Appendix B test vector:
        // verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        // challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        let v = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            challenge_for(v),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn new_verifier_produces_distinct_values() {
        let a = new_verifier();
        let b = new_verifier();
        assert_ne!(a, b);
    }

    #[test]
    fn challenge_round_trips_new_verifier() {
        let v = new_verifier();
        let c = challenge_for(&v);
        // 32 bytes → base64url(32) = 43 chars unpadded.
        assert_eq!(c.len(), 43);
    }
}
