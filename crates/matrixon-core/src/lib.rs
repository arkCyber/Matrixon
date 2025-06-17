//! Matrixon Core Library
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.11.0-alpha
//! Date: 2024-03-21
//! 
//! This is the core library for Matrixon, providing fundamental types, traits,
//! and utilities used throughout the Matrixon ecosystem.
//! 
//! # Features
//! - Core Matrix protocol types and traits
//! - Error handling and logging
//! - Configuration management
//! - Utility functions
//! 
//! # Examples
//! ```rust
//! use matrixon_core::{MatrixonConfig, Result};
//! 
//! async fn example() -> Result<()> {
//!     let config = MatrixonConfig::default();
//!     // Use the configuration...
//!     Ok(())
//! }
//! ```

use std::time::Instant;
use tracing::{debug, info, instrument, Level};
use chrono::{DateTime, Utc};

pub mod types;
pub mod traits;
pub mod utils;
pub mod error;
pub mod config;

pub use error::{MatrixonError, Result};

/// Core configuration for Matrixon
#[derive(Debug, Clone)]
pub struct MatrixonConfig {
    /// The server name for this Matrixon instance
    pub server_name: String,
    /// Database connection URL
    pub database_url: String,
    /// Whether federation is enabled
    pub federation_enabled: bool,
    /// Whether end-to-end encryption is enabled
    pub e2ee_enabled: bool,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Log level for the application
    pub log_level: Level,
    /// Server start time
    pub start_time: DateTime<Utc>,
}

impl Default for MatrixonConfig {
    fn default() -> Self {
        Self {
            server_name: "matrixon.local".to_string(),
            database_url: "postgres://matrixon:matrixon@localhost/matrixon".to_string(),
            federation_enabled: true,
            e2ee_enabled: true,
            max_connections: 200_000,
            log_level: Level::INFO,
            start_time: Utc::now(),
        }
    }
}

impl MatrixonConfig {
    /// Creates a new configuration with the given server name
    #[instrument(level = "debug")]
    pub fn new<T: Into<String> + std::fmt::Debug>(server_name: T) -> Self {
        let start = Instant::now();
        debug!("🔧 Creating new MatrixonConfig");
        
        let config = Self {
            server_name: server_name.into(),
            ..Default::default()
        };
        
        info!("✅ Created new config in {:?}", start.elapsed());
        config
    }

    /// Validates the configuration
    #[instrument(level = "debug")]
    pub fn validate(&self) -> Result<()> {
        let start = Instant::now();
        debug!("🔧 Validating MatrixonConfig");
        
        if self.server_name.is_empty() {
            return Err(MatrixonError::InvalidConfig("Server name cannot be empty".into()));
        }
        
        if self.database_url.is_empty() {
            return Err(MatrixonError::InvalidConfig("Database URL cannot be empty".into()));
        }
        
        info!("✅ Config validation completed in {:?}", start.elapsed());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_config_default() {
        let config = MatrixonConfig::default();
        assert_eq!(config.server_name, "matrixon.local");
        assert!(config.federation_enabled);
        assert!(config.e2ee_enabled);
        assert_eq!(config.max_connections, 200_000);
    }

    #[test]
    fn test_config_new() {
        let config = MatrixonConfig::new("test.server");
        assert_eq!(config.server_name, "test.server");
    }

    #[test]
    fn test_config_validation() {
        let mut config = MatrixonConfig::default();
        assert!(config.validate().is_ok());

        config.server_name = String::new();
        assert!(config.validate().is_err());
    }
}
