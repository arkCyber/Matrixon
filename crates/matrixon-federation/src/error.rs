// =============================================================================
// Matrixon Federation - Error Module
// =============================================================================
//
// Author: arkSong <arksong2018@gmail.com>
// Version: 0.11.0-alpha
// Date: 2024-03-21
//
// This module defines error types and handling for the Matrixon federation
// implementation. It provides comprehensive error types for all federation
// operations with detailed context and proper error handling.
//
// =============================================================================

use std::fmt;
use thiserror::Error;
use tracing::{error, warn};

/// Federation-specific error types
#[derive(Error, Debug)]
pub enum FederationError {
    /// Configuration validation error
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Network-related errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Matrix protocol errors
    #[error("Matrix protocol error: {0}")]
    Matrix(String),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Rate limiting errors
    #[error("Rate limited: {0}")]
    RateLimited(String),

    /// Timeout errors
    #[error("Operation timed out after {0}ms")]
    Timeout(u64),

    /// Server errors
    #[error("Server error: {0}")]
    Server(String),

    /// Client errors
    #[error("Client error: {0}")]
    Client(String),

    /// Internal errors
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for federation operations
pub type Result<T> = std::result::Result<T, FederationError>;

impl FederationError {
    /// Creates a new configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::InvalidConfig(msg.into())
    }

    /// Creates a new Matrix protocol error
    pub fn matrix(msg: impl Into<String>) -> Self {
        Self::Matrix(msg.into())
    }

    /// Creates a new authentication error
    pub fn auth(msg: impl Into<String>) -> Self {
        Self::Auth(msg.into())
    }

    /// Creates a new rate limiting error
    pub fn rate_limited(msg: impl Into<String>) -> Self {
        Self::RateLimited(msg.into())
    }

    /// Creates a new timeout error
    pub fn timeout(ms: u64) -> Self {
        Self::Timeout(ms)
    }

    /// Creates a new server error
    pub fn server(msg: impl Into<String>) -> Self {
        Self::Server(msg.into())
    }

    /// Creates a new client error
    pub fn client(msg: impl Into<String>) -> Self {
        Self::Client(msg.into())
    }

    /// Creates a new internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Logs the error with appropriate level and context
    pub fn log(&self) {
        match self {
            Self::InvalidConfig(msg) => error!("❌ Invalid configuration: {}", msg),
            Self::Network(err) => error!("❌ Network error: {}", err),
            Self::Json(err) => error!("❌ JSON error: {}", err),
            Self::Matrix(msg) => error!("❌ Matrix protocol error: {}", msg),
            Self::Auth(msg) => error!("❌ Authentication error: {}", msg),
            Self::RateLimited(msg) => warn!("⚠️ Rate limited: {}", msg),
            Self::Timeout(ms) => warn!("⚠️ Operation timed out after {}ms", ms),
            Self::Server(msg) => error!("❌ Server error: {}", msg),
            Self::Client(msg) => error!("❌ Client error: {}", msg),
            Self::Internal(msg) => error!("❌ Internal error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_error_creation() {
        let config_err = FederationError::config("test config error");
        assert!(matches!(config_err, FederationError::InvalidConfig(_)));

        let matrix_err = FederationError::matrix("test matrix error");
        assert!(matches!(matrix_err, FederationError::Matrix(_)));

        let auth_err = FederationError::auth("test auth error");
        assert!(matches!(auth_err, FederationError::Auth(_)));

        let rate_err = FederationError::rate_limited("test rate limit");
        assert!(matches!(rate_err, FederationError::RateLimited(_)));

        let timeout_err = FederationError::timeout(5000);
        assert!(matches!(timeout_err, FederationError::Timeout(5000)));

        let server_err = FederationError::server("test server error");
        assert!(matches!(server_err, FederationError::Server(_)));

        let client_err = FederationError::client("test client error");
        assert!(matches!(client_err, FederationError::Client(_)));

        let internal_err = FederationError::internal("test internal error");
        assert!(matches!(internal_err, FederationError::Internal(_)));
    }

    #[test]
    fn test_error_display() {
        let err = FederationError::config("test error");
        assert_eq!(err.to_string(), "Invalid configuration: test error");

        let err = FederationError::matrix("test error");
        assert_eq!(err.to_string(), "Matrix protocol error: test error");

        let err = FederationError::timeout(5000);
        assert_eq!(err.to_string(), "Operation timed out after 5000ms");
    }
} 
