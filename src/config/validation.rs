use std::{net::IpAddr, str::FromStr};

use sqlx::postgres::PgConnectOptions;

use super::{
    environment::Environment,
    settings::{Settings, SettingsError},
};

fn invalid(name: &'static str, expected: &'static str) -> SettingsError {
    SettingsError::InvalidValue { name, expected }
}

fn valid_host(host: &str) -> bool {
    if host.parse::<IpAddr>().is_ok() {
        return true;
    }

    if host.is_empty() || host.len() > 253 || !host.is_ascii() {
        return false;
    }

    host.split('.').all(|label| {
        let bytes = label.as_bytes();

        !bytes.is_empty()
            && bytes.len() <= 63
            && bytes[0].is_ascii_alphanumeric()
            && bytes[bytes.len() - 1].is_ascii_alphanumeric()
            && bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
    })
}

impl Settings {
    /// Validate all settings before application startup.
    pub fn validate(&self) -> Result<(), SettingsError> {
        // Server host
        if !valid_host(&self.server.host) {
            return Err(invalid("SERVER_HOST", "a valid IP address or hostname"));
        }

        // Server port
        if self.server.port == 0 {
            return Err(invalid("SERVER_PORT", "an integer between 1 and 65535"));
        }

        // Shutdown timeout
        let shutdown = self.server.graceful_shutdown_timeout.as_secs();

        if !(1..=300).contains(&shutdown) {
            return Err(invalid(
                "SERVER_SHUTDOWN_TIMEOUT_SECS",
                "an integer between 1 and 300",
            ));
        }

        // PostgreSQL URL
        let database_url = self.database.url.expose_secret();

        let valid_scheme =
            database_url.starts_with("postgresql://") || database_url.starts_with("postgres://");

        if !valid_scheme || PgConnectOptions::from_str(database_url).is_err() {
            return Err(invalid("DATABASE_URL", "a valid PostgreSQL connection URL"));
        }

        // PostgreSQL maximum connections
        if !(1..=100).contains(&self.database.max_connections) {
            return Err(invalid(
                "DATABASE_MAX_CONNECTIONS",
                "an integer between 1 and 100",
            ));
        }

        // PostgreSQL minimum connections
        if self.database.min_connections == 0
            || self.database.min_connections > self.database.max_connections
        {
            return Err(invalid(
                "DATABASE_MIN_CONNECTIONS",
                "an integer between 1 and DATABASE_MAX_CONNECTIONS",
            ));
        }

        // PostgreSQL acquire timeout
        let acquire = self.database.acquire_timeout.as_secs();

        if !(1..=120).contains(&acquire) {
            return Err(invalid(
                "DATABASE_ACQUIRE_TIMEOUT_SECS",
                "an integer between 1 and 120",
            ));
        }

        // Redis URL
        let redis_url = self.redis.url.expose_secret();

        if redis::Client::open(redis_url).is_err() {
            return Err(invalid("REDIS_URL", "a valid Redis connection URL"));
        }

        // Redis timeout
        let redis_timeout = self.redis.operation_timeout.as_secs();

        if !(1..=30).contains(&redis_timeout) {
            return Err(invalid(
                "REDIS_OPERATION_TIMEOUT_SECS",
                "an integer between 1 and 30",
            ));
        }

        // Health timeout
        let health_timeout = self.health.check_timeout.as_secs();

        if !(1..=30).contains(&health_timeout) {
            return Err(invalid(
                "HEALTH_CHECK_TIMEOUT_SECS",
                "an integer between 1 and 30",
            ));
        }

        // Production wildcard binding policy
        let is_wildcard = self.server.host == "0.0.0.0" || self.server.host == "::";

        if self.environment == Environment::Production
            && is_wildcard
            && !self.security.allow_public_bind_in_production
        {
            return Err(invalid(
                "ALLOW_PUBLIC_BIND_IN_PRODUCTION",
                "true when binding production to all network interfaces",
            ));
        }

        Ok(())
    }
}
