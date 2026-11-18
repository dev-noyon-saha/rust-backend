/// Configuration for production security policies.
#[derive(Debug, Clone, Default)]
pub struct SecurityConfig {
    /// Allow binding production HTTP server to all interfaces.
    ///
    /// Default: false.
    ///
    /// Set true explicitly when deploying behind a properly
    /// configured reverse proxy, firewall or container network.
    pub allow_public_bind_in_production: bool,
}
