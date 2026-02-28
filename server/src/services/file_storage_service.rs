use std::path::{Path, PathBuf};

use tokio::fs;

use crate::AppError;

#[derive(Debug, Clone)]
pub enum FileStorageProvider {
    Local(LocalFileStorageProvider),
}

impl FileStorageProvider {
    pub fn local(root_dir: impl Into<String>) -> Self {
        Self::Local(LocalFileStorageProvider::new(root_dir.into()))
    }

    pub async fn write(&self, storage_key: &str, bytes: &[u8]) -> Result<(), AppError> {
        match self {
            Self::Local(provider) => provider.write(storage_key, bytes).await,
        }
    }

    pub async fn read(&self, storage_key: &str) -> Result<Vec<u8>, AppError> {
        match self {
            Self::Local(provider) => provider.read(storage_key).await,
        }
    }

    pub async fn delete(&self, storage_key: &str) -> Result<(), AppError> {
        match self {
            Self::Local(provider) => provider.delete(storage_key).await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalFileStorageProvider {
    root_dir: String,
}

impl LocalFileStorageProvider {
    pub fn new(root_dir: String) -> Self {
        Self { root_dir }
    }

    pub async fn write(&self, storage_key: &str, bytes: &[u8]) -> Result<(), AppError> {
        validate_storage_key(storage_key)?;
        fs::create_dir_all(&self.root_dir)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        fs::write(self.file_path(storage_key), bytes)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    pub async fn read(&self, storage_key: &str) -> Result<Vec<u8>, AppError> {
        validate_storage_key(storage_key)?;
        fs::read(self.file_path(storage_key)).await.map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound
            } else {
                AppError::Internal(err.to_string())
            }
        })
    }

    pub async fn delete(&self, storage_key: &str) -> Result<(), AppError> {
        validate_storage_key(storage_key)?;
        let path = self.file_path(storage_key);
        if let Err(err) = fs::remove_file(path).await
            && err.kind() != std::io::ErrorKind::NotFound
        {
            return Err(AppError::Internal(err.to_string()));
        }
        Ok(())
    }

    fn file_path(&self, storage_key: &str) -> PathBuf {
        Path::new(&self.root_dir).join(storage_key)
    }
}

pub fn validate_storage_key(storage_key: &str) -> Result<(), AppError> {
    let trimmed = storage_key.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError(
            "storage key must not be empty".to_string(),
        ));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(AppError::ValidationError(
            "storage key contains invalid path separators".to_string(),
        ));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(AppError::ValidationError(
            "storage key contains invalid path segments".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs as std_fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("discool-file-storage-test-{nanos}"))
    }

    #[test]
    fn validate_storage_key_rejects_unsafe_values() {
        assert!(validate_storage_key("").is_err());
        assert!(validate_storage_key("dir/file.png").is_err());
        assert!(validate_storage_key("dir\\file.png").is_err());
        assert!(validate_storage_key(".").is_err());
        assert!(validate_storage_key("..").is_err());
        assert!(validate_storage_key("safe-file.png").is_ok());
    }

    #[tokio::test]
    async fn local_provider_writes_reads_and_deletes_files() {
        let root = temp_dir();
        let provider = LocalFileStorageProvider::new(root.to_string_lossy().to_string());
        let storage_key = "attachment-1.png";
        let bytes = b"hello-bytes";

        provider.write(storage_key, bytes).await.unwrap();
        let loaded = provider.read(storage_key).await.unwrap();
        assert_eq!(loaded, bytes);

        provider.delete(storage_key).await.unwrap();
        let missing = provider.read(storage_key).await;
        assert!(matches!(missing, Err(AppError::NotFound)));

        let _ = std_fs::remove_dir_all(&root);
    }
}
