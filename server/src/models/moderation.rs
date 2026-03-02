use crate::{AppError, db::DbPool};
use sqlx::QueryBuilder;

pub const MODERATION_ACTION_TYPE_MUTE: &str = "mute";
pub const MODERATION_ACTION_TYPE_KICK: &str = "kick";
pub const MODERATION_ACTION_TYPE_BAN: &str = "ban";
pub const MODERATION_ACTION_TYPE_VOICE_KICK: &str = "voice_kick";
pub const MODERATION_ACTION_TYPE_MESSAGE_DELETE: &str = "message_delete";
pub const MODERATION_ACTION_TYPE_WARN: &str = "warn";
pub const REPORT_TARGET_TYPE_MESSAGE: &str = "message";
pub const REPORT_TARGET_TYPE_USER: &str = "user";
pub const REPORT_CATEGORY_SPAM: &str = "spam";
pub const REPORT_CATEGORY_HARASSMENT: &str = "harassment";
pub const REPORT_CATEGORY_RULE_VIOLATION: &str = "rule_violation";
pub const REPORT_CATEGORY_OTHER: &str = "other";
pub const REPORT_STATUS_PENDING: &str = "pending";
pub const REPORT_STATUS_REVIEWED: &str = "reviewed";
pub const REPORT_STATUS_ACTIONED: &str = "actioned";
pub const REPORT_STATUS_DISMISSED: &str = "dismissed";

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModerationActionRecord {
    pub id: String,
    pub action_type: String,
    pub guild_id: String,
    pub actor_user_id: String,
    pub target_user_id: String,
    pub reason: String,
    pub duration_seconds: Option<i64>,
    pub expires_at: Option<String>,
    pub is_active: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModerationLogCursor {
    pub created_at: String,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModerationLogSortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModerationLogRow {
    pub id: String,
    pub action_type: String,
    pub reason: String,
    pub created_at: String,
    pub actor_user_id: String,
    pub actor_username: String,
    pub actor_display_name: Option<String>,
    pub actor_avatar_color: Option<String>,
    pub target_user_id: String,
    pub target_username: String,
    pub target_display_name: Option<String>,
    pub target_avatar_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModerationLogPage {
    pub entries: Vec<ModerationLogRow>,
    pub has_more: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReportRecord {
    pub id: String,
    pub guild_id: String,
    pub reporter_user_id: String,
    pub target_type: String,
    pub target_message_id: Option<String>,
    pub target_user_id: Option<String>,
    pub reason: String,
    pub category: Option<String>,
    pub status: String,
    pub reviewed_at: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub actioned_at: Option<String>,
    pub actioned_by_user_id: Option<String>,
    pub dismissed_at: Option<String>,
    pub dismissed_by_user_id: Option<String>,
    pub dismissal_reason: Option<String>,
    pub moderation_action_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportQueueCursor {
    pub created_at: String,
    pub id: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReportQueueRow {
    pub id: String,
    pub guild_id: String,
    pub reporter_user_id: String,
    pub reporter_username: String,
    pub reporter_display_name: Option<String>,
    pub reporter_avatar_color: Option<String>,
    pub target_type: String,
    pub target_message_id: Option<String>,
    pub target_user_id: Option<String>,
    pub target_username: Option<String>,
    pub target_display_name: Option<String>,
    pub target_avatar_color: Option<String>,
    pub target_message_content: Option<String>,
    pub reason: String,
    pub category: Option<String>,
    pub status: String,
    pub reviewed_at: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub actioned_at: Option<String>,
    pub actioned_by_user_id: Option<String>,
    pub dismissed_at: Option<String>,
    pub dismissed_by_user_id: Option<String>,
    pub dismissal_reason: Option<String>,
    pub moderation_action_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ReportQueuePage {
    pub entries: Vec<ReportQueueRow>,
    pub has_more: bool,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_moderation_action(
    pool: &DbPool,
    id: &str,
    action_type: &str,
    guild_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
    reason: &str,
    duration_seconds: Option<i64>,
    expires_at: Option<&str>,
    is_active: bool,
    created_at: &str,
    updated_at: &str,
) -> Result<(), AppError> {
    let is_active_value = if is_active { 1_i64 } else { 0_i64 };
    match pool {
        DbPool::Postgres(pool) => sqlx::query(
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
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(action_type)
        .bind(guild_id)
        .bind(actor_user_id)
        .bind(target_user_id)
        .bind(reason)
        .bind(duration_seconds)
        .bind(expires_at)
        .bind(is_active_value)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await
        .map(|_| ()),
        DbPool::Sqlite(pool) => sqlx::query(
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
        .bind(id)
        .bind(action_type)
        .bind(guild_id)
        .bind(actor_user_id)
        .bind(target_user_id)
        .bind(reason)
        .bind(duration_seconds)
        .bind(expires_at)
        .bind(is_active_value)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await
        .map(|_| ()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_report(
    pool: &DbPool,
    id: &str,
    guild_id: &str,
    reporter_user_id: &str,
    target_type: &str,
    target_message_id: Option<&str>,
    target_user_id: Option<&str>,
    reason: &str,
    category: Option<&str>,
    status: &str,
    created_at: &str,
    updated_at: &str,
) -> Result<(), AppError> {
    let result = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO reports (
                    id,
                    guild_id,
                    reporter_user_id,
                    target_type,
                    target_message_id,
                    target_user_id,
                    reason,
                    category,
                    status,
                    created_at,
                    updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(guild_id)
        .bind(reporter_user_id)
        .bind(target_type)
        .bind(target_message_id)
        .bind(target_user_id)
        .bind(reason)
        .bind(category)
        .bind(status)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await
        .map(|_| ()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO reports (
                    id,
                    guild_id,
                    reporter_user_id,
                    target_type,
                    target_message_id,
                    target_user_id,
                    reason,
                    category,
                    status,
                    created_at,
                    updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(id)
        .bind(guild_id)
        .bind(reporter_user_id)
        .bind(target_type)
        .bind(target_message_id)
        .bind(target_user_id)
        .bind(reason)
        .bind(category)
        .bind(status)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await
        .map(|_| ()),
    };
    result.map_err(report_insert_error_to_app_error)?;
    Ok(())
}

pub async fn list_reports_by_guild_and_status(
    pool: &DbPool,
    guild_id: &str,
    status: &str,
    limit: i64,
) -> Result<Vec<ReportRecord>, AppError> {
    let normalized_limit = limit.clamp(1, 200);
    let rows = match pool {
        DbPool::Postgres(pool) => {
            let mut query = QueryBuilder::<sqlx::Postgres>::new(
                "SELECT id,
                        guild_id,
                        reporter_user_id,
                        target_type,
                        target_message_id,
                        target_user_id,
                        reason,
                        category,
                        status,
                        reviewed_at,
                        reviewed_by_user_id,
                        actioned_at,
                        actioned_by_user_id,
                        dismissed_at,
                        dismissed_by_user_id,
                        dismissal_reason,
                        moderation_action_id,
                        created_at,
                        updated_at
                 FROM reports
                 WHERE guild_id = ",
            );
            query.push_bind(guild_id);
            query.push(" AND status = ");
            query.push_bind(status);
            query.push(" ORDER BY created_at ASC, id ASC LIMIT ");
            query.push_bind(normalized_limit);
            query.build_query_as().fetch_all(pool).await
        }
        DbPool::Sqlite(pool) => {
            let mut query = QueryBuilder::<sqlx::Sqlite>::new(
                "SELECT id,
                        guild_id,
                        reporter_user_id,
                        target_type,
                        target_message_id,
                        target_user_id,
                        reason,
                        category,
                        status,
                        reviewed_at,
                        reviewed_by_user_id,
                        actioned_at,
                        actioned_by_user_id,
                        dismissed_at,
                        dismissed_by_user_id,
                        dismissal_reason,
                        moderation_action_id,
                        created_at,
                        updated_at
                 FROM reports
                 WHERE guild_id = ",
            );
            query.push_bind(guild_id);
            query.push(" AND status = ");
            query.push_bind(status);
            query.push(" ORDER BY created_at ASC, id ASC LIMIT ");
            query.push_bind(normalized_limit);
            query.build_query_as().fetch_all(pool).await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(rows)
}

pub async fn find_report_by_id(
    pool: &DbPool,
    report_id: &str,
) -> Result<Option<ReportRecord>, AppError> {
    let report = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id,
                        guild_id,
                        reporter_user_id,
                        target_type,
                        target_message_id,
                        target_user_id,
                        reason,
                        category,
                        status,
                        reviewed_at,
                        reviewed_by_user_id,
                        actioned_at,
                        actioned_by_user_id,
                        dismissed_at,
                        dismissed_by_user_id,
                        dismissal_reason,
                        moderation_action_id,
                        created_at,
                        updated_at
                 FROM reports
                 WHERE id = $1
                 LIMIT 1",
            )
            .bind(report_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id,
                        guild_id,
                        reporter_user_id,
                        target_type,
                        target_message_id,
                        target_user_id,
                        reason,
                        category,
                        status,
                        reviewed_at,
                        reviewed_by_user_id,
                        actioned_at,
                        actioned_by_user_id,
                        dismissed_at,
                        dismissed_by_user_id,
                        dismissal_reason,
                        moderation_action_id,
                        created_at,
                        updated_at
                 FROM reports
                 WHERE id = ?1
                 LIMIT 1",
            )
            .bind(report_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(report)
}

pub async fn find_report_queue_row_by_id(
    pool: &DbPool,
    report_id: &str,
) -> Result<Option<ReportQueueRow>, AppError> {
    let row = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT
                    r.id,
                    r.guild_id,
                    r.reporter_user_id,
                    reporter.username AS reporter_username,
                    reporter.display_name AS reporter_display_name,
                    reporter.avatar_color AS reporter_avatar_color,
                    r.target_type,
                    r.target_message_id,
                    r.target_user_id,
                    target.username AS target_username,
                    target.display_name AS target_display_name,
                    target.avatar_color AS target_avatar_color,
                    msg.content AS target_message_content,
                    r.reason,
                    r.category,
                    r.status,
                    r.reviewed_at,
                    r.reviewed_by_user_id,
                    r.actioned_at,
                    r.actioned_by_user_id,
                    r.dismissed_at,
                    r.dismissed_by_user_id,
                    r.dismissal_reason,
                    r.moderation_action_id,
                    r.created_at,
                    r.updated_at
                 FROM reports r
                 JOIN users reporter ON reporter.id = r.reporter_user_id
                 LEFT JOIN messages msg ON msg.id = r.target_message_id
                 LEFT JOIN users target ON target.id = COALESCE(r.target_user_id, msg.author_user_id)
                 WHERE r.id = $1
                 LIMIT 1",
            )
            .bind(report_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT
                    r.id,
                    r.guild_id,
                    r.reporter_user_id,
                    reporter.username AS reporter_username,
                    reporter.display_name AS reporter_display_name,
                    reporter.avatar_color AS reporter_avatar_color,
                    r.target_type,
                    r.target_message_id,
                    r.target_user_id,
                    target.username AS target_username,
                    target.display_name AS target_display_name,
                    target.avatar_color AS target_avatar_color,
                    msg.content AS target_message_content,
                    r.reason,
                    r.category,
                    r.status,
                    r.reviewed_at,
                    r.reviewed_by_user_id,
                    r.actioned_at,
                    r.actioned_by_user_id,
                    r.dismissed_at,
                    r.dismissed_by_user_id,
                    r.dismissal_reason,
                    r.moderation_action_id,
                    r.created_at,
                    r.updated_at
                 FROM reports r
                 JOIN users reporter ON reporter.id = r.reporter_user_id
                 LEFT JOIN messages msg ON msg.id = r.target_message_id
                 LEFT JOIN users target ON target.id = COALESCE(r.target_user_id, msg.author_user_id)
                 WHERE r.id = ?1
                 LIMIT 1",
            )
            .bind(report_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(row)
}

pub async fn list_report_queue_page_by_guild_id(
    pool: &DbPool,
    guild_id: &str,
    status: Option<&str>,
    cursor: Option<&ReportQueueCursor>,
    limit: i64,
) -> Result<ReportQueuePage, AppError> {
    let normalized_limit = limit.clamp(1, 200);
    let fetch_limit = normalized_limit + 1;
    let mut entries = match pool {
        DbPool::Postgres(pool) => {
            let mut query = QueryBuilder::<sqlx::Postgres>::new(
                "SELECT
                    r.id,
                    r.guild_id,
                    r.reporter_user_id,
                    reporter.username AS reporter_username,
                    reporter.display_name AS reporter_display_name,
                    reporter.avatar_color AS reporter_avatar_color,
                    r.target_type,
                    r.target_message_id,
                    r.target_user_id,
                    target.username AS target_username,
                    target.display_name AS target_display_name,
                    target.avatar_color AS target_avatar_color,
                    msg.content AS target_message_content,
                    r.reason,
                    r.category,
                    r.status,
                    r.reviewed_at,
                    r.reviewed_by_user_id,
                    r.actioned_at,
                    r.actioned_by_user_id,
                    r.dismissed_at,
                    r.dismissed_by_user_id,
                    r.dismissal_reason,
                    r.moderation_action_id,
                    r.created_at,
                    r.updated_at
                 FROM reports r
                 JOIN users reporter ON reporter.id = r.reporter_user_id
                 LEFT JOIN messages msg ON msg.id = r.target_message_id
                 LEFT JOIN users target ON target.id = COALESCE(r.target_user_id, msg.author_user_id)
                 WHERE r.guild_id = ",
            );
            query.push_bind(guild_id);
            if let Some(status) = status {
                query.push(" AND r.status = ");
                query.push_bind(status);
            }
            if let Some(cursor) = cursor {
                query.push(" AND (r.created_at < ");
                query.push_bind(&cursor.created_at);
                query.push(" OR (r.created_at = ");
                query.push_bind(&cursor.created_at);
                query.push(" AND r.id < ");
                query.push_bind(&cursor.id);
                query.push("))");
            }
            query.push(" ORDER BY r.created_at DESC, r.id DESC LIMIT ");
            query.push_bind(fetch_limit);
            query.build_query_as().fetch_all(pool).await
        }
        DbPool::Sqlite(pool) => {
            let mut query = QueryBuilder::<sqlx::Sqlite>::new(
                "SELECT
                    r.id,
                    r.guild_id,
                    r.reporter_user_id,
                    reporter.username AS reporter_username,
                    reporter.display_name AS reporter_display_name,
                    reporter.avatar_color AS reporter_avatar_color,
                    r.target_type,
                    r.target_message_id,
                    r.target_user_id,
                    target.username AS target_username,
                    target.display_name AS target_display_name,
                    target.avatar_color AS target_avatar_color,
                    msg.content AS target_message_content,
                    r.reason,
                    r.category,
                    r.status,
                    r.reviewed_at,
                    r.reviewed_by_user_id,
                    r.actioned_at,
                    r.actioned_by_user_id,
                    r.dismissed_at,
                    r.dismissed_by_user_id,
                    r.dismissal_reason,
                    r.moderation_action_id,
                    r.created_at,
                    r.updated_at
                 FROM reports r
                 JOIN users reporter ON reporter.id = r.reporter_user_id
                 LEFT JOIN messages msg ON msg.id = r.target_message_id
                 LEFT JOIN users target ON target.id = COALESCE(r.target_user_id, msg.author_user_id)
                 WHERE r.guild_id = ",
            );
            query.push_bind(guild_id);
            if let Some(status) = status {
                query.push(" AND r.status = ");
                query.push_bind(status);
            }
            if let Some(cursor) = cursor {
                query.push(" AND (r.created_at < ");
                query.push_bind(&cursor.created_at);
                query.push(" OR (r.created_at = ");
                query.push_bind(&cursor.created_at);
                query.push(" AND r.id < ");
                query.push_bind(&cursor.id);
                query.push("))");
            }
            query.push(" ORDER BY r.created_at DESC, r.id DESC LIMIT ");
            query.push_bind(fetch_limit);
            query.build_query_as().fetch_all(pool).await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let page_limit = usize::try_from(normalized_limit).unwrap_or(200);
    let has_more = entries.len() > page_limit;
    if has_more {
        entries.truncate(page_limit);
    }

    Ok(ReportQueuePage { entries, has_more })
}

pub async fn update_report_reviewed(
    pool: &DbPool,
    report_id: &str,
    reviewed_at: &str,
    reviewed_by_user_id: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE reports
                 SET status = $1,
                     reviewed_at = $2,
                     reviewed_by_user_id = $3,
                     actioned_at = NULL,
                     actioned_by_user_id = NULL,
                     dismissed_at = NULL,
                     dismissed_by_user_id = NULL,
                     dismissal_reason = NULL,
                     moderation_action_id = NULL,
                     updated_at = $4
                 WHERE id = $5
                   AND status = $6",
        )
        .bind(REPORT_STATUS_REVIEWED)
        .bind(reviewed_at)
        .bind(reviewed_by_user_id)
        .bind(updated_at)
        .bind(report_id)
        .bind(REPORT_STATUS_PENDING)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE reports
                 SET status = ?1,
                     reviewed_at = ?2,
                     reviewed_by_user_id = ?3,
                     actioned_at = NULL,
                     actioned_by_user_id = NULL,
                     dismissed_at = NULL,
                     dismissed_by_user_id = NULL,
                     dismissal_reason = NULL,
                     moderation_action_id = NULL,
                     updated_at = ?4
                 WHERE id = ?5
                   AND status = ?6",
        )
        .bind(REPORT_STATUS_REVIEWED)
        .bind(reviewed_at)
        .bind(reviewed_by_user_id)
        .bind(updated_at)
        .bind(report_id)
        .bind(REPORT_STATUS_PENDING)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected == 1)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_report_dismissed(
    pool: &DbPool,
    report_id: &str,
    expected_status: &str,
    reviewed_at: &str,
    reviewed_by_user_id: &str,
    dismissed_at: &str,
    dismissed_by_user_id: &str,
    dismissal_reason: Option<&str>,
    updated_at: &str,
) -> Result<bool, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE reports
                 SET status = $1,
                     reviewed_at = $2,
                     reviewed_by_user_id = $3,
                     actioned_at = NULL,
                     actioned_by_user_id = NULL,
                     dismissed_at = $4,
                     dismissed_by_user_id = $5,
                     dismissal_reason = $6,
                     moderation_action_id = NULL,
                     updated_at = $7
                 WHERE id = $8
                   AND status = $9",
        )
        .bind(REPORT_STATUS_DISMISSED)
        .bind(reviewed_at)
        .bind(reviewed_by_user_id)
        .bind(dismissed_at)
        .bind(dismissed_by_user_id)
        .bind(dismissal_reason)
        .bind(updated_at)
        .bind(report_id)
        .bind(expected_status)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE reports
                 SET status = ?1,
                     reviewed_at = ?2,
                     reviewed_by_user_id = ?3,
                     actioned_at = NULL,
                     actioned_by_user_id = NULL,
                     dismissed_at = ?4,
                     dismissed_by_user_id = ?5,
                     dismissal_reason = ?6,
                     moderation_action_id = NULL,
                     updated_at = ?7
                 WHERE id = ?8
                   AND status = ?9",
        )
        .bind(REPORT_STATUS_DISMISSED)
        .bind(reviewed_at)
        .bind(reviewed_by_user_id)
        .bind(dismissed_at)
        .bind(dismissed_by_user_id)
        .bind(dismissal_reason)
        .bind(updated_at)
        .bind(report_id)
        .bind(expected_status)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected == 1)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_report_actioned(
    pool: &DbPool,
    report_id: &str,
    expected_status: &str,
    reviewed_at: &str,
    reviewed_by_user_id: &str,
    actioned_at: &str,
    actioned_by_user_id: &str,
    moderation_action_id: Option<&str>,
    updated_at: &str,
) -> Result<bool, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE reports
                 SET status = $1,
                     reviewed_at = $2,
                     reviewed_by_user_id = $3,
                     actioned_at = $4,
                     actioned_by_user_id = $5,
                     dismissed_at = NULL,
                     dismissed_by_user_id = NULL,
                     dismissal_reason = NULL,
                     moderation_action_id = $6,
                     updated_at = $7
                 WHERE id = $8
                   AND status = $9",
        )
        .bind(REPORT_STATUS_ACTIONED)
        .bind(reviewed_at)
        .bind(reviewed_by_user_id)
        .bind(actioned_at)
        .bind(actioned_by_user_id)
        .bind(moderation_action_id)
        .bind(updated_at)
        .bind(report_id)
        .bind(expected_status)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE reports
                 SET status = ?1,
                     reviewed_at = ?2,
                     reviewed_by_user_id = ?3,
                     actioned_at = ?4,
                     actioned_by_user_id = ?5,
                     dismissed_at = NULL,
                     dismissed_by_user_id = NULL,
                     dismissal_reason = NULL,
                     moderation_action_id = ?6,
                     updated_at = ?7
                 WHERE id = ?8
                   AND status = ?9",
        )
        .bind(REPORT_STATUS_ACTIONED)
        .bind(reviewed_at)
        .bind(reviewed_by_user_id)
        .bind(actioned_at)
        .bind(actioned_by_user_id)
        .bind(moderation_action_id)
        .bind(updated_at)
        .bind(report_id)
        .bind(expected_status)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected == 1)
}

pub async fn revert_report_actioned_to_reviewed(
    pool: &DbPool,
    report_id: &str,
    reviewed_at: &str,
    reviewed_by_user_id: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE reports
                 SET status = $1,
                     reviewed_at = $2,
                     reviewed_by_user_id = $3,
                     actioned_at = NULL,
                     actioned_by_user_id = NULL,
                     dismissed_at = NULL,
                     dismissed_by_user_id = NULL,
                     dismissal_reason = NULL,
                     moderation_action_id = NULL,
                     updated_at = $4
                 WHERE id = $5
                   AND status = $6",
        )
        .bind(REPORT_STATUS_REVIEWED)
        .bind(reviewed_at)
        .bind(reviewed_by_user_id)
        .bind(updated_at)
        .bind(report_id)
        .bind(REPORT_STATUS_ACTIONED)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE reports
                 SET status = ?1,
                     reviewed_at = ?2,
                     reviewed_by_user_id = ?3,
                     actioned_at = NULL,
                     actioned_by_user_id = NULL,
                     dismissed_at = NULL,
                     dismissed_by_user_id = NULL,
                     dismissal_reason = NULL,
                     moderation_action_id = NULL,
                     updated_at = ?4
                 WHERE id = ?5
                   AND status = ?6",
        )
        .bind(REPORT_STATUS_REVIEWED)
        .bind(reviewed_at)
        .bind(reviewed_by_user_id)
        .bind(updated_at)
        .bind(report_id)
        .bind(REPORT_STATUS_ACTIONED)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected == 1)
}

pub async fn reconcile_stale_report_action_reservations(
    pool: &DbPool,
    guild_id: &str,
    stale_before: &str,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE reports
                 SET status = $1,
                     actioned_at = NULL,
                     actioned_by_user_id = NULL,
                     moderation_action_id = NULL,
                     updated_at = $2
                 WHERE guild_id = $3
                   AND status = $4
                   AND moderation_action_id IS NULL
                   AND actioned_at IS NOT NULL
                   AND actioned_at < $5",
        )
        .bind(REPORT_STATUS_REVIEWED)
        .bind(updated_at)
        .bind(guild_id)
        .bind(REPORT_STATUS_ACTIONED)
        .bind(stale_before)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE reports
                 SET status = ?1,
                     actioned_at = NULL,
                     actioned_by_user_id = NULL,
                     moderation_action_id = NULL,
                     updated_at = ?2
                 WHERE guild_id = ?3
                   AND status = ?4
                   AND moderation_action_id IS NULL
                   AND actioned_at IS NOT NULL
                   AND actioned_at < ?5",
        )
        .bind(REPORT_STATUS_REVIEWED)
        .bind(updated_at)
        .bind(guild_id)
        .bind(REPORT_STATUS_ACTIONED)
        .bind(stale_before)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected)
}

#[allow(clippy::too_many_arguments)]
pub async fn apply_message_delete_action(
    pool: &DbPool,
    id: &str,
    guild_id: &str,
    channel_id: &str,
    message_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
    reason: &str,
    now: &str,
) -> Result<bool, AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
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
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(id)
            .bind(MODERATION_ACTION_TYPE_MESSAGE_DELETE)
            .bind(guild_id)
            .bind(actor_user_id)
            .bind(target_user_id)
            .bind(reason)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let updated_rows = sqlx::query(
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
            .bind(now)
            .bind(actor_user_id)
            .bind(reason)
            .bind(id)
            .bind(now)
            .bind(message_id)
            .bind(guild_id)
            .bind(channel_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();
            if updated_rows != 1 {
                return Ok(false);
            }

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
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
            .bind(id)
            .bind(MODERATION_ACTION_TYPE_MESSAGE_DELETE)
            .bind(guild_id)
            .bind(actor_user_id)
            .bind(target_user_id)
            .bind(reason)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let updated_rows = sqlx::query(
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
            .bind(now)
            .bind(actor_user_id)
            .bind(reason)
            .bind(id)
            .bind(now)
            .bind(message_id)
            .bind(guild_id)
            .bind(channel_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();
            if updated_rows != 1 {
                return Ok(false);
            }

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }

    Ok(true)
}

pub async fn find_latest_active_mute_for_target(
    pool: &DbPool,
    guild_id: &str,
    target_user_id: &str,
) -> Result<Option<ModerationActionRecord>, AppError> {
    let record = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id,
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
             FROM moderation_actions
             WHERE guild_id = $1
               AND target_user_id = $2
               AND action_type = $3
               AND is_active = 1
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id,
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
             FROM moderation_actions
             WHERE guild_id = ?1
               AND target_user_id = ?2
               AND action_type = ?3
               AND is_active = 1
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(record)
}

pub async fn deactivate_active_mutes_for_target(
    pool: &DbPool,
    guild_id: &str,
    target_user_id: &str,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE moderation_actions
                 SET is_active = 0,
                     updated_at = $1
                 WHERE guild_id = $2
                   AND target_user_id = $3
                   AND action_type = $4
                   AND is_active = 1",
        )
        .bind(updated_at)
        .bind(guild_id)
        .bind(target_user_id)
        .bind(MODERATION_ACTION_TYPE_MUTE)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE moderation_actions
                 SET is_active = 0,
                     updated_at = ?1
                 WHERE guild_id = ?2
                   AND target_user_id = ?3
                   AND action_type = ?4
                   AND is_active = 1",
        )
        .bind(updated_at)
        .bind(guild_id)
        .bind(target_user_id)
        .bind(MODERATION_ACTION_TYPE_MUTE)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected)
}

fn report_insert_error_to_app_error(err: sqlx::Error) -> AppError {
    if is_report_unique_violation(&err) {
        return AppError::Conflict(
            "You have already reported this target in this guild".to_string(),
        );
    }
    AppError::Internal(err.to_string())
}

fn is_report_unique_violation(err: &sqlx::Error) -> bool {
    let sqlx::Error::Database(db_err) = err else {
        return false;
    };
    if let Some(code) = db_err.code() {
        let code = code.as_ref();
        if code == "23505" || code == "2067" || code == "1555" {
            return true;
        }
    }
    let message = db_err.message().to_ascii_lowercase();
    message.contains("unique") || message.contains("duplicate")
}

pub async fn list_moderation_log_page_by_guild_id(
    pool: &DbPool,
    guild_id: &str,
    action_type: Option<&str>,
    cursor: Option<&ModerationLogCursor>,
    limit: i64,
    order: ModerationLogSortOrder,
) -> Result<ModerationLogPage, AppError> {
    let normalized_limit = limit.clamp(1, 200);
    let fetch_limit = normalized_limit + 1;
    let order_sql = match order {
        ModerationLogSortOrder::Asc => "ASC",
        ModerationLogSortOrder::Desc => "DESC",
    };

    let mut entries = match pool {
        DbPool::Postgres(pool) => {
            let mut query = QueryBuilder::<sqlx::Postgres>::new(
                "SELECT
                    ma.id,
                    ma.action_type,
                    ma.reason,
                    ma.created_at,
                    ma.actor_user_id,
                    actor.username AS actor_username,
                    actor.display_name AS actor_display_name,
                    actor.avatar_color AS actor_avatar_color,
                    ma.target_user_id,
                    target.username AS target_username,
                    target.display_name AS target_display_name,
                    target.avatar_color AS target_avatar_color
                 FROM moderation_actions ma
                 JOIN users actor ON actor.id = ma.actor_user_id
                 JOIN users target ON target.id = ma.target_user_id
                 WHERE ma.guild_id = ",
            );
            query.push_bind(guild_id);
            if let Some(action_type) = action_type {
                query.push(" AND ma.action_type = ");
                query.push_bind(action_type);
            }
            if let Some(cursor) = cursor {
                match order {
                    ModerationLogSortOrder::Desc => {
                        query.push(" AND (ma.created_at < ");
                        query.push_bind(&cursor.created_at);
                        query.push(" OR (ma.created_at = ");
                        query.push_bind(&cursor.created_at);
                        query.push(" AND ma.id < ");
                        query.push_bind(&cursor.id);
                        query.push("))");
                    }
                    ModerationLogSortOrder::Asc => {
                        query.push(" AND (ma.created_at > ");
                        query.push_bind(&cursor.created_at);
                        query.push(" OR (ma.created_at = ");
                        query.push_bind(&cursor.created_at);
                        query.push(" AND ma.id > ");
                        query.push_bind(&cursor.id);
                        query.push("))");
                    }
                }
            }
            query.push(" ORDER BY ma.created_at ");
            query.push(order_sql);
            query.push(", ma.id ");
            query.push(order_sql);
            query.push(" LIMIT ");
            query.push_bind(fetch_limit);
            query.build_query_as().fetch_all(pool).await
        }
        DbPool::Sqlite(pool) => {
            let mut query = QueryBuilder::<sqlx::Sqlite>::new(
                "SELECT
                    ma.id,
                    ma.action_type,
                    ma.reason,
                    ma.created_at,
                    ma.actor_user_id,
                    actor.username AS actor_username,
                    actor.display_name AS actor_display_name,
                    actor.avatar_color AS actor_avatar_color,
                    ma.target_user_id,
                    target.username AS target_username,
                    target.display_name AS target_display_name,
                    target.avatar_color AS target_avatar_color
                 FROM moderation_actions ma
                 JOIN users actor ON actor.id = ma.actor_user_id
                 JOIN users target ON target.id = ma.target_user_id
                 WHERE ma.guild_id = ",
            );
            query.push_bind(guild_id);
            if let Some(action_type) = action_type {
                query.push(" AND ma.action_type = ");
                query.push_bind(action_type);
            }
            if let Some(cursor) = cursor {
                match order {
                    ModerationLogSortOrder::Desc => {
                        query.push(" AND (ma.created_at < ");
                        query.push_bind(&cursor.created_at);
                        query.push(" OR (ma.created_at = ");
                        query.push_bind(&cursor.created_at);
                        query.push(" AND ma.id < ");
                        query.push_bind(&cursor.id);
                        query.push("))");
                    }
                    ModerationLogSortOrder::Asc => {
                        query.push(" AND (ma.created_at > ");
                        query.push_bind(&cursor.created_at);
                        query.push(" OR (ma.created_at = ");
                        query.push_bind(&cursor.created_at);
                        query.push(" AND ma.id > ");
                        query.push_bind(&cursor.id);
                        query.push("))");
                    }
                }
            }
            query.push(" ORDER BY ma.created_at ");
            query.push(order_sql);
            query.push(", ma.id ");
            query.push(order_sql);
            query.push(" LIMIT ");
            query.push_bind(fetch_limit);
            query.build_query_as().fetch_all(pool).await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let page_limit = usize::try_from(normalized_limit).unwrap_or(200);
    let has_more = entries.len() > page_limit;
    if has_more {
        entries.truncate(page_limit);
    }

    Ok(ModerationLogPage { entries, has_more })
}

async fn postgres_actor_outranks_target_member_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    guild_id: &str,
    owner_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
) -> Result<bool, AppError> {
    if actor_user_id == target_user_id {
        return Ok(false);
    }

    let target_is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
             SELECT 1
             FROM guild_members
             WHERE guild_id = $1 AND user_id = $2
         )",
    )
    .bind(guild_id)
    .bind(target_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    if actor_user_id == owner_id {
        return Ok(true);
    }
    if target_user_id == owner_id {
        return Ok(false);
    }

    let actor_is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
             SELECT 1
             FROM guild_members
             WHERE guild_id = $1 AND user_id = $2
         )",
    )
    .bind(guild_id)
    .bind(actor_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    if !actor_is_member {
        return Ok(false);
    }
    if !target_is_member {
        return Ok(true);
    }

    let default_position = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(
             (SELECT position
              FROM roles
              WHERE guild_id = $1 AND is_default = 1
              LIMIT 1),
             $2
         )",
    )
    .bind(guild_id)
    .bind(i64::MAX)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let actor_min_position = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MIN(r.position)
         FROM role_assignments ra
         JOIN roles r ON r.id = ra.role_id
         WHERE ra.guild_id = $1
           AND ra.user_id = $2
           AND r.guild_id = $1",
    )
    .bind(guild_id)
    .bind(actor_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let target_min_position = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MIN(r.position)
         FROM role_assignments ra
         JOIN roles r ON r.id = ra.role_id
         WHERE ra.guild_id = $1
           AND ra.user_id = $2
           AND r.guild_id = $1",
    )
    .bind(guild_id)
    .bind(target_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let actor_position =
        actor_min_position.map_or(default_position, |value| value.min(default_position));
    let target_position =
        target_min_position.map_or(default_position, |value| value.min(default_position));
    Ok(actor_position < target_position)
}

async fn sqlite_actor_outranks_target_member_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    guild_id: &str,
    owner_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
) -> Result<bool, AppError> {
    if actor_user_id == target_user_id {
        return Ok(false);
    }

    let target_is_member = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(
             SELECT 1
             FROM guild_members
             WHERE guild_id = ?1 AND user_id = ?2
         )",
    )
    .bind(guild_id)
    .bind(target_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
        != 0;

    if actor_user_id == owner_id {
        return Ok(true);
    }
    if target_user_id == owner_id {
        return Ok(false);
    }

    let actor_is_member = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(
             SELECT 1
             FROM guild_members
             WHERE guild_id = ?1 AND user_id = ?2
         )",
    )
    .bind(guild_id)
    .bind(actor_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
        != 0;
    if !actor_is_member {
        return Ok(false);
    }
    if !target_is_member {
        return Ok(true);
    }

    let default_position = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(
             (SELECT position
              FROM roles
              WHERE guild_id = ?1 AND is_default = 1
              LIMIT 1),
             ?2
         )",
    )
    .bind(guild_id)
    .bind(i64::MAX)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let actor_min_position = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MIN(r.position)
         FROM role_assignments ra
         JOIN roles r ON r.id = ra.role_id
         WHERE ra.guild_id = ?1
           AND ra.user_id = ?2
           AND r.guild_id = ?1",
    )
    .bind(guild_id)
    .bind(actor_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let target_min_position = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MIN(r.position)
         FROM role_assignments ra
         JOIN roles r ON r.id = ra.role_id
         WHERE ra.guild_id = ?1
           AND ra.user_id = ?2
           AND r.guild_id = ?1",
    )
    .bind(guild_id)
    .bind(target_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let actor_position =
        actor_min_position.map_or(default_position, |value| value.min(default_position));
    let target_position =
        target_min_position.map_or(default_position, |value| value.min(default_position));
    Ok(actor_position < target_position)
}

