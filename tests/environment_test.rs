use backend_project::config::Environment;

// 1. Parse development
#[test]
fn development_environment_parses() {
    let environment: Environment = "development".parse().unwrap();

    assert_eq!(environment, Environment::Development);
}

// 2. Parse testing
#[test]
fn testing_environment_parses() {
    let environment: Environment = "testing".parse().unwrap();

    assert_eq!(environment, Environment::Testing);
}

// 3. Parse production
#[test]
fn production_environment_parses() {
    let environment: Environment = "production".parse().unwrap();

    assert_eq!(environment, Environment::Production);
}

// 4. Accept whitespace and uppercase
#[test]
fn environment_parsing_is_case_insensitive() {
    let environment: Environment = "  PRODUCTION  ".parse().unwrap();

    assert_eq!(environment, Environment::Production);
}

// 5. Default when APP_ENV is missing
#[test]
fn missing_environment_defaults_to_development() {
    let environment = Environment::from_optional_str(None).unwrap();

    assert_eq!(environment, Environment::Development);
}

// 6. Reject unsupported values
#[test]
fn invalid_environment_is_rejected() {
    let result = "staging".parse::<Environment>();

    assert!(result.is_err());
}

// 7. Reject empty values
#[test]
fn empty_environment_is_rejected() {
    let result = "   ".parse::<Environment>();

    assert!(result.is_err());
}

// 8. Verify environment helpers
#[test]
fn environment_helpers_work() {
    assert_eq!(Environment::Development.as_str(), "development");

    assert_eq!(Environment::Testing.to_string(), "testing");

    assert!(Environment::Production.is_production());

    assert!(!Environment::Development.is_production());
}
