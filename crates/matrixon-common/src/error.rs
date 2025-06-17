//! Error types for Matrixon
//! 
//! This module defines the error types used throughout the Matrixon system.
//! All errors are designed to be user-friendly and provide clear context
//! about what went wrong and how to fix it.

use thiserror::Error;
use std::io;
use ruma::api::error::MatrixError;

/// Matrixon error types
#[derive(Debug, Error)]
pub enum MatrixonError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Authorization error
    #[error("Authorization error: {0}")]
    Authorization(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Rate limit error
    #[error("Rate limit error: {0}")]
    RateLimit(String),

    /// Federation error
    #[error("Federation error: {0}")]
    Federation(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Matrix protocol error
    #[error("Matrix protocol error: {0}")]
    Matrix(#[from] MatrixError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Connection pool error
    #[error("Connection pool error: {0}")]
    ConnectionPool(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Resource already exists
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Bad request error
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Other error
    #[error("Error: {0}")]
    Other(String),

    /// IoT specific error
    #[error("IoT error: {0}")]
    IoT(String),
}

/// Result type for Matrixon operations
pub type Result<T> = std::result::Result<T, MatrixonError>;

impl From<serde_json::Error> for MatrixonError {
    fn from(err: serde_json::Error) -> Self {
        MatrixonError::Serialization(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for MatrixonError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        MatrixonError::Timeout(err.to_string())
    }
}

impl From<deadpool::managed::PoolError<String>> for MatrixonError {
    fn from(err: deadpool::managed::PoolError<String>) -> Self {
        MatrixonError::ConnectionPool(err.to_string())
    }
}

impl From<ruma::api::client::error::Error> for MatrixonError {
    fn from(err: ruma::api::client::error::Error) -> Self {
        MatrixonError::Federation(err.to_string())
    }
}

// Add specific conversion for IoTError
impl From<String> for MatrixonError {
    fn from(err: String) -> Self {
        MatrixonError::Other(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_error_conversion() {
        let err = MatrixonError::Config("test".to_string());
        assert_eq!(err.to_string(), "Configuration error: test");

        let err = MatrixonError::Network("test".to_string());
        assert_eq!(err.to_string(), "Network error: test");

        let err = MatrixonError::Auth("test".to_string());
        assert_eq!(err.to_string(), "Authentication error: test");

        let err = MatrixonError::Authorization("test".to_string());
        assert_eq!(err.to_string(), "Authorization error: test");

        let err = MatrixonError::Database("test".to_string());
        assert_eq!(err.to_string(), "Database error: test");

        let err = MatrixonError::Validation("test".to_string());
        assert_eq!(err.to_string(), "Validation error: test");

        let err = MatrixonError::Serialization("test".to_string());
        assert_eq!(err.to_string(), "Serialization error: test");

        let err = MatrixonError::Deserialization("test".to_string());
        assert_eq!(err.to_string(), "Deserialization error: test");

        let err = MatrixonError::RateLimit("test".to_string());
        assert_eq!(err.to_string(), "Rate limit error: test");

        let err = MatrixonError::Federation("test".to_string());
        assert_eq!(err.to_string(), "Federation error: test");

        let err = MatrixonError::Internal("test".to_string());
        assert_eq!(err.to_string(), "Internal error: test");

        let err = MatrixonError::InvalidConfig("test".to_string());
        assert_eq!(err.to_string(), "Invalid configuration: test");

        let err = MatrixonError::ConnectionPool("test".to_string());
        assert_eq!(err.to_string(), "Connection pool error: test");

        let err = MatrixonError::Timeout("test".to_string());
        assert_eq!(err.to_string(), "Operation timed out: test");

        let err = MatrixonError::NotFound("test".to_string());
        assert_eq!(err.to_string(), "Resource not found: test");

        let err = MatrixonError::AlreadyExists("test".to_string());
        assert_eq!(err.to_string(), "Resource already exists: test");

        let err = MatrixonError::InvalidState("test".to_string());
        assert_eq!(err.to_string(), "Invalid state: test");

        let err = MatrixonError::BadRequest("test".to_string());
        assert_eq!(err.to_string(), "Bad request: test");

        let err = MatrixonError::Other("test".to_string());
        assert_eq!(err.to_string(), "Error: test");
    }

    #[test]
    fn test_error_conversion_from_io() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "test");
        let err: MatrixonError = io_err.into();
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_error_conversion_from_serde() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err: MatrixonError = json_err.into();
        assert!(err.to_string().contains("Serialization error"));
    }
}
