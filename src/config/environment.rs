use std::{error::Error, fmt, str::FromStr};

/// Supported application environments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Testing,
    Production,
}

impl Environment {
    /// Return the canonical environment name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Testing => "testing",
            Self::Production => "production",
        }
    }

    /// Check whether the application is in production.
    pub const fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }

    /// Parse an optional environment value.
    ///
    /// If APP_ENV is absent, use development to preserve
    /// backward compatibility with existing local setups.
    pub fn from_optional_str(value: Option<&str>) -> Result<Self, EnvironmentParseError> {
        match value {
            Some(value) => value.parse(),
            None => Ok(Self::Development),
        }
    }
}

/// Error for unsupported environment values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentParseError;

impl fmt::Display for EnvironmentParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid APP_ENV. Expected: development, testing, or production"
        )
    }
}

impl Error for EnvironmentParseError {}

impl FromStr for Environment {
    type Err = EnvironmentParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "development" => Ok(Self::Development),
            "testing" => Ok(Self::Testing),
            "production" => Ok(Self::Production),
            _ => Err(EnvironmentParseError),
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
