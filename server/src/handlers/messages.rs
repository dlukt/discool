use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{AppError, AppState, middleware::auth::AuthenticatedUser, services::message_service};

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    #[serde(default)]
    pub limit: Option<i64>,
}

pub async fn list_messages(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, channel_slug)): Path<(String, String)>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Response, AppError> {
    let messages = message_service::list_channel_messages(
        &state.pool,
        &user.user_id,
        &guild_slug,
        &channel_slug,
        query.limit.unwrap_or(50),
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": messages }))).into_response())
}
