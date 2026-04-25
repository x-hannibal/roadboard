use roadboard_storage::migrate;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

pub async fn setup_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory connect");
    migrate(&pool).await.expect("migrate");
    pool
}
