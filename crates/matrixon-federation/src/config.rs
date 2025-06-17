// =============================================================================
// Matrixon Federation - Config Module
// =============================================================================
//
// Author: arkSong <arksong2018@gmail.com>
// Version: 0.11.0-alpha
// Date: 2024-03-21
//
// This module defines the configuration structures and validation logic
// for the Matrixon federation implementation.
//
// =============================================================================

use std::{
    net::SocketAddr,
    path::PathBuf,
    time::Duration,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

use crate::error::{Result, FederationError};

/// Federation server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    /// Server name for this Matrixon instance
    pub server_name: String,
    
    /// Whether federation is enabled
    pub enabled: bool,
    
    /// Maximum number of concurrent federation connections
    pub max_connections: usize,
    
    /// Log level for federation operations
    pub log_level: String,
    
    /// Server start time
    pub start_time: u64,
    
    /// Federation timeout in milliseconds
    pub timeout_ms: u64,
    
    /// Maximum retry attempts for failed requests
    pub max_retries: u32,
    
    /// Whether to verify SSL certificates
    pub verify_ssl: bool,
    
    /// Server bind address
    pub bind_address: SocketAddr,
    
    /// Path to server signing key
    pub signing_key_path: PathBuf,
    
    /// Path to server certificate
    pub cert_path: Option<PathBuf>,
    
    /// Path to server private key
    pub key_path: Option<PathBuf>,
    
    /// Trusted server list
    pub trusted_servers: Vec<String>,
    
    /// Blocked server list
    pub blocked_servers: Vec<String>,
    
    /// Maximum event size in bytes
    pub max_event_size: usize,
    
    /// Maximum state events per room
    pub max_state_events: usize,
    
    /// Maximum events per request
    pub max_events_per_request: usize,
    
    /// Maximum concurrent requests per server
    pub max_concurrent_requests: usize,
    
    /// Request rate limit per second
    pub rate_limit_per_second: u32,
    
    /// Request rate limit burst
    pub rate_limit_burst: u32,
    
    /// Whether to enable debug mode
    pub debug: bool,
    
    /// Whether to enable metrics collection
    pub metrics_enabled: bool,
    
    /// Metrics collection interval in seconds
    pub metrics_interval: u64,
    
    /// Whether to enable health checks
    pub health_checks_enabled: bool,
    
    /// Health check interval in seconds
    pub health_check_interval: u64,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            server_name: "localhost".to_string(),
            enabled: true,
            max_connections: 1000,
            log_level: "info".to_string(),
            start_time: chrono::Utc::now().timestamp_millis() as u64,
            timeout_ms: 5000,
            max_retries: 3,
            verify_ssl: true,
            bind_address: "0.0.0.0:8448".parse().unwrap(),
            signing_key_path: PathBuf::from("signing.key"),
            cert_path: None,
            key_path: None,
            trusted_servers: vec![],
            blocked_servers: vec![],
            max_event_size: 1024 * 1024, // 1MB
            max_state_events: 1000,
            max_events_per_request: 100,
            max_concurrent_requests: 50,
            rate_limit_per_second: 100,
            rate_limit_burst: 200,
            debug: false,
            metrics_enabled: true,
            metrics_interval: 60,
            health_checks_enabled: true,
            health_check_interval: 30,
        }
    }
}

impl FederationConfig {
    /// Creates a new federation configuration
    #[instrument(level = "debug")]
    pub fn new(server_name: String) -> Self {
        let mut config = Self::default();
        config.server_name = server_name;
        config
    }
    
    /// Validates the configuration
    #[instrument(level = "debug")]
    pub fn validate(&self) -> Result<()> {
        if self.server_name.is_empty() {
            return Err(FederationError::InvalidConfig("Server name cannot be empty".into()));
        }
        
        if self.max_connections == 0 {
            return Err(FederationError::InvalidConfig("Max connections must be greater than 0".into()));
        }
        
        if self.timeout_ms == 0 {
            return Err(FederationError::InvalidConfig("Timeout must be greater than 0".into()));
        }
        
        if self.max_retries == 0 {
            return Err(FederationError::InvalidConfig("Max retries must be greater than 0".into()));
        }
        
        if !self.signing_key_path.exists() {
            return Err(FederationError::InvalidConfig("Signing key file does not exist".into()));
        }
        
        if let Some(cert_path) = &self.cert_path {
            if !cert_path.exists() {
                return Err(FederationError::InvalidConfig("Certificate file does not exist".into()));
            }
        }
        
        if let Some(key_path) = &self.key_path {
            if !key_path.exists() {
                return Err(FederationError::InvalidConfig("Private key file does not exist".into()));
            }
        }
        
        if self.max_event_size == 0 {
            return Err(FederationError::InvalidConfig("Max event size must be greater than 0".into()));
        }
        
        if self.max_state_events == 0 {
            return Err(FederationError::InvalidConfig("Max state events must be greater than 0".into()));
        }
        
        if self.max_events_per_request == 0 {
            return Err(FederationError::InvalidConfig("Max events per request must be greater than 0".into()));
        }
        
        if self.max_concurrent_requests == 0 {
            return Err(FederationError::InvalidConfig("Max concurrent requests must be greater than 0".into()));
        }
        
        if self.rate_limit_per_second == 0 {
            return Err(FederationError::InvalidConfig("Rate limit must be greater than 0".into()));
        }
        
        if self.rate_limit_burst == 0 {
            return Err(FederationError::InvalidConfig("Rate limit burst must be greater than 0".into()));
        }
        
        if self.metrics_enabled && self.metrics_interval == 0 {
            return Err(FederationError::InvalidConfig("Metrics interval must be greater than 0".into()));
        }
        
        if self.health_checks_enabled && self.health_check_interval == 0 {
            return Err(FederationError::InvalidConfig("Health check interval must be greater than 0".into()));
        }
        
        Ok(())
    }
    
