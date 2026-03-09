use std::time::Instant;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::RngExt;

#[derive(Debug, Clone)]
pub struct CrossInstanceOnboarding {
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChallengeRecord {
    pub challenge: String,
    pub did_key: String,
    pub created_at: Instant,
    pub cross_instance: Option<CrossInstanceOnboarding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyError {
    InvalidPublicKey,
    InvalidSignature,
}

pub fn generate_challenge() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    bytes_to_hex(&bytes)
}

pub fn verify_signature(
    public_key_bytes: &[u8; 32],
    challenge: &str,
    signature_bytes: &[u8; 64],
) -> Result<(), VerifyError> {
    if public_key_bytes.iter().all(|b| *b == 0) {
        return Err(VerifyError::InvalidPublicKey);
    }
    let vk =
        VerifyingKey::from_bytes(public_key_bytes).map_err(|_| VerifyError::InvalidPublicKey)?;
    let sig = Signature::from_bytes(signature_bytes);
    vk.verify(challenge.as_bytes(), &sig)
        .map_err(|_| VerifyError::InvalidSignature)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    #[test]
    fn generate_challenge_is_64_hex_chars() {
        let challenge = generate_challenge();
        assert_eq!(challenge.len(), 64);
        assert!(challenge.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn verify_signature_accepts_valid_signature() {
        let signing = SigningKey::from_bytes(&[1u8; 32]);
        let public = signing.verifying_key().to_bytes();

        let challenge = "aabbcc";
        let sig = signing.sign(challenge.as_bytes()).to_bytes();

        assert_eq!(verify_signature(&public, challenge, &sig), Ok(()));
    }

    #[test]
    fn verify_signature_rejects_wrong_challenge() {
        let signing = SigningKey::from_bytes(&[1u8; 32]);
        let public = signing.verifying_key().to_bytes();

        let sig = signing.sign(b"other").to_bytes();
        assert_eq!(
            verify_signature(&public, "aabbcc", &sig),
            Err(VerifyError::InvalidSignature)
        );
    }

    #[test]
    fn verify_signature_rejects_invalid_public_key() {
        let signing = SigningKey::from_bytes(&[1u8; 32]);
        let sig = signing.sign(b"aabbcc").to_bytes();

        assert_eq!(
            verify_signature(&[0u8; 32], "aabbcc", &sig),
            Err(VerifyError::InvalidPublicKey)
        );
    }
}
