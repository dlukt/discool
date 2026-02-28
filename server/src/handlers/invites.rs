use axum::{
    Json,
    extract::rejection::JsonRejection,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    AppError, AppState,
    middleware::auth::AuthenticatedUser,
    services::guild_invite_service::{self, CreateGuildInviteInput},
};

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    #[serde(rename = "type")]
    pub invite_type: Option<String>,
}

pub async fn list_invites(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
) -> Result<Response, AppError> {
    let invites =
        guild_invite_service::list_invites(&state.pool, &user.user_id, &guild_slug).await?;
    Ok((StatusCode::OK, Json(json!({ "data": invites }))).into_response())
}

pub async fn create_invite(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    payload: Result<Json<CreateInviteRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let invite_type = req
        .invite_type
        .ok_or_else(|| AppError::ValidationError("type is required".to_string()))?;
    let invite = guild_invite_service::create_invite(
        &state.pool,
        &user.user_id,
        &guild_slug,
        CreateGuildInviteInput { invite_type },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": invite }))).into_response())
}

pub async fn revoke_invite(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, invite_code)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let result =
        guild_invite_service::revoke_invite(&state.pool, &user.user_id, &guild_slug, &invite_code)
            .await?;
    Ok((StatusCode::OK, Json(json!({ "data": result }))).into_response())
}
