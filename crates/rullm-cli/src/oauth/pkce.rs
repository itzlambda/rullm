//! PKCE (Proof Key for Code Exchange) implementation for OAuth 2.0.
//!
//! Implements RFC 7636 for secure authorization code flow.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// PKCE challenge pair consisting of verifier and challenge.
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    /// The code verifier (sent with token request)
    pub verifier: String,
    /// The code challenge (sent with authorization request)
    pub challenge: String,
}

impl PkceChallenge {
    /// Generate a new PKCE challenge pair.
    ///
    /// Creates a 64-byte random code verifier and derives the S256 challenge from it.
    pub fn generate() -> Self {
        // Generate 64 random bytes for the code verifier
        let mut verifier_bytes = [0u8; 64];
        rand::rng().fill_bytes(&mut verifier_bytes);

        // Base64url encode the verifier (no padding)
        let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

        // Create code challenge: base64url(sha256(verifier))
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let challenge = URL_SAFE_NO_PAD.encode(hash);

        Self {
            verifier,
            challenge,
        }
    }

    /// Get the challenge method (always "S256").
    pub fn method(&self) -> &'static str {
        "S256"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = PkceChallenge::generate();

        // Verifier should be base64url encoded 64 bytes = 86 chars
        assert_eq!(pkce.verifier.len(), 86);

        // Challenge should be base64url encoded SHA256 = 43 chars
        assert_eq!(pkce.challenge.len(), 43);

        // Method should be S256
        assert_eq!(pkce.method(), "S256");
    }

    #[test]
    fn test_pkce_uniqueness() {
        let pkce1 = PkceChallenge::generate();
        let pkce2 = PkceChallenge::generate();

        // Each generation should produce unique values
        assert_ne!(pkce1.verifier, pkce2.verifier);
        assert_ne!(pkce1.challenge, pkce2.challenge);
    }

    #[test]
    fn test_challenge_derivation() {
        // Verify that the challenge is correctly derived from verifier
        let pkce = PkceChallenge::generate();

        // Manually compute the expected challenge
        let mut hasher = Sha256::new();
        hasher.update(pkce.verifier.as_bytes());
        let hash = hasher.finalize();
        let expected_challenge = URL_SAFE_NO_PAD.encode(hash);

        assert_eq!(pkce.challenge, expected_challenge);
    }
}
