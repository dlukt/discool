use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    models::{guild_member, user_block},
};

#[derive(Debug, Clone, Serialize)]
pub struct UserBlockEntryResponse {
    pub blocked_user_id: String,
    pub blocked_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblocked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_user_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_user_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_user_avatar_color: Option<String>,
}

pub async fn list_user_blocks(
    pool: &DbPool,
    owner_user_id: &str,
) -> Result<Vec<UserBlockEntryResponse>, AppError> {
    let owner_user_id = normalize_user_id(owner_user_id, "owner_user_id")?;
    let records = user_block::list_user_blocks_for_owner(pool, &owner_user_id).await?;
    Ok(records.into_iter().map(to_response).collect())
}

pub async fn block_user(
    pool: &DbPool,
    owner_user_id: &str,
    blocked_user_id: &str,
) -> Result<UserBlockEntryResponse, AppError> {
    let owner_user_id = normalize_user_id(owner_user_id, "owner_user_id")?;
    let blocked_user_id = normalize_user_id(blocked_user_id, "blocked_user_id")?;
    if owner_user_id == blocked_user_id {
        return Err(AppError::ValidationError(
            "Cannot block yourself".to_string(),
        ));
    }

    let blocked_profile = guild_member::find_user_profile_by_id(pool, &blocked_user_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if let Some(existing) =
        user_block::find_active_user_block(pool, &owner_user_id, &blocked_user_id).await?
    {
        return Ok(to_response(existing));
    }

    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    let display_name = blocked_profile
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| blocked_profile.username.clone());
    user_block::insert_user_block(
        pool,
        &id,
        &owner_user_id,
        &blocked_user_id,
        &now,
        Some(display_name.as_str()),
        Some(blocked_profile.username.as_str()),
        blocked_profile.avatar_color.as_deref(),
        &now,
        &now,
    )
    .await?;

    let created = user_block::find_user_block_by_id(pool, &id)
        .await?
        .ok_or_else(|| AppError::Internal("Created block record not found".to_string()))?;
    Ok(to_response(created))
}

pub async fn unblock_user(
    pool: &DbPool,
    owner_user_id: &str,
    blocked_user_id: &str,
) -> Result<(), AppError> {
    let owner_user_id = normalize_user_id(owner_user_id, "owner_user_id")?;
    let blocked_user_id = normalize_user_id(blocked_user_id, "blocked_user_id")?;
    if owner_user_id == blocked_user_id {
        return Err(AppError::ValidationError(
            "Cannot unblock yourself".to_string(),
        ));
    }
    let now = Utc::now().to_rfc3339();
    user_block::unblock_active_user_blocks(pool, &owner_user_id, &blocked_user_id, &now).await?;
    Ok(())
}

fn normalize_user_id(value: &str, field: &str) -> Result<String, AppError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(AppError::ValidationError(format!("{field} is required")));
    }
    Ok(normalized.to_string())
}

fn to_response(record: user_block::UserBlockRecord) -> UserBlockEntryResponse {
    UserBlockEntryResponse {
        blocked_user_id: record.blocked_user_id,
        blocked_at: record.blocked_at,
        unblocked_at: record.unblocked_at,
        blocked_user_display_name: record.blocked_user_display_name,
        blocked_user_username: record.blocked_user_username,
        blocked_user_avatar_color: record.blocked_user_avatar_color,
    }
}
