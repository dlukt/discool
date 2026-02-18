use sqlx::{AnyPool, migrate::Migrator};

static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn run_migrations(pool: &AnyPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}
