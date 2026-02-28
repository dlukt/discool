use axum::{
    Json,
    extract::rejection::QueryRejection,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{AppError, AppState, middleware::auth::AuthenticatedUser, services::message_service};

const DEFAULT_MESSAGES_LIMIT: i64 = 50;

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    #[serde(default)]
    pub limit: Option<String>,
    #[serde(default)]
    pub before: Option<String>,
}

pub async fn list_messages(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, channel_slug)): Path<(String, String)>,
    query: Result<Query<ListMessagesQuery>, QueryRejection>,
) -> Result<Response, AppError> {
    let Query(query) =
        query.map_err(|_| AppError::ValidationError("Invalid query parameters".to_string()))?;
    let limit = parse_limit(query.limit.as_deref())?;

    let page = message_service::list_channel_messages(
        &state.pool,
        &user.user_id,
        &guild_slug,
        &channel_slug,
        message_service::ListChannelMessagesInput {
            limit,
            before: query.before,
        },
    )
    .await?;
    Ok((
        StatusCode::OK,
        Json(json!({ "data": page.messages, "cursor": page.cursor })),
    )
        .into_response())
}

fn parse_limit(raw: Option<&str>) -> Result<i64, AppError> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_MESSAGES_LIMIT);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(DEFAULT_MESSAGES_LIMIT);
    }
    let parsed = trimmed
        .parse::<i64>()
        .map_err(|_| AppError::ValidationError("limit must be a valid integer".to_string()))?;
    Ok(parsed.clamp(1, 200))
}

#[cfg(test)]
mod tests {
    use super::parse_limit;

    #[test]
    fn parse_limit_defaults_and_clamps() {
        assert_eq!(parse_limit(None).unwrap(), 50);
        assert_eq!(parse_limit(Some("")).unwrap(), 50);
        assert_eq!(parse_limit(Some("0")).unwrap(), 1);
        assert_eq!(parse_limit(Some("500")).unwrap(), 200);
        assert_eq!(parse_limit(Some("25")).unwrap(), 25);
    }

    #[test]
    fn parse_limit_rejects_non_numeric_values() {
        assert!(parse_limit(Some("abc")).is_err());
    }
}
