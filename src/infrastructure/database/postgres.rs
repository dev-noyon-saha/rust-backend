use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::config::DatabaseConfig;

pub async fn connect_with_config(config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect(config.url.expose_secret())
        .await?;

    // Verify the database is actually responding.
    sqlx::query("SELECT 1").execute(&pool).await?;

    Ok(pool)
}

/// Backward-compatible connector.
///
/// Existing integration tests can continue using:
/// postgres::connect(&database_url)
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let config = DatabaseConfig::new(database_url.to_string());

    connect_with_config(&config).await
}
