use axum::{
    Json,
    extract::rejection::QueryRejection,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{AppError, AppState, middleware::auth::AuthenticatedUser, services::dm_service};

const DEFAULT_MESSAGES_LIMIT: i64 = 50;

#[derive(Debug, Deserialize)]
pub struct OpenDmRequest {
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ListDmMessagesQuery {
    #[serde(default)]
    pub limit: Option<String>,
    #[serde(default)]
    pub before: Option<String>,
}

pub async fn open_dm(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<OpenDmRequest>,
) -> Result<Response, AppError> {
    let dm = dm_service::open_or_create_dm(
        &state.pool,
        &user.user_id,
        dm_service::OpenDmInput {
            user_id: payload.user_id,
        },
    )
    .await?;

    Ok((StatusCode::OK, Json(json!({ "data": dm }))).into_response())
}

pub async fn list_dms(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Response, AppError> {
    let dms = dm_service::list_dms(&state.pool, &user.user_id).await?;
    Ok((StatusCode::OK, Json(json!({ "data": dms }))).into_response())
}

pub async fn list_dm_messages(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(dm_slug): Path<String>,
    query: Result<Query<ListDmMessagesQuery>, QueryRejection>,
) -> Result<Response, AppError> {
    let Query(query) =
        query.map_err(|_| AppError::ValidationError("Invalid query parameters".to_string()))?;
    let limit = parse_limit(query.limit.as_deref())?;
    let page = dm_service::list_dm_messages(
        &state.pool,
        &user.user_id,
        &dm_slug,
        dm_service::ListDmMessagesInput {
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
