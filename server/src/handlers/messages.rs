use axum::{
    Json,
    body::Body,
    extract::rejection::QueryRejection,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    AppError, AppState,
    middleware::auth::AuthenticatedUser,
    services::message_service,
    ws::{gateway, protocol::ServerOp, registry},
};

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

pub async fn create_message_attachment(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, channel_slug)): Path<(String, String)>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let mut attachment_bytes: Option<Vec<u8>> = None;
    let mut attachment_content_type: Option<String> = None;
    let mut attachment_filename: Option<String> = None;
    let mut content: Option<String> = None;
    let mut client_nonce: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::ValidationError("Invalid multipart payload".to_string()))?
    {
        match field.name() {
            Some("file") => {
                attachment_content_type = field.content_type().map(str::to_string);
                attachment_filename = field.file_name().map(str::to_string);
                let bytes = field.bytes().await.map_err(|_| {
                    AppError::ValidationError("Invalid attachment payload".to_string())
                })?;
                attachment_bytes = Some(bytes.to_vec());
            }
            Some("content") => {
                content = Some(field.text().await.map_err(|_| {
                    AppError::ValidationError("Invalid content payload".to_string())
                })?);
            }
            Some("client_nonce") => {
                client_nonce = Some(field.text().await.map_err(|_| {
                    AppError::ValidationError("Invalid client_nonce payload".to_string())
                })?);
            }
            _ => {}
        }
    }

    let attachment_bytes = attachment_bytes
        .ok_or_else(|| AppError::ValidationError("file field is required".to_string()))?;
    let filename = attachment_filename.unwrap_or_else(|| "attachment.bin".to_string());
    let message = message_service::create_attachment_message(
        &state.pool,
        &state.config.attachments,
        &user.user_id,
        message_service::CreateAttachmentMessageInput {
            guild_slug,
            channel_slug,
            content,
            client_nonce,
            filename,
            declared_content_type: attachment_content_type,
            file_bytes: attachment_bytes,
        },
    )
    .await?;

    registry::broadcast_to_channel(
        &message.guild_slug,
        &message.channel_slug,
        ServerOp::MessageCreate,
        &message,
    );
    gateway::emit_channel_activity_event(&state.pool, &message).await?;

    Ok((StatusCode::CREATED, Json(json!({ "data": message }))).into_response())
}

pub async fn get_message_attachment(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((guild_slug, channel_slug, attachment_id)): Path<(String, String, String)>,
) -> Result<Response, AppError> {
    let (bytes, mime, filename) = message_service::load_message_attachment(
        &state.pool,
        &state.config.attachments,
        &user.user_id,
        &guild_slug,
        &channel_slug,
        &attachment_id,
    )
    .await?;
    let safe_filename = sanitize_download_filename(&filename);

    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        mime.parse()
            .map_err(|_| AppError::Internal("Invalid attachment MIME type".to_string()))?,
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, max-age=0, must-revalidate"),
    );
    let content_disposition = format!("attachment; filename=\"{safe_filename}\"");
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&content_disposition).map_err(|_| {
            AppError::Internal("Invalid attachment content-disposition header".to_string())
        })?,
    );
    Ok(response)
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

fn sanitize_download_filename(name: &str) -> String {
    let mut file = name.rsplit(['/', '\\']).next().unwrap_or("");
    file = file.trim();
    if file.is_empty() {
        return "attachment.bin".to_string();
    }
    let sanitized = file
        .chars()
        .filter(|ch| !ch.is_control() && *ch != '"' && *ch != '\\')
        .collect::<String>();
    if sanitized.is_empty() {
        "attachment.bin".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_limit, sanitize_download_filename};

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

    #[test]
    fn sanitize_download_filename_rejects_unsafe_characters() {
        assert_eq!(sanitize_download_filename("..\\secret\".png"), "secret.png");
        assert_eq!(sanitize_download_filename("   "), "attachment.bin");
    }
}
