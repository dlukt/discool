use std::collections::HashMap;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    models::{dm_channel, dm_message, guild_member},
};

const DEFAULT_DM_MESSAGES_LIMIT: i64 = 50;
const MAX_DM_MESSAGES_LIMIT: i64 = 200;
const MAX_MESSAGE_CHARS: usize = 2000;
const MAX_CLIENT_NONCE_CHARS: usize = 120;
const DEFAULT_ROLE_COLOR: &str = "#99aab5";
const DM_CHANNEL_LOOKUP_RETRY_ATTEMPTS: usize = 5;
const DM_CHANNEL_LOOKUP_RETRY_DELAY_MILLIS: u64 = 10;

#[derive(Debug, Clone)]
pub struct OpenDmInput {
    pub user_id: String,
}

#[derive(Debug, Clone)]
pub struct ListDmMessagesInput {
    pub limit: i64,
    pub before: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateDmMessageInput {
    pub dm_slug: String,
    pub content: String,
    pub client_nonce: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DmParticipantResponse {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DmChannelResponse {
    pub dm_slug: String,
    pub participant: DmParticipantResponse,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DmMessageResponse {
    pub id: String,
    pub dm_slug: String,
    pub author_user_id: String,
    pub author_username: String,
    pub author_display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_avatar_color: Option<String>,
    pub author_role_color: String,
    pub content: String,
    pub is_system: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_nonce: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListDmMessagesResult {
    pub messages: Vec<DmMessageResponse>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreatedDmMessage {
    pub message: DmMessageResponse,
    pub participant_user_ids: Vec<String>,
}

pub async fn open_or_create_dm(
    pool: &DbPool,
    actor_user_id: &str,
    input: OpenDmInput,
) -> Result<DmChannelResponse, AppError> {
    let actor_user_id = normalize_id(actor_user_id, "actor_user_id")?;
    let peer_user_id = normalize_id(&input.user_id, "user_id")?;
    if actor_user_id == peer_user_id {
        return Err(AppError::ValidationError(
            "Cannot open a DM with yourself".to_string(),
        ));
    }

    let peer_profile = guild_member::find_user_profile_by_id(pool, &peer_user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let shares_guild = guild_member::users_share_guild(pool, &actor_user_id, &peer_user_id).await?;
    if !shares_guild {
        return Err(AppError::Forbidden(
            "Direct messages require a shared guild".to_string(),
        ));
    }

    let (user_low_id, user_high_id) = canonical_pair(&actor_user_id, &peer_user_id);
    let channel = if let Some(existing) =
        dm_channel::find_dm_channel_by_participant_pair(pool, &user_low_id, &user_high_id).await?
    {
        existing
    } else {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        let slug = Uuid::new_v4().to_string();
        let inserted = dm_channel::insert_dm_channel(
            pool,
            &id,
            &slug,
            &user_low_id,
            &user_high_id,
            &now,
            &now,
        )
        .await?;
        if inserted {
            dm_channel::DmChannel {
                id,
                slug,
                user_low_id,
                user_high_id,
                created_at: now.clone(),
                updated_at: now,
            }
        } else {
            resolve_dm_channel_after_conflict(pool, &user_low_id, &user_high_id).await?
        }
    };

    Ok(DmChannelResponse {
        dm_slug: channel.slug,
        participant: to_dm_participant_response(peer_profile),
        created_at: channel.created_at,
        updated_at: channel.updated_at,
        last_message_preview: None,
        last_message_at: None,
    })
}

pub async fn list_dms(pool: &DbPool, user_id: &str) -> Result<Vec<DmChannelResponse>, AppError> {
    let user_id = normalize_id(user_id, "user_id")?;
    let channels = dm_channel::list_dm_channels_for_user(pool, &user_id).await?;
    let mut user_cache = HashMap::<String, guild_member::UserProfile>::new();
    let mut responses = Vec::with_capacity(channels.len());

    for entry in channels {
        let peer_user_id = other_participant_id(&entry.user_low_id, &entry.user_high_id, &user_id)
            .ok_or_else(|| {
                AppError::Internal("Invalid DM participant pairing for user".to_string())
            })?;
        let participant_profile = if let Some(cached) = user_cache.get(&peer_user_id) {
            cached.clone()
        } else {
            let profile = guild_member::find_user_profile_by_id(pool, &peer_user_id)
                .await?
                .ok_or(AppError::NotFound)?;
            user_cache.insert(peer_user_id.clone(), profile.clone());
            profile
        };

        let last_message_preview = entry
            .last_message_content
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        responses.push(DmChannelResponse {
            dm_slug: entry.slug,
            participant: to_dm_participant_response(participant_profile),
            created_at: entry.created_at,
            updated_at: entry.updated_at,
            last_message_preview,
            last_message_at: entry.last_message_created_at,
        });
    }

    Ok(responses)
}

pub async fn list_dm_messages(
    pool: &DbPool,
    user_id: &str,
    dm_slug: &str,
    input: ListDmMessagesInput,
) -> Result<ListDmMessagesResult, AppError> {
    let user_id = normalize_id(user_id, "user_id")?;
    let dm_slug = normalize_dm_slug(dm_slug)?;
    let channel = assert_dm_participant(pool, &user_id, &dm_slug).await?;
    let limit = normalize_limit(input.limit);
    let before = decode_before_cursor(input.before.as_deref())?;
    let page =
        dm_message::list_messages_page_by_dm_channel_id(pool, &channel.id, before.as_ref(), limit)
            .await?;

    let next_cursor = if page.has_more {
        page.messages.first().map(|message| {
            encode_cursor(&dm_message::DmMessageCursor {
                created_at: message.created_at.clone(),
                id: message.id.clone(),
            })
        })
    } else {
        None
    };

    let mut user_cache = HashMap::<String, guild_member::UserProfile>::new();
    let mut responses = Vec::with_capacity(page.messages.len());
    for message in page.messages {
        let author_profile = if let Some(cached) = user_cache.get(&message.author_user_id) {
            cached.clone()
        } else {
            let profile = guild_member::find_user_profile_by_id(pool, &message.author_user_id)
                .await?
                .ok_or(AppError::NotFound)?;
            user_cache.insert(message.author_user_id.clone(), profile.clone());
            profile
        };

        responses.push(DmMessageResponse {
            id: message.id,
            dm_slug: channel.slug.clone(),
            author_user_id: author_profile.user_id.clone(),
            author_username: author_profile.username.clone(),
            author_display_name: profile_display_name(&author_profile),
            author_avatar_color: author_profile.avatar_color.clone(),
            author_role_color: DEFAULT_ROLE_COLOR.to_string(),
            content: message.content,
            is_system: message.is_system != 0,
            created_at: message.created_at.clone(),
            updated_at: message.updated_at,
            client_nonce: None,
        });
    }

    Ok(ListDmMessagesResult {
        messages: responses,
        cursor: next_cursor,
    })
}

pub async fn create_dm_message(
    pool: &DbPool,
    author_user_id: &str,
    input: CreateDmMessageInput,
) -> Result<CreatedDmMessage, AppError> {
    let author_user_id = normalize_id(author_user_id, "author_user_id")?;
    let dm_slug = normalize_dm_slug(&input.dm_slug)?;
    let content = normalize_message_content(&input.content)?;
    let client_nonce = normalize_client_nonce(input.client_nonce)?;
    let channel = assert_dm_participant(pool, &author_user_id, &dm_slug).await?;
    let peer_user_id =
        other_participant_id(&channel.user_low_id, &channel.user_high_id, &author_user_id)
            .ok_or_else(|| {
                AppError::Internal("Invalid DM participant pairing for user".to_string())
            })?;
    let shares_guild =
        guild_member::users_share_guild(pool, &author_user_id, &peer_user_id).await?;
    if !shares_guild {
        return Err(AppError::Forbidden(
            "Direct messages require a shared guild".to_string(),
        ));
    }

    let now = Utc::now().to_rfc3339();
    let message_id = Uuid::new_v4().to_string();
    let inserted = dm_message::insert_dm_message(
        pool,
        &message_id,
        &channel.id,
        &author_user_id,
        &content,
        false,
        &now,
        &now,
    )
    .await?;
    if !inserted {
        return Err(AppError::Conflict(
            "Failed to persist DM message".to_string(),
        ));
    }
    let _ = dm_channel::touch_dm_channel(pool, &channel.id, &now).await?;

    let author_profile = guild_member::find_user_profile_by_id(pool, &author_user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let response = DmMessageResponse {
        id: message_id,
        dm_slug: channel.slug,
        author_user_id: author_profile.user_id.clone(),
        author_username: author_profile.username.clone(),
        author_display_name: profile_display_name(&author_profile),
        author_avatar_color: author_profile.avatar_color,
        author_role_color: DEFAULT_ROLE_COLOR.to_string(),
        content,
        is_system: false,
        created_at: now.clone(),
        updated_at: now,
        client_nonce,
    };

    Ok(CreatedDmMessage {
        message: response,
        participant_user_ids: vec![channel.user_low_id, channel.user_high_id],
    })
}

pub async fn assert_dm_participant(
    pool: &DbPool,
    user_id: &str,
    dm_slug: &str,
) -> Result<dm_channel::DmChannel, AppError> {
    let user_id = normalize_id(user_id, "user_id")?;
    let dm_slug = normalize_dm_slug(dm_slug)?;
    let channel = dm_channel::find_dm_channel_by_slug(pool, &dm_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if channel.user_low_id != user_id && channel.user_high_id != user_id {
        return Err(AppError::Forbidden(
            "Only DM participants can access this conversation".to_string(),
        ));
    }
    Ok(channel)
}

fn to_dm_participant_response(profile: guild_member::UserProfile) -> DmParticipantResponse {
    let display_name = profile_display_name(&profile);
    DmParticipantResponse {
        user_id: profile.user_id,
        username: profile.username.clone(),
        display_name,
        avatar_color: profile.avatar_color,
    }
}

fn profile_display_name(profile: &guild_member::UserProfile) -> String {
    profile
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&profile.username)
        .to_string()
}

fn canonical_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

fn other_participant_id(
    user_low_id: &str,
    user_high_id: &str,
    current_user_id: &str,
) -> Option<String> {
    if user_low_id == current_user_id {
        return Some(user_high_id.to_string());
    }
    if user_high_id == current_user_id {
        return Some(user_low_id.to_string());
    }
    None
}

fn normalize_id(value: &str, field: &str) -> Result<String, AppError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(AppError::ValidationError(format!("{field} is required")));
    }
    if normalized.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(format!(
            "{field} contains invalid characters"
        )));
    }
    Ok(normalized.to_string())
}

fn normalize_dm_slug(value: &str) -> Result<String, AppError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(AppError::ValidationError("dm_slug is required".to_string()));
    }
    if normalized.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "dm_slug contains invalid characters".to_string(),
        ));
    }
    Ok(normalized.to_string())
}

