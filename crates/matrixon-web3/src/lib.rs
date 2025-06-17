//! Matrixon Web3 Integration Library
//!
//! Author: arkSong (arksong2018@gmail.com)
//! Date: 2025-06-15
//! Version: 0.1.0
//!
//! Provides Web3 functionality integration for Matrixon server,
//! including wallet operations, smart contract interactions,
//! and blockchain event handling.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

use thiserror::Error;

/// Main error type for Web3 operations
#[derive(Debug, Error)]
pub enum Web3Error {
    /// Invalid wallet operation
    #[error("Wallet error: {0}")]
    WalletError(String),
    
    /// Contract interaction failure
    #[error("Contract error: {0}")]
    ContractError(String),
    
    /// Event parsing failure 
    #[error("Event error: {0}")]
    EventError(String),
    
    /// Underlying Web3 provider error
    #[error("Web3 provider error: {0}")]
    ProviderError(#[from] web3::Error),
}

/// Re-export of common types
pub mod prelude {
    pub use super::{Web3Error, Web3Result};
    pub use web3::types::*;
}

/// Convenience type alias for Web3 results
pub type Web3Result<T> = Result<T, Web3Error>;

// Include other modules
pub mod client;
pub mod contracts;
pub mod events;
pub mod wallet;
