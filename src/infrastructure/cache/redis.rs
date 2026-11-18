use redis::{Client, ErrorKind, RedisError, RedisResult, aio::ConnectionManager};

pub async fn connect(redis_url: &str) -> RedisResult<ConnectionManager> {
    // Create Redis client
    let client = Client::open(redis_url)?;

    // Create async connection manager
    let mut manager = client.get_connection_manager().await?;

    // Verify Redis connectivity
    let pong: String = redis::cmd("PING").query_async(&mut manager).await?;

    // Validate Redis response
    if pong != "PONG" {
        return Err(RedisError::from((
            ErrorKind::UnexpectedReturnType,
            "Unexpected Redis PING response",
        )));
    }

    Ok(manager)
}
