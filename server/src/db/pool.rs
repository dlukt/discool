use std::str::FromStr;
use std::time::Duration;
use std::{io, io::ErrorKind};

use sqlx::{
    PgPool, SqlitePool,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use crate::config::DatabaseConfig;

#[derive(Clone)]
pub enum DbPool {
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

impl DbPool {
    pub fn size(&self) -> u32 {
        match self {
            Self::Postgres(pool) => pool.size(),
            Self::Sqlite(pool) => pool.size(),
        }
    }

    pub fn num_idle(&self) -> usize {
        match self {
            Self::Postgres(pool) => pool.num_idle(),
            Self::Sqlite(pool) => pool.num_idle(),
        }
    }

    pub async fn close(&self) {
        match self {
            Self::Postgres(pool) => pool.close().await,
            Self::Sqlite(pool) => pool.close().await,
        }
    }
}

pub async fn init_pool(config: &DatabaseConfig) -> Result<DbPool, sqlx::Error> {
    let backend = crate::db::DatabaseBackend::from_url(&config.url).map_err(|err| {
        sqlx::Error::Configuration(Box::new(io::Error::new(ErrorKind::InvalidInput, err)))
    })?;

    let sqlite_in_memory = config.url.starts_with("sqlite:")
        && (config.url.contains(":memory:") || config.url.contains("mode=memory"));
    let max_connections = if sqlite_in_memory {
        // In-memory SQLite is per-connection; multiple connections would create multiple databases.
        if config.max_connections != 1 {
            tracing::warn!(
                requested_max_connections = config.max_connections,
                "sqlite in-memory database detected; forcing max_connections=1"
            );
        }
        1
    } else {
        config.max_connections
    };

    // Keep the single in-memory connection alive so the DB doesn't disappear mid-process.
    let (min_connections, idle_timeout, max_lifetime) = if sqlite_in_memory {
        (1, None, None)
    } else {
        (
            0,
            Some(Duration::from_secs(60)),
            Some(Duration::from_secs(30 * 60)),
        )
    };

    match backend {
        crate::db::DatabaseBackend::Postgres => PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(Duration::from_secs(15))
            .idle_timeout(idle_timeout)
            .max_lifetime(max_lifetime)
            .connect(&config.url)
            .await
            .map(DbPool::Postgres),
        crate::db::DatabaseBackend::Sqlite => SqlitePoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(Duration::from_secs(15))
            .idle_timeout(idle_timeout)
            .max_lifetime(max_lifetime)
            .connect_with(SqliteConnectOptions::from_str(&config.url)?.pragma("foreign_keys", "ON"))
            .await
            .map(DbPool::Sqlite),
    }
}
