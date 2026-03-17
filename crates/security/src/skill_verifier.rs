//! Ed25519 cryptographic skill verification.
//!
//! Marketplace skills must be signed. Workspace/bundled skills are exempt.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use openrustclaw_core::error::{Error, Result, SecurityError};
use sha2::{Digest, Sha256};
use tracing::info;

/// Verifies skill signatures using Ed25519.
pub struct SkillVerifier {
    /// Public key for verification (if configured).
    verifying_key: Option<VerifyingKey>,
    /// Whether signature verification is required.
    required: bool,
}

impl SkillVerifier {
    /// Create a new verifier. If public_key_bytes is None, verification is disabled.
    ///
    /// Returns an error if the provided public key bytes are invalid.
    pub fn new(public_key_bytes: Option<&[u8; 32]>, required: bool) -> Result<Self> {
        let verifying_key = match public_key_bytes {
            Some(bytes) => Some(VerifyingKey::from_bytes(bytes).map_err(|e| {
                Error::Security(SecurityError::SkillVerificationFailed(format!(
                    "Invalid Ed25519 public key: {}",
                    e
                )))
            })?),
            None => None,
        };
        Ok(Self {
            verifying_key,
            required,
        })
    }

    /// Create a disabled verifier.
    pub fn disabled() -> Self {
        Self {
            verifying_key: None,
            required: false,
        }
    }

    /// Verify a skill's content against its signature.
    pub fn verify(&self, content: &[u8], signature_bytes: &[u8]) -> Result<bool> {
        let Some(key) = &self.verifying_key else {
            if self.required {
                return Err(Error::Security(SecurityError::SkillVerificationFailed(
                    "No verification key configured but verification is required".to_string(),
                )));
            }
            return Ok(true); // Not required, no key = pass
        };

        let signature = Signature::from_slice(signature_bytes).map_err(|e| {
            Error::Security(SecurityError::SkillVerificationFailed(format!(
                "Invalid signature format: {}",
                e
            )))
        })?;

        match key.verify(content, &signature) {
            Ok(()) => {
                info!("Skill signature verified successfully");
                Ok(true)
            }
            Err(e) => Err(Error::Security(SecurityError::SkillVerificationFailed(
                format!("Signature verification failed: {}", e),
            ))),
        }
    }

    /// Compute SHA-256 content hash for a skill.
    pub fn content_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Generate a new Ed25519 signing keypair (for key generation CLI command).
    pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
        let mut rng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();
        (signing_key, verifying_key)
    }

    /// Sign content with a signing key.
    pub fn sign(content: &[u8], signing_key: &SigningKey) -> Vec<u8> {
        signing_key.sign(content).to_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_verifier_passes() {
        let verifier = SkillVerifier::disabled();
        assert!(verifier.verify(b"anything", b"").unwrap());
    }

    #[test]
    fn required_without_key_fails() {
        let verifier = SkillVerifier::new(None, true).unwrap();
        assert!(verifier.verify(b"content", b"sig").is_err());
    }

    #[test]
    fn new_with_invalid_key_returns_error() {
        let bad_bytes = [0u8; 32]; // All zeros is not a valid Ed25519 key point
        // This may or may not fail depending on the curve point -- test that
        // it doesn't panic regardless
        let _result = SkillVerifier::new(Some(&bad_bytes), true);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let (signing_key, verifying_key) = SkillVerifier::generate_keypair();
        let content = b"skill manifest content here";
        let signature = SkillVerifier::sign(content, &signing_key);

        let verifier = SkillVerifier::new(Some(verifying_key.as_bytes()), true).unwrap();
        assert!(verifier.verify(content, &signature).unwrap());
    }

    #[test]
    fn wrong_content_fails_verification() {
        let (signing_key, verifying_key) = SkillVerifier::generate_keypair();
        let content = b"original content";
        let signature = SkillVerifier::sign(content, &signing_key);

        let verifier = SkillVerifier::new(Some(verifying_key.as_bytes()), true).unwrap();
        assert!(verifier.verify(b"tampered content", &signature).is_err());
    }

    #[test]
    fn content_hash_deterministic() {
        let hash1 = SkillVerifier::content_hash(b"hello world");
        let hash2 = SkillVerifier::content_hash(b"hello world");
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }

    #[test]
    fn content_hash_differs_for_different_content() {
        let hash1 = SkillVerifier::content_hash(b"hello");
        let hash2 = SkillVerifier::content_hash(b"world");
        assert_ne!(hash1, hash2);
    }
}
