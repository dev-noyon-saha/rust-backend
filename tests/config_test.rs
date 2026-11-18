use std::{collections::HashMap, env::VarError, time::Duration};

use backend_project::config::{Environment, Settings, SettingsError};

type TestEnv = HashMap<String, String>;

/// Create the minimum required test environment.
fn test_env() -> TestEnv {
    HashMap::from([
        (
            "DATABASE_URL".to_string(),
            "postgresql://dev:secret_pg@localhost:5434/dwar_ai".to_string(),
        ),
        (
            "REDIS_URL".to_string(),
            "redis://:secret_redis@localhost:6379/0".to_string(),
        ),
    ])
}

/// Load settings without modifying process environment.
fn load(vars: &TestEnv) -> Result<Settings, SettingsError> {
    Settings::from_lookup(|name| vars.get(name).cloned().ok_or(VarError::NotPresent))
}

// Test 01
#[test]
fn default_configuration_values_are_correct() {
    let settings = load(&test_env()).unwrap();

    assert_eq!(settings.environment, Environment::Development);

    assert_eq!(settings.server.host, "127.0.0.1");
    assert_eq!(settings.server.port, 8000);

    assert_eq!(
        settings.server.graceful_shutdown_timeout,
        Duration::from_secs(30)
    );

    assert_eq!(settings.database.max_connections, 5);
    assert_eq!(settings.database.min_connections, 1);

    assert_eq!(settings.database.acquire_timeout, Duration::from_secs(5));

    assert_eq!(settings.redis.operation_timeout, Duration::from_secs(2));

    assert_eq!(settings.health.check_timeout, Duration::from_secs(2));
}

// Test 02
#[test]
fn custom_configuration_values_are_loaded() {
    let mut vars = test_env();

    vars.insert("SERVER_HOST".into(), "0.0.0.0".into());
    vars.insert("SERVER_PORT".into(), "9000".into());

    vars.insert("SERVER_SHUTDOWN_TIMEOUT_SECS".into(), "45".into());

    vars.insert("DATABASE_MAX_CONNECTIONS".into(), "20".into());

    vars.insert("DATABASE_MIN_CONNECTIONS".into(), "3".into());

    vars.insert("DATABASE_ACQUIRE_TIMEOUT_SECS".into(), "10".into());

    vars.insert("REDIS_OPERATION_TIMEOUT_SECS".into(), "4".into());

    vars.insert("HEALTH_CHECK_TIMEOUT_SECS".into(), "3".into());

    let settings = load(&vars).unwrap();

    assert_eq!(settings.server.host, "0.0.0.0");
    assert_eq!(settings.server.port, 9000);

    assert_eq!(settings.server.bind_address(), "0.0.0.0:9000");

    assert_eq!(
        settings.server.graceful_shutdown_timeout,
        Duration::from_secs(45)
    );

    assert_eq!(settings.database.max_connections, 20);
    assert_eq!(settings.database.min_connections, 3);

    assert_eq!(settings.database.acquire_timeout, Duration::from_secs(10));

    assert_eq!(settings.redis.operation_timeout, Duration::from_secs(4));

    assert_eq!(settings.health.check_timeout, Duration::from_secs(3));
}

// Test 03
#[test]
fn production_environment_is_supported() {
    let mut vars = test_env();

    vars.insert("APP_ENV".into(), "production".into());

    let settings = load(&vars).unwrap();

    assert_eq!(settings.environment, Environment::Production);

    assert!(settings.environment.is_production());
}

// Test 04
#[test]
fn missing_database_url_is_rejected() {
    let mut vars = test_env();

    vars.remove("DATABASE_URL");

    let error = load(&vars).expect_err("Expected an error");

    assert!(matches!(
        error,
        SettingsError::EnvironmentVariable {
            name: "DATABASE_URL",
            ..
        }
    ));
}

// Test 05
#[test]
fn missing_redis_url_is_rejected() {
    let mut vars = test_env();

    vars.remove("REDIS_URL");

    let error = load(&vars).expect_err("Expected an error");

    assert!(matches!(
        error,
        SettingsError::EnvironmentVariable {
            name: "REDIS_URL",
            ..
        }
    ));
}

// Test 06
#[test]
fn invalid_environment_is_rejected() {
    let mut vars = test_env();

    vars.insert("APP_ENV".into(), "invalid".into());

    let error = load(&vars).expect_err("Expected an error");

    assert!(matches!(error, SettingsError::InvalidEnvironment(_)));
}

// Test 07
#[test]
fn invalid_server_port_is_rejected() {
    let mut vars = test_env();

    vars.insert("SERVER_PORT".into(), "not-a-port".into());

    let error = load(&vars).expect_err("Expected an error");

    assert!(matches!(
        error,
        SettingsError::InvalidValue {
            name: "SERVER_PORT",
            ..
        }
    ));
}

// Test 08
#[test]
fn port_overflow_is_rejected() {
    let mut vars = test_env();

    vars.insert("SERVER_PORT".into(), "70000".into());

    assert!(load(&vars).is_err());
}

// Test 09
#[test]
fn invalid_database_pool_number_is_rejected() {
    let mut vars = test_env();

    vars.insert("DATABASE_MAX_CONNECTIONS".into(), "many".into());

    let error = load(&vars).expect_err("Expected an error");

    assert!(matches!(
        error,
        SettingsError::InvalidValue {
            name: "DATABASE_MAX_CONNECTIONS",
            ..
        }
    ));
}

// Test 10
#[test]
fn invalid_timeout_is_rejected() {
    let mut vars = test_env();

    vars.insert("HEALTH_CHECK_TIMEOUT_SECS".into(), "-5".into());

    assert!(load(&vars).is_err());
}

// Test 11
#[test]
fn debug_output_does_not_expose_credentials() {
    let settings = load(&test_env()).unwrap();

    let database_debug = format!("{:?}", settings.database);

    let redis_debug = format!("{:?}", settings.redis);

    assert!(!database_debug.contains("secret_pg"));
    assert!(!redis_debug.contains("secret_redis"));

    assert!(database_debug.contains("[REDACTED]"));
    assert!(redis_debug.contains("[REDACTED]"));
}
