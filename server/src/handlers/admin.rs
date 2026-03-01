use axum::{
    Json,
    body::Body,
    extract::State,
    http::{
        HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use std::io::ErrorKind;
use std::process::Stdio;
use std::time::Duration;

use crate::middleware::auth::AuthenticatedUser;
use crate::{AppError, AppState, db::DbPool, services::presence_service};

#[cfg(target_os = "linux")]
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Serialize)]
pub struct AdminHealth {
    pub cpu_usage_percent: f64,
    pub memory_rss_bytes: u64,
    pub uptime_seconds: u64,
    pub db_size_bytes: u64,
    pub db_pool_active: u32,
    pub db_pool_idle: u32,
    pub db_pool_max: u32,
    pub websocket_connections: u32,
    pub p2p_discovered_instances: u32,
    pub p2p_connection_count: u32,
    pub p2p_message_rate_per_minute: f64,
    pub p2p_ingress_total: u64,
    pub p2p_rejected_total: u64,
    pub p2p_throttled_total: u64,
    pub p2p_healthy_peer_count: u32,
    pub p2p_bootstrap_failures: u32,
    pub p2p_degraded: bool,
    pub p2p_degraded_reason: Option<String>,
    pub p2p_discovery_enabled: bool,
    pub p2p_discovery_label: String,
}

async fn require_admin(pool: &DbPool, username: &str) -> Result<(), AppError> {
    let admin_username: Option<String> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar("SELECT username FROM admin_users LIMIT 1")
                .fetch_optional(pool)
                .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar("SELECT username FROM admin_users LIMIT 1")
                .fetch_optional(pool)
                .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    match admin_username {
        Some(admin) if admin == username => Ok(()),
        Some(_) => Err(AppError::Forbidden("Admin access required".to_string())),
        None => Err(AppError::Forbidden(
            "Instance admin is not configured".to_string(),
        )),
    }
}

