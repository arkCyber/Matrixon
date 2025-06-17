//! Error types for Matrixon backup operations
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2025-06-15

use std::{io, path::PathBuf};
use thiserror::Error;

/// Main error type for backup operations
#[derive(Error, Debug)]
pub enum BackupError {
    /// I/O error during backup operation
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Database backup error
    #[error("Database backup failed: {0}")]
    Database(String),

    /// Configuration error
    #[error("Invalid configuration: {0}")]
    Config(String),

    /// Compression error
    #[error("Compression failed: {0}")]
    Compression(String),

    /// Invalid backup path
    #[error("Invalid backup path: {0}")]
    InvalidPath(PathBuf),

    /// Other backup error
    #[error("Backup error: {0}")]
    Other(String),

    /// Matrixon core error
    #[error("Matrixon core error: {0}")]
    Core(#[from] matrixon_common::error::MatrixonError),

    /// Matrixon core error (direct)
    #[error("Matrixon core error: {0}")]
    CoreDirect(#[from] matrixon_core::error::MatrixonError),

    /// SQLx database error
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}


impl BackupError {
    /// Create a new configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a new database error
    pub fn database(msg: impl Into<String>) -> Self {
        Self::Database(msg.into())
    }

    /// Create a new other error
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

/// Result type for backup operations
pub type BackupResult<T> = Result<T, BackupError>;
