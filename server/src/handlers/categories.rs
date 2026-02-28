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
    services::category_service::{
        self, CreateCategoryInput, ReorderCategoriesInput, UpdateCategoryCollapseInput,
        UpdateCategoryInput,
    },
};

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    #[serde(default)]
    pub name: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderCategoriesRequest {
    pub category_slugs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryCollapseRequest {
    pub collapsed: Option<bool>,
}

pub async fn list_categories(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
) -> Result<Response, AppError> {
    let categories =
        category_service::list_categories(&state.pool, &user.user_id, &guild_slug).await?;
    Ok((StatusCode::OK, Json(json!({ "data": categories }))).into_response())
}

pub async fn create_category(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    payload: Result<Json<CreateCategoryRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let category = category_service::create_category(
        &state.pool,
        &user.user_id,
        &guild_slug,
        CreateCategoryInput {
            name: req.name.unwrap_or_default(),
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": category }))).into_response())
}

pub async fn update_category(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, category_slug)): Path<(String, String)>,
    payload: Result<Json<UpdateCategoryRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let name = match req.name {
        Some(Some(value)) => Some(value),
        Some(None) => return Err(AppError::ValidationError("name cannot be null".to_string())),
        None => None,
    };
    let category = category_service::update_category(
        &state.pool,
        &user.user_id,
        &guild_slug,
        &category_slug,
        UpdateCategoryInput { name },
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": category }))).into_response())
}

pub async fn delete_category(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, category_slug)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let deleted =
        category_service::delete_category(&state.pool, &user.user_id, &guild_slug, &category_slug)
            .await?;
    Ok((StatusCode::OK, Json(json!({ "data": deleted }))).into_response())
}

pub async fn reorder_categories(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    payload: Result<Json<ReorderCategoriesRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let categories = category_service::reorder_categories(
        &state.pool,
        &user.user_id,
        &guild_slug,
        ReorderCategoriesInput {
            category_slugs: req.category_slugs.unwrap_or_default(),
        },
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": categories }))).into_response())
}

pub async fn update_category_collapse(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, category_slug)): Path<(String, String)>,
    payload: Result<Json<UpdateCategoryCollapseRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let collapsed = req
        .collapsed
        .ok_or_else(|| AppError::ValidationError("collapsed is required".to_string()))?;
    let category = category_service::update_category_collapse(
        &state.pool,
        &user.user_id,
        &guild_slug,
        &category_slug,
        UpdateCategoryCollapseInput { collapsed },
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": category }))).into_response())
}
