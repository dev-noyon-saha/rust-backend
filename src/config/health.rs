use std::time::Duration;

/// Configuration for dependency health checks.
#[derive(Debug, Clone)]
pub struct HealthConfig {
    pub check_timeout: Duration,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_timeout: Duration::from_secs(2),
        }
    }
}
