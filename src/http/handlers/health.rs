use std::time::Duration;

use axum::{Json, extract::State, http::StatusCode};

use chrono::{SecondsFormat, Utc};
use redis::aio::ConnectionManager;
use serde::Serialize;
use sqlx::PgPool;
use tokio::time::timeout;

use crate::{
    app::state::HealthState,
    http::response::{ApiErrorInfo, ApiResponse},
};

// Maximum time allowed per dependency check
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(2);

// Dependency health status
#[derive(Debug, Serialize)]
pub struct HealthChecks {
    pub postgresql: &'static str,
    pub redis: &'static str,
}

// Health response data
#[derive(Debug, Serialize)]
pub struct HealthData {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub uptime_seconds: u64,
    pub timestamp: String,
    pub checks: HealthChecks,
}

// Check PostgreSQL connectivity
async fn check_postgres(db: &PgPool) -> &'static str {
    let result = timeout(
        HEALTH_CHECK_TIMEOUT,
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(db),
    )
    .await;

    match result {
        Ok(Ok(1)) => "healthy",
        _ => "unhealthy",
    }
}

// Check Redis connectivity
async fn check_redis(manager: &ConnectionManager) -> &'static str {
    let mut connection = manager.clone();

    let result = timeout(HEALTH_CHECK_TIMEOUT, async {
        let pong: String = redis::cmd("PING").query_async(&mut connection).await?;

        Ok::<String, redis::RedisError>(pong)
    })
    .await;

    match result {
        Ok(Ok(ref pong)) if pong == "PONG" => "healthy",
        _ => "unhealthy",
    }
}

// GET /health
pub async fn health(
    State(state): State<HealthState>,
) -> (StatusCode, Json<ApiResponse<HealthData>>) {
    // Calculate uptime
    let uptime_seconds = state.started_at.elapsed().as_secs();

    // Generate UTC timestamp
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    // Check dependencies in parallel
    let (postgresql, redis) = match &state.dependencies {
        Some(dependencies) => {
            tokio::join!(
                check_postgres(&dependencies.db),
                check_redis(&dependencies.redis),
            )
        }

        // Lightweight HTTP test router
        None => ("not_checked", "not_checked"),
    };

    // Dependency availability
    let is_healthy = postgresql != "unhealthy" && redis != "unhealthy";

    // Build health response
    let data = HealthData {
        status: if is_healthy { "healthy" } else { "unhealthy" },
        service: "backend-ai-api",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds,
        timestamp,
        checks: HealthChecks { postgresql, redis },
    };

    // Return 200 when healthy
    if is_healthy {
        return (
            StatusCode::OK,
            Json(ApiResponse::success("Service is healthy", data)),
        );
    }

    // Return 503 when a dependency fails
    let response = ApiResponse {
        success: false,
        message: "Service is unhealthy".to_string(),
        data: Some(data),
        error: Some(ApiErrorInfo {
            code: "SERVICE_UNAVAILABLE",
            details: None,
        }),
    };

    (StatusCode::SERVICE_UNAVAILABLE, Json(response))
}
