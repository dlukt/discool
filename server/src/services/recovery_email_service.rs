use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use axum::http::HeaderMap;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::{DateTime, Duration, Utc};
use lettre::Address;
use rand::Rng;
use sha2::{Digest, Sha256};

use crate::{
    AppError,
    config::EmailConfig,
    db::DbPool,
    models::recovery_email::{
        EmailVerificationToken, RecoveryEmailAssociation, RecoveryEmailStatusResponse,
    },
};

const MAX_EMAIL_LEN: usize = 254;
const MAX_ENCRYPTED_PRIVATE_KEY_LEN: usize = 32 * 1024;
const INVALID_OR_EXPIRED_TOKEN: &str = "Invalid or expired verification token";
const TOKEN_ALREADY_USED: &str = "Verification token already used";
const START_RATE_LIMIT_ERROR: &str = "Too many recovery email requests. Please try again later.";
const VERIFY_RATE_LIMIT_ERROR: &str = "Too many verification attempts. Please try again later.";

#[derive(Debug, Clone)]
pub struct StartRecoveryEmailInput {
    pub email: String,
    pub encrypted_private_key: String,
    pub encryption_algorithm: String,
    pub encryption_version: i64,
    pub requester_ip: String,
}

#[derive(Debug, Clone)]
pub struct StartRecoveryEmailResult {
    pub token: String,
    pub normalized_email: String,
    pub status: RecoveryEmailStatusResponse,
}

pub fn requester_ip_from_headers(headers: &HeaderMap) -> String {
    if let Some(value) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        let ip = value
            .split(',')
            .next()
            .map(str::trim)
            .filter(|candidate| !candidate.is_empty())
            .unwrap_or("unknown");
        return ip.to_string();
    }

    if let Some(value) = headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
    {
        let ip = value.trim();
        if !ip.is_empty() {
            return ip.to_string();
        }
    }

    "unknown".to_string()
}

pub async fn get_recovery_email_status(
    pool: &DbPool,
    user_id: &str,
) -> Result<RecoveryEmailStatusResponse, AppError> {
    let association = fetch_recovery_email_association(pool, user_id).await?;
    Ok(match association {
        Some(association) => RecoveryEmailStatusResponse::from_association(&association),
        None => RecoveryEmailStatusResponse::none(),
    })
}

pub async fn start_recovery_email_association(
    pool: &DbPool,
    config: &EmailConfig,
    user_id: &str,
    input: &StartRecoveryEmailInput,
) -> Result<StartRecoveryEmailResult, AppError> {
    let normalized_email = normalize_email(&input.email)?;
    validate_encrypted_private_key_payload(&input.encrypted_private_key)?;
    if input.encryption_algorithm.trim().is_empty() {
        return Err(AppError::ValidationError(
            "encryption_context.algorithm is required".to_string(),
        ));
    }
    if input.encryption_version <= 0 {
        return Err(AppError::ValidationError(
            "encryption_context.version must be >= 1".to_string(),
        ));
    }

    enforce_start_rate_limit(pool, config, user_id, &input.requester_ip).await?;

    let now = Utc::now();
    let now_rfc3339 = now.to_rfc3339();
    let expires_at = (now
        + Duration::seconds(config.token_ttl_seconds.try_into().map_err(|_| {
            AppError::Internal("email.token_ttl_seconds is too large".to_string())
        })?))
    .to_rfc3339();
    let email_masked = mask_email(&normalized_email);

    let (encrypted_private_key, key_nonce) =
        encrypt_private_key_payload(config, &normalized_email, &input.encrypted_private_key)?;
    let token = generate_token();
    let token_hash = sha256_hex(token.as_bytes());

    upsert_pending_recovery_email_association(
        pool,
        user_id,
        &normalized_email,
        &email_masked,
        &now_rfc3339,
    )
    .await?;
    invalidate_pending_verification_tokens(pool, user_id, &now_rfc3339).await?;
    insert_verification_token(
        pool,
        &EmailVerificationTokenInsert {
            token_hash: token_hash.as_str(),
            user_id,
            target_email: &normalized_email,
            email_masked: &email_masked,
            encrypted_private_key: &encrypted_private_key,
            encryption_algorithm: input.encryption_algorithm.trim(),
            encryption_version: input.encryption_version,
            key_nonce: &key_nonce,
            requester_ip: &input.requester_ip,
            expires_at: &expires_at,
            created_at: &now_rfc3339,
        },
    )
    .await?;

    Ok(StartRecoveryEmailResult {
        token,
        normalized_email,
        status: RecoveryEmailStatusResponse {
            associated: true,
            email_masked: Some(email_masked),
            verified: false,
            verified_at: None,
        },
    })
}

