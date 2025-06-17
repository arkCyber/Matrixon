//! A2A Protocol Error Types
//! 
//! This module defines error types for the A2A protocol implementation.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use thiserror::Error;
use std::io;
use crate::state::StateError;
use matrixon_common::error::MatrixonError;

/// A2A Protocol error types
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("State error: {0}")]
    State(#[from] StateError),

    #[error("Message error: {0}")]
    Message(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<Error> for MatrixonError {
    fn from(err: Error) -> Self {
        MatrixonError::Other(format!("A2A error: {}", err))
    }
}