pub async fn get_health(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Response, AppError> {
    // TODO(Epic 2): Replace this pre-auth guard with real admin authorization.
    if !super::instance::is_initialized(&state.pool).await? {
        return Err(AppError::Forbidden(
            "Instance is not initialized".to_string(),
        ));
    }

    require_admin(&state.pool, &user.username).await?;

    let cpu_usage_percent = cpu_usage_percent().await;
    let memory_rss_bytes = memory_rss_bytes().await;

    let db_pool_max = state
        .config
        .database
        .as_ref()
        .map(|db| db.max_connections)
        .unwrap_or(state.pool.size());

    let db_pool_idle = u32::try_from(state.pool.num_idle()).unwrap_or(state.pool.size());
    let db_pool_active = state.pool.size().saturating_sub(db_pool_idle);

    let db_size_bytes = db_size_bytes(&state).await;
    let p2p_discovered_instances = crate::p2p::discovery::count_discovered_instances(&state.pool)
        .await
        .map_err(AppError::Internal)?;
    let (
        runtime_discovery_enabled,
        p2p_connection_count,
        p2p_message_rate_per_minute,
        p2p_ingress_total,
        p2p_rejected_total,
        p2p_throttled_total,
        p2p_healthy_peer_count,
        p2p_bootstrap_failures,
        p2p_degraded,
        p2p_degraded_reason,
    ) = {
        let p2p_metadata = state
            .p2p_metadata
            .read()
            .map_err(|err| AppError::Internal(format!("p2p metadata lock poisoned: {err}")))?;
        (
            p2p_metadata.discovery_enabled,
            p2p_metadata.connection_count,
            p2p_metadata.message_rate_per_minute,
            p2p_metadata.ingress_total,
            p2p_metadata.rejected_total,
            p2p_metadata.throttled_total,
            p2p_metadata.healthy_peer_count,
            p2p_metadata.bootstrap_failures,
            p2p_metadata.degraded,
            p2p_metadata.degraded_reason.clone(),
        )
    };
    let p2p_discovery_enabled = if let Some(enabled) = runtime_discovery_enabled {
        enabled
    } else if !state.config.p2p.enabled {
        false
    } else {
        let instance_discovery_enabled =
            crate::p2p::discovery::load_instance_discovery_enabled(&state.pool)
                .await
                .map_err(AppError::Internal)?;
        crate::p2p::discovery::resolve_effective_discovery_enabled(
            state.config.p2p.discovery.enabled,
            instance_discovery_enabled,
        )
    };

    let health = AdminHealth {
        cpu_usage_percent,
        memory_rss_bytes,
        uptime_seconds: state.start_time.elapsed().as_secs(),
        db_size_bytes,
        db_pool_active,
        db_pool_idle,
        db_pool_max,
        websocket_connections: presence_service::websocket_connection_count(),
        p2p_discovered_instances,
        p2p_connection_count,
        p2p_message_rate_per_minute,
        p2p_ingress_total,
        p2p_rejected_total,
        p2p_throttled_total,
        p2p_healthy_peer_count,
        p2p_bootstrap_failures,
        p2p_degraded,
        p2p_degraded_reason,
        p2p_discovery_enabled,
        p2p_discovery_label: crate::p2p::discovery::discovery_mode_label(p2p_discovery_enabled)
            .to_string(),
    };

    Ok((StatusCode::OK, Json(json!({ "data": health }))).into_response())
}

pub async fn create_backup(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Response, AppError> {
    // TODO(Epic 2): Replace this pre-auth guard with real admin authorization.
    if !super::instance::is_initialized(&state.pool).await? {
        return Err(AppError::Forbidden(
            "Instance is not initialized".to_string(),
        ));
    }

    require_admin(&state.pool, &user.username).await?;

    let db = state
        .config
        .database
        .as_ref()
        .ok_or_else(|| AppError::Internal("Database is not configured".to_string()))?;

    let backend = crate::db::DatabaseBackend::from_url(&db.url).map_err(AppError::Internal)?;

    let timestamp = Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
    let backup_id = Uuid::new_v4();
    let ext = match backend {
        crate::db::DatabaseBackend::Sqlite => "db",
        crate::db::DatabaseBackend::Postgres => "sql",
    };
    let filename = format!("discool-backup-{timestamp}.{ext}");
    // Avoid predictable temp filenames in shared temp dirs (collision + symlink risk).
    let temp_filename = format!("discool-backup-{timestamp}-{backup_id}.{ext}");
    let temp_path = std::env::temp_dir().join(&temp_filename);

    let result: Result<Response, AppError> = async {
        match backend {
            crate::db::DatabaseBackend::Sqlite => {
                match &state.pool {
                    DbPool::Sqlite(pool) => {
                        sqlx::query("VACUUM INTO ?1")
                            .bind(temp_path.to_string_lossy().as_ref())
                            .execute(pool)
                            .await
                            .map_err(|err| AppError::Internal(err.to_string()))?;
                    }
                    DbPool::Postgres(_) => {
                        return Err(AppError::Internal(
                            "Database backend mismatch (expected sqlite pool)".to_string(),
                        ));
                    }
                }
            }
            crate::db::DatabaseBackend::Postgres => {
                use tokio::io::AsyncReadExt;

                let (pg_dump_dbname, pg_password) = postgres_dbname_and_password(&db.url);
                let mut cmd = tokio::process::Command::new("pg_dump");
                cmd.kill_on_drop(true);
                if let Some(pg_password) = pg_password {
                    cmd.env("PGPASSWORD", pg_password);
                }

                let mut child = cmd
                    // Non-interactive: never prompt for passwords; fail fast instead.
                    .arg("--no-password")
                    // Improve restore portability on a fresh instance/user.
                    .arg("--no-owner")
                    .arg("--no-privileges")
                    .arg("--dbname")
                    .arg(&pg_dump_dbname)
                    .arg("--format=plain")
                    .arg("--file")
                    .arg(&temp_path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::piped())
                    .spawn()
                    .map_err(|err| {
                        if err.kind() == ErrorKind::NotFound {
                            AppError::Internal(
                                "pg_dump not found. Install PostgreSQL client tools to enable backups.".to_string(),
                            )
                        } else {
                            AppError::Internal(err.to_string())
                        }
                    })?;

                let mut stderr = child.stderr.take().ok_or_else(|| {
                    AppError::Internal("Failed to capture pg_dump stderr".to_string())
                })?;
                let stderr_task = tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let _ = stderr.read_to_end(&mut buf).await;
                    buf
                });

                let status = match tokio::time::timeout(Duration::from_secs(30), child.wait()).await
                {
                    Ok(status) => status.map_err(|err| AppError::Internal(err.to_string()))?,
                    Err(_) => {
                        let _ = child.kill().await;
                        let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
                        stderr_task.abort();
                        let _ = stderr_task.await;
                        return Err(AppError::Internal(
                            "pg_dump timed out after 30 seconds".to_string(),
                        ));
                    }
                };

                let stderr_bytes = stderr_task.await.unwrap_or_default();

                if !status.success() {
                    let stderr = String::from_utf8_lossy(&stderr_bytes)
                        .replace(&db.url, "[REDACTED]")
                        .replace(&pg_dump_dbname, "[REDACTED]")
                        .trim()
                        .to_string();
                    return Err(AppError::Internal(if stderr.is_empty() {
                        "pg_dump failed".to_string()
                    } else {
                        format!("pg_dump failed: {stderr}")
                    }));
                }
            }
        }

        let bytes = tokio::fs::read(&temp_path).await.map_err(|err| {
            AppError::Internal(format!(
                "Failed to read backup file {}: {err}",
                temp_path.display()
            ))
        })?;

        if let Some(output_dir) = state
            .config
            .backup
            .as_ref()
            .and_then(|b| b.output_dir.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let base = std::path::Path::new(output_dir);
            let mut dst = base.join(&filename);
            if tokio::fs::metadata(&dst).await.is_ok() {
                dst = base.join(format!("discool-backup-{timestamp}-{backup_id}.{ext}"));
            }
            if let Err(err) = tokio::fs::write(&dst, &bytes).await {
                tracing::warn!(
                    error = %err,
                    output_dir = %output_dir,
                    "Failed to save backup to output directory"
                );
            }
        }

        let mut res = Response::new(Body::from(bytes));
        *res.status_mut() = StatusCode::OK;
        res.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        res.headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));

        let content_disposition = format!("attachment; filename=\"{filename}\"");
        res.headers_mut().insert(
            CONTENT_DISPOSITION,
            HeaderValue::from_str(&content_disposition)
                .map_err(|err| AppError::Internal(err.to_string()))?,
        );

        Ok(res)
    }
    .await;

    if let Err(err) = tokio::fs::remove_file(&temp_path).await
        && err.kind() != ErrorKind::NotFound
    {
        tracing::warn!(
            error = %err,
            path = %temp_path.display(),
            "Failed to remove temp backup file"
        );
    }

    result
}

