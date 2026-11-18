use axum::Json;
use serde::Serialize;

use crate::http::response::ApiResponse;

#[derive(Debug, Serialize)]
pub struct RootData {
    pub service: &'static str,
    pub version: &'static str,
}

// GET/
pub async fn root() -> Json<ApiResponse<RootData>> {
    Json(ApiResponse::success(
        "AI backend server is running",
        RootData {
            service: "backend-ai-api",
            version: env!("CARGO_PKG_VERSION"),
        },
    ))
}
