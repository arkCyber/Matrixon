//! Error handling for Matrixon AI services
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2025-06-14

use crate::mcp::error::McpError;
use http::StatusCode;
use ruma::api::client::error::{Error as RumaError, ErrorBody, ErrorKind};
use thiserror::Error;

/// Error types for the AI Assistant
/// Error types for the AI Assistant module
#[derive(Error, Debug)]
pub enum Error {
    /// Error originating from Matrix operations
    #[error("Matrix error: {0}")]
    Matrix(#[from] RumaError),

    /// Error when joining async tasks
    #[error("Task join error: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),

    /// Redis related errors
    #[error("Redis error: {0}")]
    Redis(String),

    /// Invalid request format or parameters
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Internal server processing errors
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Malformed or invalid client requests
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Errors from the Language Model service
    #[error("LLM error: {0}")]
    Llm(String),

    /// IO operation failures
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Database operation failures
    #[error("Database error: {0}")]
    Database(String),

    /// JSON serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Translation service failures
    #[error("Translation error: {0}")]
    Translation(String),

    /// Question answering service failures
    #[error("Q&A error: {0}")]
    Qa(String),

    /// Content analysis failures
    #[error("Analysis error: {0}")]
    Analysis(String),

    /// Text summarization failures
    #[error("Summarization error: {0}")]
    Summarization(String),

    /// Recommendation service failures
    #[error("Recommendation error: {0}")]
    Recommendation(String),

    /// Unclassified errors
    #[error("Unknown error: {0}")]
    Unknown(String),

    /// Model Context Protocol (MCP) related errors
    #[error("MCP error: {0}")]
    Mcp(#[from] McpError),
}

impl Error {
    /// Get the HTTP status code corresponding to this error
    ///
    /// # Returns
    /// Appropriate HTTP status code for the error type
    pub fn status_code(&self) -> StatusCode {
        match self {
            Error::Matrix(e) => e.status_code,
            Error::TaskJoin(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::Llm(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Serialization(_) => StatusCode::BAD_REQUEST,
            Error::Translation(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Qa(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Analysis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Summarization(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Recommendation(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Mcp(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Result type for the AI Assistant
pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for RumaError {
    fn from(err: Error) -> Self {
        match err {
            Error::BadRequest(msg) => RumaError::new(
                StatusCode::BAD_REQUEST,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: msg,
                },
            ),
            Error::Llm(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e,
                },
            ),
            Error::Io(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e.to_string(),
                },
            ),
            Error::Database(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e,
                },
            ),
            Error::Redis(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e,
                },
            ),
            Error::Serialization(e) => RumaError::new(
                StatusCode::BAD_REQUEST,
                ErrorBody::Standard {
                    kind: ErrorKind::BadJson,
                    message: e.to_string(),
                },
            ),
            Error::Matrix(e) => e,
            Error::TaskJoin(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e.to_string(),
                },
            ),
            Error::InvalidRequest(msg) => RumaError::new(
                StatusCode::BAD_REQUEST,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: msg,
                },
            ),
            Error::Internal(msg) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: msg,
                },
            ),
            Error::Translation(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e,
                },
            ),
            Error::Qa(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e,
                },
            ),
            Error::Analysis(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e,
                },
            ),
            Error::Summarization(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e,
                },
            ),
            Error::Recommendation(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e,
                },
            ),
            Error::Unknown(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e,
                },
            ),
            Error::Mcp(e) => RumaError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody::Standard {
                    kind: ErrorKind::Unknown,
                    message: e.to_string(),
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let test_cases = vec![
            (
                Error::BadRequest("test".to_string()),
                StatusCode::BAD_REQUEST,
                ruma::api::client::error::ErrorKind::Unknown,
            ),
            (
                Error::Llm("test".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
                ruma::api::client::error::ErrorKind::Unknown,
            ),
            (
                Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "test")),
                StatusCode::INTERNAL_SERVER_ERROR,
                ruma::api::client::error::ErrorKind::Unknown,
            ),
            (
                Error::Database("test".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
                ruma::api::client::error::ErrorKind::Unknown,
            ),
            (
                Error::Redis("test".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
                ruma::api::client::error::ErrorKind::Unknown,
            ),
            (
                Error::Serialization(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "test",
                ))),
                StatusCode::BAD_REQUEST,
                ruma::api::client::error::ErrorKind::BadJson,
            ),
        ];

        for (input, expected_status, expected_kind) in test_cases {
            let ruma_error: RumaError = input.into();
            assert_eq!(ruma_error.status_code, expected_status);
            match &ruma_error.body {
                ErrorBody::Standard { kind, .. } => {
                    assert_eq!(kind, &expected_kind);
                }
                _ => panic!("Expected Standard error body"),
            }
        }
    }
}
