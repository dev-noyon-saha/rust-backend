use std::time::Duration;

use super::secrets::SecretString;

/// Redis client configuration.
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: SecretString,
    pub operation_timeout: Duration,
}

impl RedisConfig {
    pub fn new(url: String) -> Self {
        Self {
            url: SecretString::new(url),
            operation_timeout: Duration::from_secs(2),
        }
    }
}