pub async fn verify_recovery_email_token(
    pool: &DbPool,
    config: &EmailConfig,
    token: &str,
    requester_ip: &str,
) -> Result<RecoveryEmailStatusResponse, AppError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::ValidationError("token is required".to_string()));
    }
    if token.len() > 256 {
        return Err(AppError::ValidationError("token is too long".to_string()));
    }

    let now = Utc::now();
    let now_rfc3339 = now.to_rfc3339();
    record_verify_attempt(pool, requester_ip, &now_rfc3339).await?;
    enforce_verify_rate_limit(pool, config, requester_ip).await?;

    let token_hash = sha256_hex(token.as_bytes());
    let Some(token_record) = fetch_verification_token(pool, &token_hash).await? else {
        return Err(AppError::Unauthorized(INVALID_OR_EXPIRED_TOKEN.to_string()));
    };

    if token_record.used_at.is_some() {
        return Err(AppError::Unauthorized(TOKEN_ALREADY_USED.to_string()));
    }

    let expires_at = DateTime::parse_from_rfc3339(&token_record.expires_at)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::Unauthorized(INVALID_OR_EXPIRED_TOKEN.to_string()))?;
    if expires_at <= now {
        return Err(AppError::Unauthorized(INVALID_OR_EXPIRED_TOKEN.to_string()));
    }
    let marked_used =
        mark_verification_token_used(pool, &token_hash, requester_ip, &now_rfc3339).await?;
    if !marked_used {
        return Err(AppError::Unauthorized(TOKEN_ALREADY_USED.to_string()));
    }

    upsert_verified_recovery_email_association(pool, &token_record, &now_rfc3339).await?;

    Ok(RecoveryEmailStatusResponse {
        associated: true,
        email_masked: Some(token_record.email_masked),
        verified: true,
        verified_at: Some(now_rfc3339),
    })
}

fn normalize_email(value: &str) -> Result<String, AppError> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(AppError::ValidationError("email is required".to_string()));
    }
    if normalized.len() > MAX_EMAIL_LEN {
        return Err(AppError::ValidationError("email is too long".to_string()));
    }
    normalized
        .parse::<Address>()
        .map_err(|_| AppError::ValidationError("Invalid email address".to_string()))?;
    Ok(normalized)
}

fn validate_encrypted_private_key_payload(value: &str) -> Result<(), AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError(
            "encrypted_private_key is required".to_string(),
        ));
    }
    if trimmed.len() > MAX_ENCRYPTED_PRIVATE_KEY_LEN {
        return Err(AppError::ValidationError(
            "encrypted_private_key is too large".to_string(),
        ));
    }
    Ok(())
}

fn mask_email(normalized_email: &str) -> String {
    let Some((local_part, domain)) = normalized_email.split_once('@') else {
        return "***".to_string();
    };
    let mut chars = local_part.chars();
    let first = chars.next().unwrap_or('*');
    format!("{first}***@{domain}")
}

fn encrypt_private_key_payload(
    config: &EmailConfig,
    normalized_email: &str,
    payload: &str,
) -> Result<(String, String), AppError> {
    let key = derive_email_encryption_key(&config.server_secret, normalized_email);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| AppError::Internal("Failed to initialize recovery encryption".to_string()))?;
    let nonce: [u8; 12] = rand::rng().random();
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), payload.as_bytes())
        .map_err(|_| AppError::Internal("Failed to encrypt recovery payload".to_string()))?;
    Ok((BASE64.encode(ciphertext), BASE64.encode(nonce)))
}

fn derive_email_encryption_key(server_secret: &str, normalized_email: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(server_secret.as_bytes());
    hasher.update(b":");
    hasher.update(normalized_email.as_bytes());
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

fn generate_token() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    hex_encode(&bytes)
}

