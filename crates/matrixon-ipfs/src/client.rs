//! IPFS client
//! 
//! This module implements the IPFS client for interacting with IPFS nodes.
//! It provides methods for storing, retrieving, and managing data on IPFS.

use crate::{
    config::StorageConfig,
    error::Result, 
    storage::IpfsStorage,
    types::{IpfsData, IpfsMetadata}
};
use std::time::Duration;
use tokio_stream::StreamExt;
use ipfs_api::IpfsApi;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// IPFS client configuration
#[derive(Debug, Clone)]
pub struct IpfsClientConfig {
    /// IPFS API endpoint
    pub api_endpoint: String,
    /// Storage configuration
    pub storage: StorageConfig,
}

impl Default for IpfsClientConfig {
    fn default() -> Self {
        Self {
            api_endpoint: "http://localhost:5001".to_string(),
            storage: StorageConfig::default(),
        }
    }
}

/// IPFS client
pub struct IpfsClient {
    /// IPFS API client
    api: ipfs_api::IpfsClient,
    /// Storage manager
    storage: Arc<RwLock<IpfsStorage>>,
}

impl IpfsClient {
    /// Create new IPFS client
    pub async fn new(config: IpfsClientConfig) -> Result<Self> {
        debug!("🔧 Creating new IPFS client");
        let start = std::time::Instant::now();

        let api = ipfs_api::IpfsClient::default();
        let storage = Arc::new(RwLock::new(IpfsStorage::new(config.storage).await?));

        let client = Self { api, storage };

        info!("✅ IPFS client created in {:?}", start.elapsed());
        Ok(client)
    }

    /// Store data
    pub async fn store(&self, data: &[u8], content_type: &str) -> Result<String> {
        debug!("🔧 Storing data");
        let start = std::time::Instant::now();

        let cursor = std::io::Cursor::new(data.to_vec());
        let add_response = self.api.add(cursor).await?;
        let cid_str = add_response.hash;

        let metadata = IpfsMetadata {
            cid: cid_str.clone(),
            size: data.len(),
            content_type: content_type.to_string(),
            created_at: std::time::SystemTime::now(),
            expires_at: None,
            name: "data".to_string(),
            modified_at: std::time::SystemTime::now(),
            extra: serde_json::Value::Null,
        };

        self.storage.write().await.store_metadata(&cid_str, metadata).await?;

        info!("✅ Data stored in {:?}", start.elapsed());
        Ok(cid_str)
    }

    /// Retrieve data
    pub async fn retrieve(&self, cid: &str) -> Result<IpfsData> {
        debug!("🔧 Retrieving data");
        let start = std::time::Instant::now();

        let mut stream = self.api.cat(cid);
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk?);
        }
        let metadata = self.storage.read().await.get_metadata(cid).await?;

        let ipfs_data = IpfsData {
            cid: cid.to_string(),
            data,
            content_type: metadata.map_or("application/octet-stream".to_string(), |m| m.content_type),
        };

        info!("✅ Data retrieved in {:?}", start.elapsed());
        Ok(ipfs_data)
    }

    /// Pin data
    pub async fn pin(&self, cid: &str) -> Result<()> {
        debug!("🔧 Pinning data");
        let start = std::time::Instant::now();

        self.api.pin_add(cid, true).await?;

        info!("✅ Data pinned in {:?}", start.elapsed());
        Ok(())
    }

    /// Unpin data
    pub async fn unpin(&self, cid: &str) -> Result<()> {
        debug!("🔧 Unpinning data");
        let start = std::time::Instant::now();

        self.api.pin_rm(cid, true).await?;

        info!("✅ Data unpinned in {:?}", start.elapsed());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    

    #[tokio::test]
    async fn test_client_creation() {
        let temp_dir = tempdir().unwrap();
        let config = IpfsClientConfig {
            api_endpoint: "http://localhost:5001".to_string(),
            storage: StorageConfig {
                path: temp_dir.path().to_string_lossy().to_string(),
                cache_size: 1024,
                retention_period: Duration::from_secs(3600),
                max_size: 1024 * 1024,
                gc_enabled: true,
                gc_interval: Duration::from_secs(3600),
            },
        };

        let client = IpfsClient::new(config).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let temp_dir = tempdir().unwrap();
        let config = IpfsClientConfig {
            api_endpoint: "http://localhost:5001".to_string(),
            storage: StorageConfig {
                path: temp_dir.path().to_string_lossy().to_string(),
                cache_size: 1024,
                retention_period: Duration::from_secs(3600),
                max_size: 1024 * 1024,
                gc_enabled: true,
                gc_interval: Duration::from_secs(3600),
            },
        };

        let client = IpfsClient::new(config).await.unwrap();
        let data = b"test data";
        let content_type = "text/plain";

        let cid = client.store(data, content_type).await.unwrap();
        let retrieved = client.retrieve(&cid).await.unwrap();

        assert_eq!(retrieved.data, data);
        assert_eq!(retrieved.content_type, content_type);
    }

    #[tokio::test]
    async fn test_pin_operations() {
        let temp_dir = tempdir().unwrap();
        let config = IpfsClientConfig {
            api_endpoint: "http://localhost:5001".to_string(),
            storage: StorageConfig {
                path: temp_dir.path().to_string_lossy().to_string(),
                cache_size: 1024,
                retention_period: Duration::from_secs(3600),
                max_size: 1024 * 1024,
                gc_enabled: true,
                gc_interval: Duration::from_secs(3600),
            },
        };

        let client = IpfsClient::new(config).await.unwrap();
        let data = b"test data";
        let content_type = "text/plain";

        let cid = client.store(data, content_type).await.unwrap();
        client.pin(&cid).await.unwrap();
        client.unpin(&cid).await.unwrap();
    }
}
