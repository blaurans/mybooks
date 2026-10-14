use sqlx::sqlite::SqliteConnectOptions;
pub use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use std::str::FromStr;

pub async fn connect(database_url: &str) -> anyhow::Result<SqlitePool> {
    // Accept both sqlite:// and sqlite: URLs.
    let opts = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;
    Ok(pool)
}