fn sha256_hex(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

async fn enforce_start_rate_limit(
    pool: &DbPool,
    config: &EmailConfig,
    user_id: &str,
    requester_ip: &str,
) -> Result<(), AppError> {
    let window_start = (Utc::now() - Duration::hours(1)).to_rfc3339();
    let user_request_count =
        count_recent_start_requests_for_user(pool, user_id, &window_start).await?;
    if user_request_count >= i64::from(config.start_rate_limit_per_hour) {
        return Err(AppError::ValidationError(
            START_RATE_LIMIT_ERROR.to_string(),
        ));
    }

    let ip_request_count =
        count_recent_start_requests_for_ip(pool, requester_ip, &window_start).await?;
    if ip_request_count >= i64::from(config.start_rate_limit_per_hour) {
        return Err(AppError::ValidationError(
            START_RATE_LIMIT_ERROR.to_string(),
        ));
    }

    Ok(())
}

async fn enforce_verify_rate_limit(
    pool: &DbPool,
    config: &EmailConfig,
    requester_ip: &str,
) -> Result<(), AppError> {
    let window_start = (Utc::now() - Duration::hours(1)).to_rfc3339();
    let ip_verify_count =
        count_recent_verify_attempts_for_ip(pool, requester_ip, &window_start).await?;
    if ip_verify_count > i64::from(config.verify_rate_limit_per_hour) {
        return Err(AppError::ValidationError(
            VERIFY_RATE_LIMIT_ERROR.to_string(),
        ));
    }
    Ok(())
}

async fn record_verify_attempt(
    pool: &DbPool,
    requester_ip: &str,
    attempted_at: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO email_verification_attempts (requester_ip, attempted_at) VALUES ($1, $2)",
        )
        .bind(requester_ip)
        .bind(attempted_at)
        .execute(pool)
        .await
        .map(|_| ()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO email_verification_attempts (requester_ip, attempted_at) VALUES (?1, ?2)",
        )
        .bind(requester_ip)
        .bind(attempted_at)
        .execute(pool)
        .await
        .map(|_| ()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(())
}

async fn count_recent_start_requests_for_user(
    pool: &DbPool,
    user_id: &str,
    window_start: &str,
) -> Result<i64, AppError> {
    let count: i64 = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_verification_tokens WHERE user_id = $1 AND created_at >= $2",
            )
            .bind(user_id)
            .bind(window_start)
            .fetch_one(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_verification_tokens WHERE user_id = ?1 AND created_at >= ?2",
            )
            .bind(user_id)
            .bind(window_start)
            .fetch_one(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(count)
}

async fn count_recent_start_requests_for_ip(
    pool: &DbPool,
    requester_ip: &str,
    window_start: &str,
) -> Result<i64, AppError> {
    let count: i64 = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_verification_tokens WHERE requester_ip = $1 AND created_at >= $2",
            )
            .bind(requester_ip)
            .bind(window_start)
            .fetch_one(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_verification_tokens WHERE requester_ip = ?1 AND created_at >= ?2",
            )
            .bind(requester_ip)
            .bind(window_start)
            .fetch_one(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(count)
}

async fn count_recent_verify_attempts_for_ip(
    pool: &DbPool,
    requester_ip: &str,
    window_start: &str,
) -> Result<i64, AppError> {
    let count: i64 = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_verification_attempts WHERE requester_ip = $1 AND attempted_at >= $2",
            )
            .bind(requester_ip)
            .bind(window_start)
            .fetch_one(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_verification_attempts WHERE requester_ip = ?1 AND attempted_at >= ?2",
            )
            .bind(requester_ip)
            .bind(window_start)
            .fetch_one(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(count)
}

async fn fetch_recovery_email_association(
    pool: &DbPool,
    user_id: &str,
) -> Result<Option<RecoveryEmailAssociation>, AppError> {
    let association: Option<RecoveryEmailAssociation> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT user_id, normalized_email, email_masked, verified_at, encrypted_private_key, encryption_algorithm, encryption_version, key_nonce, created_at, updated_at FROM user_recovery_email WHERE user_id = $1 LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT user_id, normalized_email, email_masked, verified_at, encrypted_private_key, encryption_algorithm, encryption_version, key_nonce, created_at, updated_at FROM user_recovery_email WHERE user_id = ?1 LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(association)
}

async fn upsert_pending_recovery_email_association(
    pool: &DbPool,
    user_id: &str,
    normalized_email: &str,
    email_masked: &str,
    now_rfc3339: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO user_recovery_email (user_id, normalized_email, email_masked, verified_at, encrypted_private_key, encryption_algorithm, encryption_version, key_nonce, created_at, updated_at)\nVALUES ($1, $2, $3, NULL, NULL, NULL, NULL, NULL, $4, $4)\nON CONFLICT (user_id) DO UPDATE SET normalized_email = EXCLUDED.normalized_email, email_masked = EXCLUDED.email_masked, verified_at = NULL, encrypted_private_key = NULL, encryption_algorithm = NULL, encryption_version = NULL, key_nonce = NULL, updated_at = EXCLUDED.updated_at",
        )
        .bind(user_id)
        .bind(normalized_email)
        .bind(email_masked)
        .bind(now_rfc3339)
        .execute(pool)
        .await
        .map(|_| ()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO user_recovery_email (user_id, normalized_email, email_masked, verified_at, encrypted_private_key, encryption_algorithm, encryption_version, key_nonce, created_at, updated_at)\nVALUES (?1, ?2, ?3, NULL, NULL, NULL, NULL, NULL, ?4, ?4)\nON CONFLICT (user_id) DO UPDATE SET normalized_email = excluded.normalized_email, email_masked = excluded.email_masked, verified_at = NULL, encrypted_private_key = NULL, encryption_algorithm = NULL, encryption_version = NULL, key_nonce = NULL, updated_at = excluded.updated_at",
        )
        .bind(user_id)
        .bind(normalized_email)
        .bind(email_masked)
        .bind(now_rfc3339)
        .execute(pool)
        .await
        .map(|_| ()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(())
}

async fn invalidate_pending_verification_tokens(
    pool: &DbPool,
    user_id: &str,
    invalidated_at: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE email_verification_tokens SET used_at = $1, used_by_ip = $2 WHERE user_id = $3 AND used_at IS NULL",
        )
        .bind(invalidated_at)
        .bind("superseded")
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE email_verification_tokens SET used_at = ?1, used_by_ip = ?2 WHERE user_id = ?3 AND used_at IS NULL",
        )
        .bind(invalidated_at)
        .bind("superseded")
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(())
}

