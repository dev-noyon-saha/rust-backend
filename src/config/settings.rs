use std::{
    env::{self, VarError},
    error::Error,
    fmt,
    str::FromStr,
    time::Duration,
};

use super::{
    database::DatabaseConfig,
    environment::{Environment, EnvironmentParseError},
    health::HealthConfig,
    redis::RedisConfig,
    security::SecurityConfig,
    server::ServerConfig,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentVariableFailure {
    Missing,
    NonUnicode,
}

impl From<VarError> for EnvironmentVariableFailure {
    fn from(error: VarError) -> Self {
        match error {
            VarError::NotPresent => Self::Missing,
            VarError::NotUnicode(_) => Self::NonUnicode,
        }
    }
}

/// Centralized typed configuration.
#[derive(Clone)]
pub struct Settings {
    pub environment: Environment,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub health: HealthConfig,
    pub security: SecurityConfig,
}

/// Settings Debug uses the redacted Debug implementations
/// of DatabaseConfig and RedisConfig.
impl fmt::Debug for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings")
            .field("environment", &self.environment)
            .field("server", &self.server)
            .field("database", &self.database)
            .field("redis", &self.redis)
            .field("health", &self.health)
            .field("security", &self.security)
            .finish()
    }
}

pub enum SettingsError {
    EnvironmentVariable {
        name: &'static str,
        reason: EnvironmentVariableFailure,
    },
    InvalidEnvironment(EnvironmentParseError),
    InvalidValue {
        name: &'static str,
        expected: &'static str,
    },
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnvironmentVariable { name, reason } => match reason {
                EnvironmentVariableFailure::Missing => {
                    write!(f, "Required environment variable {name} is missing")
                }
                EnvironmentVariableFailure::NonUnicode => {
                    write!(f, "Environment variable {name} is not valid Unicode")
                }
            },
            Self::InvalidEnvironment(error) => {
                write!(f, "{error}")
            }
            Self::InvalidValue { name, expected } => {
                write!(f, "Invalid configuration {name}: expected {expected}")
            }
        }
    }
}

impl fmt::Debug for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for SettingsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidEnvironment(error) => Some(error),
            Self::EnvironmentVariable { .. } | Self::InvalidValue { .. } => None,
        }
    }
}

impl From<EnvironmentParseError> for SettingsError {
    fn from(error: EnvironmentParseError) -> Self {
        Self::InvalidEnvironment(error)
    }
}

fn safe_var_error(name: &'static str, error: VarError) -> SettingsError {
    SettingsError::EnvironmentVariable {
        name,
        reason: error.into(),
    }
}

fn required<F>(lookup: &F, name: &'static str) -> Result<String, SettingsError>
where
    F: Fn(&str) -> Result<String, VarError>,
{
    lookup(name).map_err(|error| safe_var_error(name, error))
}

fn optional_string<F>(
    lookup: &F,
    name: &'static str,
    default: &str,
) -> Result<String, SettingsError>
where
    F: Fn(&str) -> Result<String, VarError>,
{
    match lookup(name) {
        Ok(value) => Ok(value),
        Err(VarError::NotPresent) => Ok(default.to_string()),
        Err(error) => Err(safe_var_error(name, error)),
    }
}

fn optional_parsed<T, F>(
    lookup: &F,
    name: &'static str,
    default: T,
    expected: &'static str,
) -> Result<T, SettingsError>
where
    T: FromStr,
    F: Fn(&str) -> Result<String, VarError>,
{
    match lookup(name) {
        Ok(value) => value
            .parse::<T>()
            .map_err(|_| SettingsError::InvalidValue { name, expected }),
        Err(VarError::NotPresent) => Ok(default),
        Err(error) => Err(safe_var_error(name, error)),
    }
}

impl Settings {
    pub fn from_env() -> Result<Self, SettingsError> {
        Self::from_lookup(|name| env::var(name))
    }

    pub fn from_lookup<F>(lookup: F) -> Result<Self, SettingsError>
    where
        F: Fn(&str) -> Result<String, VarError>,
    {
        // 1. Application environment
        let environment = match lookup("APP_ENV") {
            Ok(value) => Environment::from_optional_str(Some(&value))?,
            Err(VarError::NotPresent) => Environment::Development,
            Err(error) => {
                return Err(safe_var_error("APP_ENV", error));
            }
        };

        // 2. Required connection URLs
        let database_url = required(&lookup, "DATABASE_URL")?;

        let redis_url = required(&lookup, "REDIS_URL")?;

        // 3. HTTP server
        let mut server = ServerConfig::default();

        server.host = optional_string(&lookup, "SERVER_HOST", &server.host)?;

        server.port = optional_parsed(
            &lookup,
            "SERVER_PORT",
            server.port,
            "an integer between 1 and 65535",
        )?;

        let shutdown_seconds: u64 = optional_parsed(
            &lookup,
            "SERVER_SHUTDOWN_TIMEOUT_SECS",
            server.graceful_shutdown_timeout.as_secs(),
            "an integer between 1 and 300",
        )?;

        server.graceful_shutdown_timeout = Duration::from_secs(shutdown_seconds);

        // 4. PostgreSQL
        let mut database = DatabaseConfig::new(database_url);

        database.max_connections = optional_parsed(
            &lookup,
            "DATABASE_MAX_CONNECTIONS",
            database.max_connections,
            "an integer between 1 and 100",
        )?;

        database.min_connections = optional_parsed(
            &lookup,
            "DATABASE_MIN_CONNECTIONS",
            database.min_connections,
            "an integer between 1 and DATABASE_MAX_CONNECTIONS",
        )?;

        let acquire_seconds: u64 = optional_parsed(
            &lookup,
            "DATABASE_ACQUIRE_TIMEOUT_SECS",
            database.acquire_timeout.as_secs(),
            "an integer between 1 and 120",
        )?;

        database.acquire_timeout = Duration::from_secs(acquire_seconds);

        // 5. Redis
        let mut redis = RedisConfig::new(redis_url);

        let redis_timeout_seconds: u64 = optional_parsed(
            &lookup,
            "REDIS_OPERATION_TIMEOUT_SECS",
            redis.operation_timeout.as_secs(),
            "an integer between 1 and 30",
        )?;

        redis.operation_timeout = Duration::from_secs(redis_timeout_seconds);

        // 6. Health checks
        let mut health = HealthConfig::default();

        let health_timeout_seconds: u64 = optional_parsed(
            &lookup,
            "HEALTH_CHECK_TIMEOUT_SECS",
            health.check_timeout.as_secs(),
            "an integer between 1 and 30",
        )?;

        health.check_timeout = Duration::from_secs(health_timeout_seconds);

        // 7. Security
        let security = SecurityConfig {
            allow_public_bind_in_production: optional_parsed(
                &lookup,
                "ALLOW_PUBLIC_BIND_IN_PRODUCTION",
                false,
                "true or false",
            )?,
        };

        // 8. Build and validate
        let settings = Self {
            environment,
            server,
            database,
            redis,
            health,
            security,
        };

        settings.validate()?;

        Ok(settings)
    }
}
