use crate::{
    AppError,
    config::{AttachmentConfig, AvatarConfig},
    db::DbPool,
    models::user,
    services::file_storage_service::FileStorageProvider,
};

pub async fn delete_user_account(
    pool: &DbPool,
    avatar_config: &AvatarConfig,
    attachment_config: &AttachmentConfig,
    user_id: &str,
) -> Result<(), AppError> {
    let normalized_user_id = normalize_user_id(user_id)?;

    let user = user::find_user_by_id(pool, &normalized_user_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let attachment_storage = FileStorageProvider::local(attachment_config.upload_dir.clone());
    let avatar_storage = FileStorageProvider::local(avatar_config.upload_dir.clone());

    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            ensure_user_does_not_own_guilds_postgres(&mut tx, &normalized_user_id).await?;
            let attachment_storage_keys =
                list_attachment_storage_keys_postgres(&mut tx, &normalized_user_id).await?;
            let deleted_rows = sqlx::query("DELETE FROM users WHERE id = $1")
                .bind(&normalized_user_id)
                .execute(&mut *tx)
                .await
                .map_err(map_delete_user_error)?
                .rows_affected();
            if deleted_rows != 1 {
                return Err(AppError::NotFound);
            }

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

            delete_user_files(
                &avatar_storage,
                user.avatar_storage_key.as_deref(),
                &attachment_storage,
                &attachment_storage_keys,
            )
            .await?;
        }
        DbPool::Sqlite(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            ensure_user_does_not_own_guilds_sqlite(&mut tx, &normalized_user_id).await?;
            let attachment_storage_keys =
                list_attachment_storage_keys_sqlite(&mut tx, &normalized_user_id).await?;
            let deleted_rows = sqlx::query("DELETE FROM users WHERE id = ?1")
                .bind(&normalized_user_id)
                .execute(&mut *tx)
                .await
                .map_err(map_delete_user_error)?
                .rows_affected();
            if deleted_rows != 1 {
                return Err(AppError::NotFound);
            }

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

            delete_user_files(
                &avatar_storage,
                user.avatar_storage_key.as_deref(),
                &attachment_storage,
                &attachment_storage_keys,
            )
            .await?;
        }
    }

    Ok(())
}

fn normalize_user_id(value: &str) -> Result<String, AppError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(AppError::ValidationError("user_id is required".to_string()));
    }
    Ok(normalized.to_string())
}

async fn ensure_user_does_not_own_guilds_postgres(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<(), AppError> {
    let owned_guild_slugs = sqlx::query_scalar::<_, String>(
        "SELECT slug FROM guilds WHERE owner_id = $1 ORDER BY slug",
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    ensure_user_does_not_own_guilds(owned_guild_slugs)
}

async fn ensure_user_does_not_own_guilds_sqlite(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    user_id: &str,
) -> Result<(), AppError> {
    let owned_guild_slugs = sqlx::query_scalar::<_, String>(
        "SELECT slug FROM guilds WHERE owner_id = ?1 ORDER BY slug",
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    ensure_user_does_not_own_guilds(owned_guild_slugs)
}

async fn list_attachment_storage_keys_postgres(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<Vec<String>, AppError> {
    sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT ma.storage_key
         FROM message_attachments ma
         JOIN messages m ON m.id = ma.message_id
         WHERE m.author_user_id = $1
         ORDER BY ma.storage_key ASC",
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))
}

async fn list_attachment_storage_keys_sqlite(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    user_id: &str,
) -> Result<Vec<String>, AppError> {
    sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT ma.storage_key
         FROM message_attachments ma
         JOIN messages m ON m.id = ma.message_id
         WHERE m.author_user_id = ?1
         ORDER BY ma.storage_key ASC",
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))
}

fn ensure_user_does_not_own_guilds(mut owned_guild_slugs: Vec<String>) -> Result<(), AppError> {
    owned_guild_slugs.sort();
    owned_guild_slugs.dedup();

    if owned_guild_slugs.is_empty() {
        return Ok(());
    }

    let guild_list = owned_guild_slugs.join(", ");
    Err(AppError::Conflict(format!(
        "Cannot delete account while owning guilds. Transfer ownership or delete owned guilds first: {guild_list}"
    )))
}

fn map_delete_user_error(err: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.is_foreign_key_violation()
    {
        return AppError::Conflict(
            "Cannot delete account while owning guilds. Transfer ownership or delete owned guilds first."
                .to_string(),
        );
    }

    AppError::Internal(err.to_string())
}

async fn delete_user_files(
    avatar_storage: &FileStorageProvider,
    avatar_storage_key: Option<&str>,
    attachment_storage: &FileStorageProvider,
    attachment_storage_keys: &[String],
) -> Result<(), AppError> {
    for storage_key in attachment_storage_keys {
        attachment_storage
            .delete(storage_key)
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed to delete attachment file: {err:?}"))
            })?;
    }

    if let Some(storage_key) = avatar_storage_key {
        avatar_storage
            .delete(storage_key)
            .await
            .map_err(|err| AppError::Internal(format!("Failed to delete avatar file: {err:?}")))?;
    }

    Ok(())
}