fn normalize_limit(limit: i64) -> i64 {
    let base = if limit > 0 {
        limit
    } else {
        DEFAULT_DM_MESSAGES_LIMIT
    };
    base.clamp(1, MAX_DM_MESSAGES_LIMIT)
}

fn normalize_client_nonce(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_CLIENT_NONCE_CHARS {
        return Err(AppError::ValidationError(format!(
            "client_nonce must be {MAX_CLIENT_NONCE_CHARS} characters or less"
        )));
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "client_nonce contains invalid characters".to_string(),
        ));
    }
    Ok(Some(trimmed.to_string()))
}

fn normalize_message_content(value: &str) -> Result<String, AppError> {
    let normalized_newlines = value.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized_newlines.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("content is required".to_string()));
    }
    if trimmed.chars().count() > MAX_MESSAGE_CHARS {
        return Err(AppError::ValidationError(format!(
            "content must be {MAX_MESSAGE_CHARS} characters or less"
        )));
    }
    if trimmed
        .chars()
        .any(|ch| ch.is_control() && ch != '\n' && ch != '\t')
    {
        return Err(AppError::ValidationError(
            "content contains invalid control characters".to_string(),
        ));
    }
    Ok(escape_html(trimmed))
}

fn decode_before_cursor(
    value: Option<&str>,
) -> Result<Option<dm_message::DmMessageCursor>, AppError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    decode_cursor(trimmed).map(Some)
}

