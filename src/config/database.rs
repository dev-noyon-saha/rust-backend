use std::time::Duration;

use super::secrets::SecretString;

/// PostgreSQL connection pool configuration.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: SecretString,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
}

impl DatabaseConfig {
    pub fn new(url: String) -> Self {
        Self {
            url: SecretString::new(url),
            max_connections: 5,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(5),
        }
    }
}
