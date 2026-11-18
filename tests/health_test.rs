use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};

use backend_project::http::router;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_success_response() {
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = router().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 10240).await.unwrap();

    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["message"], "Service is healthy");
    assert_eq!(json["data"]["status"], "healthy");

    assert_eq!(json["data"]["service"], "backend-ai-api");

    assert_eq!(json["data"]["version"], env!("CARGO_PKG_VERSION"));

    // Verify uptime
    assert!(json["data"]["uptime_seconds"].is_u64());

    // Verify timestamp
    let timestamp = json["data"]["timestamp"].as_str().unwrap();

    assert!(chrono::DateTime::parse_from_rfc3339(timestamp).is_ok());

    // Isolated HTTP tests skip real dependencies
    assert_eq!(json["data"]["checks"]["postgresql"], "not_checked");

    assert_eq!(json["data"]["checks"]["redis"], "not_checked");

    assert!(json["error"].is_null());
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let request = Request::builder()
        .uri("/unknown")
        .body(Body::empty())
        .unwrap();

    let response = router().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unsupported_method_returns_405() {
    let request = Request::builder()
        .method("POST")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = router().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
