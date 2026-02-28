use std::path::{Path, PathBuf};

use chrono::Utc;
use tokio::fs;
use uuid::Uuid;

use crate::{
    AppError,
    config::AvatarConfig,
    db::DbPool,
    models::{
        channel,
        guild::{self, Guild, GuildResponse},
        role,
    },
};

const MAX_GUILD_NAME_CHARS: usize = 64;
const MAX_GUILD_DESCRIPTION_CHARS: usize = 512;
const MAX_GUILD_SLUG_CHARS: usize = 48;
const MAX_GUILD_SLUG_ATTEMPTS: usize = 100;
const DEFAULT_CHANNEL_SLUG: &str = "general";
const DEFAULT_EVERYONE_ROLE_NAME: &str = "@everyone";
const DEFAULT_EVERYONE_ROLE_COLOR: &str = "#99aab5";
const DEFAULT_EVERYONE_ROLE_POSITION: i64 = 2_147_483_647;

#[derive(Debug, Clone)]
pub struct CreateGuildInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateGuildInput {
    pub name: Option<Option<String>>,
    pub description: Option<Option<String>>,
}

pub async fn list_guilds(pool: &DbPool, user_id: &str) -> Result<Vec<GuildResponse>, AppError> {
    let guilds = guild::list_guilds_for_user(pool, user_id).await?;
    Ok(guilds
        .into_iter()
        .map(|record| GuildResponse::from_guild(record, user_id))
        .collect())
}

pub async fn create_guild(
    pool: &DbPool,
    user_id: &str,
    input: CreateGuildInput,
) -> Result<GuildResponse, AppError> {
    let name = normalize_guild_name(&input.name)?;
    let description = normalize_description(input.description.as_deref())?;
    let base_slug = slugify(&name);
    let created_at = Utc::now().to_rfc3339();

    for attempt in 0..MAX_GUILD_SLUG_ATTEMPTS {
        let slug = slug_for_attempt(&base_slug, attempt);
        let id = Uuid::new_v4().to_string();
        let inserted = guild::insert_guild(
            pool,
            &id,
            &slug,
            &name,
            description.as_deref(),
            user_id,
            DEFAULT_CHANNEL_SLUG,
            &created_at,
            &created_at,
        )
        .await?;

        if inserted {
            let default_channel_id = Uuid::new_v4().to_string();
            let default_channel_inserted = channel::insert_channel(
                pool,
                &default_channel_id,
                &id,
                DEFAULT_CHANNEL_SLUG,
                DEFAULT_CHANNEL_SLUG,
                "text",
                0,
                None,
                &created_at,
                &created_at,
            )
            .await?;
            if !default_channel_inserted {
                return Err(AppError::Internal(
                    "Failed to create default channel".to_string(),
                ));
            }
            let default_role_inserted = role::insert_role(
                pool,
                &format!("role-everyone-{id}"),
                &id,
                DEFAULT_EVERYONE_ROLE_NAME,
                DEFAULT_EVERYONE_ROLE_COLOR,
                DEFAULT_EVERYONE_ROLE_POSITION,
                0,
                true,
                &created_at,
                &created_at,
            )
            .await?;
            if !default_role_inserted {
                return Err(AppError::Internal(
                    "Failed to create default role".to_string(),
                ));
            }
            let record = guild::find_guild_by_slug(pool, &slug)
                .await?
                .ok_or_else(|| AppError::Internal("Created guild not found".to_string()))?;
            return Ok(GuildResponse::from_guild(record, user_id));
        }
    }

    Err(AppError::Conflict(
        "Guild name is already in use".to_string(),
    ))
}

