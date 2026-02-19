use sqlx::migrate::Migrator;

static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn run_migrations(pool: &super::DbPool) -> Result<(), sqlx::migrate::MigrateError> {
    match pool {
        super::DbPool::Postgres(pool) => MIGRATOR.run(pool).await,
        super::DbPool::Sqlite(pool) => MIGRATOR.run(pool).await,
    }
}