fn decode_cursor(encoded: &str) -> Result<dm_message::DmMessageCursor, AppError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| AppError::ValidationError("before cursor is invalid".to_string()))?;
    let decoded_str = std::str::from_utf8(&decoded)
        .map_err(|_| AppError::ValidationError("before cursor is invalid".to_string()))?;
    let (created_at, id) = decoded_str
        .split_once('|')
        .ok_or_else(|| AppError::ValidationError("before cursor is invalid".to_string()))?;
    if id.trim().is_empty() {
        return Err(AppError::ValidationError(
            "before cursor is invalid".to_string(),
        ));
    }
    chrono::DateTime::parse_from_rfc3339(created_at)
        .map_err(|_| AppError::ValidationError("before cursor is invalid".to_string()))?;

    Ok(dm_message::DmMessageCursor {
        created_at: created_at.to_string(),
        id: id.to_string(),
    })
}

fn encode_cursor(cursor: &dm_message::DmMessageCursor) -> String {
    URL_SAFE_NO_PAD.encode(format!("{}|{}", cursor.created_at, cursor.id))
}

async fn resolve_dm_channel_after_conflict(
    pool: &DbPool,
    user_low_id: &str,
    user_high_id: &str,
) -> Result<dm_channel::DmChannel, AppError> {
    for attempt in 0..DM_CHANNEL_LOOKUP_RETRY_ATTEMPTS {
        if let Some(existing) =
            dm_channel::find_dm_channel_by_participant_pair(pool, user_low_id, user_high_id).await?
        {
            return Ok(existing);
        }
        if attempt + 1 < DM_CHANNEL_LOOKUP_RETRY_ATTEMPTS {
            tokio::time::sleep(std::time::Duration::from_millis(
                DM_CHANNEL_LOOKUP_RETRY_DELAY_MILLIS,
            ))
            .await;
        }
    }

    Err(AppError::Internal(
        "Failed to resolve DM channel after insert conflict".to_string(),
    ))
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