pub async fn update_guild(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    input: UpdateGuildInput,
) -> Result<GuildResponse, AppError> {
    if input.name.is_none() && input.description.is_none() {
        return Err(AppError::ValidationError(
            "At least one guild field is required".to_string(),
        ));
    }

    let record = load_owned_guild(pool, user_id, guild_slug).await?;
    let mut name = record.name.clone();
    let mut description = record.description.clone();

    if let Some(name_input) = input.name {
        let Some(value) = name_input else {
            return Err(AppError::ValidationError("name cannot be null".to_string()));
        };
        name = normalize_guild_name(&value)?;
    }

    if let Some(description_input) = input.description {
        description = normalize_description(description_input.as_deref())?;
    }

    let updated_at = Utc::now().to_rfc3339();
    let rows =
        guild::update_guild_profile(pool, &record.id, &name, description.as_deref(), &updated_at)
            .await?;
    if rows == 0 {
        return Err(AppError::NotFound);
    }

    let updated = guild::find_guild_by_slug(pool, guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(GuildResponse::from_guild(updated, user_id))
}

pub async fn save_guild_icon(
    pool: &DbPool,
    avatar_config: &AvatarConfig,
    user_id: &str,
    guild_slug: &str,
    declared_content_type: Option<&str>,
    bytes: &[u8],
) -> Result<GuildResponse, AppError> {
    if bytes.is_empty() {
        return Err(AppError::ValidationError(
            "Guild icon file is required".to_string(),
        ));
    }
    if bytes.len() > avatar_config.max_size_bytes {
        return Err(AppError::ValidationError(format!(
            "Guild icon exceeds the {} byte limit",
            avatar_config.max_size_bytes
        )));
    }

    let sniffed_mime = sniff_image_mime(bytes).ok_or_else(|| {
        AppError::ValidationError("Unsupported guild icon image type".to_string())
    })?;
    let declared_mime = match declared_content_type {
        Some(value) => Some(normalize_declared_mime(value)?),
        None => None,
    };
    if let Some(declared_mime) = declared_mime
        && declared_mime != sniffed_mime
    {
        return Err(AppError::ValidationError(
            "Guild icon MIME type does not match file content".to_string(),
        ));
    }

    let record = load_owned_guild(pool, user_id, guild_slug).await?;
    let extension = extension_for_mime(sniffed_mime);
    let storage_key = format!("guild-{}.{}", Uuid::new_v4(), extension);
    let icon_path = guild_icon_file_path(&avatar_config.upload_dir, &storage_key);
    if let Some(parent) = icon_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
    }
    fs::write(&icon_path, bytes)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let icon_size_bytes: i64 = bytes
        .len()
        .try_into()
        .map_err(|_| AppError::Internal("Guild icon size is too large".to_string()))?;
    let updated_at = Utc::now().to_rfc3339();
    let rows = guild::update_guild_icon(
        pool,
        &record.id,
        &storage_key,
        sniffed_mime,
        icon_size_bytes,
        &updated_at,
        &updated_at,
    )
    .await?;

    if rows == 0 {
        if let Err(err) = fs::remove_file(&icon_path).await {
            tracing::warn!(
                error = %err,
                path = %icon_path.display(),
                "Failed to clean up guild icon file after missing guild"
            );
        }
        return Err(AppError::NotFound);
    }

    if let Some(old_key) = record.icon_storage_key
        && old_key != storage_key
    {
        if storage_key_is_safe(&old_key) {
            let old_path = guild_icon_file_path(&avatar_config.upload_dir, &old_key);
            if let Err(err) = fs::remove_file(old_path).await
                && err.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!(error = %err, "Failed to remove old guild icon file");
            }
        } else {
            tracing::warn!("Skipping old guild icon cleanup due to invalid storage key");
        }
    }

    let updated = guild::find_guild_by_slug(pool, guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(GuildResponse::from_guild(updated, user_id))
}

pub async fn load_guild_icon(
    pool: &DbPool,
    avatar_config: &AvatarConfig,
    user_id: &str,
    guild_slug: &str,
) -> Result<(Vec<u8>, String), AppError> {
    let record = load_owned_guild(pool, user_id, guild_slug).await?;
    let storage_key = record.icon_storage_key.ok_or(AppError::NotFound)?;
    if !storage_key_is_safe(&storage_key) {
        return Err(AppError::Internal(
            "Invalid guild icon storage key".to_string(),
        ));
    }

    let mime = record
        .icon_mime_type
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let path = guild_icon_file_path(&avatar_config.upload_dir, &storage_key);
    let bytes = fs::read(path).await.map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound
        } else {
            AppError::Internal(err.to_string())
        }
    })?;
    Ok((bytes, mime))
}

async fn load_owned_guild(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Guild, AppError> {
    let record = guild::find_guild_by_slug(pool, guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if record.owner_id != user_id {
        return Err(AppError::Forbidden(
            "Only guild owners can update guild settings".to_string(),
        ));
    }
    Ok(record)
}

fn normalize_guild_name(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("name is required".to_string()));
    }
    if trimmed.chars().count() > MAX_GUILD_NAME_CHARS {
        return Err(AppError::ValidationError(format!(
            "name must be {MAX_GUILD_NAME_CHARS} characters or less"
        )));
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "name contains invalid characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_description(value: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(value) = value.map(str::trim) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > MAX_GUILD_DESCRIPTION_CHARS {
        return Err(AppError::ValidationError(format!(
            "description must be {MAX_GUILD_DESCRIPTION_CHARS} characters or less"
        )));
    }
    if value.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "description contains invalid characters".to_string(),
        ));
    }
    Ok(Some(value.to_string()))
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut dash_pending = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            if slug.len() >= MAX_GUILD_SLUG_CHARS {
                break;
            }
            slug.push(ch.to_ascii_lowercase());
            dash_pending = false;
        } else if !slug.is_empty() {
            dash_pending = true;
        }

        if dash_pending && !slug.ends_with('-') && slug.len() < MAX_GUILD_SLUG_CHARS {
            slug.push('-');
            dash_pending = false;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "guild".to_string()
    } else {
        slug
    }
}

fn slug_for_attempt(base_slug: &str, attempt: usize) -> String {
    if attempt == 0 {
        return base_slug.to_string();
    }

    let suffix = format!("-{}", attempt + 1);
    let max_base_chars = MAX_GUILD_SLUG_CHARS.saturating_sub(suffix.len());
    let mut truncated: String = base_slug.chars().take(max_base_chars).collect();
    while truncated.ends_with('-') {
        truncated.pop();
    }
    if truncated.is_empty() {
        truncated = "guild".to_string();
    }
    format!("{truncated}{suffix}")
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
            "Unsupported guild icon image type".to_string(),
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

fn guild_icon_file_path(upload_dir: &str, storage_key: &str) -> PathBuf {
    Path::new(upload_dir).join("guilds").join(storage_key)
}

fn storage_key_is_safe(storage_key: &str) -> bool {
    !storage_key.contains('/') && !storage_key.contains('\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_generates_ascii_slug() {
        assert_eq!(slugify("Makers & Builders"), "makers-builders");
        assert_eq!(slugify("   "), "guild");
        assert_eq!(slugify("Guild___Name"), "guild-name");
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
