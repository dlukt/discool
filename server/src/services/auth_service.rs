use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    identity::challenge::{ChallengeRecord, generate_challenge},
    models::{session::Session, user::User},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    ChallengeNotFound,
    ChallengeMismatch,
    ChallengeExpired,
}

pub fn create_challenge(challenges: &DashMap<String, ChallengeRecord>, did_key: &str) -> String {
    let challenge = generate_challenge();
    challenges.insert(
        did_key.to_string(),
        ChallengeRecord {
            challenge: challenge.clone(),
            did_key: did_key.to_string(),
            created_at: Instant::now(),
        },
    );
    challenge
}

pub fn validate_challenge(
    challenges: &DashMap<String, ChallengeRecord>,
    did_key: &str,
    challenge: &str,
    ttl_seconds: u64,
) -> Result<(), AuthError> {
    let ttl = Duration::from_secs(ttl_seconds);
    let record = challenges
        .get(did_key)
        .ok_or(AuthError::ChallengeNotFound)?;

    let expired = record.created_at.elapsed() > ttl;
    let matches = record.challenge == challenge;
    drop(record);

    if expired {
        // Lazily clean up expired records without removing a fresh replacement.
        challenges.remove_if(did_key, |_, v| v.created_at.elapsed() > ttl);
        return Err(AuthError::ChallengeExpired);
    }

    if !matches {
        return Err(AuthError::ChallengeMismatch);
    }

    // Consume the challenge (one-time use). Use remove_if to avoid removing a fresh replacement.
    if challenges
        .remove_if(did_key, |_, v| {
            v.challenge == challenge && v.created_at.elapsed() <= ttl
        })
        .is_some()
    {
        Ok(())
    } else {
        Err(AuthError::ChallengeNotFound)
    }
}

pub fn check_challenge(
    challenges: &DashMap<String, ChallengeRecord>,
    did_key: &str,
    challenge: &str,
    ttl_seconds: u64,
) -> Result<(), AuthError> {
    let ttl = Duration::from_secs(ttl_seconds);
    let record = challenges
        .get(did_key)
        .ok_or(AuthError::ChallengeNotFound)?;

    let expired = record.created_at.elapsed() > ttl;
    let matches = record.challenge == challenge;
    drop(record);

    if expired {
        // Lazily clean up expired records without removing a fresh replacement.
        challenges.remove_if(did_key, |_, v| v.created_at.elapsed() > ttl);
        return Err(AuthError::ChallengeExpired);
    }

    if !matches {
        return Err(AuthError::ChallengeMismatch);
    }

    Ok(())
}

pub async fn create_session(
    pool: &DbPool,
    user_id: &str,
    ttl_hours: u64,
) -> Result<Session, AppError> {
    let id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().to_string();

    let now = Utc::now();
    let created_at = now.to_rfc3339();
    let ttl_hours: i64 = ttl_hours
        .try_into()
        .map_err(|_| AppError::Internal("auth.session_ttl_hours is too large".to_string()))?;
    let expires_at = (now + chrono::Duration::hours(ttl_hours)).to_rfc3339();
    let last_active_at = created_at.clone();

    match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO sessions (id, user_id, token, created_at, expires_at, last_active_at)\nVALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(&token)
        .bind(&created_at)
        .bind(&expires_at)
        .bind(&last_active_at)
        .execute(pool)
        .await
        .map(|_| ()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO sessions (id, user_id, token, created_at, expires_at, last_active_at)\nVALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(&token)
        .bind(&created_at)
        .bind(&expires_at)
        .bind(&last_active_at)
        .execute(pool)
        .await
        .map(|_| ()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Session {
        id,
        user_id: user_id.to_string(),
        token,
        created_at,
        expires_at,
        last_active_at,
    })
}

pub async fn validate_session(pool: &DbPool, token: &str) -> Result<(Session, User), AppError> {
    let session: Option<Session> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, user_id, token, created_at, expires_at, last_active_at\nFROM sessions\nWHERE token = $1\nLIMIT 1",
            )
            .bind(token)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, user_id, token, created_at, expires_at, last_active_at\nFROM sessions\nWHERE token = ?1\nLIMIT 1",
            )
            .bind(token)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let Some(session) = session else {
        return Err(AppError::Unauthorized(
            "Invalid or expired session".to_string(),
        ));
    };

    let expires_at = match DateTime::parse_from_rfc3339(&session.expires_at) {
        Ok(ts) => ts.with_timezone(&Utc),
        Err(_) => {
            tracing::warn!(
                session_id = %session.id,
                expires_at = %session.expires_at,
                "Invalid session expires_at timestamp; treating as expired"
            );
            if let Err(err) = delete_session_by_id(pool, &session.id).await
                && !matches!(err, AppError::Unauthorized(_))
            {
                tracing::warn!(
                    error = ?err,
                    session_id = %session.id,
                    "Failed to delete invalid session"
                );
            }
            return Err(AppError::Unauthorized(
                "Invalid or expired session".to_string(),
            ));
        }
    };
    if expires_at <= Utc::now() {
        if let Err(err) = delete_session_by_id(pool, &session.id).await
            && !matches!(err, AppError::Unauthorized(_))
        {
            tracing::warn!(
                error = ?err,
                session_id = %session.id,
                "Failed to delete expired session"
            );
        }
        return Err(AppError::Unauthorized(
            "Invalid or expired session".to_string(),
        ));
    }

    let user: Option<User> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at\nFROM users\nWHERE id = $1\nLIMIT 1",
            )
            .bind(&session.user_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at\nFROM users\nWHERE id = ?1\nLIMIT 1",
            )
            .bind(&session.user_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let Some(user) = user else {
        return Err(AppError::Unauthorized(
            "Invalid or expired session".to_string(),
        ));
    };

    Ok((session, user))
}

