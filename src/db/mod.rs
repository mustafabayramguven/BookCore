pub mod queries;
pub mod schema;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

use crate::config::AppConfig;
use crate::error::AppResult;

pub type DbPool = Pool<Sqlite>;

pub async fn init_db(config: &AppConfig) -> AppResult<DbPool> {
    config.ensure_dirs().map_err(|e| {
        crate::error::AppError::Internal(format!("Failed to create directories: {}", e))
    })?;

    let db_url = config.db_url();
    let connect_opts = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(connect_opts)
        .await?;

    for pragma in schema::PRAGMAS {
        sqlx::query(pragma).execute(&pool).await?;
    }

    for ddl in schema::SCHEMA_DDL {
        sqlx::query(ddl).execute(&pool).await?;
    }

    // Ensure default library exists
    let default_path = config.books_dir.to_string_lossy().to_string();
    let existing_library: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM libraries WHERE path = ? LIMIT 1")
            .bind(&default_path)
            .fetch_optional(&pool)
            .await?;

    if existing_library.is_none() {
        sqlx::query("INSERT INTO libraries (name, path) VALUES (?, ?)")
            .bind("Default Library")
            .bind(&default_path)
            .execute(&pool)
            .await?;
    }

    Ok(pool)
}
