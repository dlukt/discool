use std::collections::{HashMap, HashSet};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    models::{
        channel::{self, Channel},
        channel_permission_override,
        guild::{self, Guild},
        guild_member, message, message_reaction, role,
    },
    permissions,
};

const MAX_MESSAGE_CHARS: usize = 2_000;
const MAX_CLIENT_NONCE_CHARS: usize = 120;
const MAX_REACTION_EMOJI_CHARS: usize = 64;
const DEFAULT_ROLE_COLOR: &str = "#99aab5";
const OWNER_ROLE_COLOR: &str = "#f59e0b";
const MAX_LIST_MESSAGES_LIMIT: i64 = 200;

#[derive(Debug, Clone)]
pub struct CreateMessageInput {
    pub guild_slug: String,
    pub channel_slug: String,
    pub content: String,
    pub client_nonce: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateMessageInput {
    pub guild_slug: String,
    pub channel_slug: String,
    pub message_id: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct DeleteMessageInput {
    pub guild_slug: String,
    pub channel_slug: String,
    pub message_id: String,
}

#[derive(Debug, Clone)]
pub struct ToggleMessageReactionInput {
    pub guild_slug: String,
    pub channel_slug: String,
    pub message_id: String,
    pub emoji: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageReactionSummaryResponse {
    pub emoji: String,
    pub count: i64,
    pub reacted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageReactionUpdateResponse {
    pub guild_slug: String,
    pub channel_slug: String,
    pub message_id: String,
    pub actor_user_id: String,
    pub reactions: Vec<MessageReactionSummaryResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub guild_slug: String,
    pub channel_slug: String,
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
    pub reactions: Vec<MessageReactionSummaryResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageDeleteResponse {
    pub id: String,
    pub guild_slug: String,
    pub channel_slug: String,
}

#[derive(Debug, Clone)]
pub struct ListChannelMessagesInput {
    pub limit: i64,
    pub before: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListChannelMessagesResult {
    pub messages: Vec<MessageResponse>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone)]
struct ChannelAccessContext {
    guild: Guild,
    channel: Channel,
    effective_permissions: u64,
}

#[derive(Debug, Clone)]
struct MemberRoleScope {
    default_role_id: Option<String>,
    assigned_role_ids: HashSet<String>,
}

pub async fn create_message(
    pool: &DbPool,
    user_id: &str,
    input: CreateMessageInput,
) -> Result<MessageResponse, AppError> {
    let normalized_content = normalize_message_content(&input.content)?;
    let normalized_nonce = normalize_client_nonce(input.client_nonce)?;
    let access =
        load_channel_with_send_access(pool, user_id, &input.guild_slug, &input.channel_slug)
            .await?;

    let now = Utc::now().to_rfc3339();
    let message_id = Uuid::new_v4().to_string();
    let inserted = message::insert_message(
        pool,
        &message_id,
        &access.guild.id,
        &access.channel.id,
        user_id,
        &normalized_content,
        false,
        &now,
        &now,
    )
    .await?;
    if !inserted {
        return Err(AppError::Conflict(
            "Message id collision while creating message".to_string(),
        ));
    }

    let stored = message::find_message_by_id(pool, &message_id)
        .await?
        .ok_or_else(|| AppError::Internal("Created message not found".to_string()))?;
    build_message_response(
        pool,
        &access.guild,
        &access.channel,
        stored,
        user_id,
        normalized_nonce,
        None,
    )
    .await
}

pub async fn update_message(
    pool: &DbPool,
    user_id: &str,
    input: UpdateMessageInput,
) -> Result<MessageResponse, AppError> {
    let normalized_message_id = normalize_message_id(&input.message_id)?;
    let normalized_content = normalize_message_content(&input.content)?;
    let access =
        load_channel_with_send_access(pool, user_id, &input.guild_slug, &input.channel_slug)
            .await?;

    let existing = message::find_message_by_id(pool, &normalized_message_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if existing.guild_id != access.guild.id || existing.channel_id != access.channel.id {
        return Err(AppError::NotFound);
    }
    if existing.author_user_id != user_id {
        return Err(AppError::Forbidden(
            "You can only edit your own messages".to_string(),
        ));
    }

    let updated_at = Utc::now().to_rfc3339();
    let updated = message::update_message_content_by_id_channel_and_author(
        pool,
        &normalized_message_id,
        &access.channel.id,
        user_id,
        &normalized_content,
        &updated_at,
    )
    .await?;
    if !updated {
        return Err(AppError::NotFound);
    }

    let stored = message::find_message_by_id(pool, &normalized_message_id)
        .await?
        .ok_or_else(|| AppError::Internal("Updated message not found".to_string()))?;
    build_message_response(
        pool,
        &access.guild,
        &access.channel,
        stored,
        user_id,
        None,
        None,
    )
    .await
}

pub async fn delete_message(
    pool: &DbPool,
    user_id: &str,
    input: DeleteMessageInput,
) -> Result<MessageDeleteResponse, AppError> {
    let normalized_message_id = normalize_message_id(&input.message_id)?;
    let access =
        load_channel_with_view_access(pool, user_id, &input.guild_slug, &input.channel_slug)
            .await?;

    let existing = message::find_message_by_id(pool, &normalized_message_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if existing.guild_id != access.guild.id || existing.channel_id != access.channel.id {
        return Err(AppError::NotFound);
    }
    if existing.author_user_id != user_id {
        return Err(AppError::Forbidden(
            "You can only delete your own messages".to_string(),
        ));
    }

    let deleted = message::delete_message_by_id_channel_and_author(
        pool,
        &normalized_message_id,
        &access.channel.id,
        user_id,
    )
    .await?;
    if !deleted {
        return Err(AppError::NotFound);
    }

    Ok(MessageDeleteResponse {
        id: normalized_message_id,
        guild_slug: access.guild.slug,
        channel_slug: access.channel.slug,
    })
}

pub async fn toggle_message_reaction(
    pool: &DbPool,
    user_id: &str,
    input: ToggleMessageReactionInput,
) -> Result<MessageReactionUpdateResponse, AppError> {
    let normalized_message_id = normalize_message_id(&input.message_id)?;
    let normalized_emoji = normalize_reaction_emoji(&input.emoji)?;
    let access =
        load_channel_with_view_access(pool, user_id, &input.guild_slug, &input.channel_slug)
            .await?;

    let existing = message::find_message_by_id(pool, &normalized_message_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if existing.guild_id != access.guild.id || existing.channel_id != access.channel.id {
        return Err(AppError::NotFound);
    }

    let already_reacted = message_reaction::has_message_reaction(
        pool,
        &normalized_message_id,
        user_id,
        &normalized_emoji,
    )
    .await?;
    if !already_reacted
        && !permissions::has_permission(access.effective_permissions, permissions::ADD_REACTIONS)
    {
        return Err(AppError::Forbidden(
            "Missing ADD_REACTIONS permission in this channel".to_string(),
        ));
    }

    if already_reacted {
        message_reaction::delete_message_reaction(
            pool,
            &normalized_message_id,
            user_id,
            &normalized_emoji,
        )
        .await?;
    } else {
        message_reaction::insert_message_reaction(
            pool,
            &normalized_message_id,
            user_id,
            &normalized_emoji,
            &Utc::now().to_rfc3339(),
        )
        .await?;
    }

    Ok(MessageReactionUpdateResponse {
        guild_slug: access.guild.slug,
        channel_slug: access.channel.slug,
        message_id: normalized_message_id.clone(),
        actor_user_id: user_id.to_string(),
        reactions: build_message_reaction_summaries(pool, &normalized_message_id, user_id).await?,
    })
}

pub async fn list_message_reaction_summaries_for_viewer(
    pool: &DbPool,
    message_id: &str,
    viewer_user_id: &str,
) -> Result<Vec<MessageReactionSummaryResponse>, AppError> {
    let normalized_message_id = normalize_message_id(message_id)?;
    build_message_reaction_summaries(pool, &normalized_message_id, viewer_user_id).await
}

pub async fn list_message_reaction_summaries_for_viewers(
    pool: &DbPool,
    message_id: &str,
    viewer_user_ids: &[String],
) -> Result<HashMap<String, Vec<MessageReactionSummaryResponse>>, AppError> {
    let normalized_message_id = normalize_message_id(message_id)?;

    let mut unique_viewer_user_ids = Vec::new();
    let mut seen_viewer_user_ids = HashSet::new();
    for viewer_user_id in viewer_user_ids {
        let trimmed = viewer_user_id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized_viewer_user_id = trimmed.to_string();
        if seen_viewer_user_ids.insert(normalized_viewer_user_id.clone()) {
            unique_viewer_user_ids.push(normalized_viewer_user_id);
        }
    }
    if unique_viewer_user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let reaction_entries =
        message_reaction::list_reaction_entries_by_message_id(pool, &normalized_message_id).await?;
    let mut counts_by_emoji = HashMap::<String, i64>::new();
    let mut reactors_by_emoji = HashMap::<String, HashSet<String>>::new();
    for entry in reaction_entries {
        *counts_by_emoji.entry(entry.emoji.clone()).or_insert(0) += 1;
        reactors_by_emoji
            .entry(entry.emoji)
            .or_default()
            .insert(entry.user_id);
    }

    let mut ordered_emoji_counts = counts_by_emoji.into_iter().collect::<Vec<_>>();
    ordered_emoji_counts.sort_by(|(left_emoji, left_count), (right_emoji, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_emoji.cmp(right_emoji))
    });

    let mut summaries_by_viewer = HashMap::with_capacity(unique_viewer_user_ids.len());
    for viewer_user_id in unique_viewer_user_ids {
        let mut summaries = Vec::with_capacity(ordered_emoji_counts.len());
        for (emoji, count) in &ordered_emoji_counts {
            let reacted = reactors_by_emoji
                .get(emoji)
                .is_some_and(|reactors| reactors.contains(&viewer_user_id));
            summaries.push(MessageReactionSummaryResponse {
                emoji: emoji.clone(),
                count: *count,
                reacted,
            });
        }
        summaries_by_viewer.insert(viewer_user_id, summaries);
    }

    Ok(summaries_by_viewer)
}

pub async fn list_channel_messages(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    channel_slug: &str,
    input: ListChannelMessagesInput,
) -> Result<ListChannelMessagesResult, AppError> {
    let normalized_limit = input.limit.clamp(1, MAX_LIST_MESSAGES_LIMIT);
    let access = load_channel_with_view_access(pool, user_id, guild_slug, channel_slug).await?;
    let before_cursor = decode_before_cursor(input.before.as_deref())?;
    let page = message::list_messages_page_by_channel_id(
        pool,
        &access.channel.id,
        before_cursor.as_ref(),
        normalized_limit,
    )
    .await?;

    let next_cursor = if page.has_more {
        page.messages.first().map(|message| {
            encode_cursor(&message::MessageCursor {
                created_at: message.created_at.clone(),
                id: message.id.clone(),
            })
        })
    } else {
        None
    };

    let message_ids = page
        .messages
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    let reaction_map =
        message_reaction::list_reaction_summaries_by_message_ids(pool, &message_ids, user_id)
            .await?;

    let mut responses = Vec::with_capacity(page.messages.len());
    for item in page.messages {
        let preloaded_reactions = Some(reaction_map.get(&item.id).cloned().unwrap_or_default());
        responses.push(
            build_message_response(
                pool,
                &access.guild,
                &access.channel,
                item,
                user_id,
                None,
                preloaded_reactions,
            )
            .await?,
        );
    }
    Ok(ListChannelMessagesResult {
        messages: responses,
        cursor: next_cursor,
    })
}

fn decode_before_cursor(value: Option<&str>) -> Result<Option<message::MessageCursor>, AppError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    decode_cursor(trimmed).map(Some)
}

fn decode_cursor(encoded: &str) -> Result<message::MessageCursor, AppError> {
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

    Ok(message::MessageCursor {
        created_at: created_at.to_string(),
        id: id.to_string(),
    })
}

fn encode_cursor(cursor: &message::MessageCursor) -> String {
    URL_SAFE_NO_PAD.encode(format!("{}|{}", cursor.created_at, cursor.id))
}

fn to_message_reaction_response(
    summary: message_reaction::MessageReactionSummary,
) -> MessageReactionSummaryResponse {
    MessageReactionSummaryResponse {
        emoji: summary.emoji,
        count: summary.count,
        reacted: summary.reacted,
    }
}

async fn build_message_reaction_summaries(
    pool: &DbPool,
    message_id: &str,
    viewer_user_id: &str,
) -> Result<Vec<MessageReactionSummaryResponse>, AppError> {
    let summaries =
        message_reaction::list_reaction_summaries_by_message_id(pool, message_id, viewer_user_id)
            .await?;
    Ok(summaries
        .into_iter()
        .map(to_message_reaction_response)
        .collect())
}

async fn build_message_response(
    pool: &DbPool,
    guild: &Guild,
    channel: &Channel,
    message: message::Message,
    viewer_user_id: &str,
    client_nonce: Option<String>,
    preloaded_reactions: Option<Vec<message_reaction::MessageReactionSummary>>,
) -> Result<MessageResponse, AppError> {
    let profile = guild_member::find_user_profile_by_id(pool, &message.author_user_id)
        .await?
        .ok_or_else(|| AppError::Internal("Message author profile is missing".to_string()))?;
    let default_role_color = role::find_default_role_by_guild_id(pool, &guild.id)
        .await?
        .map(|record| record.color)
        .unwrap_or_else(|| DEFAULT_ROLE_COLOR.to_string());
    let role_color =
        resolve_highest_role_color(pool, guild, &message.author_user_id, &default_role_color)
            .await?;
    let display_name = profile
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&profile.username)
        .to_string();
    let reactions = match preloaded_reactions {
        Some(items) => items
            .into_iter()
            .map(to_message_reaction_response)
            .collect(),
        None => build_message_reaction_summaries(pool, &message.id, viewer_user_id).await?,
    };

    Ok(MessageResponse {
        id: message.id,
        guild_slug: guild.slug.clone(),
        channel_slug: channel.slug.clone(),
        author_user_id: profile.user_id,
        author_username: profile.username,
        author_display_name: display_name,
        author_avatar_color: profile.avatar_color,
        author_role_color: role_color,
        content: message.content,
        is_system: message.is_system != 0,
        created_at: message.created_at,
        updated_at: message.updated_at,
        client_nonce,
        reactions,
    })
}

async fn resolve_highest_role_color(
    pool: &DbPool,
    guild: &Guild,
    user_id: &str,
    default_color: &str,
) -> Result<String, AppError> {
    if user_id == guild.owner_id {
        return Ok(OWNER_ROLE_COLOR.to_string());
    }
    let assigned_role_ids = role::list_assigned_role_ids(pool, &guild.id, user_id).await?;
    if assigned_role_ids.is_empty() {
        return Ok(default_color.to_string());
    }
    let roles = role::list_roles_by_guild_id(pool, &guild.id).await?;
    for role_id in assigned_role_ids {
        if let Some(role) = roles.iter().find(|record| record.id == role_id) {
            return Ok(role.color.clone());
        }
    }
    Ok(default_color.to_string())
}

async fn load_channel_with_view_access(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    channel_slug: &str,
) -> Result<ChannelAccessContext, AppError> {
    let guild = guild::find_guild_by_slug(pool, guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if !permissions::can_view_guild(pool, &guild, user_id).await? {
        return Err(AppError::Forbidden(
            "Only guild members can view channel messages".to_string(),
        ));
    }

    let channel = channel::find_channel_by_slug(pool, &guild.id, channel_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    let effective =
        resolve_effective_channel_permissions(pool, &guild, &channel.id, user_id).await?;
    if !permissions::has_permission(effective, permissions::VIEW_CHANNEL) {
        return Err(AppError::Forbidden(
            "Missing VIEW_CHANNEL permission in this channel".to_string(),
        ));
    }

    Ok(ChannelAccessContext {
        guild,
        channel,
        effective_permissions: effective,
    })
}

async fn load_channel_with_send_access(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    channel_slug: &str,
) -> Result<ChannelAccessContext, AppError> {
    let access = load_channel_with_view_access(pool, user_id, guild_slug, channel_slug).await?;
    if !has_required_channel_permissions(access.effective_permissions) {
        return Err(AppError::Forbidden(
            "Missing SEND_MESSAGES permission in this channel".to_string(),
        ));
    }
    Ok(access)
}

async fn resolve_effective_channel_permissions(
    pool: &DbPool,
    guild: &Guild,
    channel_id: &str,
    user_id: &str,
) -> Result<u64, AppError> {
    let base_permissions = permissions::effective_guild_permissions(pool, guild, user_id).await?;
    if guild.owner_id == user_id {
        return Ok(base_permissions);
    }

    let role_scope = member_role_scope_for_channel_permissions(pool, &guild.id, user_id).await?;
    let overrides =
        channel_permission_override::list_overrides_by_channel_id(pool, channel_id).await?;
    let mut effective = base_permissions;

    if let Some(default_role_id) = role_scope.default_role_id.as_deref()
        && let Some(default_override) = overrides
            .iter()
            .find(|item| item.role_id.as_str() == default_role_id)
    {
        let allow_mask = permissions::stored_permissions_to_mask(default_override.allow_bitflag)?;
        let deny_mask = permissions::stored_permissions_to_mask(default_override.deny_bitflag)?;
        effective = permissions::apply_channel_overrides(effective, allow_mask, deny_mask);
    }

    let mut role_allow_mask = 0_u64;
    let mut role_deny_mask = 0_u64;
    for override_item in overrides {
        if !role_scope
            .assigned_role_ids
            .contains(&override_item.role_id)
        {
            continue;
        }
        role_allow_mask |= permissions::stored_permissions_to_mask(override_item.allow_bitflag)?;
        role_deny_mask |= permissions::stored_permissions_to_mask(override_item.deny_bitflag)?;
    }
    Ok(permissions::apply_channel_overrides(
        effective,
        role_allow_mask,
        role_deny_mask,
    ))
}

async fn member_role_scope_for_channel_permissions(
    pool: &DbPool,
    guild_id: &str,
    user_id: &str,
) -> Result<MemberRoleScope, AppError> {
    let default_role_id = role::find_default_role_by_guild_id(pool, guild_id)
        .await?
        .map(|record| record.id);
    let assigned_role_ids = role::list_assigned_role_ids(pool, guild_id, user_id)
        .await?
        .into_iter()
        .collect();
    Ok(MemberRoleScope {
        default_role_id,
        assigned_role_ids,
    })
}

fn has_required_channel_permissions(mask: u64) -> bool {
    permissions::has_permission(mask, permissions::VIEW_CHANNEL)
        && permissions::has_permission(mask, permissions::SEND_MESSAGES)
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

fn normalize_message_id(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError(
            "message_id is required".to_string(),
        ));
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "message_id contains invalid characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_reaction_emoji(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("emoji is required".to_string()));
    }
    if trimmed.chars().count() > MAX_REACTION_EMOJI_CHARS {
        return Err(AppError::ValidationError(format!(
            "emoji must be {MAX_REACTION_EMOJI_CHARS} characters or less"
        )));
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "emoji contains invalid control characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
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

    let escaped = escape_html(trimmed);
    if escaped.trim().is_empty() {
        return Err(AppError::ValidationError("content is required".to_string()));
    }
    Ok(escaped)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::DatabaseConfig,
        db::{init_pool, run_migrations},
    };

    async fn setup_service_pool() -> DbPool {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        seed_service_fixture(&pool).await;
        pool
    }

    async fn seed_service_fixture(pool: &DbPool) {
        let DbPool::Sqlite(pool) = pool else {
            panic!("service test fixture expects sqlite pool");
        };

        let created_at = "2026-02-28T00:00:00Z";
        sqlx::query(
            "INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind("owner-user-id")
        .bind("did:key:z6MkOwner")
        .bind("zOwner")
        .bind("owner-user")
        .bind("#3366ff")
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind("author-user-id")
        .bind("did:key:z6MkAuthor")
        .bind("zAuthor")
        .bind("author-user")
        .bind("#22aa88")
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind("member-user-id")
        .bind("did:key:z6MkMember")
        .bind("zMember")
        .bind("member-user")
        .bind("#aa2288")
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO guilds (id, slug, name, description, owner_id, default_channel_slug, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7)",
        )
        .bind("guild-id")
        .bind("test-guild")
        .bind("Test Guild")
        .bind("owner-user-id")
        .bind("general")
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO channels (id, guild_id, slug, name, channel_type, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind("channel-id")
        .bind("guild-id")
        .bind("general")
        .bind("general")
        .bind("text")
        .bind(0_i64)
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        for member_id in ["author-user-id", "member-user-id"] {
            sqlx::query(
                "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code)
                 VALUES (?1, ?2, ?3, NULL)",
            )
            .bind("guild-id")
            .bind(member_id)
            .bind(created_at)
            .execute(pool)
            .await
            .unwrap();
        }
    }

    async fn insert_fixture_message(pool: &DbPool, message_id: &str, content: &str) {
        message::insert_message(
            pool,
            message_id,
            "guild-id",
            "channel-id",
            "author-user-id",
            content,
            false,
            "2026-02-28T00:00:01Z",
            "2026-02-28T00:00:01Z",
        )
        .await
        .unwrap();
    }

    async fn deny_add_reactions_for_default_role(pool: &DbPool) {
        let DbPool::Sqlite(pool) = pool else {
            panic!("service test fixture expects sqlite pool");
        };

        let existing_default_role_id = sqlx::query_scalar::<_, String>(
            "SELECT id
             FROM roles
             WHERE guild_id = ?1
               AND is_default = 1
             LIMIT 1",
        )
        .bind("guild-id")
        .fetch_optional(pool)
        .await
        .unwrap();
        let default_role_id = match existing_default_role_id {
            Some(role_id) => role_id,
            None => {
                let role_id = "role-everyone-guild-id".to_string();
                sqlx::query(
                    "INSERT INTO roles (
                        id,
                        guild_id,
                        name,
                        color,
                        position,
                        permissions_bitflag,
                        is_default,
                        created_at,
                        updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                )
                .bind(&role_id)
                .bind("guild-id")
                .bind("@everyone")
                .bind("#99aab5")
                .bind(2_147_483_647_i64)
                .bind(permissions::default_everyone_permissions_i64())
                .bind(1_i64)
                .bind("2026-02-28T00:00:00Z")
                .bind("2026-02-28T00:00:00Z")
                .execute(pool)
                .await
                .unwrap();
                role_id
            }
        };

        sqlx::query(
            "INSERT INTO channel_permission_overrides (channel_id, role_id, allow_bitflag, deny_bitflag)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT DO NOTHING",
        )
        .bind("channel-id")
        .bind(default_role_id)
        .bind(0_i64)
        .bind(permissions::ADD_REACTIONS as i64)
        .execute(pool)
        .await
        .unwrap();
    }

    #[test]
    fn normalize_message_content_rejects_empty_and_control_characters() {
        assert!(normalize_message_content("   ").is_err());
        assert!(normalize_message_content("hello\u{0007}world").is_err());
    }

    #[test]
    fn normalize_message_content_sanitizes_html_and_preserves_newlines() {
        let normalized = normalize_message_content("  hello <b>team</b>\r\nnext line  ").unwrap();
        assert_eq!(normalized, "hello &lt;b&gt;team&lt;/b&gt;\nnext line");
    }

    #[test]
    fn normalize_client_nonce_validates_length_and_whitespace() {
        assert_eq!(
            normalize_client_nonce(Some("   ".to_string())).unwrap(),
            None
        );
        let too_long = "x".repeat(MAX_CLIENT_NONCE_CHARS + 1);
        assert!(normalize_client_nonce(Some(too_long)).is_err());
        assert_eq!(
            normalize_client_nonce(Some(" nonce-1 ".to_string())).unwrap(),
            Some("nonce-1".to_string())
        );
    }

    #[test]
    fn normalize_message_id_rejects_invalid_values() {
        assert!(normalize_message_id("   ").is_err());
        assert!(normalize_message_id("message\u{0007}").is_err());
        assert_eq!(normalize_message_id(" message-1 ").unwrap(), "message-1");
    }

    #[test]
    fn normalize_reaction_emoji_rejects_invalid_values() {
        assert!(normalize_reaction_emoji("   ").is_err());
        assert!(normalize_reaction_emoji("😀\u{0007}").is_err());
        let too_long = "😀".repeat(MAX_REACTION_EMOJI_CHARS + 1);
        assert!(normalize_reaction_emoji(&too_long).is_err());
        assert_eq!(normalize_reaction_emoji(" 😀 ").unwrap(), "😀");
    }

    #[test]
    fn has_required_channel_permissions_requires_view_and_send() {
        assert!(has_required_channel_permissions(
            permissions::VIEW_CHANNEL | permissions::SEND_MESSAGES
        ));
        assert!(!has_required_channel_permissions(permissions::VIEW_CHANNEL));
        assert!(!has_required_channel_permissions(
            permissions::SEND_MESSAGES
        ));
    }

    #[test]
    fn decode_cursor_rejects_invalid_values() {
        assert!(decode_cursor("not-a-valid-cursor").is_err());
        let malformed = URL_SAFE_NO_PAD.encode("bad-format");
        assert!(decode_cursor(&malformed).is_err());
    }

    #[test]
    fn encode_and_decode_cursor_round_trip() {
        let cursor = message::MessageCursor {
            created_at: "2026-02-28T00:00:00Z".to_string(),
            id: "message-123".to_string(),
        };
        let encoded = encode_cursor(&cursor);
        let decoded = decode_cursor(&encoded).unwrap();
        assert_eq!(decoded.created_at, cursor.created_at);
        assert_eq!(decoded.id, cursor.id);
    }

    #[test]
    fn decode_before_cursor_handles_defaults() {
        assert!(decode_before_cursor(None).unwrap().is_none());
        assert!(decode_before_cursor(Some("   ")).unwrap().is_none());
    }

    #[tokio::test]
    async fn update_message_enforces_ownership_and_updates_content() {
        let pool = setup_service_pool().await;
        insert_fixture_message(&pool, "message-1", "hello").await;

        let forbidden = update_message(
            &pool,
            "member-user-id",
            UpdateMessageInput {
                guild_slug: "test-guild".to_string(),
                channel_slug: "general".to_string(),
                message_id: "message-1".to_string(),
                content: "nope".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(forbidden, AppError::Forbidden(_)));

        let updated = update_message(
            &pool,
            "author-user-id",
            UpdateMessageInput {
                guild_slug: "test-guild".to_string(),
                channel_slug: "general".to_string(),
                message_id: "message-1".to_string(),
                content: " edited <b>content</b> ".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.id, "message-1");
        assert_eq!(updated.content, "edited &lt;b&gt;content&lt;/b&gt;");
        assert_eq!(updated.created_at, "2026-02-28T00:00:01Z");
        assert_ne!(updated.updated_at, updated.created_at);
    }

    #[tokio::test]
    async fn delete_message_enforces_ownership() {
        let pool = setup_service_pool().await;
        insert_fixture_message(&pool, "message-2", "delete-me").await;

        let forbidden = delete_message(
            &pool,
            "member-user-id",
            DeleteMessageInput {
                guild_slug: "test-guild".to_string(),
                channel_slug: "general".to_string(),
                message_id: "message-2".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(forbidden, AppError::Forbidden(_)));

        let deleted = delete_message(
            &pool,
            "author-user-id",
            DeleteMessageInput {
                guild_slug: "test-guild".to_string(),
                channel_slug: "general".to_string(),
                message_id: "message-2".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(deleted.id, "message-2");
        assert_eq!(deleted.guild_slug, "test-guild");
        assert_eq!(deleted.channel_slug, "general");

        let persisted = message::find_message_by_id(&pool, "message-2")
            .await
            .unwrap();
        assert!(persisted.is_none());
    }

    #[tokio::test]
    async fn toggle_message_reaction_enforces_permission_for_add_and_allows_remove() {
        let pool = setup_service_pool().await;
        insert_fixture_message(&pool, "message-3", "react-me").await;
        deny_add_reactions_for_default_role(&pool).await;

        let forbidden = toggle_message_reaction(
            &pool,
            "member-user-id",
            ToggleMessageReactionInput {
                guild_slug: "test-guild".to_string(),
                channel_slug: "general".to_string(),
                message_id: "message-3".to_string(),
                emoji: "😀".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(forbidden, AppError::Forbidden(_)));

        let seeded = message_reaction::insert_message_reaction(
            &pool,
            "message-3",
            "member-user-id",
            "😀",
            "2026-02-28T00:00:02Z",
        )
        .await
        .unwrap();
        assert!(seeded);

        let removed = toggle_message_reaction(
            &pool,
            "member-user-id",
            ToggleMessageReactionInput {
                guild_slug: "test-guild".to_string(),
                channel_slug: "general".to_string(),
                message_id: "message-3".to_string(),
                emoji: "😀".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(removed.message_id, "message-3");
        assert!(removed.reactions.is_empty());
    }

    #[tokio::test]
    async fn toggle_message_reaction_aggregates_multi_user_counts() {
        let pool = setup_service_pool().await;
        insert_fixture_message(&pool, "message-4", "count-me").await;

        let first = toggle_message_reaction(
            &pool,
            "author-user-id",
            ToggleMessageReactionInput {
                guild_slug: "test-guild".to_string(),
                channel_slug: "general".to_string(),
                message_id: "message-4".to_string(),
                emoji: "👍".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(first.reactions.len(), 1);
        assert_eq!(first.reactions[0].emoji, "👍");
        assert_eq!(first.reactions[0].count, 1);
        assert!(first.reactions[0].reacted);

        let second = toggle_message_reaction(
            &pool,
            "member-user-id",
            ToggleMessageReactionInput {
                guild_slug: "test-guild".to_string(),
                channel_slug: "general".to_string(),
                message_id: "message-4".to_string(),
                emoji: "👍".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(second.reactions.len(), 1);
        assert_eq!(second.reactions[0].count, 2);
        assert!(second.reactions[0].reacted);

        let author_remove = toggle_message_reaction(
            &pool,
            "author-user-id",
            ToggleMessageReactionInput {
                guild_slug: "test-guild".to_string(),
                channel_slug: "general".to_string(),
                message_id: "message-4".to_string(),
                emoji: "👍".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(author_remove.reactions.len(), 1);
        assert_eq!(author_remove.reactions[0].count, 1);
        assert!(!author_remove.reactions[0].reacted);
    }

    #[tokio::test]
    async fn list_message_reaction_summaries_for_viewers_returns_personalized_reacted_flags() {
        let pool = setup_service_pool().await;
        insert_fixture_message(&pool, "message-5", "viewers").await;
        message_reaction::insert_message_reaction(
            &pool,
            "message-5",
            "author-user-id",
            "😀",
            "2026-02-28T00:00:02Z",
        )
        .await
        .unwrap();
        message_reaction::insert_message_reaction(
            &pool,
            "message-5",
            "member-user-id",
            "😀",
            "2026-02-28T00:00:03Z",
        )
        .await
        .unwrap();
        message_reaction::insert_message_reaction(
            &pool,
            "message-5",
            "member-user-id",
            "🎉",
            "2026-02-28T00:00:04Z",
        )
        .await
        .unwrap();

        let viewer_summaries = list_message_reaction_summaries_for_viewers(
            &pool,
            "message-5",
            &[
                "author-user-id".to_string(),
                "member-user-id".to_string(),
                "author-user-id".to_string(),
            ],
        )
        .await
        .unwrap();

        let author_view = viewer_summaries
            .get("author-user-id")
            .expect("missing author view");
        assert_eq!(author_view.len(), 2);
        assert_eq!(author_view[0].emoji, "😀");
        assert_eq!(author_view[0].count, 2);
        assert!(author_view[0].reacted);
        assert_eq!(author_view[1].emoji, "🎉");
        assert_eq!(author_view[1].count, 1);
        assert!(!author_view[1].reacted);

        let member_view = viewer_summaries
            .get("member-user-id")
            .expect("missing member view");
        assert_eq!(member_view.len(), 2);
        assert_eq!(member_view[0].emoji, "😀");
        assert_eq!(member_view[0].count, 2);
        assert!(member_view[0].reacted);
        assert_eq!(member_view[1].emoji, "🎉");
        assert_eq!(member_view[1].count, 1);
        assert!(member_view[1].reacted);
    }
}
