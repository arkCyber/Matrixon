//! Error Handling Module
//!
//! Centralized error handling for Web3 operations.
//! Author: arkSong (arksong2018@gmail.com)
//! Version: 0.1.0
//! Date: 2025-06-15

use thiserror::Error;

/// Unified error type for Web3 operations
#[derive(Debug, Error)]
pub enum Error {
    #[error("Web3 error: {0}")]
    Web3(#[from] web3::Error),

    #[error("Contract error: {0}")]
    Contract(#[from] web3::contract::Error),

    #[error("Invalid contract index")]
    InvalidContractIndex,

    #[error("Invalid address")]
    InvalidAddress,

    #[error("Invalid bytecode")]
    InvalidBytecode,

    #[error("Invalid ABI")]
    InvalidAbi,

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type for Web3 operations
pub type Result<T> = std::result::Result<T, Error>;
