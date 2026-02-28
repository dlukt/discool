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

pub async fn list_messages_by_channel_id(
    pool: &DbPool,
    channel_id: &str,
    limit: i64,
) -> Result<Vec<Message>, AppError> {
    let normalized_limit = limit.clamp(1, 200);
    let messages = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at
                 FROM messages
                 WHERE channel_id = $1
                 ORDER BY created_at ASC
                 LIMIT $2",
            )
            .bind(channel_id)
            .bind(normalized_limit)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at
                 FROM messages
                 WHERE channel_id = ?1
                 ORDER BY created_at ASC
                 LIMIT ?2",
            )
            .bind(channel_id)
            .bind(normalized_limit)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(messages)
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
}
