//! Error Handling Module
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Date: 2024-03-21
//! Version: 0.1.0
//!
//! Purpose: Defines error types for the Matrixon monitoring system.
//! Implements comprehensive error handling with detailed context and logging.

use thiserror::Error;

/// Main error type for the monitoring system
#[derive(Debug, Error)]
pub enum MonitorError {
    /// Configuration related errors
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// System monitoring errors
    #[error("System monitoring error: {0}")]
    SystemError(String),

    /// Metrics collection errors
    #[error("Metrics error: {0}")]
    MetricsError(String),

    /// Network communication errors
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Internal errors
    #[error("Internal error: {0}")]
    InternalError(String),

    /// HTTP server errors
    #[error("HTTP error: {0}")]
    HttpError(String),

    /// Logging system errors
    #[error("Logging error: {0}")]
    LoggingError(String),

    /// Alert system errors
    #[error("Alert error: {0}")]
    AlertError(String),
}

/// Result type alias for the monitoring system
pub type Result<T, E = MonitorError> = std::result::Result<T, E>;

impl MonitorError {
    /// Log the error with appropriate severity
    pub fn log(&self) {
        match self {
            MonitorError::ConfigError(msg) => tracing::error!("❌ Config error: {}", msg),
            MonitorError::SystemError(msg) => tracing::error!("❌ System error: {}", msg),
            MonitorError::MetricsError(msg) => tracing::warn!("⚠️ Metrics error: {}", msg),
            MonitorError::NetworkError(msg) => tracing::error!("❌ Network error: {}", msg),
            MonitorError::InternalError(msg) => tracing::error!("❌ Internal error: {}", msg),
            MonitorError::HttpError(msg) => tracing::error!("❌ HTTP error: {}", msg),
            MonitorError::LoggingError(msg) => tracing::error!("❌ Logging error: {}", msg),
            MonitorError::AlertError(msg) => tracing::warn!("⚠️ Alert error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn test_error_logging() {
        let err = MonitorError::ConfigError("invalid config".to_string());
        err.log();
        // logs_contain is automatically checked by traced_test
    }
}
