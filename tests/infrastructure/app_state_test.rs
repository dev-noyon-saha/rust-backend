use std::time::Duration;

use axum::{
    body::{Body, to_bytes},
    extract::FromRef,
    http::{Request, StatusCode},
};

use backend_project::{
    app::state::{AppState, HealthState},
    config::Settings,
    http,
    infrastructure::{cache::redis as redis_cache, database::postgres},
};

use tower::ServiceExt;

fn settings() -> Settings {
    dotenvy::dotenv().ok();

    Settings::from_env().expect("Failed to load configuration")
}

#[tokio::test]
async fn app_state_integration_test() {
    let mut config = settings();

    // Override health timeout for this test.
    config.health.check_timeout = Duration::from_secs(4);

    // Use typed PostgreSQL configuration.
    let db = postgres::connect_with_config(&config.database)
        .await
        .expect("PostgreSQL connection failed");

    // Use typed Redis configuration.
    let redis_manager = redis_cache::connect_with_config(&config.redis)
        .await
        .expect("Redis connection failed");

    let state =
        AppState::with_health_timeout(db.clone(), redis_manager, config.health.check_timeout);

    let cloned_state = state.clone();

    assert_eq!(state.started_at, cloned_state.started_at);

    // Verify configured timeout reaches HealthState.
    let health_state = HealthState::from_ref(&state);

    assert_eq!(health_state.check_timeout, Duration::from_secs(4));

    // Verify PostgreSQL.
    let result: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&cloned_state.db)
        .await
        .expect("PostgreSQL query failed");

    assert_eq!(result, 1);

    // Verify Redis.
    let mut redis = cloned_state.redis.clone();

    let pong: String = redis::cmd("PING")
        .query_async(&mut redis)
        .await
        .expect("Redis PING failed");

    assert_eq!(pong, "PONG");

    // Test production HTTP router.
    let app = http::router_with_state(state);

    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.expect("HTTP request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 10240).await.unwrap();

    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["status"], "healthy");

    assert_eq!(json["data"]["checks"]["postgresql"], "healthy");

    assert_eq!(json["data"]["checks"]["redis"], "healthy");

    assert!(json["error"].is_null());

    db.close().await;
}

#[tokio::test]
async fn health_returns_503_when_postgres_fails() {
    let config = settings();

    let db = postgres::connect_with_config(&config.database)
        .await
        .expect("PostgreSQL connection failed");

    let redis_manager = redis_cache::connect_with_config(&config.redis)
        .await
        .expect("Redis connection failed");

    let state =
        AppState::with_health_timeout(db.clone(), redis_manager, config.health.check_timeout);

    // Close only this test's PostgreSQL pool.
    db.close().await;

    let app = http::router_with_state(state);

    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.expect("HTTP request failed");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = to_bytes(response.into_body(), 10240).await.unwrap();

    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert_eq!(json["data"]["status"], "unhealthy");

    assert_eq!(json["data"]["checks"]["postgresql"], "unhealthy");

    assert_eq!(json["data"]["checks"]["redis"], "healthy");

    assert_eq!(json["error"]["code"], "SERVICE_UNAVAILABLE");
}
