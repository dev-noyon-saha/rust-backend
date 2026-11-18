pub mod error;
pub mod handlers;
pub mod response;
pub mod state;

use axum::{Router, http::StatusCode, routing::get};

use tower_http::trace::TraceLayer;

use self::{error::ApiError, state::AppState};

// Handle unknown endpoints
async fn not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        "The requested endpoint does not exist",
    )
}

// Handle unsupported HTTP methods
async fn method_not_allowed() -> ApiError {
    ApiError::new(
        StatusCode::METHOD_NOT_ALLOWED,
        "METHOD_NOT_ALLOWED",
        "HTTP method not allowed for this endpoint",
    )
}

// Register API routes
pub fn router() -> Router {
    // Initialize application state once
    let app_state = AppState::new();

    Router::new()
        .route("/", get(handlers::root::root))
        .route("/health", get(handlers::health::health))
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state)
}
