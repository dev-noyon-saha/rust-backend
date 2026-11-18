use std::{net::IpAddr, time::Duration};

/// HTTP server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub graceful_shutdown_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8000,
            graceful_shutdown_timeout: Duration::from_secs(30),
        }
    }
}

impl ServerConfig {
    /// Return a valid bind address for IPv4, IPv6,
    /// or DNS-style hostnames.
    pub fn bind_address(&self) -> String {
        match self.host.parse::<IpAddr>() {
            Ok(IpAddr::V6(_)) => {
                format!("[{}]:{}", self.host, self.port)
            }
            _ => {
                format!("{}:{}", self.host, self.port)
            }
        }
    }
}
