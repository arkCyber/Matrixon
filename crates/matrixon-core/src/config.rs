//! Configuration module for Matrixon
//! 
//! This module defines the configuration structures used throughout the Matrixon system.
//! These configurations are used to customize the behavior of various components
//! and can be loaded from files or environment variables.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::Result;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server name (e.g., matrixon.local)
    pub server_name: String,
    
    /// Server port
    pub port: u16,
    
    /// TLS configuration
    pub tls: Option<TlsConfig>,
    
    /// Federation configuration
    pub federation: FederationConfig,
    
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
    
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    
    /// Cache configuration
    pub cache: CacheConfig,
    
    /// Metrics configuration
    pub metrics: MetricsConfig,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file
    pub cert_path: PathBuf,
    
    /// Path to private key file
    pub key_path: PathBuf,
    
    /// Minimum TLS version
    pub min_version: String,
    
    /// Cipher suites
    pub cipher_suites: Vec<String>,
}

/// Federation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    /// Whether federation is enabled
    pub enabled: bool,
    
    /// List of trusted servers
    pub trusted_servers: Vec<String>,
    
    /// Federation timeout in seconds
    pub timeout: u64,
    
    /// Maximum number of concurrent federation requests
    pub max_concurrent: u32,
    
    /// Whether to verify TLS certificates
    pub verify_tls: bool,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    
    /// Maximum number of connections
    pub max_connections: u32,
    
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    
    /// Minimum number of idle connections
    pub min_idle: u32,
    
    /// Maximum lifetime of connections in seconds
    pub max_lifetime: u64,
    
    /// Whether to use prepared statements
    pub use_prepared_statements: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    
    /// Log file path
    pub file_path: Option<PathBuf>,
    
    /// Whether to log to console
    pub console: bool,
    
    /// Log format
    pub format: String,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled
    pub enabled: bool,
    
    /// Requests per second
    pub requests_per_second: u32,
    
    /// Burst size
    pub burst_size: u32,
    
    /// Rate limit window in seconds
    pub window: u64,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,
    
    /// Cache size in bytes
    pub size: u64,
    
    /// Cache TTL in seconds
    pub ttl: u64,
    
    /// Cache type (memory, redis, etc.)
    pub cache_type: String,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether metrics are enabled
    pub enabled: bool,
    
    /// Metrics port
    pub port: u16,
    
    /// Metrics path
    pub path: String,
    
    /// Metrics interval in seconds
    pub interval: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_name: "matrixon.local".to_string(),
            port: 8448,
            tls: None,
            federation: FederationConfig::default(),
            database: DatabaseConfig::default(),
            logging: LoggingConfig::default(),
            rate_limit: RateLimitConfig::default(),
            cache: CacheConfig::default(),
            metrics: MetricsConfig::default(),
        }
    }
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trusted_servers: Vec::new(),
            timeout: 30,
            max_concurrent: 100,
            verify_tls: true,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://matrixon:matrixon@localhost/matrixon".to_string(),
            max_connections: 100,
            connection_timeout: 30,
            min_idle: 10,
            max_lifetime: 3600,
            use_prepared_statements: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: None,
            console: true,
            format: "json".to_string(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second: 100,
            burst_size: 200,
            window: 60,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            size: 1024 * 1024 * 100, // 100MB
            ttl: 3600,
            cache_type: "memory".to_string(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9090,
            path: "/metrics".to_string(),
            interval: 15,
        }
    }
}

impl ServerConfig {
    /// Load configuration from a file
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| crate::MatrixonError::Config(format!("Failed to read config file: {}", e)))?;
        serde_json::from_str(&contents)
            .map_err(|e| crate::MatrixonError::Config(format!("Failed to parse config file: {}", e)))
    }

    /// Save configuration to a file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| crate::MatrixonError::Config(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(path, contents)
            .map_err(|e| crate::MatrixonError::Config(format!("Failed to write config file: {}", e)))
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.server_name.is_empty() {
            return Err(crate::MatrixonError::Config("Server name cannot be empty".into()));
        }
        if self.port == 0 {
            return Err(crate::MatrixonError::Config("Port cannot be 0".into()));
        }
        if let Some(tls) = &self.tls {
            if !tls.cert_path.exists() {
                return Err(crate::MatrixonError::Config("TLS certificate file does not exist".into()));
            }
            if !tls.key_path.exists() {
                return Err(crate::MatrixonError::Config("TLS key file does not exist".into()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.server_name, "matrixon.local");
        assert_eq!(config.port, 8448);
        assert!(config.tls.is_none());
        assert!(config.federation.enabled);
        assert!(config.logging.console);
        assert!(config.rate_limit.enabled);
        assert!(config.cache.enabled);
        assert!(config.metrics.enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = ServerConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: ServerConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config.server_name, deserialized.server_name);
    }

    #[test]
    fn test_config_file_operations() {
        let config = ServerConfig::default();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Test saving
        config.save_to_file(&path).unwrap();

        // Test loading
        let loaded = ServerConfig::from_file(&path).unwrap();
        assert_eq!(config.server_name, loaded.server_name);

        // Cleanup
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_config_validation() {
        let mut config = ServerConfig::default();
        assert!(config.validate().is_ok());

        config.server_name = String::new();
        assert!(config.validate().is_err());

        config = ServerConfig::default();
        config.port = 0;
        assert!(config.validate().is_err());
    }
} 
