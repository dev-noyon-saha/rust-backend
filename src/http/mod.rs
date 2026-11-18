pub mod error;
pub mod handlers;
pub mod response;

use axum::{Router, extract::FromRef, http::StatusCode, routing::get};

use tower_http::trace::TraceLayer;

use crate::app::state::{AppState, HealthState};

use self::error::ApiError;

async fn not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        "The requested endpoint does not exist",
    )
}

async fn method_not_allowed() -> ApiError {
    ApiError::new(
        StatusCode::METHOD_NOT_ALLOWED,
        "METHOD_NOT_ALLOWED",
        "HTTP method not allowed for this endpoint",
    )
}

// Common route definitions
fn build_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    HealthState: FromRef<S>,
{
    Router::<S>::new()
        .route("/", get(handlers::root::root))
        .route("/health", get(handlers::health::health))
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
}

// Lightweight router for HTTP tests
pub fn router() -> Router {
    build_router::<HealthState>().with_state(HealthState::new())
}

// Production router with real dependencies
pub fn router_with_state(state: AppState) -> Router {
    build_router::<AppState>().with_state(state)
}
