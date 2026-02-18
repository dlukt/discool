use discool_server::{config::DatabaseConfig, db};

#[tokio::test]
async fn pool_connects_to_sqlite_in_memory() {
    let cfg = DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        max_connections: 5,
    };

    let pool = db::init_pool(&cfg).await.unwrap();
    sqlx::query("SELECT 1").execute(&pool).await.unwrap();
}

#[tokio::test]
async fn migrations_run_on_sqlite_in_memory() {
    let cfg = DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        max_connections: 5,
    };

    let pool = db::init_pool(&cfg).await.unwrap();
    db::run_migrations(&pool).await.unwrap();

    let value: String = sqlx::query_scalar(
        "SELECT value FROM schema_metadata WHERE key = 'initialized_at' LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!value.is_empty());
}