async fn db_size_bytes(state: &AppState) -> u64 {
    let Some(db) = state.config.database.as_ref() else {
        return 0;
    };

    let backend = crate::db::DatabaseBackend::from_url(&db.url).ok();
    match backend {
        Some(crate::db::DatabaseBackend::Postgres) => match &state.pool {
            DbPool::Postgres(pool) => {
                match sqlx::query_scalar::<_, i64>("SELECT pg_database_size(current_database())")
                    .fetch_one(pool)
                    .await
                {
                    Ok(v) => u64::try_from(v).unwrap_or(0),
                    Err(err) => {
                        tracing::warn!(error = %err, "Failed to query postgres database size");
                        0
                    }
                }
            }
            DbPool::Sqlite(_) => 0,
        },
        Some(crate::db::DatabaseBackend::Sqlite) => sqlite_db_size_bytes(&db.url).await,
        None => 0,
    }
}

async fn sqlite_db_size_bytes(db_url: &str) -> u64 {
    if db_url.contains(":memory:") || db_url.contains("mode=memory") {
        return 0;
    }

    let Some(path) = sqlite_db_path(db_url) else {
        return 0;
    };

    match tokio::fs::metadata(path).await {
        Ok(m) => m.len(),
        Err(err) => {
            tracing::warn!(error = %err, "Failed to stat sqlite database file");
            0
        }
    }
}

