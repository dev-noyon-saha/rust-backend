use std::fmt;

/// Placeholder used when displaying sensitive values.
pub const REDACTED: &str = "[REDACTED]";

/// Wrapper for sensitive configuration values.
///
/// Debug and Display never reveal the underlying value.
/// Access to the original string must be explicit.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString {
    value: String,
}

impl SecretString {
    /// Create a new secret.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Explicitly access the underlying secret.
    ///
    /// Never log or serialize this returned value.
    pub fn expose_secret(&self) -> &str {
        &self.value
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString([REDACTED])")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
