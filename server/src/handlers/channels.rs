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
    services::channel_service::{
        self, CreateChannelInput, ReorderChannelsInput, UpdateChannelInput,
        UpsertChannelPermissionOverrideInput,
    },
};

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: Option<String>,
    pub channel_type: Option<String>,
    pub category_slug: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    #[serde(default)]
    pub name: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderChannelsRequest {
    pub channel_slugs: Option<Vec<String>>,
    pub channel_positions: Option<Vec<ReorderChannelPositionRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderChannelPositionRequest {
    pub channel_slug: Option<String>,
    pub category_slug: Option<Option<String>>,
    pub position: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertChannelPermissionOverrideRequest {
    pub allow_bitflag: Option<i64>,
    pub deny_bitflag: Option<i64>,
}

pub async fn list_channels(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
) -> Result<Response, AppError> {
    let channels = channel_service::list_channels(&state.pool, &user.user_id, &guild_slug).await?;
    Ok((StatusCode::OK, Json(json!({ "data": channels }))).into_response())
}

pub async fn create_channel(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    payload: Result<Json<CreateChannelRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let channel = channel_service::create_channel(
        &state.pool,
        &user.user_id,
        &guild_slug,
        CreateChannelInput {
            name: req.name.unwrap_or_default(),
            channel_type: req.channel_type.unwrap_or_default(),
            category_slug: req.category_slug,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": channel }))).into_response())
}

pub async fn update_channel(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, channel_slug)): Path<(String, String)>,
    payload: Result<Json<UpdateChannelRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let name = match req.name {
        Some(Some(value)) => Some(value),
        Some(None) => {
            return Err(AppError::ValidationError("name cannot be null".to_string()));
        }
        None => None,
    };
    let channel = channel_service::update_channel(
        &state.pool,
        &user.user_id,
        &guild_slug,
        &channel_slug,
        UpdateChannelInput { name },
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": channel }))).into_response())
}

pub async fn delete_channel(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, channel_slug)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let deleted =
        channel_service::delete_channel(&state.pool, &user.user_id, &guild_slug, &channel_slug)
            .await?;
    Ok((StatusCode::OK, Json(json!({ "data": deleted }))).into_response())
}

pub async fn reorder_channels(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    payload: Result<Json<ReorderChannelsRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let channels = channel_service::reorder_channels(
        &state.pool,
        &user.user_id,
        &guild_slug,
        ReorderChannelsInput {
            channel_slugs: req.channel_slugs.unwrap_or_default(),
            channel_positions: req
                .channel_positions
                .unwrap_or_default()
                .into_iter()
                .map(|item| {
                    Ok(channel_service::ReorderChannelPositionInput {
                        channel_slug: item.channel_slug.ok_or_else(|| {
                            AppError::ValidationError(
                                "channel_positions.channel_slug is required".to_string(),
                            )
                        })?,
                        category_slug: item.category_slug.unwrap_or(None),
                        position: item.position.ok_or_else(|| {
                            AppError::ValidationError(
                                "channel_positions.position is required".to_string(),
                            )
                        })?,
                    })
                })
                .collect::<Result<Vec<_>, AppError>>()?,
        },
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": channels }))).into_response())
}

pub async fn list_channel_permission_overrides(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, channel_slug)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let overrides = channel_service::list_channel_permission_overrides(
        &state.pool,
        &user.user_id,
        &guild_slug,
        &channel_slug,
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": overrides }))).into_response())
}

pub async fn upsert_channel_permission_override(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, channel_slug, role_id)): Path<(String, String, String)>,
    payload: Result<Json<UpsertChannelPermissionOverrideRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let allow_bitflag = req
        .allow_bitflag
        .ok_or_else(|| AppError::ValidationError("allow_bitflag is required".to_string()))?;
    let deny_bitflag = req
        .deny_bitflag
        .ok_or_else(|| AppError::ValidationError("deny_bitflag is required".to_string()))?;

    let saved = channel_service::upsert_channel_permission_override(
        &state.pool,
        &user.user_id,
        &guild_slug,
        &channel_slug,
        &role_id,
        UpsertChannelPermissionOverrideInput {
            allow_bitflag,
            deny_bitflag,
        },
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": saved }))).into_response())
}

pub async fn delete_channel_permission_override(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, channel_slug, role_id)): Path<(String, String, String)>,
) -> Result<Response, AppError> {
    let deleted = channel_service::delete_channel_permission_override(
        &state.pool,
        &user.user_id,
        &guild_slug,
        &channel_slug,
        &role_id,
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": deleted }))).into_response())
}