    /// Gets the timeout duration
    #[instrument(level = "debug")]
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
    
    /// Checks if a server is trusted
    #[instrument(level = "debug")]
    pub fn is_trusted(&self, server_name: &str) -> bool {
        self.trusted_servers.contains(&server_name.to_string())
    }
    
    /// Checks if a server is blocked
    #[instrument(level = "debug")]
    pub fn is_blocked(&self, server_name: &str) -> bool {
        self.blocked_servers.contains(&server_name.to_string())
    }
    
    /// Adds a trusted server
    #[instrument(level = "debug")]
    pub fn add_trusted_server(&mut self, server_name: String) {
        if !self.trusted_servers.contains(&server_name) {
            self.trusted_servers.push(server_name);
        }
    }
    
    /// Removes a trusted server
    #[instrument(level = "debug")]
    pub fn remove_trusted_server(&mut self, server_name: &str) {
        self.trusted_servers.retain(|s| s != server_name);
    }
    
    /// Adds a blocked server
    #[instrument(level = "debug")]
    pub fn add_blocked_server(&mut self, server_name: String) {
        if !self.blocked_servers.contains(&server_name) {
            self.blocked_servers.push(server_name);
        }
    }
    
    /// Removes a blocked server
    #[instrument(level = "debug")]
    pub fn remove_blocked_server(&mut self, server_name: &str) {
        self.blocked_servers.retain(|s| s != server_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_default_config() {
        let config = FederationConfig::default();
        assert_eq!(config.server_name, "localhost");
        assert!(config.enabled);
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.log_level, "info");
        assert!(config.start_time > 0);
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.max_retries, 3);
        assert!(config.verify_ssl);
    }
    
    #[test]
    fn test_new_config() {
        let config = FederationConfig::new("example.com".to_string());
        assert_eq!(config.server_name, "example.com");
        assert!(config.enabled);
        assert_eq!(config.max_connections, 1000);
    }
    
    #[test]
    fn test_validate_config() {
        let mut config = FederationConfig::default();
        assert!(config.validate().is_ok());
        
        config.server_name = String::new();
        assert!(config.validate().is_err());
        
        config = FederationConfig::default();
        config.max_connections = 0;
        assert!(config.validate().is_err());
        
        config = FederationConfig::default();
        config.timeout_ms = 0;
        assert!(config.validate().is_err());
        
        config = FederationConfig::default();
        config.max_retries = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_timeout() {
        let config = FederationConfig::default();
        assert_eq!(config.timeout(), Duration::from_millis(5000));
    }
    
    #[test]
    fn test_trusted_servers() {
        let mut config = FederationConfig::default();
        
        assert!(!config.is_trusted("example.com"));
        
        config.add_trusted_server("example.com".to_string());
        assert!(config.is_trusted("example.com"));
        
        config.remove_trusted_server("example.com");
        assert!(!config.is_trusted("example.com"));
    }
    
    #[test]
    fn test_blocked_servers() {
        let mut config = FederationConfig::default();
        
        assert!(!config.is_blocked("example.com"));
        
        config.add_blocked_server("example.com".to_string());
        assert!(config.is_blocked("example.com"));
        
        config.remove_blocked_server("example.com");
        assert!(!config.is_blocked("example.com"));
    }
    
    #[test]
    fn test_signing_key_validation() {
        let mut config = FederationConfig::default();
        
        // Create a temporary signing key file
        let temp_file = NamedTempFile::new().unwrap();
        config.signing_key_path = temp_file.path().to_path_buf();
        
        assert!(config.validate().is_ok());
        
        // Remove the file and check validation
        drop(temp_file);
        assert!(config.validate().is_err());
    }
} 
