use chrono::Utc;
use serde::Serialize;

use crate::{
    AppError,
    db::DbPool,
    models::{
        dm_message, guild_member, message, message_attachment, message_reaction, recovery_email,
        user, user_block,
    },
};

const AVATAR_URL_PATH: &str = "/api/v1/users/me/avatar";

#[derive(Debug, Clone, Serialize)]
pub struct UserDataExportResponse {
    pub profile: UserDataExportProfile,
    pub guild_memberships: Vec<UserGuildMembershipExport>,
    pub messages: Vec<UserMessageExport>,
    pub dm_messages: Vec<UserDmMessageExport>,
    pub reactions: Vec<UserReactionExport>,
    pub uploaded_files: Vec<UserUploadedFileExport>,
    pub block_list: Vec<UserBlockExport>,
    pub exported_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserDataExportProfile {
    pub user_id: String,
    pub did_key: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserGuildMembershipExport {
    pub guild_id: String,
    pub joined_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joined_via_invite_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserMessageExport {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub author_user_id: String,
    pub content: String,
    pub is_system: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserDmMessageExport {
    pub id: String,
    pub dm_channel_id: String,
    pub author_user_id: String,
    pub content: String,
    pub is_system: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserReactionExport {
    pub message_id: String,
    pub emoji: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserUploadedFileExport {
    pub id: String,
    pub message_id: String,
    pub storage_key: String,
    pub original_filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserBlockExport {
    pub blocked_user_id: String,
    pub blocked_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblocked_at: Option<String>,
}

pub async fn build_user_data_export(
    pool: &DbPool,
    user_id: &str,
) -> Result<UserDataExportResponse, AppError> {
    let user_id = normalize_user_id(user_id)?;
    let user = user::find_user_by_id(pool, &user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let associated_email = recovery_email::find_recovery_email_for_user(pool, &user_id).await?;
    let guild_memberships = guild_member::list_guild_memberships_for_user(pool, &user_id).await?;
    let messages = message::list_messages_by_author_user_id(pool, &user_id).await?;
    let dm_messages = dm_message::list_dm_messages_by_author_user_id(pool, &user_id).await?;
    let reactions = message_reaction::list_message_reactions_by_user_id(pool, &user_id).await?;
    let uploaded_files =
        message_attachment::list_message_attachments_by_author_user_id(pool, &user_id).await?;
    let block_records = user_block::list_user_blocks_for_owner(pool, &user_id).await?;

    let profile = UserDataExportProfile {
        user_id: user.id,
        did_key: user.did_key,
        username: user.username.clone(),
        display_name: normalize_display_name(&user.username, user.display_name.as_deref()),
        avatar_color: user.avatar_color,
        avatar_url: user.avatar_storage_key.map(|_| AVATAR_URL_PATH.to_string()),
        email: associated_email
            .as_ref()
            .map(|record| record.normalized_email.clone()),
        email_verified_at: associated_email.and_then(|record| record.verified_at),
        created_at: user.created_at,
        updated_at: user.updated_at,
    };

    Ok(UserDataExportResponse {
        profile,
        guild_memberships: guild_memberships
            .into_iter()
            .map(|entry| UserGuildMembershipExport {
                guild_id: entry.guild_id,
                joined_at: entry.joined_at,
                joined_via_invite_code: entry.joined_via_invite_code,
            })
            .collect(),
        messages: messages
            .into_iter()
            .map(|entry| UserMessageExport {
                id: entry.id,
                guild_id: entry.guild_id,
                channel_id: entry.channel_id,
                author_user_id: entry.author_user_id,
                content: entry.content,
                is_system: entry.is_system != 0,
                created_at: entry.created_at,
                updated_at: entry.updated_at,
            })
            .collect(),
        dm_messages: dm_messages
            .into_iter()
            .map(|entry| UserDmMessageExport {
                id: entry.id,
                dm_channel_id: entry.dm_channel_id,
                author_user_id: entry.author_user_id,
                content: entry.content,
                is_system: entry.is_system != 0,
                created_at: entry.created_at,
                updated_at: entry.updated_at,
            })
            .collect(),
        reactions: reactions
            .into_iter()
            .map(|entry| UserReactionExport {
                message_id: entry.message_id,
                emoji: entry.emoji,
                created_at: entry.created_at,
            })
            .collect(),
        uploaded_files: uploaded_files
            .into_iter()
            .map(|entry| UserUploadedFileExport {
                id: entry.id,
                message_id: entry.message_id,
                storage_key: entry.storage_key,
                original_filename: entry.original_filename,
                mime_type: entry.mime_type,
                size_bytes: entry.size_bytes,
                created_at: entry.created_at,
            })
            .collect(),
        block_list: block_records
            .into_iter()
            .map(|entry| UserBlockExport {
                blocked_user_id: entry.blocked_user_id,
                blocked_at: entry.blocked_at,
                unblocked_at: entry.unblocked_at,
            })
            .collect(),
        exported_at: Utc::now().to_rfc3339(),
    })
}

fn normalize_user_id(value: &str) -> Result<String, AppError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(AppError::ValidationError("user_id is required".to_string()));
    }
    Ok(normalized.to_string())
}

fn normalize_display_name(username: &str, display_name: Option<&str>) -> String {
    display_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(username)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::DatabaseConfig,
        db::{init_pool, run_migrations},
    };

    async fn setup_export_pool() -> DbPool {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        pool
    }

    async fn seed_export_fixture(pool: &DbPool, include_email: bool) -> String {
        let DbPool::Sqlite(pool) = pool else {
            panic!("user_data_export_service tests expect sqlite pool");
        };

        let requester_user_id = "requester-user-id";
        let other_user_id = "other-user-id";
        let created_at = "2026-03-01T00:00:00Z";

        for (id, did_key, public_key, username) in [
            (
                requester_user_id,
                "did:key:z6MkRequester",
                "zRequester",
                "requester",
            ),
            (other_user_id, "did:key:z6MkOther", "zOther", "other"),
        ] {
            sqlx::query(
                "INSERT INTO users (id, did_key, public_key_multibase, username, display_name, avatar_color, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .bind(id)
            .bind(did_key)
            .bind(public_key)
            .bind(username)
            .bind(Some(username))
            .bind("#3b82f6")
            .bind(created_at)
            .bind(created_at)
            .execute(pool)
            .await
            .unwrap();
        }

        sqlx::query(
            "INSERT INTO guilds (id, slug, name, description, owner_id, default_channel_slug, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7)",
        )
        .bind("guild-one")
        .bind("guild-one")
        .bind("Guild One")
        .bind(requester_user_id)
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
        .bind("channel-one")
        .bind("guild-one")
        .bind("general")
        .bind("general")
        .bind("text")
        .bind(0_i64)
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind("guild-one")
        .bind(requester_user_id)
        .bind("2026-03-01T00:00:01Z")
        .bind(Some("INVITE-ONE"))
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind("guild-one")
        .bind(other_user_id)
        .bind("2026-03-01T00:00:02Z")
        .bind(None::<String>)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO messages (id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind("message-requester")
        .bind("guild-one")
        .bind("channel-one")
        .bind(requester_user_id)
        .bind("requester message")
        .bind(0_i64)
        .bind("2026-03-01T00:00:03Z")
        .bind("2026-03-01T00:00:03Z")
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO messages (id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind("message-other")
        .bind("guild-one")
        .bind("channel-one")
        .bind(other_user_id)
        .bind("other message")
        .bind(0_i64)
        .bind("2026-03-01T00:00:04Z")
        .bind("2026-03-01T00:00:04Z")
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO dm_channels (id, slug, user_low_id, user_high_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind("dm-channel-one")
        .bind("dm-channel-one")
        .bind(other_user_id)
        .bind(requester_user_id)
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO dm_messages (id, dm_channel_id, author_user_id, content, is_system, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind("dm-message-requester")
        .bind("dm-channel-one")
        .bind(requester_user_id)
        .bind("requester dm")
        .bind(0_i64)
        .bind("2026-03-01T00:00:05Z")
        .bind("2026-03-01T00:00:05Z")
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO dm_messages (id, dm_channel_id, author_user_id, content, is_system, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind("dm-message-other")
        .bind("dm-channel-one")
        .bind(other_user_id)
        .bind("other dm")
        .bind(0_i64)
        .bind("2026-03-01T00:00:06Z")
        .bind("2026-03-01T00:00:06Z")
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO message_reactions (message_id, user_id, emoji, created_at)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind("message-other")
        .bind(requester_user_id)
        .bind("🎉")
        .bind("2026-03-01T00:00:07Z")
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO message_reactions (message_id, user_id, emoji, created_at)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind("message-requester")
        .bind(other_user_id)
        .bind("😀")
        .bind("2026-03-01T00:00:08Z")
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO message_attachments (id, message_id, storage_key, original_filename, mime_type, size_bytes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind("attachment-requester")
        .bind("message-requester")
        .bind("attachments/requester.png")
        .bind("requester.png")
        .bind("image/png")
        .bind(42_i64)
        .bind("2026-03-01T00:00:09Z")
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO message_attachments (id, message_id, storage_key, original_filename, mime_type, size_bytes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind("attachment-other")
        .bind("message-other")
        .bind("attachments/other.png")
        .bind("other.png")
        .bind("image/png")
        .bind(43_i64)
        .bind("2026-03-01T00:00:10Z")
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO user_blocks (
                 id,
                 owner_user_id,
                 blocked_user_id,
                 blocked_at,
                 unblocked_at,
                 blocked_user_display_name,
                 blocked_user_username,
                 blocked_user_avatar_color,
                 created_at,
                 updated_at
             ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind("block-record-one")
        .bind(requester_user_id)
        .bind(other_user_id)
        .bind("2026-03-01T00:00:11Z")
        .bind("Other")
        .bind("other")
        .bind("#ef4444")
        .bind("2026-03-01T00:00:11Z")
        .bind("2026-03-01T00:00:11Z")
        .execute(pool)
        .await
        .unwrap();

        if include_email {
            sqlx::query(
                "INSERT INTO user_recovery_email (
                     user_id,
                     normalized_email,
                     email_masked,
                     verified_at,
                     encrypted_private_key,
                     encryption_algorithm,
                     encryption_version,
                     key_nonce,
                     created_at,
                     updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )
            .bind(requester_user_id)
            .bind("requester@example.com")
            .bind("r***@example.com")
            .bind(Some("2026-03-01T00:00:12Z"))
            .bind(Some("encrypted"))
            .bind(Some("aes-256-gcm"))
            .bind(Some(1_i64))
            .bind(Some("nonce"))
            .bind("2026-03-01T00:00:12Z")
            .bind("2026-03-01T00:00:12Z")
            .execute(pool)
            .await
            .unwrap();
        }

        requester_user_id.to_string()
    }

    #[tokio::test]
    async fn build_user_data_export_filters_to_requester_and_includes_required_sections() {
        let pool = setup_export_pool().await;
        let requester_user_id = seed_export_fixture(&pool, true).await;

        let export = build_user_data_export(&pool, &requester_user_id)
            .await
            .unwrap();

        assert_eq!(export.profile.user_id, requester_user_id);
        assert_eq!(export.profile.username, "requester");
        assert_eq!(
            export.profile.email.as_deref(),
            Some("requester@example.com")
        );
        assert_eq!(
            export.profile.email_verified_at.as_deref(),
            Some("2026-03-01T00:00:12Z")
        );

        assert_eq!(export.guild_memberships.len(), 1);
        assert_eq!(export.guild_memberships[0].guild_id, "guild-one");

        assert_eq!(export.messages.len(), 1);
        assert_eq!(export.messages[0].id, "message-requester");
        assert!(
            export
                .messages
                .iter()
                .all(|entry| entry.author_user_id == requester_user_id)
        );

        assert_eq!(export.dm_messages.len(), 1);
        assert_eq!(export.dm_messages[0].id, "dm-message-requester");
        assert!(
            export
                .dm_messages
                .iter()
                .all(|entry| entry.author_user_id == requester_user_id)
        );

        assert_eq!(export.reactions.len(), 1);
        assert_eq!(export.reactions[0].message_id, "message-other");
        assert_eq!(export.reactions[0].emoji, "🎉");

        assert_eq!(export.uploaded_files.len(), 1);
        assert_eq!(export.uploaded_files[0].id, "attachment-requester");

        assert_eq!(export.block_list.len(), 1);
        assert_eq!(export.block_list[0].blocked_user_id, "other-user-id");
        assert!(!export.exported_at.is_empty());
    }

    #[tokio::test]
    async fn build_user_data_export_omits_email_when_not_associated() {
        let pool = setup_export_pool().await;
        let requester_user_id = seed_export_fixture(&pool, false).await;

        let export = build_user_data_export(&pool, &requester_user_id)
            .await
            .unwrap();

        assert_eq!(export.profile.email, None);
        assert_eq!(export.profile.email_verified_at, None);
    }
}
