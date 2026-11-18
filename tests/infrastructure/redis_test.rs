use std::time::{SystemTime, UNIX_EPOCH};

use backend_project::infrastructure::cache::redis as redis_infra;

fn redis_url() -> String {
    dotenvy::dotenv().ok();

    std::env::var("REDIS_URL").expect("REDIS_URL must be configured")
}

#[tokio::test]
async fn redis_connection_successful() {
    let url = redis_url();

    let mut connection = redis_infra::connect(&url)
        .await
        .expect("Failed to connect to Redis");

    let pong: String = redis::cmd("PING")
        .query_async(&mut connection)
        .await
        .expect("Redis PING failed");

    assert_eq!(pong, "PONG");
}

// Test Redis SET, GET and DELETE
#[tokio::test]
async fn redis_read_write_successful() {
    let url = redis_url();

    let mut connection = redis_infra::connect(&url)
        .await
        .expect("Failed to connect to Redis");

    // Generate unique test key
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let key = format!("dwar:test:{}:{}", std::process::id(), timestamp);

    let value = "redis_integration_test";

    // SET with 60-second expiration
    let set_result: String = redis::cmd("SET")
        .arg(&key)
        .arg(value)
        .arg("EX")
        .arg(60)
        .query_async(&mut connection)
        .await
        .expect("Redis SET failed");

    assert_eq!(set_result, "OK");

    // GET
    let stored_value: String = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut connection)
        .await
        .expect("Redis GET failed");

    assert_eq!(stored_value, value);

    // DELETE
    let deleted: i64 = redis::cmd("DEL")
        .arg(&key)
        .query_async(&mut connection)
        .await
        .expect("Redis DELETE failed");

    assert_eq!(deleted, 1);
}
