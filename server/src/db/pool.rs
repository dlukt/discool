use std::time::Duration;

use sqlx::{AnyPool, any::AnyPoolOptions};

use crate::config::DatabaseConfig;

pub async fn init_pool(config: &DatabaseConfig) -> Result<AnyPool, sqlx::Error> {
    // CRITICAL: Must be called before any AnyPool/AnyConnection creation.
    sqlx::any::install_default_drivers();

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

    AnyPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(15))
        .idle_timeout(idle_timeout)
        .max_lifetime(max_lifetime)
        .connect(&config.url)
        .await
}
