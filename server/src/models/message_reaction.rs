use std::collections::HashMap;

use sqlx::QueryBuilder;

use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageReactionSummary {
    pub emoji: String,
    pub count: i64,
    pub reacted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageReactionEntry {
    pub emoji: String,
    pub user_id: String,
}

pub async fn has_message_reaction(
    pool: &DbPool,
    message_id: &str,
    user_id: &str,
    emoji: &str,
) -> Result<bool, AppError> {
    let exists = match pool {
        DbPool::Postgres(pool) => sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                    SELECT 1
                    FROM message_reactions
                    WHERE message_id = $1
                      AND user_id = $2
                      AND emoji = $3
                )",
        )
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .fetch_one(pool)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?,
        DbPool::Sqlite(pool) => {
            let exists = sqlx::query_scalar::<_, i64>(
                "SELECT EXISTS(
                    SELECT 1
                    FROM message_reactions
                    WHERE message_id = ?1
                      AND user_id = ?2
                      AND emoji = ?3
                )",
            )
            .bind(message_id)
            .bind(user_id)
            .bind(emoji)
            .fetch_one(pool)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
            exists != 0
        }
    };
    Ok(exists)
}

pub async fn insert_message_reaction(
    pool: &DbPool,
    message_id: &str,
    user_id: &str,
    emoji: &str,
    created_at: &str,
) -> Result<bool, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO message_reactions (message_id, user_id, emoji, created_at)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT DO NOTHING",
        )
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .bind(created_at)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO message_reactions (message_id, user_id, emoji, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT DO NOTHING",
        )
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .bind(created_at)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected == 1)
}

pub async fn delete_message_reaction(
    pool: &DbPool,
    message_id: &str,
    user_id: &str,
    emoji: &str,
) -> Result<bool, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "DELETE FROM message_reactions
                 WHERE message_id = $1
                   AND user_id = $2
                   AND emoji = $3",
        )
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "DELETE FROM message_reactions
                 WHERE message_id = ?1
                   AND user_id = ?2
                   AND emoji = ?3",
        )
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected == 1)
}

pub async fn list_reaction_summaries_by_message_id(
    pool: &DbPool,
    message_id: &str,
    viewer_user_id: &str,
) -> Result<Vec<MessageReactionSummary>, AppError> {
    let summaries_by_message =
        list_reaction_summaries_by_message_ids(pool, &[message_id.to_string()], viewer_user_id)
            .await?;
    Ok(summaries_by_message
        .get(message_id)
        .cloned()
        .unwrap_or_default())
}

pub async fn list_reaction_entries_by_message_id(
    pool: &DbPool,
    message_id: &str,
) -> Result<Vec<MessageReactionEntry>, AppError> {
    let rows: Vec<(String, String)> = match pool {
        DbPool::Postgres(pool) => sqlx::query_as::<_, (String, String)>(
            "SELECT emoji, user_id
                 FROM message_reactions
                 WHERE message_id = $1
                 ORDER BY emoji ASC, user_id ASC",
        )
        .bind(message_id)
        .fetch_all(pool)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?,
        DbPool::Sqlite(pool) => sqlx::query_as::<_, (String, String)>(
            "SELECT emoji, user_id
                 FROM message_reactions
                 WHERE message_id = ?1
                 ORDER BY emoji ASC, user_id ASC",
        )
        .bind(message_id)
        .fetch_all(pool)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?,
    };

    Ok(rows
        .into_iter()
        .map(|(emoji, user_id)| MessageReactionEntry { emoji, user_id })
        .collect())
}

