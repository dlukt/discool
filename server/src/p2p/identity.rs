use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use libp2p::{PeerId, identity::Keypair};

#[derive(Debug, Clone)]
pub struct InstanceIdentity {
    pub keypair: Keypair,
    pub peer_id: PeerId,
}

#[derive(Debug)]
pub enum IdentityError {
    Read {
        path: String,
        source: std::io::Error,
    },
    Write {
        path: String,
        source: std::io::Error,
    },
    Encode(String),
    Decode(String),
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentityError::Read { path, source } => {
                write!(f, "failed to read identity key from {path}: {source}")
            }
            IdentityError::Write { path, source } => {
                write!(f, "failed to write identity key to {path}: {source}")
            }
            IdentityError::Encode(msg) => write!(f, "failed to encode identity key: {msg}"),
            IdentityError::Decode(msg) => write!(f, "failed to decode identity key: {msg}"),
        }
    }
}

impl std::error::Error for IdentityError {}

pub fn load_or_create_identity(
    identity_key_path: &Path,
) -> Result<InstanceIdentity, IdentityError> {
    let keypair = if identity_key_path.exists() {
        load_identity(identity_key_path)?
    } else {
        create_identity(identity_key_path)?
    };
    let peer_id = keypair.public().to_peer_id();
    Ok(InstanceIdentity { keypair, peer_id })
}

fn load_identity(identity_key_path: &Path) -> Result<Keypair, IdentityError> {
    let bytes = fs::read(identity_key_path).map_err(|source| IdentityError::Read {
        path: identity_key_path.display().to_string(),
        source,
    })?;
    Keypair::from_protobuf_encoding(&bytes).map_err(|err| IdentityError::Decode(err.to_string()))
}

fn create_identity(identity_key_path: &Path) -> Result<Keypair, IdentityError> {
    if let Some(parent_dir) = identity_key_path.parent()
        && !parent_dir.as_os_str().is_empty()
    {
        fs::create_dir_all(parent_dir).map_err(|source| IdentityError::Write {
            path: parent_dir.display().to_string(),
            source,
        })?;
    }

    let keypair = Keypair::generate_ed25519();
    let encoded = keypair
        .to_protobuf_encoding()
        .map_err(|err| IdentityError::Encode(err.to_string()))?;

    let temp_path = identity_key_path.with_extension("tmp");
    let mut file = create_private_file(&temp_path)?;
    file.write_all(&encoded)
        .map_err(|source| IdentityError::Write {
            path: temp_path.display().to_string(),
            source,
        })?;
    file.sync_all().map_err(|source| IdentityError::Write {
        path: temp_path.display().to_string(),
        source,
    })?;

    fs::rename(&temp_path, identity_key_path).map_err(|source| IdentityError::Write {
        path: identity_key_path.display().to_string(),
        source,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(identity_key_path, fs::Permissions::from_mode(0o600)).map_err(
            |source| IdentityError::Write {
                path: identity_key_path.display().to_string(),
                source,
            },
        )?;
    }

    Ok(keypair)
}

fn create_private_file(path: &Path) -> Result<File, IdentityError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(|source| IdentityError::Write {
                path: path.display().to_string(),
                source,
            })
    }

    #[cfg(not(unix))]
    {
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|source| IdentityError::Write {
                path: path.display().to_string(),
                source,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn new_temp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "discool-p2p-identity-test-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn generates_new_key_when_file_is_missing() {
        let dir = new_temp_dir();
        let identity_path = dir.join("identity.key");

        let identity = load_or_create_identity(&identity_path).unwrap();
        assert!(identity_path.exists());
        assert_eq!(identity.keypair.public().to_peer_id(), identity.peer_id);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::metadata(&identity_path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reloads_same_key_and_peer_id_across_restarts() {
        let dir = new_temp_dir();
        let identity_path = dir.join("identity.key");

        let first = load_or_create_identity(&identity_path).unwrap();
        let second = load_or_create_identity(&identity_path).unwrap();

        assert_eq!(first.peer_id, second.peer_id);
        assert_eq!(first.keypair.public(), second.keypair.public());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_malformed_key_file() {
        let dir = new_temp_dir();
        let identity_path = dir.join("identity.key");
        fs::write(&identity_path, b"not-a-protobuf-key").unwrap();

        let err = load_or_create_identity(&identity_path).unwrap_err();
        assert!(
            matches!(err, IdentityError::Decode(_)),
            "expected decode error, got: {err}"
        );

        let _ = fs::remove_dir_all(dir);
    }
}
