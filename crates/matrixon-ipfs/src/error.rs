//! Error types for IPFS integration
//! 
//! This module defines the error types used throughout the IPFS integration.
//! It provides comprehensive error handling for all IPFS operations.

use thiserror::Error;
use std::io;

/// IPFS error types
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(String),

    /// IPFS API error
    #[error("IPFS API error: {0}")]
    IpfsApi(String),

    /// CID error
    #[error("CID error: {0}")]
    Cid(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Node error
    #[error("Node error: {0}")]
    Node(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Timeout error
    #[error("Timeout error: {0}")]
    Timeout(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid state error
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),
}

/// Result type for IPFS operations
pub type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err.to_string())
    }
}

impl From<ipfs_api::Error> for Error {
    fn from(err: ipfs_api::Error) -> Self {
        Error::IpfsApi(err.to_string())
    }
}

impl From<cid::Error> for Error {
    fn from(err: cid::Error) -> Self {
        Error::Cid(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "test error");
        let error = Error::Io(io_error.to_string());
        assert!(error.to_string().contains("test error"));
    }
} 
