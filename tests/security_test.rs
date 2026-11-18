use std::{collections::HashMap, env::VarError, error::Error, ffi::OsString};

use backend_project::config::{EnvironmentVariableFailure, SecretString, Settings, SettingsError};

type TestEnv = HashMap<String, String>;

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

fn load(vars: &TestEnv) -> Result<Settings, SettingsError> {
    Settings::from_lookup(|name| vars.get(name).cloned().ok_or(VarError::NotPresent))
}

// Test 01
#[test]
fn secret_string_debug_is_redacted() {
    let secret = SecretString::new("my_private_password");

    let output = format!("{secret:?}");

    assert!(output.contains("[REDACTED]"));
    assert!(!output.contains("my_private_password"));
}

// Test 02
#[test]
fn secret_string_display_is_redacted() {
    let secret = SecretString::new("very_secret_token");

    assert_eq!(secret.to_string(), "[REDACTED]");
}

// Test 03
#[test]
fn secret_requires_explicit_access() {
    let secret = SecretString::new("test_password");

    assert_eq!(secret.expose_secret(), "test_password");
}

// Test 04
#[test]
fn database_debug_does_not_leak_password() {
    let settings = load(&test_env()).unwrap();

    let output = format!("{:?}", settings.database);

    assert!(!output.contains("secret_pg"));
    assert!(output.contains("[REDACTED]"));
}

// Test 05
#[test]
fn redis_debug_does_not_leak_password() {
    let settings = load(&test_env()).unwrap();

    let output = format!("{:?}", settings.redis);

    assert!(!output.contains("secret_redis"));
    assert!(output.contains("[REDACTED]"));
}

// Test 06
#[test]
fn complete_settings_debug_is_safe() {
    let settings = load(&test_env()).unwrap();

    let output = format!("{settings:?}");

    assert!(!output.contains("secret_pg"));
    assert!(!output.contains("secret_redis"));

    assert!(output.contains("[REDACTED]"));
}

// Test 07
#[test]
fn invalid_environment_error_does_not_leak_value() {
    let mut vars = test_env();

    vars.insert("APP_ENV".into(), "private_environment_secret".into());

    let error = load(&vars).expect_err("Expected invalid environment error");

    let output = format!("{error:?} {error}");

    assert!(!output.contains("private_environment_secret"));

    assert!(output.contains("APP_ENV"));
}

// Test 08
#[test]
fn non_unicode_environment_error_is_sanitized() {
    let vars = test_env();

    let error = Settings::from_lookup(|name| {
        if name == "DATABASE_URL" {
            return Err(VarError::NotUnicode(OsString::from(
                "sensitive_database_secret",
            )));
        }

        vars.get(name).cloned().ok_or(VarError::NotPresent)
    })
    .expect_err("Expected environment error");

    let output = format!("{error:?} {error}");

    assert!(!output.contains("sensitive_database_secret"));
    assert!(output.contains("DATABASE_URL"));

    assert!(matches!(
        error,
        SettingsError::EnvironmentVariable {
            name: "DATABASE_URL",
            reason: EnvironmentVariableFailure::NonUnicode,
        }
    ));

    // Raw VarError is not exposed through source().
    assert!(error.source().is_none());
}

// Test 09
#[test]
fn validation_error_does_not_leak_url_password() {
    let mut vars = test_env();

    vars.insert(
        "DATABASE_URL".into(),
        "http://user:top_secret@localhost".into(),
    );

    let error = load(&vars).expect_err("Expected invalid URL error");

    let output = format!("{error:?} {error}");

    assert!(!output.contains("top_secret"));
    assert!(output.contains("DATABASE_URL"));
}

// Test 10
#[test]
fn production_rejects_public_ipv4_bind_by_default() {
    let mut vars = test_env();

    vars.insert("APP_ENV".into(), "production".into());
    vars.insert("SERVER_HOST".into(), "0.0.0.0".into());

    let error = load(&vars).expect_err("Expected production security error");

    assert!(matches!(
        error,
        SettingsError::InvalidValue {
            name: "ALLOW_PUBLIC_BIND_IN_PRODUCTION",
            ..
        }
    ));
}

// Test 11
#[test]
fn production_rejects_public_ipv6_bind_by_default() {
    let mut vars = test_env();

    vars.insert("APP_ENV".into(), "production".into());
    vars.insert("SERVER_HOST".into(), "::".into());

    assert!(load(&vars).is_err());
}

// Test 12
#[test]
fn production_allows_explicit_public_bind() {
    let mut vars = test_env();

    vars.insert("APP_ENV".into(), "production".into());
    vars.insert("SERVER_HOST".into(), "0.0.0.0".into());

    vars.insert("ALLOW_PUBLIC_BIND_IN_PRODUCTION".into(), "true".into());

    let settings = load(&vars).unwrap();

    assert!(settings.security.allow_public_bind_in_production);

    assert_eq!(settings.server.bind_address(), "0.0.0.0:8000");
}

// Test 13
#[test]
fn development_can_bind_public_interfaces() {
    let mut vars = test_env();

    vars.insert("APP_ENV".into(), "development".into());
    vars.insert("SERVER_HOST".into(), "0.0.0.0".into());

    let settings = load(&vars).unwrap();

    assert_eq!(settings.server.host, "0.0.0.0");
}

// Test 14
#[test]
fn invalid_security_boolean_is_rejected() {
    let mut vars = test_env();

    vars.insert("ALLOW_PUBLIC_BIND_IN_PRODUCTION".into(), "yes".into());

    let error = load(&vars).expect_err("Expected boolean parsing error");

    assert!(matches!(
        error,
        SettingsError::InvalidValue {
            name: "ALLOW_PUBLIC_BIND_IN_PRODUCTION",
            ..
        }
    ));
}
