use crate::{AppError, db::DbPool};
use sqlx::QueryBuilder;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Message {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub author_user_id: String,
    pub content: String,
    pub is_system: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageCursor {
    pub created_at: String,
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct MessagePage {
    pub messages: Vec<Message>,
    pub has_more: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GuildAuthorMessageHistoryRow {
    pub id: String,
    pub channel_slug: String,
    pub channel_name: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildAuthorMessageHistoryCursor {
    pub created_at: String,
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct GuildAuthorMessageHistoryPage {
    pub entries: Vec<GuildAuthorMessageHistoryRow>,
    pub has_more: bool,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_message(
    pool: &DbPool,
    id: &str,
    guild_id: &str,
    channel_id: &str,
    author_user_id: &str,
    content: &str,
    is_system: bool,
    created_at: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let is_system_value = if is_system { 1_i64 } else { 0_i64 };
    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO messages (id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(guild_id)
            .bind(channel_id)
            .bind(author_user_id)
            .bind(content)
            .bind(is_system_value)
            .bind(created_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO messages (id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(guild_id)
            .bind(channel_id)
            .bind(author_user_id)
            .bind(content)
            .bind(is_system_value)
            .bind(created_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn find_message_by_id(
    pool: &DbPool,
    message_id: &str,
) -> Result<Option<Message>, AppError> {
    let message = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at
                 FROM messages
                 WHERE id = $1
                   AND deleted_at IS NULL
                 LIMIT 1",
            )
            .bind(message_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at
                 FROM messages
                 WHERE id = ?1
                   AND deleted_at IS NULL
                 LIMIT 1",
            )
            .bind(message_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(message)
}

pub async fn update_message_content_by_id_channel_and_author(
    pool: &DbPool,
    message_id: &str,
    channel_id: &str,
    author_user_id: &str,
    content: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    update_message_content_if_unmodified_by_id_channel_and_author(
        pool,
        message_id,
        channel_id,
        author_user_id,
        content,
        updated_at,
        None,
    )
    .await
}

pub async fn update_message_content_if_unmodified_by_id_channel_and_author(
    pool: &DbPool,
    message_id: &str,
    channel_id: &str,
    author_user_id: &str,
    content: &str,
    updated_at: &str,
    previous_updated_at: Option<&str>,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => match previous_updated_at {
            Some(expected_updated_at) => sqlx::query(
                "UPDATE messages
                     SET content = $1, updated_at = $2
                     WHERE id = $3
                       AND channel_id = $4
                       AND author_user_id = $5
                       AND deleted_at IS NULL
                       AND updated_at = $6",
            )
            .bind(content)
            .bind(updated_at)
            .bind(message_id)
            .bind(channel_id)
            .bind(author_user_id)
            .bind(expected_updated_at)
            .execute(pool)
            .await
            .map(|result| result.rows_affected()),
            None => sqlx::query(
                "UPDATE messages
                     SET content = $1, updated_at = $2
                     WHERE id = $3
                       AND channel_id = $4
                       AND author_user_id = $5
                       AND deleted_at IS NULL",
            )
            .bind(content)
            .bind(updated_at)
            .bind(message_id)
            .bind(channel_id)
            .bind(author_user_id)
            .execute(pool)
            .await
            .map(|result| result.rows_affected()),
        },
        DbPool::Sqlite(pool) => match previous_updated_at {
            Some(expected_updated_at) => sqlx::query(
                "UPDATE messages
                     SET content = ?1, updated_at = ?2
                     WHERE id = ?3
                       AND channel_id = ?4
                       AND author_user_id = ?5
                       AND deleted_at IS NULL
                       AND updated_at = ?6",
            )
            .bind(content)
            .bind(updated_at)
            .bind(message_id)
            .bind(channel_id)
            .bind(author_user_id)
            .bind(expected_updated_at)
            .execute(pool)
            .await
            .map(|result| result.rows_affected()),
            None => sqlx::query(
                "UPDATE messages
                     SET content = ?1, updated_at = ?2
                     WHERE id = ?3
                       AND channel_id = ?4
                       AND author_user_id = ?5
                       AND deleted_at IS NULL",
            )
            .bind(content)
            .bind(updated_at)
            .bind(message_id)
            .bind(channel_id)
            .bind(author_user_id)
            .execute(pool)
            .await
            .map(|result| result.rows_affected()),
        },
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn delete_message_by_id_channel_and_author(
    pool: &DbPool,
    message_id: &str,
    channel_id: &str,
    author_user_id: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "DELETE FROM messages
                 WHERE id = $1
                   AND channel_id = $2
                   AND author_user_id = $3
                   AND deleted_at IS NULL",
        )
        .bind(message_id)
        .bind(channel_id)
        .bind(author_user_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "DELETE FROM messages
                 WHERE id = ?1
                   AND channel_id = ?2
                   AND author_user_id = ?3
                   AND deleted_at IS NULL",
        )
        .bind(message_id)
        .bind(channel_id)
        .bind(author_user_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

#[allow(clippy::too_many_arguments)]
pub async fn soft_delete_message_by_id_for_moderation(
    pool: &DbPool,
    message_id: &str,
    guild_id: &str,
    channel_id: &str,
    deleted_by_user_id: &str,
    deleted_reason: &str,
    deleted_moderation_action_id: &str,
    deleted_at: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE messages
                 SET deleted_at = $1,
                     deleted_by_user_id = $2,
                     deleted_reason = $3,
                     deleted_moderation_action_id = $4,
                     updated_at = $5
                 WHERE id = $6
                   AND guild_id = $7
                   AND channel_id = $8
                   AND deleted_at IS NULL",
        )
        .bind(deleted_at)
        .bind(deleted_by_user_id)
        .bind(deleted_reason)
        .bind(deleted_moderation_action_id)
        .bind(updated_at)
        .bind(message_id)
        .bind(guild_id)
        .bind(channel_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE messages
                 SET deleted_at = ?1,
                     deleted_by_user_id = ?2,
                     deleted_reason = ?3,
                     deleted_moderation_action_id = ?4,
                     updated_at = ?5
                 WHERE id = ?6
                   AND guild_id = ?7
                   AND channel_id = ?8
                   AND deleted_at IS NULL",
        )
        .bind(deleted_at)
        .bind(deleted_by_user_id)
        .bind(deleted_reason)
        .bind(deleted_moderation_action_id)
        .bind(updated_at)
        .bind(message_id)
        .bind(guild_id)
        .bind(channel_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn list_message_ids_by_guild_and_author_since(
    pool: &DbPool,
    guild_id: &str,
    author_user_id: &str,
    created_at_since: Option<&str>,
) -> Result<Vec<String>, AppError> {
    let message_ids = match pool {
        DbPool::Postgres(pool) => match created_at_since {
            Some(since) => {
                sqlx::query_scalar::<_, String>(
                    "SELECT id
                 FROM messages
                 WHERE guild_id = $1
                    AND author_user_id = $2
                    AND deleted_at IS NULL
                    AND created_at >= $3",
                )
                .bind(guild_id)
                .bind(author_user_id)
                .bind(since)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_scalar::<_, String>(
                    "SELECT id
                 FROM messages
                 WHERE guild_id = $1
                    AND author_user_id = $2
                    AND deleted_at IS NULL",
                )
                .bind(guild_id)
                .bind(author_user_id)
                .fetch_all(pool)
                .await
            }
        },
        DbPool::Sqlite(pool) => match created_at_since {
            Some(since) => {
                sqlx::query_scalar::<_, String>(
                    "SELECT id
                 FROM messages
                 WHERE guild_id = ?1
                    AND author_user_id = ?2
                    AND deleted_at IS NULL
                    AND created_at >= ?3",
                )
                .bind(guild_id)
                .bind(author_user_id)
                .bind(since)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_scalar::<_, String>(
                    "SELECT id
                 FROM messages
                 WHERE guild_id = ?1
                    AND author_user_id = ?2
                    AND deleted_at IS NULL",
                )
                .bind(guild_id)
                .bind(author_user_id)
                .fetch_all(pool)
                .await
            }
        },
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(message_ids)
}

#[allow(clippy::too_many_arguments)]
pub async fn list_message_history_page_by_guild_and_author(
    pool: &DbPool,
    guild_id: &str,
    author_user_id: &str,
    channel_slug: Option<&str>,
    from_created_at: Option<&str>,
    to_created_at: Option<&str>,
    before: Option<&GuildAuthorMessageHistoryCursor>,
    limit: i64,
) -> Result<GuildAuthorMessageHistoryPage, AppError> {
    let normalized_limit = limit.clamp(1, 200);
    let fetch_limit = normalized_limit + 1;
    let mut entries = match pool {
        DbPool::Postgres(pool) => {
            let mut query_builder = QueryBuilder::<sqlx::Postgres>::new(
                "SELECT m.id,
                        c.slug AS channel_slug,
                        c.name AS channel_name,
                        m.content,
                        m.created_at
                 FROM messages m
                 JOIN channels c ON c.id = m.channel_id
                 WHERE m.guild_id = ",
            );
            query_builder.push_bind(guild_id);
            query_builder.push(" AND c.guild_id = ");
            query_builder.push_bind(guild_id);
            query_builder.push(" AND m.author_user_id = ");
            query_builder.push_bind(author_user_id);
            query_builder.push(" AND m.deleted_at IS NULL");
            if let Some(channel_slug) = channel_slug {
                query_builder.push(" AND c.slug = ");
                query_builder.push_bind(channel_slug);
            }
            if let Some(from_created_at) = from_created_at {
                query_builder.push(" AND m.created_at >= ");
                query_builder.push_bind(from_created_at);
            }
            if let Some(to_created_at) = to_created_at {
                query_builder.push(" AND m.created_at <= ");
                query_builder.push_bind(to_created_at);
            }
            if let Some(cursor) = before {
                query_builder.push(" AND (m.created_at < ");
                query_builder.push_bind(&cursor.created_at);
                query_builder.push(" OR (m.created_at = ");
                query_builder.push_bind(&cursor.created_at);
                query_builder.push(" AND m.id < ");
                query_builder.push_bind(&cursor.id);
                query_builder.push("))");
            }
            query_builder.push(" ORDER BY m.created_at DESC, m.id DESC LIMIT ");
            query_builder.push_bind(fetch_limit);
            query_builder
                .build_query_as::<GuildAuthorMessageHistoryRow>()
                .fetch_all(pool)
                .await
        }
        DbPool::Sqlite(pool) => {
            let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(
                "SELECT m.id,
                        c.slug AS channel_slug,
                        c.name AS channel_name,
                        m.content,
                        m.created_at
                 FROM messages m
                 JOIN channels c ON c.id = m.channel_id
                 WHERE m.guild_id = ",
            );
            query_builder.push_bind(guild_id);
            query_builder.push(" AND c.guild_id = ");
            query_builder.push_bind(guild_id);
            query_builder.push(" AND m.author_user_id = ");
            query_builder.push_bind(author_user_id);
            query_builder.push(" AND m.deleted_at IS NULL");
            if let Some(channel_slug) = channel_slug {
                query_builder.push(" AND c.slug = ");
                query_builder.push_bind(channel_slug);
            }
            if let Some(from_created_at) = from_created_at {
                query_builder.push(" AND m.created_at >= ");
                query_builder.push_bind(from_created_at);
            }
            if let Some(to_created_at) = to_created_at {
                query_builder.push(" AND m.created_at <= ");
                query_builder.push_bind(to_created_at);
            }
            if let Some(cursor) = before {
                query_builder.push(" AND (m.created_at < ");
                query_builder.push_bind(&cursor.created_at);
                query_builder.push(" OR (m.created_at = ");
                query_builder.push_bind(&cursor.created_at);
                query_builder.push(" AND m.id < ");
                query_builder.push_bind(&cursor.id);
                query_builder.push("))");
            }
            query_builder.push(" ORDER BY m.created_at DESC, m.id DESC LIMIT ");
            query_builder.push_bind(fetch_limit);
            query_builder
                .build_query_as::<GuildAuthorMessageHistoryRow>()
                .fetch_all(pool)
                .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let page_limit = usize::try_from(normalized_limit).unwrap_or(200);
    let has_more = entries.len() > page_limit;
    if has_more {
        entries.truncate(page_limit);
    }

    Ok(GuildAuthorMessageHistoryPage { entries, has_more })
}

pub async fn delete_messages_by_ids(
    pool: &DbPool,
    message_ids: &[String],
) -> Result<u64, AppError> {
    if message_ids.is_empty() {
        return Ok(0);
    }

    let rows = match pool {
        DbPool::Postgres(pool) => {
            let mut query_builder =
                QueryBuilder::<sqlx::Postgres>::new("DELETE FROM messages WHERE id IN (");
            let mut separated = query_builder.separated(", ");
            for message_id in message_ids {
                separated.push_bind(message_id);
            }
            query_builder.push(")");
            query_builder
                .build()
                .execute(pool)
                .await
                .map(|result| result.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            let mut query_builder =
                QueryBuilder::<sqlx::Sqlite>::new("DELETE FROM messages WHERE id IN (");
            let mut separated = query_builder.separated(", ");
            for message_id in message_ids {
                separated.push_bind(message_id);
            }
            query_builder.push(")");
            query_builder
                .build()
                .execute(pool)
                .await
                .map(|result| result.rows_affected())
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}

pub async fn list_messages_by_channel_id(
    pool: &DbPool,
    channel_id: &str,
    limit: i64,
) -> Result<Vec<Message>, AppError> {
    let page = list_messages_page_by_channel_id(pool, channel_id, None, limit).await?;
    Ok(page.messages)
}

pub async fn list_messages_page_by_channel_id(
    pool: &DbPool,
    channel_id: &str,
    before: Option<&MessageCursor>,
    limit: i64,
) -> Result<MessagePage, AppError> {
    let normalized_limit = limit.clamp(1, 200);
    let fetch_limit = normalized_limit + 1;
    let mut messages = match pool {
        DbPool::Postgres(pool) => match before {
            Some(cursor) => {
                sqlx::query_as(
                    "SELECT id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at
                     FROM messages
                     WHERE channel_id = $1
                       AND deleted_at IS NULL
                       AND (created_at < $2 OR (created_at = $2 AND id < $3))
                     ORDER BY created_at DESC, id DESC
                     LIMIT $4",
                )
                .bind(channel_id)
                .bind(&cursor.created_at)
                .bind(&cursor.id)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as(
                    "SELECT id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at
                     FROM messages
                     WHERE channel_id = $1
                       AND deleted_at IS NULL
                     ORDER BY created_at DESC, id DESC
                     LIMIT $2",
                )
                .bind(channel_id)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await
            }
        }
        DbPool::Sqlite(pool) => match before {
            Some(cursor) => {
                sqlx::query_as(
                    "SELECT id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at
                     FROM messages
                     WHERE channel_id = ?1
                       AND deleted_at IS NULL
                       AND (created_at < ?2 OR (created_at = ?2 AND id < ?3))
                     ORDER BY created_at DESC, id DESC
                     LIMIT ?4",
                )
                .bind(channel_id)
                .bind(&cursor.created_at)
                .bind(&cursor.id)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as(
                    "SELECT id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at
                     FROM messages
                     WHERE channel_id = ?1
                       AND deleted_at IS NULL
                     ORDER BY created_at DESC, id DESC
                     LIMIT ?2",
                )
                .bind(channel_id)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await
            }
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let page_limit = usize::try_from(normalized_limit).unwrap_or(200);
    let has_more = messages.len() > page_limit;
    if has_more {
        messages.truncate(page_limit);
    }
    messages.reverse();

    Ok(MessagePage { messages, has_more })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        config::DatabaseConfig,
        db::{init_pool, run_migrations},
    };

    async fn seed_message_fixture(pool: &DbPool) {
        let DbPool::Sqlite(pool) = pool else {
            panic!("test fixture expects sqlite pool");
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
            "INSERT INTO guilds (id, slug, name, description, owner_id, default_channel_slug, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7)",
        )
        .bind("guild-id")
        .bind("lobby")
        .bind("Lobby")
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
    }

    #[tokio::test]
    async fn sqlite_insert_and_list_messages_round_trip() {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        seed_message_fixture(&pool).await;

        let created_at = "2026-02-28T00:00:01Z";
        let created = insert_message(
            &pool,
            "message-1",
            "guild-id",
            "channel-id",
            "author-user-id",
            "Hello world",
            false,
            created_at,
            created_at,
        )
        .await
        .unwrap();
        assert!(created);

        let fetched = find_message_by_id(&pool, "message-1").await.unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.content, "Hello world");
        assert_eq!(fetched.is_system, 0);

        let listed = list_messages_by_channel_id(&pool, "channel-id", 50)
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "message-1");
    }

    #[tokio::test]
    async fn sqlite_cursor_pagination_orders_by_created_at_and_id_with_limit_clamp() {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        seed_message_fixture(&pool).await;

        let messages = [
            ("msg-001", "older-a", "2026-02-28T00:00:01Z"),
            ("msg-002", "older-b", "2026-02-28T00:00:01Z"),
            ("msg-003", "recent-a", "2026-02-28T00:00:02Z"),
            ("msg-004", "recent-b", "2026-02-28T00:00:03Z"),
        ];
        for (id, content, created_at) in messages {
            insert_message(
                &pool,
                id,
                "guild-id",
                "channel-id",
                "author-user-id",
                content,
                false,
                created_at,
                created_at,
            )
            .await
            .unwrap();
        }

        let first_page = list_messages_page_by_channel_id(&pool, "channel-id", None, 2)
            .await
            .unwrap();
        assert!(first_page.has_more);
        assert_eq!(
            first_page
                .messages
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["msg-003", "msg-004"]
        );

        let before = MessageCursor {
            created_at: "2026-02-28T00:00:02Z".to_string(),
            id: "msg-003".to_string(),
        };
        let second_page = list_messages_page_by_channel_id(&pool, "channel-id", Some(&before), 2)
            .await
            .unwrap();
        assert!(!second_page.has_more);
        assert_eq!(
            second_page
                .messages
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["msg-001", "msg-002"]
        );

        let clamped_page = list_messages_page_by_channel_id(&pool, "channel-id", None, 0)
            .await
            .unwrap();
        assert_eq!(clamped_page.messages.len(), 1);
        assert_eq!(clamped_page.messages[0].id, "msg-004");
    }

    #[tokio::test]
    async fn sqlite_message_history_page_filters_and_skips_soft_deleted_rows() {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        seed_message_fixture(&pool).await;

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("test fixture expects sqlite pool");
        };
        sqlx::query(
            "INSERT INTO channels (id, guild_id, slug, name, channel_type, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind("channel-random")
        .bind("guild-id")
        .bind("random")
        .bind("random")
        .bind("text")
        .bind(1_i64)
        .bind("2026-02-28T00:00:00Z")
        .bind("2026-02-28T00:00:00Z")
        .execute(sqlite_pool)
        .await
        .unwrap();

        for (id, channel_id, author_user_id, content, created_at) in [
            (
                "history-001",
                "channel-id",
                "author-user-id",
                "first",
                "2026-02-28T00:00:01Z",
            ),
            (
                "history-002",
                "channel-random",
                "author-user-id",
                "second",
                "2026-02-28T00:00:02Z",
            ),
            (
                "history-003",
                "channel-id",
                "author-user-id",
                "third",
                "2026-02-28T00:00:03Z",
            ),
            (
                "history-004",
                "channel-id",
                "owner-user-id",
                "owner-message",
                "2026-02-28T00:00:04Z",
            ),
            (
                "history-soft-deleted",
                "channel-id",
                "author-user-id",
                "soft-deleted",
                "2026-02-28T00:00:05Z",
            ),
        ] {
            insert_message(
                &pool,
                id,
                "guild-id",
                channel_id,
                author_user_id,
                content,
                false,
                created_at,
                created_at,
            )
            .await
            .unwrap();
        }

        sqlx::query(
            "INSERT INTO moderation_actions (
                id,
                action_type,
                guild_id,
                actor_user_id,
                target_user_id,
                reason,
                duration_seconds,
                expires_at,
                is_active,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind("mod-action-history")
        .bind("message_delete")
        .bind("guild-id")
        .bind("owner-user-id")
        .bind("author-user-id")
        .bind("cleanup")
        .bind(None::<i64>)
        .bind(None::<&str>)
        .bind(0_i64)
        .bind("2026-02-28T00:00:06Z")
        .bind("2026-02-28T00:00:06Z")
        .execute(sqlite_pool)
        .await
        .unwrap();

        soft_delete_message_by_id_for_moderation(
            &pool,
            "history-soft-deleted",
            "guild-id",
            "channel-id",
            "owner-user-id",
            "cleanup",
            "mod-action-history",
            "2026-02-28T00:00:06Z",
            "2026-02-28T00:00:06Z",
        )
        .await
        .unwrap();

        let first_page = list_message_history_page_by_guild_and_author(
            &pool,
            "guild-id",
            "author-user-id",
            None,
            None,
            None,
            None,
            2,
        )
        .await
        .unwrap();
        assert!(first_page.has_more);
        assert_eq!(
            first_page
                .entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["history-003", "history-002"]
        );
        assert_eq!(first_page.entries[0].channel_slug, "general");
        assert_eq!(first_page.entries[1].channel_slug, "random");

        let second_page = list_message_history_page_by_guild_and_author(
            &pool,
            "guild-id",
            "author-user-id",
            None,
            None,
            None,
            Some(&GuildAuthorMessageHistoryCursor {
                created_at: "2026-02-28T00:00:02Z".to_string(),
                id: "history-002".to_string(),
            }),
            2,
        )
        .await
        .unwrap();
        assert!(!second_page.has_more);
        assert_eq!(second_page.entries.len(), 1);
        assert_eq!(second_page.entries[0].id, "history-001");

        let filtered_by_channel = list_message_history_page_by_guild_and_author(
            &pool,
            "guild-id",
            "author-user-id",
            Some("general"),
            None,
            None,
            None,
            10,
        )
        .await
        .unwrap();
        assert_eq!(
            filtered_by_channel
                .entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["history-003", "history-001"]
        );

        let filtered_by_time = list_message_history_page_by_guild_and_author(
            &pool,
            "guild-id",
            "author-user-id",
            None,
            Some("2026-02-28T00:00:02Z"),
            Some("2026-02-28T00:00:03Z"),
            None,
            10,
        )
        .await
        .unwrap();
        assert_eq!(
            filtered_by_time
                .entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["history-003", "history-002"]
        );
    }

    #[tokio::test]
    async fn sqlite_update_message_respects_channel_and_author_scope() {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        seed_message_fixture(&pool).await;

        let created_at = "2026-02-28T00:00:01Z";
        insert_message(
            &pool,
            "message-1",
            "guild-id",
            "channel-id",
            "author-user-id",
            "hello",
            false,
            created_at,
            created_at,
        )
        .await
        .unwrap();

        let blocked = update_message_content_by_id_channel_and_author(
            &pool,
            "message-1",
            "channel-id",
            "owner-user-id",
            "blocked",
            "2026-02-28T00:00:02Z",
        )
        .await
        .unwrap();
        assert!(!blocked);

        let updated = update_message_content_by_id_channel_and_author(
            &pool,
            "message-1",
            "channel-id",
            "author-user-id",
            "updated",
            "2026-02-28T00:00:03Z",
        )
        .await
        .unwrap();
        assert!(updated);

        let stored = find_message_by_id(&pool, "message-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.content, "updated");
        assert_eq!(stored.created_at, created_at);
        assert_eq!(stored.updated_at, "2026-02-28T00:00:03Z");
    }

    #[tokio::test]
    async fn sqlite_delete_message_respects_channel_and_author_scope() {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        seed_message_fixture(&pool).await;

        let created_at = "2026-02-28T00:00:01Z";
        insert_message(
            &pool,
            "message-2",
            "guild-id",
            "channel-id",
            "author-user-id",
            "to-delete",
            false,
            created_at,
            created_at,
        )
        .await
        .unwrap();

        let blocked = delete_message_by_id_channel_and_author(
            &pool,
            "message-2",
            "channel-id",
            "owner-user-id",
        )
        .await
        .unwrap();
        assert!(!blocked);

        let deleted = delete_message_by_id_channel_and_author(
            &pool,
            "message-2",
            "channel-id",
            "author-user-id",
        )
        .await
        .unwrap();
        assert!(deleted);
        assert!(
            find_message_by_id(&pool, "message-2")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn sqlite_soft_delete_hides_message_from_find_list_and_author_queries() {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        seed_message_fixture(&pool).await;

        let first_timestamp = "2026-02-28T00:00:01Z";
        let second_timestamp = "2026-02-28T00:00:02Z";
        insert_message(
            &pool,
            "message-visible",
            "guild-id",
            "channel-id",
            "author-user-id",
            "visible",
            false,
            first_timestamp,
            first_timestamp,
        )
        .await
        .unwrap();
        insert_message(
            &pool,
            "message-soft-deleted",
            "guild-id",
            "channel-id",
            "author-user-id",
            "deleted",
            false,
            second_timestamp,
            second_timestamp,
        )
        .await
        .unwrap();

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("test fixture expects sqlite pool");
        };
        sqlx::query(
            "INSERT INTO moderation_actions (
                id,
                action_type,
                guild_id,
                actor_user_id,
                target_user_id,
                reason,
                duration_seconds,
                expires_at,
                is_active,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind("mod-action-1")
        .bind("message_delete")
        .bind("guild-id")
        .bind("owner-user-id")
        .bind("author-user-id")
        .bind("policy violation")
        .bind(None::<i64>)
        .bind(None::<&str>)
        .bind(0_i64)
        .bind("2026-02-28T00:00:03Z")
        .bind("2026-02-28T00:00:03Z")
        .execute(sqlite_pool)
        .await
        .unwrap();

        let soft_deleted = soft_delete_message_by_id_for_moderation(
            &pool,
            "message-soft-deleted",
            "guild-id",
            "channel-id",
            "owner-user-id",
            "policy violation",
            "mod-action-1",
            "2026-02-28T00:00:03Z",
            "2026-02-28T00:00:03Z",
        )
        .await
        .unwrap();
        assert!(soft_deleted);

        assert!(
            find_message_by_id(&pool, "message-soft-deleted")
                .await
                .unwrap()
                .is_none()
        );
        let listed = list_messages_by_channel_id(&pool, "channel-id", 50)
            .await
            .unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|message| message.id.as_str())
                .collect::<Vec<_>>(),
            vec!["message-visible"]
        );
        let author_message_ids =
            list_message_ids_by_guild_and_author_since(&pool, "guild-id", "author-user-id", None)
                .await
                .unwrap();
        assert_eq!(author_message_ids, vec!["message-visible".to_string()]);
    }
}