fn sqlite_db_path(db_url: &str) -> Option<&str> {
    let path = if let Some(rest) = db_url.strip_prefix("sqlite://") {
        rest
    } else {
        db_url.strip_prefix("sqlite:")?
    };

    let (path, _query) = path.split_once('?').unwrap_or((path, ""));
    if path.is_empty() {
        return None;
    }

    Some(path)
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
struct CpuSample {
    total_ticks: u64,
    proc_ticks: u64,
    cpu_count: u32,
    starttime_ticks: u64,
}

#[cfg(target_os = "linux")]
static LAST_CPU_SAMPLE: OnceLock<Mutex<Option<CpuSample>>> = OnceLock::new();

#[cfg(target_os = "linux")]
async fn cpu_usage_percent() -> f64 {
    let (proc_stat, proc_self_stat) = tokio::join!(
        tokio::fs::read_to_string("/proc/stat"),
        tokio::fs::read_to_string("/proc/self/stat")
    );

    let Ok(proc_stat) = proc_stat else {
        return 0.0;
    };
    let Ok(proc_self_stat) = proc_self_stat else {
        return 0.0;
    };

    let Some((total_ticks, cpu_count)) = parse_proc_stat(&proc_stat) else {
        return 0.0;
    };
    let Some(proc) = parse_proc_self_stat(&proc_self_stat) else {
        return 0.0;
    };

    let sample = CpuSample {
        total_ticks,
        proc_ticks: proc.utime_ticks + proc.stime_ticks,
        cpu_count,
        starttime_ticks: proc.starttime_ticks,
    };

    let prev = {
        let lock = LAST_CPU_SAMPLE.get_or_init(|| Mutex::new(None));
        let mut guard = match lock.lock() {
            Ok(guard) => guard,
            Err(err) => err.into_inner(),
        };
        let prev = *guard;
        *guard = Some(sample);
        prev
    };

    if let Some(prev) = prev
        && let Some(usage) = cpu_usage_percent_delta(prev, sample)
    {
        return usage;
    }

    cpu_usage_percent_average(sample)
}

#[cfg(not(target_os = "linux"))]
async fn cpu_usage_percent() -> f64 {
    0.0
}

#[cfg(target_os = "linux")]
fn cpu_usage_percent_delta(prev: CpuSample, curr: CpuSample) -> Option<f64> {
    let delta_total = curr.total_ticks.checked_sub(prev.total_ticks)?;
    let delta_proc = curr.proc_ticks.checked_sub(prev.proc_ticks)?;
    if delta_total == 0 {
        return None;
    }

    let usage = (delta_proc as f64 / delta_total as f64) * 100.0;
    if usage.is_finite() && usage >= 0.0 {
        Some(usage.min(100.0))
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn cpu_usage_percent_average(sample: CpuSample) -> f64 {
    let cpu_count = sample.cpu_count.max(1) as f64;
    let uptime_ticks_per_cpu = sample.total_ticks as f64 / cpu_count;
    let age_ticks = uptime_ticks_per_cpu - sample.starttime_ticks as f64;
    if age_ticks <= 0.0 {
        return 0.0;
    }

    let usage = ((sample.proc_ticks as f64 / age_ticks) / cpu_count) * 100.0;
    if usage.is_finite() && usage >= 0.0 {
        usage.min(100.0)
    } else {
        0.0
    }
}

#[cfg(target_os = "linux")]
async fn memory_rss_bytes() -> u64 {
    let Ok(status) = tokio::fs::read_to_string("/proc/self/status").await else {
        return 0;
    };

    status
        .lines()
        .find(|line| line.starts_with("VmRSS:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|kb| kb.parse::<u64>().ok())
        .map(|kb| kb * 1024)
        .unwrap_or(0)
}

#[cfg(not(target_os = "linux"))]
async fn memory_rss_bytes() -> u64 {
    0
}

#[cfg(target_os = "linux")]
struct ProcSelfStat {
    utime_ticks: u64,
    stime_ticks: u64,
    starttime_ticks: u64,
}

#[cfg(target_os = "linux")]
fn parse_proc_self_stat(contents: &str) -> Option<ProcSelfStat> {
    let end = contents.rfind(')')?;
    let after = contents.get((end + 2)..)?; // ") "
    let parts: Vec<&str> = after.split_whitespace().collect();

    Some(ProcSelfStat {
        // fields are 1-indexed; after the (comm) field we start at field 3.
        utime_ticks: parts.get(11)?.parse().ok()?,
        stime_ticks: parts.get(12)?.parse().ok()?,
        starttime_ticks: parts.get(19)?.parse().ok()?,
    })
}

#[cfg(target_os = "linux")]
fn parse_proc_stat(contents: &str) -> Option<(u64, u32)> {
    let mut total_ticks: Option<u64> = None;
    let mut cpu_count: u32 = 0;

    for line in contents.lines() {
        if line.starts_with("cpu ") {
            let mut sum = 0u64;
            for part in line.split_whitespace().skip(1) {
                sum = sum.checked_add(part.parse::<u64>().ok()?)?;
            }
            total_ticks = Some(sum);
        } else if let Some(rest) = line.strip_prefix("cpu")
            && rest.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            cpu_count += 1;
        }
    }

    let total_ticks = total_ticks?;
    if cpu_count == 0 {
        cpu_count = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);
    }

    Some((total_ticks, cpu_count.max(1)))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use axum::{body::to_bytes, extract::State, response::IntoResponse};
    use dashmap::DashMap;

    use super::*;

    fn test_user() -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: "user-1".to_string(),
            session_id: "session-1".to_string(),
            username: "tomas".to_string(),
            did_key: "did:key:z6Mk-test".to_string(),
        }
    }

    async fn test_state() -> AppState {
        let mut cfg = crate::config::Config::default();
        cfg.database = Some(crate::config::DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        });

        let pool = crate::db::init_pool(cfg.database.as_ref().unwrap())
            .await
            .unwrap();
        crate::db::run_migrations(&pool).await.unwrap();

        AppState {
            config: Arc::new(cfg),
            pool,
            start_time: Instant::now() - Duration::from_secs(5),
            challenges: Arc::new(DashMap::new()),
            p2p_metadata: Arc::new(std::sync::RwLock::new(crate::p2p::P2pMetadata::default())),
            voice_runtime: Arc::new(crate::webrtc::voice_channel::VoiceRuntime::new(
                crate::config::VoiceConfig::default(),
            )),
        }
    }

    async fn test_state_file_db() -> (AppState, std::path::PathBuf) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("discool-test-db-{nanos}.db"));
        let _ = std::fs::File::create(&path).unwrap();

        let mut cfg = crate::config::Config::default();
        cfg.database = Some(crate::config::DatabaseConfig {
            url: format!("sqlite://{}", path.display()),
            max_connections: 1,
        });

        let pool = crate::db::init_pool(cfg.database.as_ref().unwrap())
            .await
            .unwrap();
        crate::db::run_migrations(&pool).await.unwrap();

        (
            AppState {
                config: Arc::new(cfg),
                pool,
                start_time: Instant::now() - Duration::from_secs(5),
                challenges: Arc::new(DashMap::new()),
                p2p_metadata: Arc::new(std::sync::RwLock::new(crate::p2p::P2pMetadata::default())),
                voice_runtime: Arc::new(crate::webrtc::voice_channel::VoiceRuntime::new(
                    crate::config::VoiceConfig::default(),
                )),
            },
            path,
        )
    }

    async fn json_value(res: Response) -> serde_json::Value {
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn create_backup_returns_403_when_instance_is_not_initialized() {
        let state = test_state().await;
        let err = create_backup(State(state), test_user()).await.unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "FORBIDDEN", "message": "Instance is not initialized", "details": {} } })
        );
    }

    #[tokio::test]
    async fn create_backup_returns_sqlite_backup_with_expected_headers_and_data_when_initialized() {
        use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE};

        let (state, db_path) = test_state_file_db().await;
        match &state.pool {
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO instance_settings (key, value)\nVALUES ('initialized_at', CURRENT_TIMESTAMP)",
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO admin_users (id, username, avatar_color)\nVALUES ('admin-1', 'tomas', NULL)",
                )
                .execute(pool)
                .await
                .unwrap();
            }
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO instance_settings (key, value)\nVALUES ('initialized_at', CURRENT_TIMESTAMP)",
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO admin_users (id, username, avatar_color)\nVALUES ('admin-1', 'tomas', NULL)",
                )
                .execute(pool)
                .await
                .unwrap();
            }
        }

        let res = create_backup(State(state), test_user()).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let content_type = res
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(content_type, "application/octet-stream");

        let cache_control = res
            .headers()
            .get(CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(cache_control, "no-store");

        let content_disposition = res
            .headers()
            .get(CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_disposition.contains("attachment"),
            "unexpected content-disposition: {content_disposition}"
        );
        assert!(
            content_disposition.contains(".db"),
            "expected .db filename; content-disposition: {content_disposition}"
        );

        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let bytes = body.to_vec();

        assert!(
            bytes.starts_with(b"SQLite format 3\0"),
            "expected sqlite magic bytes at start"
        );

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("discool-backup-test-{nanos}.db"));
        tokio::fs::write(&path, &bytes).await.unwrap();

        let url = format!("sqlite:{}", path.display());
        let pool = sqlx::SqlitePool::connect(&url).await.unwrap();

        let initialized_at: String = sqlx::query_scalar(
            "SELECT value FROM instance_settings WHERE key = 'initialized_at' LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!initialized_at.is_empty());

        let schema_initialized_at: String = sqlx::query_scalar(
            "SELECT value FROM schema_metadata WHERE key = 'initialized_at' LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!schema_initialized_at.is_empty());

        let username: String =
            sqlx::query_scalar("SELECT username FROM admin_users WHERE username = 'tomas' LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(username, "tomas");

        drop(pool);
        tokio::fs::remove_file(&path).await.unwrap();

        let _ = tokio::fs::remove_file(&db_path).await;
        let _ = tokio::fs::remove_file(db_path.with_extension("db-wal")).await;
        let _ = tokio::fs::remove_file(db_path.with_extension("db-shm")).await;
    }

    #[tokio::test]
    async fn create_backup_copies_backup_to_output_dir_when_configured() {
        use axum::http::header::CONTENT_DISPOSITION;

        let (mut state, db_path) = test_state_file_db().await;
        match &state.pool {
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO instance_settings (key, value)\nVALUES ('initialized_at', CURRENT_TIMESTAMP)",
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO admin_users (id, username, avatar_color)\nVALUES ('admin-1', 'tomas', NULL)",
                )
                .execute(pool)
                .await
                .unwrap();
            }
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO instance_settings (key, value)\nVALUES ('initialized_at', CURRENT_TIMESTAMP)",
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO admin_users (id, username, avatar_color)\nVALUES ('admin-1', 'tomas', NULL)",
                )
                .execute(pool)
                .await
                .unwrap();
            }
        }

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output_dir = std::env::temp_dir().join(format!("discool-backup-out-{nanos}"));
        tokio::fs::create_dir_all(&output_dir).await.unwrap();

        let mut cfg = (*state.config).clone();
        cfg.backup = Some(crate::config::BackupConfig {
            output_dir: Some(output_dir.to_string_lossy().to_string()),
        });
        state.config = Arc::new(cfg);

        let res = create_backup(State(state), test_user()).await.unwrap();
        let content_disposition = res
            .headers()
            .get(CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let filename = content_disposition
            .split("filename=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .expect("expected quoted filename in content-disposition");

        let saved_path = output_dir.join(filename);
        let bytes = tokio::fs::read(&saved_path).await.unwrap();
        assert!(
            bytes.starts_with(b"SQLite format 3\0"),
            "expected sqlite magic bytes at start"
        );

        let _ = tokio::fs::remove_dir_all(&output_dir).await;
        let _ = tokio::fs::remove_file(&db_path).await;
        let _ = tokio::fs::remove_file(db_path.with_extension("db-wal")).await;
        let _ = tokio::fs::remove_file(db_path.with_extension("db-shm")).await;
    }

    #[tokio::test]
    async fn get_health_returns_403_when_instance_is_not_initialized() {
        let state = test_state().await;
        let err = get_health(State(state), test_user()).await.unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "FORBIDDEN", "message": "Instance is not initialized", "details": {} } })
        );
    }

    #[tokio::test]
    async fn get_health_returns_200_with_expected_json_shape_when_initialized() {
        let state = test_state().await;
        match &state.pool {
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO instance_settings (key, value)\nVALUES ('initialized_at', CURRENT_TIMESTAMP)",
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO admin_users (id, username, avatar_color)\nVALUES ('admin-1', 'tomas', NULL)",
                )
                .execute(pool)
                .await
                .unwrap();
            }
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO instance_settings (key, value)\nVALUES ('initialized_at', CURRENT_TIMESTAMP)",
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO admin_users (id, username, avatar_color)\nVALUES ('admin-1', 'tomas', NULL)",
                )
                .execute(pool)
                .await
                .unwrap();
            }
        }

        let res = get_health(State(state), test_user()).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let value = json_value(res).await;
        let data = value.get("data").and_then(|v| v.as_object()).unwrap();

        let uptime_seconds = data
            .get("uptime_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        assert!(uptime_seconds > 0, "expected uptime_seconds > 0");

        let db_pool_max = data
            .get("db_pool_max")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        assert!(db_pool_max > 0, "expected db_pool_max > 0");

        assert!(
            data.get("db_pool_idle").and_then(|v| v.as_u64()).is_some(),
            "expected db_pool_idle to be a number"
        );

        assert_eq!(data.get("websocket_connections"), Some(&json!(0)));
        assert_eq!(data.get("p2p_discovered_instances"), Some(&json!(0)));
        assert_eq!(data.get("p2p_connection_count"), Some(&json!(0)));
        assert_eq!(data.get("p2p_message_rate_per_minute"), Some(&json!(0.0)));
        assert_eq!(data.get("p2p_ingress_total"), Some(&json!(0)));
        assert_eq!(data.get("p2p_rejected_total"), Some(&json!(0)));
        assert_eq!(data.get("p2p_throttled_total"), Some(&json!(0)));
        assert_eq!(data.get("p2p_healthy_peer_count"), Some(&json!(0)));
        assert_eq!(data.get("p2p_bootstrap_failures"), Some(&json!(0)));
        assert_eq!(data.get("p2p_degraded"), Some(&json!(false)));
        assert_eq!(data.get("p2p_degraded_reason"), Some(&json!(null)));
        assert_eq!(data.get("p2p_discovery_enabled"), Some(&json!(true)));
        assert_eq!(data.get("p2p_discovery_label"), Some(&json!("Enabled")));

        let cpu_usage_percent = data
            .get("cpu_usage_percent")
            .and_then(|v| v.as_f64())
            .unwrap_or(-1.0);
        assert!(cpu_usage_percent >= 0.0);

        assert!(
            data.get("memory_rss_bytes")
                .and_then(|v| v.as_u64())
                .is_some(),
            "expected memory_rss_bytes to be a number"
        );

        assert!(
            data.get("db_size_bytes").and_then(|v| v.as_u64()).is_some(),
            "expected db_size_bytes to be a number"
        );
    }

    #[tokio::test]
    async fn get_health_prefers_runtime_discovery_mode_when_available() {
        let state = test_state().await;
        match &state.pool {
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO instance_settings (key, value)\nVALUES ('initialized_at', CURRENT_TIMESTAMP)",
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO instance_settings (key, value)\nVALUES ('discovery_enabled', 'true')",
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO admin_users (id, username, avatar_color)\nVALUES ('admin-1', 'tomas', NULL)",
                )
                .execute(pool)
                .await
                .unwrap();
            }
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO instance_settings (key, value)\nVALUES ('initialized_at', CURRENT_TIMESTAMP)",
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO instance_settings (key, value)\nVALUES ('discovery_enabled', 'true')",
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO admin_users (id, username, avatar_color)\nVALUES ('admin-1', 'tomas', NULL)",
                )
                .execute(pool)
                .await
                .unwrap();
            }
        }

        {
            let mut metadata = state.p2p_metadata.write().unwrap();
            metadata.discovery_enabled = Some(false);
        }

        let res = get_health(State(state), test_user()).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let value = json_value(res).await;
        let data = value.get("data").and_then(|v| v.as_object()).unwrap();
        assert_eq!(data.get("p2p_discovery_enabled"), Some(&json!(false)));
        assert_eq!(
            data.get("p2p_discovery_label"),
            Some(&json!("Disabled (Unlisted)"))
        );
    }

    #[test]
    fn postgres_dbname_and_password_strips_userinfo_password() {
        let (dbname, password) =
            super::postgres_dbname_and_password("postgres://user:secret@localhost:5432/discool");
        assert_eq!(dbname, "postgres://user@localhost:5432/discool");
        assert_eq!(password.as_deref(), Some("secret"));
    }

    #[test]
    fn postgres_dbname_and_password_percent_decodes_password() {
        let (dbname, password) = super::postgres_dbname_and_password(
            "postgresql://user:s%40cr%23t@localhost:5432/discool",
        );
        assert_eq!(dbname, "postgresql://user@localhost:5432/discool");
        assert_eq!(password.as_deref(), Some("s@cr#t"));
    }

    #[test]
    fn postgres_dbname_and_password_strips_query_password() {
        let (dbname, password) = super::postgres_dbname_and_password(
            "postgres://user@localhost/discool?sslmode=require&PASSWORD=s%40cr%23t",
        );
        assert_eq!(dbname, "postgres://user@localhost/discool?sslmode=require");
        assert_eq!(password.as_deref(), Some("s@cr#t"));
    }

    #[test]
    fn postgres_dbname_and_password_prefers_userinfo_password_over_query_password() {
        let (dbname, password) = super::postgres_dbname_and_password(
            "postgres://user:fromuserinfo@localhost/discool?sslmode=require&password=fromquery",
        );
        assert_eq!(dbname, "postgres://user@localhost/discool?sslmode=require");
        assert_eq!(password.as_deref(), Some("fromuserinfo"));
    }

    #[test]
    fn postgres_dbname_and_password_omits_trailing_question_mark_when_query_only_contains_password()
    {
        let (dbname, password) =
            super::postgres_dbname_and_password("postgres://localhost/discool?password=secret");
        assert_eq!(dbname, "postgres://localhost/discool");
        assert_eq!(password.as_deref(), Some("secret"));
    }
}

