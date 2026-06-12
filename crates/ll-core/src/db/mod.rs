use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::path::Path;
use anyhow::Result;

pub mod queries;

pub async fn open(db_path: &Path) -> Result<SqlitePool> {
    let url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await?;
    sqlx::migrate!("./src/db/migrations").run(&pool).await?;
    Ok(pool)
}
