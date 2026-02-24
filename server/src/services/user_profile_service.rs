use std::path::{Path, PathBuf};

use chrono::Utc;
use tokio::fs;
use uuid::Uuid;

use crate::{
    AppError,
    config::AvatarConfig,
    db::DbPool,
    models::user::{User, UserResponse},
};

const MAX_DISPLAY_NAME_CHARS: usize = 64;

#[derive(Debug, Clone)]
pub struct UpdateProfileInput {
    pub display_name: Option<Option<String>>,
    pub avatar_color: Option<Option<String>>,
}

pub async fn get_profile(pool: &DbPool, user_id: &str) -> Result<UserResponse, AppError> {
    let user = fetch_user_by_id(pool, user_id).await?;
    Ok(UserResponse::from(user))
}

pub async fn update_profile(
    pool: &DbPool,
    avatar_config: &AvatarConfig,
    user_id: &str,
    input: UpdateProfileInput,
) -> Result<UserResponse, AppError> {
    if input.display_name.is_none() && input.avatar_color.is_none() {
        return Err(AppError::ValidationError(
            "At least one profile field is required".to_string(),
        ));
    }

    let user = fetch_user_by_id(pool, user_id).await?;
    let old_avatar_storage_key = user.avatar_storage_key.clone();

    let mut display_name = user.display_name.clone();
    if let Some(display_name_input) = input.display_name {
        display_name = match display_name_input {
            Some(value) => Some(normalize_display_name(&value)?),
            None => None,
        };
    }

    let mut avatar_color = user.avatar_color.clone();
    let mut avatar_storage_key = user.avatar_storage_key.clone();
    let mut avatar_mime_type = user.avatar_mime_type.clone();
    let mut avatar_size_bytes = user.avatar_size_bytes;
    let mut avatar_updated_at = user.avatar_updated_at.clone();
    let avatar_color_updated = input.avatar_color.is_some();
    if let Some(avatar_color_input) = input.avatar_color {
        avatar_color = normalize_avatar_color(avatar_color_input.as_deref())?;
    }
    if avatar_color_updated {
        avatar_storage_key = None;
        avatar_mime_type = None;
        avatar_size_bytes = None;
        avatar_updated_at = None;
    }

    let updated_at = Utc::now().to_rfc3339();
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE users SET display_name = $1, avatar_color = $2, avatar_storage_key = $3, avatar_mime_type = $4, avatar_size_bytes = $5, avatar_updated_at = $6, updated_at = $7 WHERE id = $8",
        )
        .bind(display_name.as_deref())
        .bind(avatar_color.as_deref())
        .bind(avatar_storage_key.as_deref())
        .bind(avatar_mime_type.as_deref())
        .bind(avatar_size_bytes)
        .bind(avatar_updated_at.as_deref())
        .bind(&updated_at)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE users SET display_name = ?1, avatar_color = ?2, avatar_storage_key = ?3, avatar_mime_type = ?4, avatar_size_bytes = ?5, avatar_updated_at = ?6, updated_at = ?7 WHERE id = ?8",
        )
        .bind(display_name.as_deref())
        .bind(avatar_color.as_deref())
        .bind(avatar_storage_key.as_deref())
        .bind(avatar_mime_type.as_deref())
        .bind(avatar_size_bytes)
        .bind(avatar_updated_at.as_deref())
        .bind(&updated_at)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    if rows == 0 {
        return Err(AppError::NotFound);
    }

    if avatar_color_updated && let Some(old_key) = old_avatar_storage_key {
        if storage_key_is_safe(&old_key) {
            let old_path = avatar_file_path(&avatar_config.upload_dir, &old_key);
            if let Err(err) = fs::remove_file(old_path).await
                && err.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!(error = %err, "Failed to remove old avatar file");
            }
        } else {
            tracing::warn!("Skipping old avatar file cleanup due to invalid storage key");
        }
    }

    let mut updated_user = user;
    updated_user.display_name = display_name;
    updated_user.avatar_color = avatar_color;
    updated_user.avatar_storage_key = avatar_storage_key;
    updated_user.avatar_mime_type = avatar_mime_type;
    updated_user.avatar_size_bytes = avatar_size_bytes;
    updated_user.avatar_updated_at = avatar_updated_at;
    updated_user.updated_at = updated_at;
    Ok(UserResponse::from(updated_user))
}

