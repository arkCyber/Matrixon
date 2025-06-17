//! Matrixon API Library
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.11.0-alpha
//! Date: 2024-03-21
//! 
//! This library provides the API layer for Matrixon, implementing the Matrix
//! Client-Server API specification.


pub mod client;
pub mod server;
pub mod handlers;
pub mod middleware;
pub mod routes;

/// API configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Server host
    pub host: String,
    
    /// Server port
    pub port: u16,
    
    /// TLS configuration
    pub tls: Option<TlsConfig>,
    
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
}

/// TLS configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Certificate path
    pub cert_path: String,
    
    /// Key path
    pub key_path: String,
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub max_requests: u32,
    
    /// Burst size
    pub burst_size: u32,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8448,
            tls: None,
            rate_limit: RateLimitConfig::default(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            burst_size: 200,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8448);
        assert!(config.tls.is_none());
        assert_eq!(config.rate_limit.max_requests, 100);
    }
}
