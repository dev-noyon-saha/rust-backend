use axum::{Json, extract::State};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;

use crate::http::{response::ApiResponse, state::AppState};

// Health check response DTO
#[derive(Debug, Serialize)]
pub struct HealthData {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub uptime_seconds: u64,
    pub timestamp: String,
}

// GET /health
pub async fn health(State(state): State<AppState>) -> Json<ApiResponse<HealthData>> {
    // Calculate server uptime
    let uptime_seconds = state.started_at.elapsed().as_secs();

    // Current UTC timestamp
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    // Build health response
    let data = HealthData {
        status: "healthy",
        service: "backend-ai-api",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds,
        timestamp,
    };

    Json(ApiResponse::success("Service is healthy", data))
}