pub async fn save_avatar(
    pool: &DbPool,
    avatar_config: &AvatarConfig,
    user_id: &str,
    declared_content_type: Option<&str>,
    bytes: &[u8],
) -> Result<UserResponse, AppError> {
    if bytes.is_empty() {
        return Err(AppError::ValidationError(
            "Avatar file is required".to_string(),
        ));
    }
    if bytes.len() > avatar_config.max_size_bytes {
        return Err(AppError::ValidationError(format!(
            "Avatar file exceeds the {} byte limit",
            avatar_config.max_size_bytes
        )));
    }

    let sniffed_mime = sniff_image_mime(bytes)
        .ok_or_else(|| AppError::ValidationError("Unsupported avatar image type".to_string()))?;

    let declared_mime = match declared_content_type {
        Some(value) => Some(normalize_declared_mime(value)?),
        None => None,
    };
    if let Some(declared_mime) = declared_mime
        && declared_mime != sniffed_mime
    {
        return Err(AppError::ValidationError(
            "Avatar MIME type does not match file content".to_string(),
        ));
    }

    let user = fetch_user_by_id(pool, user_id).await?;
    let extension = extension_for_mime(sniffed_mime);
    let storage_key = format!("{}.{}", Uuid::new_v4(), extension);
    let avatar_path = avatar_file_path(&avatar_config.upload_dir, &storage_key);
    fs::write(&avatar_path, bytes)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let updated_at = Utc::now().to_rfc3339();
    let avatar_size_bytes: i64 = bytes
        .len()
        .try_into()
        .map_err(|_| AppError::Internal("Avatar file size is too large".to_string()))?;

    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "UPDATE users SET avatar_storage_key = $1, avatar_mime_type = $2, avatar_size_bytes = $3, avatar_updated_at = $4, updated_at = $5 WHERE id = $6",
            )
            .bind(&storage_key)
            .bind(sniffed_mime)
            .bind(avatar_size_bytes)
            .bind(&updated_at)
            .bind(&updated_at)
            .bind(user_id)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "UPDATE users SET avatar_storage_key = ?1, avatar_mime_type = ?2, avatar_size_bytes = ?3, avatar_updated_at = ?4, updated_at = ?5 WHERE id = ?6",
            )
            .bind(&storage_key)
            .bind(sniffed_mime)
            .bind(avatar_size_bytes)
            .bind(&updated_at)
            .bind(&updated_at)
            .bind(user_id)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    if rows == 0 {
        if let Err(err) = fs::remove_file(&avatar_path).await {
            tracing::warn!(error = %err, path = %avatar_path.display(), "Failed to clean up avatar file after missing user");
        }
        return Err(AppError::NotFound);
    }

    if let Some(old_key) = user.avatar_storage_key
        && old_key != storage_key
    {
        if storage_key_is_safe(&old_key) {
            let old_path = avatar_file_path(&avatar_config.upload_dir, &old_key);
            if let Err(err) = fs::remove_file(old_path).await
                && err.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!(error = %err, "Failed to remove old avatar file");
            }
        } else {
            tracing::warn!("Skipping old avatar file cleanup due to invalid storage key");
        }
    }

    get_profile(pool, user_id).await
}

pub async fn load_avatar(
    pool: &DbPool,
    avatar_config: &AvatarConfig,
    user_id: &str,
) -> Result<(Vec<u8>, String), AppError> {
    let user = fetch_user_by_id(pool, user_id).await?;
    let Some(storage_key) = user.avatar_storage_key else {
        return Err(AppError::NotFound);
    };
    if !storage_key_is_safe(&storage_key) {
        return Err(AppError::Internal("Invalid avatar storage key".to_string()));
    }

    let mime = user
        .avatar_mime_type
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let path = avatar_file_path(&avatar_config.upload_dir, &storage_key);
    let bytes = fs::read(path).await.map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound
        } else {
            AppError::Internal(err.to_string())
        }
    })?;

    Ok((bytes, mime))
}

async fn fetch_user_by_id(pool: &DbPool, user_id: &str) -> Result<User, AppError> {
    let user: Option<User> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, did_key, public_key_multibase, username, display_name, avatar_color, avatar_storage_key, avatar_mime_type, avatar_size_bytes, avatar_updated_at, created_at, updated_at FROM users WHERE id = $1 LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, did_key, public_key_multibase, username, display_name, avatar_color, avatar_storage_key, avatar_mime_type, avatar_size_bytes, avatar_updated_at, created_at, updated_at FROM users WHERE id = ?1 LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    user.ok_or(AppError::NotFound)
}

fn normalize_display_name(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError(
            "display_name cannot be empty".to_string(),
        ));
    }
    if trimmed.chars().count() > MAX_DISPLAY_NAME_CHARS {
        return Err(AppError::ValidationError(format!(
            "display_name must be {MAX_DISPLAY_NAME_CHARS} characters or less"
        )));
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "display_name contains invalid characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_avatar_color(value: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(value) = value.map(str::trim) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    if !is_hex_color(value) {
        return Err(AppError::ValidationError(
            "avatar_color must be a hex color like #3399ff".to_string(),
        ));
    }
    Ok(Some(value.to_string()))
}

fn is_hex_color(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return false;
    }
    bytes[1..].iter().all(|b| b.is_ascii_hexdigit())
}

fn normalize_declared_mime(value: &str) -> Result<&'static str, AppError> {
    let base = value
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    match base.as_str() {
        "image/png" => Ok("image/png"),
        "image/jpeg" | "image/jpg" => Ok("image/jpeg"),
        "image/webp" => Ok("image/webp"),
        _ => Err(AppError::ValidationError(
            "Unsupported avatar image type".to_string(),
        )),
    }
}

fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some("image/jpeg");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "bin",
    }
}

fn avatar_file_path(upload_dir: &str, storage_key: &str) -> PathBuf {
    Path::new(upload_dir).join(storage_key)
}

fn storage_key_is_safe(storage_key: &str) -> bool {
    !storage_key.contains('/') && !storage_key.contains('\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_image_mime_detects_supported_formats() {
        assert_eq!(
            sniff_image_mime(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0]),
            Some("image/png")
        );
        assert_eq!(
            sniff_image_mime(&[0xFF, 0xD8, 0xFF, 0x00]),
            Some("image/jpeg")
        );
        assert_eq!(sniff_image_mime(b"RIFF1234WEBPrest"), Some("image/webp"));
        assert_eq!(sniff_image_mime(b"hello"), None);
    }

    #[test]
    fn normalize_declared_mime_rejects_unsupported_values() {
        assert_eq!(normalize_declared_mime("image/png").unwrap(), "image/png");
        assert_eq!(normalize_declared_mime("image/jpeg").unwrap(), "image/jpeg");
        assert_eq!(normalize_declared_mime("image/jpg").unwrap(), "image/jpeg");
        assert_eq!(normalize_declared_mime("image/webp").unwrap(), "image/webp");
        assert!(normalize_declared_mime("image/gif").is_err());
    }
}
