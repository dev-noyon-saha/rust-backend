use std::time::{Duration, Instant};

use axum::extract::FromRef;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::config::HealthConfig;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub started_at: Instant,
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub health_check_timeout: Duration,
}

impl AppState {
    /// Construct state using default health configuration.
    ///
    /// Preserved for backward compatibility.
    pub fn new(db: PgPool, redis: ConnectionManager) -> Self {
        let timeout = HealthConfig::default().check_timeout;

        Self::with_health_timeout(db, redis, timeout)
    }

    /// Construct state with a custom health timeout.
    pub fn with_health_timeout(
        db: PgPool,
        redis: ConnectionManager,
        health_check_timeout: Duration,
    ) -> Self {
        Self {
            started_at: Instant::now(),
            db,
            redis,
            health_check_timeout,
        }
    }
}

/// Dependencies required by the health endpoint.
#[derive(Clone)]
pub struct HealthDependencies {
    pub db: PgPool,
    pub redis: ConnectionManager,
}

/// Health-specific state.
#[derive(Clone)]
pub struct HealthState {
    pub started_at: Instant,
    pub dependencies: Option<HealthDependencies>,
    pub check_timeout: Duration,
}

impl HealthState {
    /// Lightweight state for isolated HTTP tests.
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            dependencies: None,
            check_timeout: HealthConfig::default().check_timeout,
        }
    }
}

impl Default for HealthState {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract HealthState from production AppState.
impl FromRef<AppState> for HealthState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            started_at: state.started_at,
            dependencies: Some(HealthDependencies {
                db: state.db.clone(),
                redis: state.redis.clone(),
            }),
            check_timeout: state.health_check_timeout,
        }
    }
}
