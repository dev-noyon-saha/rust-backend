use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};

use backend_project::http::{error::ApiError, response::FieldError, router};

use serde_json::Value;
use tower::ServiceExt;

// Helper: Convert HTTP response body into JSON
async fn response_json(response: Response) -> Value {
    assert_eq!(response.headers()["content-type"], "application/json");

    let body = to_bytes(response.into_body(), 10240).await.unwrap();

    serde_json::from_slice(&body).unwrap()
}

// Test 400 Bad Request
#[tokio::test]
async fn bad_request_returns_standard_error() {
    let error = ApiError::new(
        StatusCode::BAD_REQUEST,
        "BAD_REQUEST",
        "Invalid request data",
    );

    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let json = response_json(response).await;

    assert_eq!(json["success"], false);
    assert_eq!(json["message"], "Invalid request data");
    assert_eq!(json["error"]["code"], "BAD_REQUEST");
    assert!(json["data"].is_null());
}

// Test 401 Unauthorized
#[tokio::test]
async fn unauthorized_returns_401() {
    let error = ApiError::new(
        StatusCode::UNAUTHORIZED,
        "UNAUTHORIZED",
        "Authentication required",
    );

    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let json = response_json(response).await;

    assert_eq!(json["success"], false);
    assert_eq!(json["error"]["code"], "UNAUTHORIZED");
}

// Test 403 Forbidden
#[tokio::test]
async fn forbidden_returns_403() {
    let error = ApiError::new(
        StatusCode::FORBIDDEN,
        "FORBIDDEN",
        "You do not have permission",
    );

    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let json = response_json(response).await;

    assert_eq!(json["error"]["code"], "FORBIDDEN");
}

// Test 422 Validation Error
#[tokio::test]
async fn validation_error_returns_field_details() {
    let error = ApiError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        code: "VALIDATION_ERROR",
        message: "Validation failed".to_string(),
        details: Some(vec![FieldError {
            field: "message".to_string(),
            message: "Message is required".to_string(),
        }]),
    };

    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let json = response_json(response).await;

    assert_eq!(json["success"], false);
    assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(json["error"]["details"][0]["field"], "message");
    assert_eq!(
        json["error"]["details"][0]["message"],
        "Message is required"
    );
}

// Test 500 Internal Server Error
#[tokio::test]
async fn internal_server_error_returns_500() {
    let error = ApiError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_SERVER_ERROR",
        "An unexpected error occurred",
    );

    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let json = response_json(response).await;

    assert_eq!(json["success"], false);
    assert_eq!(json["error"]["code"], "INTERNAL_SERVER_ERROR");
    assert_eq!(json["message"], "An unexpected error occurred");
}

// Test 404 for unknown routes
#[tokio::test]
async fn unknown_route_returns_standard_404() {
    let request = Request::builder()
        .uri("/non-existing-endpoint")
        .body(Body::empty())
        .unwrap();

    let response = router().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let json = response_json(response).await;

    assert_eq!(json["success"], false);
    assert_eq!(json["error"]["code"], "NOT_FOUND");
    assert!(json["data"].is_null());
}

// Test 405 for unsupported methods
#[tokio::test]
async fn unsupported_method_returns_standard_405() {
    let request = Request::builder()
        .method("POST")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = router().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    let json = response_json(response).await;

    assert_eq!(json["success"], false);
    assert_eq!(json["error"]["code"], "METHOD_NOT_ALLOWED");
}