struct EmailVerificationTokenInsert<'a> {
    token_hash: &'a str,
    user_id: &'a str,
    target_email: &'a str,
    email_masked: &'a str,
    encrypted_private_key: &'a str,
    encryption_algorithm: &'a str,
    encryption_version: i64,
    key_nonce: &'a str,
    requester_ip: &'a str,
    expires_at: &'a str,
    created_at: &'a str,
}

async fn insert_verification_token(
    pool: &DbPool,
    input: &EmailVerificationTokenInsert<'_>,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO email_verification_tokens (token_hash, user_id, target_email, email_masked, encrypted_private_key, encryption_algorithm, encryption_version, key_nonce, requester_ip, expires_at, created_at)\nVALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(input.token_hash)
        .bind(input.user_id)
        .bind(input.target_email)
        .bind(input.email_masked)
        .bind(input.encrypted_private_key)
        .bind(input.encryption_algorithm)
        .bind(input.encryption_version)
        .bind(input.key_nonce)
        .bind(input.requester_ip)
        .bind(input.expires_at)
        .bind(input.created_at)
        .execute(pool)
        .await
        .map(|_| ()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO email_verification_tokens (token_hash, user_id, target_email, email_masked, encrypted_private_key, encryption_algorithm, encryption_version, key_nonce, requester_ip, expires_at, created_at)\nVALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(input.token_hash)
        .bind(input.user_id)
        .bind(input.target_email)
        .bind(input.email_masked)
        .bind(input.encrypted_private_key)
        .bind(input.encryption_algorithm)
        .bind(input.encryption_version)
        .bind(input.key_nonce)
        .bind(input.requester_ip)
        .bind(input.expires_at)
        .bind(input.created_at)
        .execute(pool)
        .await
        .map(|_| ()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(())
}

async fn fetch_verification_token(
    pool: &DbPool,
    token_hash: &str,
) -> Result<Option<EmailVerificationToken>, AppError> {
    let token: Option<EmailVerificationToken> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT token_hash, user_id, target_email, email_masked, encrypted_private_key, encryption_algorithm, encryption_version, key_nonce, requester_ip, used_by_ip, expires_at, used_at, created_at FROM email_verification_tokens WHERE token_hash = $1 LIMIT 1",
            )
            .bind(token_hash)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT token_hash, user_id, target_email, email_masked, encrypted_private_key, encryption_algorithm, encryption_version, key_nonce, requester_ip, used_by_ip, expires_at, used_at, created_at FROM email_verification_tokens WHERE token_hash = ?1 LIMIT 1",
            )
            .bind(token_hash)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(token)
}

