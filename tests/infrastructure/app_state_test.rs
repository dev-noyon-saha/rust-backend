use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};

use backend_project::{
    app::state::AppState,
    config::Settings,
    http,
    infrastructure::{cache::redis as redis_cache, database::postgres},
};

use tower::ServiceExt;

// Helper to load environment settings
fn settings() -> Settings {
    dotenvy::dotenv().ok();

    Settings::from_env().expect("Failed to load configuration")
}

// Verify real dependencies and production health
#[tokio::test]
async fn app_state_integration_test() {
    let config = settings();

    // Connect PostgreSQL
    let db = postgres::connect(&config.database_url)
        .await
        .expect("PostgreSQL connection failed");

    // Connect Redis
    let redis_manager = redis_cache::connect(&config.redis_url)
        .await
        .expect("Redis connection failed");

    // Create shared AppState
    let state = AppState::new(db.clone(), redis_manager);

    let cloned_state = state.clone();

    assert_eq!(state.started_at, cloned_state.started_at);

    // Verify PostgreSQL from shared state
    let result: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&cloned_state.db)
        .await
        .expect("PostgreSQL query failed");

    assert_eq!(result, 1);

    // Verify Redis from shared state
    let mut redis = cloned_state.redis.clone();

    let pong: String = redis::cmd("PING")
        .query_async(&mut redis)
        .await
        .expect("Redis PING failed");

    assert_eq!(pong, "PONG");

    // Build production router
    let app = http::router_with_state(state);

    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

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

// Verify 503 when PostgreSQL pool is unavailable
#[tokio::test]
async fn health_returns_503_when_postgres_fails() {
    let config = settings();

    let db = postgres::connect(&config.database_url)
        .await
        .expect("PostgreSQL connection failed");

    let redis_manager = redis_cache::connect(&config.redis_url)
        .await
        .expect("Redis connection failed");

    let state = AppState::new(db.clone(), redis_manager);

    // Simulate PostgreSQL failure locally.
    // This closes only this test's pool.
    db.close().await;

    let app = http::router_with_state(state);

    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = to_bytes(response.into_body(), 10240).await.unwrap();

    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert_eq!(json["data"]["status"], "unhealthy");

    assert_eq!(json["data"]["checks"]["postgresql"], "unhealthy");

    assert_eq!(json["data"]["checks"]["redis"], "healthy");

    assert_eq!(json["error"]["code"], "SERVICE_UNAVAILABLE");
}
