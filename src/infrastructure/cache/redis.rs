use std::io;

use redis::{
    Client, ErrorKind, RedisError, RedisResult,
    aio::{ConnectionManager, ConnectionManagerConfig},
};

use tokio::time::timeout;

use crate::config::RedisConfig;

/// Create a Redis timeout error.
fn redis_timeout_error() -> RedisError {
    RedisError::from(io::Error::new(
        io::ErrorKind::TimedOut,
        "Redis operation timed out",
    ))
}

/// Connect to Redis using typed configuration.
pub async fn connect_with_config(config: &RedisConfig) -> RedisResult<ConnectionManager> {
    // 1. Create Redis client.
    let client = Client::open(config.url.expose_secret())?;

    // 2. Configure connection and response timeouts.
    let manager_config = ConnectionManagerConfig::new()
        .set_connection_timeout(Some(config.operation_timeout))
        .set_response_timeout(Some(config.operation_timeout));

    // 3. Initialize connection manager with timeout.
    let mut manager = timeout(
        config.operation_timeout,
        client.get_connection_manager_with_config(manager_config),
    )
    .await
    .map_err(|_| redis_timeout_error())??;

    // 4. Verify Redis connectivity using PING.
    let pong: String = timeout(
        config.operation_timeout,
        redis::cmd("PING").query_async(&mut manager),
    )
    .await
    .map_err(|_| redis_timeout_error())??;

    // 5. Validate PING response.
    if pong != "PONG" {
        return Err(RedisError::from((
            ErrorKind::UnexpectedReturnType,
            "Unexpected Redis PING response",
        )));
    }

    Ok(manager)
}

/// Backward-compatible Redis connection.
///
/// Existing tests can continue using connect(redis_url).
pub async fn connect(redis_url: &str) -> RedisResult<ConnectionManager> {
    let config = RedisConfig::new(redis_url.to_string());

    connect_with_config(&config).await
}