#[allow(clippy::too_many_arguments)]
pub async fn apply_kick_action(
    pool: &DbPool,
    id: &str,
    guild_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
    reason: &str,
    now: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            let owner_id = sqlx::query_scalar::<_, String>(
                "SELECT owner_id
                 FROM guilds
                 WHERE id = $1
                 LIMIT 1",
            )
            .bind(guild_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .ok_or(AppError::NotFound)?;
            if target_user_id == owner_id {
                return Err(AppError::Forbidden(
                    "Cannot kick the guild owner".to_string(),
                ));
            }
            if !postgres_actor_outranks_target_member_in_tx(
                &mut tx,
                guild_id,
                &owner_id,
                actor_user_id,
                target_user_id,
            )
            .await?
            {
                return Err(AppError::Forbidden(
                    "You can only kick members below your highest role".to_string(),
                ));
            }
            sqlx::query(
                "UPDATE moderation_actions
                     SET is_active = 0,
                         updated_at = $1
                     WHERE guild_id = $2
                       AND target_user_id = $3
                       AND action_type = $4
                       AND is_active = 1",
            )
            .bind(now)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            sqlx::query(
                "DELETE FROM role_assignments
                     WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let removed_membership = sqlx::query(
                "DELETE FROM guild_members
                     WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();
            if removed_membership != 1 {
                return Err(AppError::ValidationError(
                    "target_user_id must belong to a guild member".to_string(),
                ));
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
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(id)
            .bind(MODERATION_ACTION_TYPE_KICK)
            .bind(guild_id)
            .bind(actor_user_id)
            .bind(target_user_id)
            .bind(reason)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            let owner_id = sqlx::query_scalar::<_, String>(
                "SELECT owner_id
                 FROM guilds
                 WHERE id = ?1
                 LIMIT 1",
            )
            .bind(guild_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .ok_or(AppError::NotFound)?;
            if target_user_id == owner_id {
                return Err(AppError::Forbidden(
                    "Cannot kick the guild owner".to_string(),
                ));
            }
            if !sqlite_actor_outranks_target_member_in_tx(
                &mut tx,
                guild_id,
                &owner_id,
                actor_user_id,
                target_user_id,
            )
            .await?
            {
                return Err(AppError::Forbidden(
                    "You can only kick members below your highest role".to_string(),
                ));
            }
            sqlx::query(
                "UPDATE moderation_actions
                     SET is_active = 0,
                         updated_at = ?1
                     WHERE guild_id = ?2
                       AND target_user_id = ?3
                       AND action_type = ?4
                       AND is_active = 1",
            )
            .bind(now)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            sqlx::query(
                "DELETE FROM role_assignments
                     WHERE guild_id = ?1 AND user_id = ?2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let removed_membership = sqlx::query(
                "DELETE FROM guild_members
                     WHERE guild_id = ?1 AND user_id = ?2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();
            if removed_membership != 1 {
                return Err(AppError::ValidationError(
                    "target_user_id must belong to a guild member".to_string(),
                ));
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
            .bind(id)
            .bind(MODERATION_ACTION_TYPE_KICK)
            .bind(guild_id)
            .bind(actor_user_id)
            .bind(target_user_id)
            .bind(reason)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn apply_ban_action(
    pool: &DbPool,
    id: &str,
    ban_id: &str,
    guild_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
    reason: &str,
    delete_messages_window_seconds: Option<i64>,
    now: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            let owner_id = sqlx::query_scalar::<_, String>(
                "SELECT owner_id
                 FROM guilds
                 WHERE id = $1
                 LIMIT 1",
            )
            .bind(guild_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .ok_or(AppError::NotFound)?;
            if target_user_id == owner_id {
                return Err(AppError::Forbidden(
                    "Cannot ban the guild owner".to_string(),
                ));
            }
            if !postgres_actor_outranks_target_member_in_tx(
                &mut tx,
                guild_id,
                &owner_id,
                actor_user_id,
                target_user_id,
            )
            .await?
            {
                return Err(AppError::Forbidden(
                    "You can only ban members below your highest role".to_string(),
                ));
            }
            sqlx::query(
                "UPDATE moderation_actions
                     SET is_active = 0,
                         updated_at = $1
                     WHERE guild_id = $2
                       AND target_user_id = $3
                       AND action_type = $4
                       AND is_active = 1",
            )
            .bind(now)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            sqlx::query(
                "DELETE FROM role_assignments
                     WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let removed_membership = sqlx::query(
                "DELETE FROM guild_members
                     WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();
            if removed_membership != 1 {
                return Err(AppError::ValidationError(
                    "target_user_id must belong to a guild member".to_string(),
                ));
            }

            sqlx::query(
                "INSERT INTO guild_bans (
                        id,
                        guild_id,
                        target_user_id,
                        actor_user_id,
                        reason,
                        delete_messages_window_seconds,
                        is_active,
                        created_at,
                        updated_at,
                        unbanned_by_user_id,
                        unbanned_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $8, NULL, NULL)",
            )
            .bind(ban_id)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(actor_user_id)
            .bind(reason)
            .bind(delete_messages_window_seconds)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

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
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(id)
            .bind(MODERATION_ACTION_TYPE_BAN)
            .bind(guild_id)
            .bind(actor_user_id)
            .bind(target_user_id)
            .bind(reason)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            let owner_id = sqlx::query_scalar::<_, String>(
                "SELECT owner_id
                 FROM guilds
                 WHERE id = ?1
                 LIMIT 1",
            )
            .bind(guild_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .ok_or(AppError::NotFound)?;
            if target_user_id == owner_id {
                return Err(AppError::Forbidden(
                    "Cannot ban the guild owner".to_string(),
                ));
            }
            if !sqlite_actor_outranks_target_member_in_tx(
                &mut tx,
                guild_id,
                &owner_id,
                actor_user_id,
                target_user_id,
            )
            .await?
            {
                return Err(AppError::Forbidden(
                    "You can only ban members below your highest role".to_string(),
                ));
            }
            sqlx::query(
                "UPDATE moderation_actions
                     SET is_active = 0,
                         updated_at = ?1
                     WHERE guild_id = ?2
                       AND target_user_id = ?3
                       AND action_type = ?4
                       AND is_active = 1",
            )
            .bind(now)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            sqlx::query(
                "DELETE FROM role_assignments
                     WHERE guild_id = ?1 AND user_id = ?2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let removed_membership = sqlx::query(
                "DELETE FROM guild_members
                     WHERE guild_id = ?1 AND user_id = ?2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();
            if removed_membership != 1 {
                return Err(AppError::ValidationError(
                    "target_user_id must belong to a guild member".to_string(),
                ));
            }

            sqlx::query(
                "INSERT INTO guild_bans (
                        id,
                        guild_id,
                        target_user_id,
                        actor_user_id,
                        reason,
                        delete_messages_window_seconds,
                        is_active,
                        created_at,
                        updated_at,
                        unbanned_by_user_id,
                        unbanned_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, NULL, NULL)",
            )
            .bind(ban_id)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(actor_user_id)
            .bind(reason)
            .bind(delete_messages_window_seconds)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

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
            .bind(id)
            .bind(MODERATION_ACTION_TYPE_BAN)
            .bind(guild_id)
            .bind(actor_user_id)
            .bind(target_user_id)
            .bind(reason)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }

    Ok(())
}

pub async fn deactivate_moderation_action_by_id(
    pool: &DbPool,
    id: &str,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE moderation_actions
                 SET is_active = 0,
                     updated_at = $1
                 WHERE id = $2
                   AND is_active = 1",
        )
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE moderation_actions
                 SET is_active = 0,
                     updated_at = ?1
                 WHERE id = ?2
                   AND is_active = 1",
        )
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected)
}
