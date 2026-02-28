use crate::{AppError, db::DbPool};

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
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE messages
                 SET content = $1, updated_at = $2
                 WHERE id = $3
                   AND channel_id = $4
                   AND author_user_id = $5",
        )
        .bind(content)
        .bind(updated_at)
        .bind(message_id)
        .bind(channel_id)
        .bind(author_user_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE messages
                 SET content = ?1, updated_at = ?2
                 WHERE id = ?3
                   AND channel_id = ?4
                   AND author_user_id = ?5",
        )
        .bind(content)
        .bind(updated_at)
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
                   AND author_user_id = $3",
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
                   AND author_user_id = ?3",
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
}