fn postgres_dbname_and_password(db_url: &str) -> (String, Option<String>) {
    let Some((scheme, rest)) = db_url.split_once("://") else {
        return (db_url.to_string(), None);
    };
    if scheme != "postgres" && scheme != "postgresql" {
        return (db_url.to_string(), None);
    }

    let (rest, fragment) = rest.split_once('#').unwrap_or((rest, ""));
    let (before_query, query) = rest.split_once('?').unwrap_or((rest, ""));

    let (before_query, mut password) =
        if let Some((userinfo, after_at)) = before_query.split_once('@') {
            if let Some((user, password)) = userinfo.split_once(':') {
                (format!("{user}@{after_at}"), Some(percent_decode(password)))
            } else {
                (before_query.to_string(), None)
            }
        } else {
            (before_query.to_string(), None)
        };

    let (query, query_password) = strip_postgres_password_from_query(query);
    if password.is_none() {
        password = query_password;
    }

    let mut dbname = format!("{scheme}://{before_query}");
    if !query.is_empty() {
        dbname.push('?');
        dbname.push_str(&query);
    }
    if !fragment.is_empty() {
        dbname.push('#');
        dbname.push_str(fragment);
    }

    (dbname, password)
}

fn strip_postgres_password_from_query(query: &str) -> (String, Option<String>) {
    if query.is_empty() {
        return (String::new(), None);
    }

    let mut parts = Vec::new();
    let mut password: Option<String> = None;

    for part in query.split('&') {
        if part.is_empty() {
            continue;
        }

        let (key, value) = part.split_once('=').unwrap_or((part, ""));
        if key.eq_ignore_ascii_case("password") {
            if !value.is_empty() {
                password = Some(percent_decode(value));
            }
            continue;
        }

        parts.push(part);
    }

    (parts.join("&"), password)
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h1), Some(h2)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2]))
        {
            out.push((h1 << 4) | h2);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