async fn mark_verification_token_used(
    pool: &DbPool,
    token_hash: &str,
    requester_ip: &str,
    used_at: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "UPDATE email_verification_tokens SET used_at = $1, used_by_ip = $2 WHERE token_hash = $3 AND used_at IS NULL",
            )
            .bind(used_at)
            .bind(requester_ip)
            .bind(token_hash)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "UPDATE email_verification_tokens SET used_at = ?1, used_by_ip = ?2 WHERE token_hash = ?3 AND used_at IS NULL",
            )
            .bind(used_at)
            .bind(requester_ip)
            .bind(token_hash)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(rows == 1)
}

async fn upsert_verified_recovery_email_association(
    pool: &DbPool,
    token: &EmailVerificationToken,
    now_rfc3339: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO user_recovery_email (user_id, normalized_email, email_masked, verified_at, encrypted_private_key, encryption_algorithm, encryption_version, key_nonce, created_at, updated_at)\nVALUES ($1, $2, $3, $4, $5, $6, $7, $8, $4, $4)\nON CONFLICT (user_id) DO UPDATE SET normalized_email = EXCLUDED.normalized_email, email_masked = EXCLUDED.email_masked, verified_at = EXCLUDED.verified_at, encrypted_private_key = EXCLUDED.encrypted_private_key, encryption_algorithm = EXCLUDED.encryption_algorithm, encryption_version = EXCLUDED.encryption_version, key_nonce = EXCLUDED.key_nonce, updated_at = EXCLUDED.updated_at",
        )
        .bind(&token.user_id)
        .bind(&token.target_email)
        .bind(&token.email_masked)
        .bind(now_rfc3339)
        .bind(&token.encrypted_private_key)
        .bind(&token.encryption_algorithm)
        .bind(token.encryption_version)
        .bind(&token.key_nonce)
        .execute(pool)
        .await
        .map(|_| ()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO user_recovery_email (user_id, normalized_email, email_masked, verified_at, encrypted_private_key, encryption_algorithm, encryption_version, key_nonce, created_at, updated_at)\nVALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?4, ?4)\nON CONFLICT (user_id) DO UPDATE SET normalized_email = excluded.normalized_email, email_masked = excluded.email_masked, verified_at = excluded.verified_at, encrypted_private_key = excluded.encrypted_private_key, encryption_algorithm = excluded.encryption_algorithm, encryption_version = excluded.encryption_version, key_nonce = excluded.key_nonce, updated_at = excluded.updated_at",
        )
        .bind(&token.user_id)
        .bind(&token.target_email)
        .bind(&token.email_masked)
        .bind(now_rfc3339)
        .bind(&token.encrypted_private_key)
        .bind(&token.encryption_algorithm)
        .bind(token.encryption_version)
        .bind(&token.key_nonce)
        .execute(pool)
        .await
        .map(|_| ()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_masks_email() {
        let email = normalize_email("  User.Name@Example.com ").unwrap();
        assert_eq!(email, "user.name@example.com");
        assert_eq!(mask_email(&email), "u***@example.com");
    }

    #[test]
    fn rejects_invalid_email() {
        assert!(normalize_email("not-an-email").is_err());
    }
}
