//! Matrixon IPFS Integration
//! 
//! This crate provides IPFS integration for Matrixon's distributed storage system.
//! It implements the core functionality for storing and retrieving data from IPFS,
//! including file storage, content addressing, and distributed hash table (DHT) operations.
//! 
//! # Features
//! - File storage and retrieval
//! - Content addressing with CID
//! - DHT operations
//! - Pin management
//! - IPFS node management
//! 
//! # Example
//! ```rust
//! use matrixon_ipfs::{IpfsClient, IpfsConfig};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = IpfsConfig::default();
//!     let client_config = IpfsClientConfig {
//!         api_endpoint: config.api_endpoint,
//!         storage: config.storage,
//!     };
//!     let client = IpfsClient::new(client_config).await?;
//!     
//!     // Store data
//!     let cid = client.store_data(b"Hello, IPFS!").await?;
//!     
//!     // Retrieve data
//!     let data = client.get_data(&cid).await?;
//!     
//!     Ok(())
//! }
//! ```

use tracing::{debug, info, instrument};
use crate::client::IpfsClientConfig;

pub mod client;
pub mod config;
pub mod error;
pub mod node;
pub mod storage;
pub mod types;

pub use client::IpfsClient;
pub use config::IpfsConfig;
pub use error::{Error, Result};
pub use node::IpfsNode;
pub use storage::IpfsStorage;
pub use types::{Cid, IpfsData, IpfsMetadata};

/// IPFS integration version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the IPFS integration
#[instrument(level = "debug")]
pub async fn init(config: IpfsConfig) -> Result<IpfsClient> {
    debug!("🔧 Initializing IPFS integration");
    let start = std::time::Instant::now();
    
    let client_config = IpfsClientConfig {
        api_endpoint: config.node_address,
        storage: config.storage,
    };
    let client = IpfsClient::new(client_config).await?;
    
    info!("✅ IPFS integration initialized in {:?}", start.elapsed());
    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_init() {
        let config = IpfsConfig::default();
        let result = init(config).await;
        assert!(result.is_ok());
    }
}