pub async fn refresh_session(pool: &DbPool, session_id: &str) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();

    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query("UPDATE sessions SET last_active_at = $1 WHERE id = $2")
                .bind(&now)
                .bind(session_id)
                .execute(pool)
                .await
                .map(|r| r.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query("UPDATE sessions SET last_active_at = ?1 WHERE id = ?2")
                .bind(&now)
                .bind(session_id)
                .execute(pool)
                .await
                .map(|r| r.rows_affected())
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    if rows == 0 {
        return Err(AppError::Unauthorized(
            "Invalid or expired session".to_string(),
        ));
    }

    Ok(())
}

pub async fn delete_session(pool: &DbPool, token: &str) -> Result<(), AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(token)
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query("DELETE FROM sessions WHERE token = ?1")
            .bind(token)
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    if rows == 0 {
        return Err(AppError::Unauthorized(
            "Invalid or expired session".to_string(),
        ));
    }

    Ok(())
}

pub async fn delete_session_by_id(pool: &DbPool, session_id: &str) -> Result<(), AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_id)
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query("DELETE FROM sessions WHERE id = ?1")
            .bind(session_id)
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    if rows == 0 {
        return Err(AppError::Unauthorized(
            "Invalid or expired session".to_string(),
        ));
    }

    Ok(())
}

