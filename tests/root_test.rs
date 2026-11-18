use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};

use backend_project::http::router;
use tower::ServiceExt;

#[tokio::test]
async fn root_returns_server_running_message() {
    let request = Request::builder()
        .method("GET")
        .uri("/")
        .body(Body::empty())
        .unwrap();

    let response = router().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/json");

    let body = to_bytes(response.into_body(), 10240).await.unwrap();

    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);

    assert_eq!(json["message"], "AI backend server is running");

    assert_eq!(json["data"]["service"], "backend-ai-api");

    assert_eq!(json["data"]["version"], env!("CARGO_PKG_VERSION"));

    assert!(json["error"].is_null());
}
