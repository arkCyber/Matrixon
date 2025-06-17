use thiserror::Error;
use std::io;

/// Matrixon global error type
#[derive(Debug, Error)]
pub enum Error {
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Configuration error: {0}")]
    BadConfig(String),
    
    #[error("Bad server response: {0}")]
    BadServerResponse(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// Matrixon global result type
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_bad_request_error() {
        let error = Error::BadRequestString("Test message".to_string());
        assert!(error.to_string().contains("Bad request"));
        assert!(error.to_string().contains("Test message"));
    }

    #[test]
    fn test_internal_error() {
        let error = Error::Internal("Test internal error".to_string());
        assert!(error.to_string().contains("Internal error"));
        assert!(error.to_string().contains("Test internal error"));
    }

    #[test]
    fn test_database_error() {
        let error = Error::Database("Connection failed".to_string());
        assert!(error.to_string().contains("Database error"));
        assert!(error.to_string().contains("Connection failed"));
    }

    #[test]
    fn test_bad_config_error() {
        let error = Error::BadConfig("Invalid configuration".to_string());
        assert!(error.to_string().contains("Configuration error"));
        assert!(error.to_string().contains("Invalid configuration"));
    }
}