pub async fn fetch_user_by_did(pool: &DbPool, did_key: &str) -> Result<User, AppError> {
    let user: Option<User> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at\nFROM users\nWHERE did_key = $1\nLIMIT 1",
            )
            .bind(did_key)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at\nFROM users\nWHERE did_key = ?1\nLIMIT 1",
            )
            .bind(did_key)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    user.ok_or(AppError::NotFound)
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;

    use dashmap::DashMap;

    use super::*;

    #[test]
    fn challenge_lifecycle_create_then_validate_consumes() {
        let challenges = DashMap::new();
        let did = "did:key:z6Mk-test";

        let challenge = create_challenge(&challenges, did);
        assert!(challenges.contains_key(did));

        assert_eq!(
            validate_challenge(&challenges, did, &challenge, 300),
            Ok(())
        );
        assert!(!challenges.contains_key(did));
    }

    #[test]
    fn challenge_validation_rejects_wrong_did() {
        let challenges = DashMap::new();
        let did = "did:key:z6Mk-test";
        let other = "did:key:z6Mk-other";

        let challenge = create_challenge(&challenges, did);
        assert_eq!(
            validate_challenge(&challenges, other, &challenge, 300),
            Err(AuthError::ChallengeNotFound)
        );
        assert!(challenges.contains_key(did));
    }

    #[test]
    fn challenge_validation_rejects_expired() {
        let challenges = DashMap::new();
        let did = "did:key:z6Mk-test";

        challenges.insert(
            did.to_string(),
            ChallengeRecord {
                challenge: "abc".to_string(),
                did_key: did.to_string(),
                created_at: Instant::now() - Duration::from_secs(301),
            },
        );

        assert_eq!(
            validate_challenge(&challenges, did, "abc", 300),
            Err(AuthError::ChallengeExpired)
        );
        assert!(!challenges.contains_key(did));
    }

    #[test]
    fn challenge_validation_rejects_mismatch_and_keeps_record() {
        let challenges = DashMap::new();
        let did = "did:key:z6Mk-test";

        let _ = create_challenge(&challenges, did);
        assert_eq!(
            validate_challenge(&challenges, did, "wrong", 300),
            Err(AuthError::ChallengeMismatch)
        );
        assert!(challenges.contains_key(did));
    }

    #[test]
    fn challenge_check_does_not_consume() {
        let challenges = DashMap::new();
        let did = "did:key:z6Mk-test";

        let challenge = create_challenge(&challenges, did);
        assert_eq!(check_challenge(&challenges, did, &challenge, 300), Ok(()));
        assert!(challenges.contains_key(did));

        assert_eq!(
            validate_challenge(&challenges, did, &challenge, 300),
            Ok(())
        );
        assert!(!challenges.contains_key(did));
    }

    #[tokio::test]
    async fn session_create_validate_refresh_delete_roundtrip() {
        let mut cfg = crate::config::Config::default();
        cfg.database = Some(crate::config::DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        });

        let pool = crate::db::init_pool(cfg.database.as_ref().unwrap())
            .await
            .unwrap();
        crate::db::run_migrations(&pool).await.unwrap();

        let user_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let updated_at = created_at.clone();
        match &pool {
            DbPool::Postgres(pool) => {
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)\nVALUES ($1, $2, $3, $4, $5, $6, $7)")
                    .bind(&user_id)
                    .bind("did:key:z6Mk-test")
                    .bind("z6Mk-test")
                    .bind("liam")
                    .bind(Option::<String>::None)
                    .bind(&created_at)
                    .bind(&updated_at)
                    .execute(pool)
                    .await
                    .unwrap();
            }
            DbPool::Sqlite(pool) => {
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)\nVALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
                    .bind(&user_id)
                    .bind("did:key:z6Mk-test")
                    .bind("z6Mk-test")
                    .bind("liam")
                    .bind(Option::<String>::None)
                    .bind(&created_at)
                    .bind(&updated_at)
                    .execute(pool)
                    .await
                    .unwrap();
            }
        }

        let session = create_session(&pool, &user_id, 168).await.unwrap();
        let (validated, user) = validate_session(&pool, &session.token).await.unwrap();
        assert_eq!(validated.id, session.id);
        assert_eq!(user.id, user_id);

        refresh_session(&pool, &session.id).await.unwrap();

        delete_session(&pool, &session.token).await.unwrap();
        let err = validate_session(&pool, &session.token).await.unwrap_err();
        let res = err.into_response();
        assert_eq!(res.status(), axum::http::StatusCode::UNAUTHORIZED);

        // Create a new session and corrupt expires_at; ensure we fail closed (401), not 500.
        let session = create_session(&pool, &user_id, 168).await.unwrap();

        // Corrupt expires_at and ensure we fail closed (401), not 500.
        match &pool {
            DbPool::Postgres(pool) => {
                sqlx::query("UPDATE sessions SET expires_at = $1 WHERE id = $2")
                    .bind("not-a-timestamp")
                    .bind(&session.id)
                    .execute(pool)
                    .await
                    .unwrap();
            }
            DbPool::Sqlite(pool) => {
                sqlx::query("UPDATE sessions SET expires_at = ?1 WHERE id = ?2")
                    .bind("not-a-timestamp")
                    .bind(&session.id)
                    .execute(pool)
                    .await
                    .unwrap();
            }
        }
        let err = validate_session(&pool, &session.token).await.unwrap_err();
        let res = err.into_response();
        assert_eq!(res.status(), axum::http::StatusCode::UNAUTHORIZED);

        let err = refresh_session(&pool, &session.id).await.unwrap_err();
        let res = err.into_response();
        assert_eq!(res.status(), axum::http::StatusCode::UNAUTHORIZED);
    }
}
