use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    NotFound,
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    ValidationError(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Resource not found".to_string(),
            ),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg),
            AppError::ValidationError(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", msg)
            }
            AppError::Internal(msg) => {
                tracing::error!(%msg, "Internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                )
            }
        };

        let body = json!({ "error": { "code": code, "message": message, "details": {} } });
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::Value;

    #[tokio::test]
    async fn internal_errors_are_sanitized() {
        let response = AppError::Internal("sqlx: relation not found".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("internal response body should serialize");
        let payload: Value =
            serde_json::from_slice(&body).expect("internal response should be valid json");

        assert_eq!(payload["error"]["code"], json!("INTERNAL_ERROR"));
        assert_eq!(
            payload["error"]["message"],
            json!("An internal error occurred")
        );
        assert_ne!(
            payload["error"]["message"],
            json!("sqlx: relation not found")
        );
    }

    #[tokio::test]
    async fn forbidden_errors_keep_user_facing_message() {
        let response =
            AppError::Forbidden("Only DM participants can access this conversation".to_string())
                .into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("forbidden response body should serialize");
        let payload: Value =
            serde_json::from_slice(&body).expect("forbidden response should be valid json");

        assert_eq!(payload["error"]["code"], json!("FORBIDDEN"));
        assert_eq!(
            payload["error"]["message"],
            json!("Only DM participants can access this conversation")
        );
    }
}
