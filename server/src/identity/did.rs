#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DidError {
    InvalidPrefix,
    InvalidMultibase,
    InvalidMulticodec,
    InvalidKeyLength,
}

/// Parse an Ed25519 `did:key:` DID and return the raw 32-byte public key.
pub fn parse_did_key(did: &str) -> Result<[u8; 32], DidError> {
    let did = did.trim();

    let Some(multibase) = did.strip_prefix("did:key:") else {
        return Err(DidError::InvalidPrefix);
    };
    let Some(base58btc) = multibase.strip_prefix('z') else {
        return Err(DidError::InvalidMultibase);
    };

    let decoded = bs58::decode(base58btc)
        .into_vec()
        .map_err(|_| DidError::InvalidMultibase)?;

    let payload = decoded
        .as_slice()
        .strip_prefix(&[0xed, 0x01])
        .ok_or(DidError::InvalidMulticodec)?;

    if payload.len() != 32 {
        return Err(DidError::InvalidKeyLength);
    }

    let mut out = [0u8; 32];
    out.copy_from_slice(payload);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn did_from_public_key(public_key: [u8; 32]) -> String {
        let mut bytes = Vec::with_capacity(34);
        bytes.extend_from_slice(&[0xed, 0x01]);
        bytes.extend_from_slice(&public_key);
        format!("did:key:z{}", bs58::encode(bytes).into_string())
    }

    #[test]
    fn parses_valid_did() {
        let public_key = [1u8; 32];
        let did = did_from_public_key(public_key);
        assert_eq!(parse_did_key(&did).unwrap(), public_key);
    }

    #[test]
    fn rejects_wrong_prefix() {
        assert_eq!(
            parse_did_key("did:web:example.com"),
            Err(DidError::InvalidPrefix)
        );
    }

    #[test]
    fn rejects_truncated_key() {
        let mut bytes = Vec::with_capacity(33);
        bytes.extend_from_slice(&[0xed, 0x01]);
        bytes.extend_from_slice(&[1u8; 31]);
        let did = format!("did:key:z{}", bs58::encode(bytes).into_string());
        assert_eq!(parse_did_key(&did), Err(DidError::InvalidKeyLength));
    }

    #[test]
    fn rejects_invalid_base58() {
        assert_eq!(
            parse_did_key("did:key:znot-base58!!!"),
            Err(DidError::InvalidMultibase)
        );
    }

    #[test]
    fn rejects_missing_multibase_prefix() {
        let did = did_from_public_key([1u8; 32]);
        let did = did.replacen("did:key:z", "did:key:", 1);
        assert_eq!(parse_did_key(&did), Err(DidError::InvalidMultibase));
    }

    #[test]
    fn rejects_wrong_multicodec() {
        let public_key = [2u8; 32];
        let mut bytes = Vec::with_capacity(34);
        bytes.extend_from_slice(&[0xef, 0x01]);
        bytes.extend_from_slice(&public_key);
        let did = format!("did:key:z{}", bs58::encode(bytes).into_string());
        assert_eq!(parse_did_key(&did), Err(DidError::InvalidMulticodec));
    }
}
