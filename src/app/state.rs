use std::time::Instant;

use axum::extract::FromRef;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

/// Shared application state for production.
#[derive(Clone)]
pub struct AppState {
    pub started_at: Instant,
    pub db: PgPool,
    pub redis: ConnectionManager,
}

impl AppState {
    pub fn new(db: PgPool, redis: ConnectionManager) -> Self {
        Self {
            started_at: Instant::now(),
            db,
            redis,
        }
    }
}

/// Dependencies required for operational health checks.
#[derive(Clone)]
pub struct HealthDependencies {
    pub db: PgPool,
    pub redis: ConnectionManager,
}

/// Lightweight health state.
///
/// In production, dependencies are available.
/// In isolated HTTP tests, they can be omitted.
#[derive(Clone)]
pub struct HealthState {
    pub started_at: Instant,
    pub dependencies: Option<HealthDependencies>,
}

impl HealthState {
    /// Create an isolated state without database access.
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            dependencies: None,
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
        }
    }
}