pub async fn list_reaction_summaries_by_message_ids(
    pool: &DbPool,
    message_ids: &[String],
    viewer_user_id: &str,
) -> Result<HashMap<String, Vec<MessageReactionSummary>>, AppError> {
    if message_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows: Vec<(String, String, i64, i64)> = match pool {
        DbPool::Postgres(pool) => {
            let mut query_builder = QueryBuilder::<sqlx::Postgres>::new(
                "SELECT message_id,
                        emoji,
                        COUNT(*) AS reaction_count,
                        MAX(CASE WHEN user_id = ",
            );
            query_builder.push_bind(viewer_user_id);
            query_builder.push(
                " THEN 1 ELSE 0 END) AS reacted
                 FROM message_reactions
                 WHERE message_id IN (",
            );
            let mut separated = query_builder.separated(", ");
            for message_id in message_ids {
                separated.push_bind(message_id);
            }
            query_builder.push(
                ")
                 GROUP BY message_id, emoji
                 ORDER BY message_id ASC, reaction_count DESC, emoji ASC",
            );

            query_builder
                .build_query_as::<(String, String, i64, i64)>()
                .fetch_all(pool)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
        }
        DbPool::Sqlite(pool) => {
            let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(
                "SELECT message_id,
                        emoji,
                        COUNT(*) AS reaction_count,
                        MAX(CASE WHEN user_id = ",
            );
            query_builder.push_bind(viewer_user_id);
            query_builder.push(
                " THEN 1 ELSE 0 END) AS reacted
                 FROM message_reactions
                 WHERE message_id IN (",
            );
            let mut separated = query_builder.separated(", ");
            for message_id in message_ids {
                separated.push_bind(message_id);
            }
            query_builder.push(
                ")
                 GROUP BY message_id, emoji
                 ORDER BY message_id ASC, reaction_count DESC, emoji ASC",
            );

            query_builder
                .build_query_as::<(String, String, i64, i64)>()
                .fetch_all(pool)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
        }
    };

    let mut grouped: HashMap<String, Vec<MessageReactionSummary>> = HashMap::new();
    for (message_id, emoji, count, reacted) in rows {
        grouped
            .entry(message_id)
            .or_default()
            .push(MessageReactionSummary {
                emoji,
                count,
                reacted: reacted != 0,
            });
    }
    Ok(grouped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::DatabaseConfig,
        db::{init_pool, run_migrations},
        models::message,
    };

    async fn setup_reaction_pool() -> DbPool {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        seed_reaction_fixture(&pool).await;
        pool
    }

    async fn seed_reaction_fixture(pool: &DbPool) {
        let DbPool::Sqlite(pool) = pool else {
            panic!("reaction model tests expect sqlite pool");
        };

        let created_at = "2026-02-28T00:00:00Z";
        for (id, username) in [("owner-user-id", "owner"), ("peer-user-id", "peer")] {
            sqlx::query(
                "INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .bind(id)
            .bind(format!("did:key:z6Mk{id}"))
            .bind(format!("z{id}"))
            .bind(username)
            .bind("#3366ff")
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

        let db_pool = DbPool::Sqlite(pool.clone());
        message::insert_message(
            &db_pool,
            "message-1",
            "guild-id",
            "channel-id",
            "owner-user-id",
            "hello",
            false,
            "2026-02-28T00:00:01Z",
            "2026-02-28T00:00:01Z",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn sqlite_reaction_toggle_persistence_and_aggregation() {
        let pool = setup_reaction_pool().await;

        let inserted = insert_message_reaction(
            &pool,
            "message-1",
            "owner-user-id",
            "😀",
            "2026-02-28T00:00:02Z",
        )
        .await
        .unwrap();
        assert!(inserted);

        let duplicate = insert_message_reaction(
            &pool,
            "message-1",
            "owner-user-id",
            "😀",
            "2026-02-28T00:00:03Z",
        )
        .await
        .unwrap();
        assert!(!duplicate);

        insert_message_reaction(
            &pool,
            "message-1",
            "peer-user-id",
            "😀",
            "2026-02-28T00:00:04Z",
        )
        .await
        .unwrap();
        insert_message_reaction(
            &pool,
            "message-1",
            "peer-user-id",
            "🎉",
            "2026-02-28T00:00:05Z",
        )
        .await
        .unwrap();

        let owner_view = list_reaction_summaries_by_message_id(&pool, "message-1", "owner-user-id")
            .await
            .unwrap();
        assert_eq!(owner_view.len(), 2);
        assert_eq!(owner_view[0].emoji, "😀");
        assert_eq!(owner_view[0].count, 2);
        assert!(owner_view[0].reacted);
        assert_eq!(owner_view[1].emoji, "🎉");
        assert_eq!(owner_view[1].count, 1);
        assert!(!owner_view[1].reacted);

        let removed = delete_message_reaction(&pool, "message-1", "owner-user-id", "😀")
            .await
            .unwrap();
        assert!(removed);

        let owner_view_after =
            list_reaction_summaries_by_message_id(&pool, "message-1", "owner-user-id")
                .await
                .unwrap();
        assert_eq!(owner_view_after.len(), 2);
        let smile = owner_view_after
            .iter()
            .find(|item| item.emoji == "😀")
            .expect("expected 😀 summary after removal");
        assert_eq!(smile.count, 1);
        assert!(!smile.reacted);
    }
}
