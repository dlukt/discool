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
    services::role_service::{self, CreateRoleInput, ReorderRolesInput, UpdateRoleInput},
};

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    #[serde(default)]
    pub name: Option<Option<String>>,
    #[serde(default)]
    pub color: Option<Option<String>>,
    #[serde(default)]
    pub permissions_bitflag: Option<Option<i64>>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderRolesRequest {
    pub role_ids: Option<Vec<String>>,
}

pub async fn list_roles(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
) -> Result<Response, AppError> {
    let roles = role_service::list_roles(&state.pool, &user.user_id, &guild_slug).await?;
    Ok((StatusCode::OK, Json(json!({ "data": roles }))).into_response())
}

pub async fn create_role(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    payload: Result<Json<CreateRoleRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let role = role_service::create_role(
        &state.pool,
        &user.user_id,
        &guild_slug,
        CreateRoleInput {
            name: req.name.unwrap_or_default(),
            color: req.color.unwrap_or_default(),
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": role }))).into_response())
}

pub async fn update_role(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, role_id)): Path<(String, String)>,
    payload: Result<Json<UpdateRoleRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let name = match req.name {
        Some(Some(value)) => Some(value),
        Some(None) => return Err(AppError::ValidationError("name cannot be null".to_string())),
        None => None,
    };
    let color = match req.color {
        Some(Some(value)) => Some(value),
        Some(None) => {
            return Err(AppError::ValidationError(
                "color cannot be null".to_string(),
            ));
        }
        None => None,
    };
    let permissions_bitflag = match req.permissions_bitflag {
        Some(Some(value)) => Some(value),
        Some(None) => {
            return Err(AppError::ValidationError(
                "permissions_bitflag cannot be null".to_string(),
            ));
        }
        None => None,
    };
    let role = role_service::update_role(
        &state.pool,
        &user.user_id,
        &guild_slug,
        &role_id,
        UpdateRoleInput {
            name,
            color,
            permissions_bitflag,
        },
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": role }))).into_response())
}

pub async fn delete_role(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, role_id)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let deleted =
        role_service::delete_role(&state.pool, &user.user_id, &guild_slug, &role_id).await?;
    Ok((StatusCode::OK, Json(json!({ "data": deleted }))).into_response())
}

pub async fn reorder_roles(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    payload: Result<Json<ReorderRolesRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let roles = role_service::reorder_roles(
        &state.pool,
        &user.user_id,
        &guild_slug,
        ReorderRolesInput {
            role_ids: req.role_ids.unwrap_or_default(),
        },
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": roles }))).into_response())
}
