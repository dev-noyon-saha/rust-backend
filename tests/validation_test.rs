use std::{collections::HashMap, env::VarError};

use backend_project::config::{Settings, SettingsError};

type TestEnv = HashMap<String, String>;

/// Create a valid test environment.
fn test_env() -> TestEnv {
    HashMap::from([
        (
            "DATABASE_URL".into(),
            "postgresql://dev:secret_pg@localhost:5434/dwar_ai".into(),
        ),
        (
            "REDIS_URL".into(),
            "redis://:secret_redis@localhost:6379/0".into(),
        ),
    ])
}

/// Load settings without changing global environment variables.
fn load(vars: &TestEnv) -> Result<Settings, SettingsError> {
    Settings::from_lookup(|name| vars.get(name).cloned().ok_or(VarError::NotPresent))
}

/// Assert that a particular variable is rejected.
fn assert_invalid(key: &str, value: &str, expected_name: &str) {
    let mut vars = test_env();

    vars.insert(key.into(), value.into());

    let error = load(&vars).expect_err("Expected validation to fail");

    match error {
        SettingsError::InvalidValue { name, .. } => {
            assert_eq!(name, expected_name);
        }

        other => panic!("Unexpected error: {other}"),
    }
}

// 01. Valid default settings
#[test]
fn accepts_valid_default_configuration() {
    let settings = load(&test_env()).unwrap();

    assert_eq!(settings.server.port, 8000);
    assert_eq!(settings.database.max_connections, 5);
    assert_eq!(settings.database.min_connections, 1);

    assert!(settings.validate().is_ok());
}

// 02. Valid DNS hostname
#[test]
fn accepts_valid_hostname() {
    let mut vars = test_env();

    vars.insert("SERVER_HOST".into(), "api.internal".into());

    let settings = load(&vars).unwrap();

    assert_eq!(settings.server.bind_address(), "api.internal:8000");
}

// 03. IPv6 server address
#[test]
fn accepts_ipv6_host() {
    let mut vars = test_env();

    vars.insert("SERVER_HOST".into(), "::1".into());

    let settings = load(&vars).unwrap();

    assert_eq!(settings.server.bind_address(), "[::1]:8000");
}

// 04. Empty server host
#[test]
fn rejects_empty_host() {
    assert_invalid("SERVER_HOST", " ", "SERVER_HOST");
}

// 05. Invalid server host
#[test]
fn rejects_invalid_host() {
    assert_invalid("SERVER_HOST", "http://localhost", "SERVER_HOST");
}

// 06. Invalid port
#[test]
fn rejects_zero_port() {
    assert_invalid("SERVER_PORT", "0", "SERVER_PORT");
}

// 07. Zero maximum pool size
#[test]
fn rejects_zero_max_connections() {
    assert_invalid("DATABASE_MAX_CONNECTIONS", "0", "DATABASE_MAX_CONNECTIONS");
}

// 08. Zero minimum pool size
#[test]
fn rejects_zero_min_connections() {
    assert_invalid("DATABASE_MIN_CONNECTIONS", "0", "DATABASE_MIN_CONNECTIONS");
}

// 09. Minimum pool size exceeds maximum
#[test]
fn rejects_min_greater_than_max() {
    let mut vars = test_env();

    vars.insert("DATABASE_MAX_CONNECTIONS".into(), "5".into());

    vars.insert("DATABASE_MIN_CONNECTIONS".into(), "10".into());

    let error = load(&vars).expect_err("Expected validation error");

    assert!(matches!(
        error,
        SettingsError::InvalidValue {
            name: "DATABASE_MIN_CONNECTIONS",
            ..
        }
    ));
}

// 10. Database acquire timeout
#[test]
fn rejects_zero_database_timeout() {
    assert_invalid(
        "DATABASE_ACQUIRE_TIMEOUT_SECS",
        "0",
        "DATABASE_ACQUIRE_TIMEOUT_SECS",
    );
}

// 11. Redis operation timeout
#[test]
fn rejects_zero_redis_timeout() {
    assert_invalid(
        "REDIS_OPERATION_TIMEOUT_SECS",
        "0",
        "REDIS_OPERATION_TIMEOUT_SECS",
    );
}

// 12. Health check timeout
#[test]
fn rejects_zero_health_timeout() {
    assert_invalid(
        "HEALTH_CHECK_TIMEOUT_SECS",
        "0",
        "HEALTH_CHECK_TIMEOUT_SECS",
    );
}

// 13. Zero graceful shutdown timeout
#[test]
fn rejects_zero_shutdown_timeout() {
    assert_invalid(
        "SERVER_SHUTDOWN_TIMEOUT_SECS",
        "0",
        "SERVER_SHUTDOWN_TIMEOUT_SECS",
    );
}

// 14. Excessively long shutdown timeout
#[test]
fn rejects_excessive_shutdown_timeout() {
    assert_invalid(
        "SERVER_SHUTDOWN_TIMEOUT_SECS",
        "301",
        "SERVER_SHUTDOWN_TIMEOUT_SECS",
    );
}

// 15. Invalid PostgreSQL URL
#[test]
fn rejects_invalid_postgres_url() {
    assert_invalid("DATABASE_URL", "http://localhost", "DATABASE_URL");
}

// 16. Invalid Redis URL
#[test]
fn rejects_invalid_redis_url() {
    assert_invalid("REDIS_URL", "http://localhost", "REDIS_URL");
}

// 17. Errors must not leak secrets
#[test]
fn validation_error_redacts_credentials() {
    let mut vars = test_env();

    vars.insert(
        "DATABASE_URL".into(),
        "http://user:top_secret@localhost".into(),
    );

    let error = load(&vars).expect_err("Expected validation error");

    let output = format!("{error:?}");

    assert!(!output.contains("top_secret"));
    assert!(output.contains("DATABASE_URL"));
}

// 18. Verify validation after configuration mutation
#[test]
fn rejects_modified_configuration_after_loading() {
    let mut settings = load(&test_env()).unwrap();

    // Initially valid.
    assert!(settings.validate().is_ok());

    // Simulate an invalid configuration mutation.
    settings.database.max_connections = 0;

    // Revalidation must reject the modified settings.
    let error = settings
        .validate()
        .expect_err("Expected configuration validation error");

    assert!(matches!(
        error,
        SettingsError::InvalidValue {
            name: "DATABASE_MAX_CONNECTIONS",
            ..
        }
    ));
}
