#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    InvalidPoint,
}

pub fn validate_ed25519_public_key(bytes: &[u8; 32]) -> Result<(), KeyError> {
    if bytes.iter().all(|b| *b == 0) {
        return Err(KeyError::InvalidPoint);
    }

    ed25519_dalek::VerifyingKey::from_bytes(bytes)
        .map(|_| ())
        .map_err(|_| KeyError::InvalidPoint)
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{SigningKey, VerifyingKey};

    use super::*;

    #[test]
    fn accepts_valid_public_key() {
        let signing = SigningKey::from_bytes(&[1u8; 32]);
        let bytes = signing.verifying_key().to_bytes();
        assert!(validate_ed25519_public_key(&bytes).is_ok());
    }

    #[test]
    fn rejects_all_zeros_public_key() {
        assert_eq!(
            validate_ed25519_public_key(&[0u8; 32]),
            Err(KeyError::InvalidPoint)
        );
    }

    #[test]
    fn rejects_some_other_invalid_bytes() {
        for i in 1u8..=255 {
            let bytes = [i; 32];
            if VerifyingKey::from_bytes(&bytes).is_err() {
                assert_eq!(
                    validate_ed25519_public_key(&bytes),
                    Err(KeyError::InvalidPoint)
                );
                return;
            }
        }

        panic!("expected to find an invalid Ed25519 public key encoding");
    }
}
