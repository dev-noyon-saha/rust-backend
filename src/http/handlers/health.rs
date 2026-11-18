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

#[derive(Debug, Serialize)]
pub struct HealthChecks {
    pub postgresql: &'static str,
    pub redis: &'static str,
}

#[derive(Debug, Serialize)]
pub struct HealthData {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub uptime_seconds: u64,
    pub timestamp: String,
    pub checks: HealthChecks,
}

/// Check PostgreSQL with the configured timeout.
async fn check_postgres(db: &PgPool, check_timeout: Duration) -> &'static str {
    let result = timeout(
        check_timeout,
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(db),
    )
    .await;

    match result {
        Ok(Ok(1)) => "healthy",
        _ => "unhealthy",
    }
}

/// Check Redis with the configured timeout.
async fn check_redis(manager: &ConnectionManager, check_timeout: Duration) -> &'static str {
    let mut connection = manager.clone();

    let result = timeout(
        check_timeout,
        redis::cmd("PING").query_async::<String>(&mut connection),
    )
    .await;

    match result {
        Ok(Ok(ref pong)) if pong == "PONG" => "healthy",
        _ => "unhealthy",
    }
}

/// GET /health
pub async fn health(
    State(state): State<HealthState>,
) -> (StatusCode, Json<ApiResponse<HealthData>>) {
    let uptime_seconds = state.started_at.elapsed().as_secs();

    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    // Run dependency checks concurrently.
    let (postgresql, redis) = match &state.dependencies {
        Some(dependencies) => {
            tokio::join!(
                check_postgres(&dependencies.db, state.check_timeout,),
                check_redis(&dependencies.redis, state.check_timeout,),
            )
        }

        // Isolated HTTP tests without dependencies.
        None => ("not_checked", "not_checked"),
    };

    let is_healthy = postgresql != "unhealthy" && redis != "unhealthy";

    let data = HealthData {
        status: if is_healthy { "healthy" } else { "unhealthy" },
        service: "backend-ai-api",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds,
        timestamp,
        checks: HealthChecks { postgresql, redis },
    };

    if is_healthy {
        return (
            StatusCode::OK,
            Json(ApiResponse::success("Service is healthy", data)),
        );
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse {
            success: false,
            message: "Service is unhealthy".to_string(),
            data: Some(data),
            error: Some(ApiErrorInfo {
                code: "SERVICE_UNAVAILABLE",
                details: None,
            }),
        }),
    )
}
