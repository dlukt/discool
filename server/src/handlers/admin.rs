use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::json;

use crate::{AppError, AppState};

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
}

pub async fn get_health(State(state): State<AppState>) -> Result<Response, AppError> {
    // TODO(Epic 2): Replace this pre-auth guard with real admin authentication/authorization.
    if !super::instance::is_initialized(&state.pool).await? {
        return Err(AppError::Forbidden(
            "Instance is not initialized".to_string(),
        ));
    }

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

    let health = AdminHealth {
        cpu_usage_percent,
        memory_rss_bytes,
        uptime_seconds: state.start_time.elapsed().as_secs(),
        db_size_bytes,
        db_pool_active,
        db_pool_idle,
        db_pool_max,
        websocket_connections: 0,
    };

    Ok((StatusCode::OK, Json(json!({ "data": health }))).into_response())
}

async fn db_size_bytes(state: &AppState) -> u64 {
    let Some(db) = state.config.database.as_ref() else {
        return 0;
    };

    let backend = crate::db::DatabaseBackend::from_url(&db.url).ok();
    match backend {
        Some(crate::db::DatabaseBackend::Postgres) => {
            match sqlx::query_scalar::<_, i64>("SELECT pg_database_size(current_database())")
                .fetch_one(&state.pool)
                .await
            {
                Ok(v) => u64::try_from(v).unwrap_or(0),
                Err(err) => {
                    tracing::warn!(error = %err, "Failed to query postgres database size");
                    0
                }
            }
        }
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
    use std::time::{Duration, Instant};

    use axum::{body::to_bytes, extract::State, response::IntoResponse};

    use super::*;

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
        }
    }

    async fn json_value(res: Response) -> serde_json::Value {
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn get_health_returns_403_when_instance_is_not_initialized() {
        let state = test_state().await;
        let err = get_health(State(state)).await.unwrap_err();

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
        sqlx::query(
            "INSERT INTO instance_settings (key, value)\nVALUES ('initialized_at', CURRENT_TIMESTAMP)",
        )
        .execute(&state.pool)
        .await
        .unwrap();

        let res = get_health(State(state)).await.unwrap();
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
}
