use axum::{
    Json,
    body::Body,
    extract::rejection::JsonRejection,
    extract::{Multipart, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    AppError, AppState,
    middleware::auth::AuthenticatedUser,
    services::guild_service::{self, CreateGuildInput, UpdateGuildInput},
};

#[derive(Debug, Deserialize)]
pub struct CreateGuildRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGuildRequest {
    #[serde(default)]
    pub name: Option<Option<String>>,
    #[serde(default)]
    pub description: Option<Option<String>>,
}

pub async fn list_guilds(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Response, AppError> {
    let guilds = guild_service::list_guilds(&state.pool, &user.user_id).await?;
    Ok((StatusCode::OK, Json(json!({ "data": guilds }))).into_response())
}

pub async fn create_guild(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    payload: Result<Json<CreateGuildRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let guild = guild_service::create_guild(
        &state.pool,
        &user.user_id,
        CreateGuildInput {
            name: req.name.unwrap_or_default(),
            description: req.description,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": guild }))).into_response())
}

pub async fn update_guild(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    payload: Result<Json<UpdateGuildRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let guild = guild_service::update_guild(
        &state.pool,
        &user.user_id,
        &guild_slug,
        UpdateGuildInput {
            name: req.name,
            description: req.description,
        },
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": guild }))).into_response())
}

pub async fn upload_guild_icon(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let mut icon_bytes: Option<Vec<u8>> = None;
    let mut icon_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::ValidationError("Invalid multipart payload".to_string()))?
    {
        if field.name() == Some("icon") {
            icon_content_type = field.content_type().map(str::to_string);
            let bytes = field
                .bytes()
                .await
                .map_err(|_| AppError::ValidationError("Invalid icon payload".to_string()))?;
            icon_bytes = Some(bytes.to_vec());
            break;
        }
    }

    let icon_bytes = icon_bytes
        .ok_or_else(|| AppError::ValidationError("icon field is required".to_string()))?;
    let guild = guild_service::save_guild_icon(
        &state.pool,
        &state.config.avatar,
        &user.user_id,
        &guild_slug,
        icon_content_type.as_deref(),
        &icon_bytes,
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": guild }))).into_response())
}

pub async fn get_guild_icon(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
) -> Result<Response, AppError> {
    let (bytes, mime) = guild_service::load_guild_icon(
        &state.pool,
        &state.config.avatar,
        &user.user_id,
        &guild_slug,
    )
    .await?;

    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        mime.parse()
            .map_err(|_| AppError::Internal("Invalid guild icon MIME type".to_string()))?,
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, max-age=0, must-revalidate"),
    );
    Ok(response)
}
