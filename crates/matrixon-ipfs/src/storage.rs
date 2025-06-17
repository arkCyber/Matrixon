//! IPFS storage
//! 
//! This module implements the storage management for IPFS data.
//! It provides methods for storing and retrieving metadata and managing storage.

use crate::{config::StorageConfig, error::{Error, Result}, types::IpfsMetadata};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tracing::warn;
use tokio::fs;
use std::time::SystemTime;
use lru;
use std::num::NonZeroUsize;

/// IPFS storage manager
pub struct IpfsStorage {
    config: StorageConfig,
    metadata_cache: Arc<RwLock<lru::LruCache<String, IpfsMetadata>>>,
}

impl IpfsStorage {
    /// Create new storage manager
    pub async fn new(config: StorageConfig) -> Result<Self> {
        // Create base directory if it doesn't exist
        fs::create_dir_all(&config.path)
            .await
            .map_err(|e| Error::Io(e.to_string()))?;

        let cache_capacity = NonZeroUsize::new(
            (config.cache_size as usize / std::mem::size_of::<IpfsMetadata>()).max(1)
        ).unwrap();

        Ok(Self {
            metadata_cache: Arc::new(RwLock::new(lru::LruCache::new(cache_capacity))),
            config,
        })
    }

    /// Store metadata
    pub async fn store_metadata(&self, cid: &str, metadata: IpfsMetadata) -> Result<()> {
        let path = PathBuf::from(&self.config.path).join(format!("{}.meta", cid));
        let data = serde_json::to_vec(&metadata)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        fs::write(&path, data)
            .await
            .map_err(|e| Error::Io(e.to_string()))?;

        // Update cache
        let mut cache = self.metadata_cache.write().await;
        cache.put(cid.to_string(), metadata);

        Ok(())
    }

    /// Get metadata
    pub async fn get_metadata(&self, cid: &str) -> Result<Option<IpfsMetadata>> {
        // Check cache first
        {
            let cache = self.metadata_cache.read().await;
            if let Some(metadata) = cache.peek(cid) {
                return Ok(Some(metadata.clone()));
            }
        }

        // Read from disk
        let path = PathBuf::from(&self.config.path).join(format!("{}.meta", cid));
        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(&path)
            .await
            .map_err(|e| Error::Io(e.to_string()))?;

        let metadata: IpfsMetadata = serde_json::from_slice(&data)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        // Update cache
        let mut cache = self.metadata_cache.write().await;
        cache.put(cid.to_string(), metadata.clone());

        Ok(Some(metadata))
    }

    /// Clean up old metadata
    pub async fn cleanup(&self) -> Result<()> {
        let now = SystemTime::now();
        let mut entries = fs::read_dir(&self.config.path)
            .await
            .map_err(|e| Error::Io(e.to_string()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| Error::Io(e.to_string()))?
        {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "meta") {
                if let Ok(metadata) = self.get_metadata(&path.file_stem().unwrap().to_string_lossy()).await {
                    if let Some(metadata) = metadata {
                        if let Ok(age) = now.duration_since(metadata.created_at) {
                            if age > self.config.retention_period {
                                if let Err(e) = fs::remove_file(&path).await {
                                    warn!("Failed to remove old metadata file: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    use crate::config::StorageConfig;
    use std::time::Duration;

    #[tokio::test]
    async fn test_storage_creation() {
        let temp_dir = tempdir().unwrap();
        let config = StorageConfig {
            path: temp_dir.path().to_string_lossy().to_string(),
            max_size: 1024 * 1024,
            gc_enabled: true,
            gc_interval: Duration::from_secs(3600),
            cache_size: 1024 * 1024,
            retention_period: Duration::from_secs(3600),
        };

        let storage = IpfsStorage::new(config).await.unwrap();
        assert!(PathBuf::from(&storage.config.path).exists());
    }

    #[tokio::test]
    async fn test_metadata_operations() {
        let temp_dir = tempdir().unwrap();
        let config = StorageConfig {
            path: temp_dir.path().to_string_lossy().to_string(),
            max_size: 1024 * 1024,
            gc_enabled: true,
            gc_interval: Duration::from_secs(3600),
            cache_size: 1024 * 1024,
            retention_period: Duration::from_secs(3600),
        };

        let storage = IpfsStorage::new(config).await.unwrap();
        let cid = "test_cid";
        let now = SystemTime::now();
        let metadata = IpfsMetadata {
            cid: cid.to_string(),
            size: 1024,
            content_type: "text/plain".to_string(),
            created_at: now,
            expires_at: None,
            name: "test.txt".to_string(),
            modified_at: now,
            extra: serde_json::json!({"author": "arkSong"}),
        };

        // Test store
        storage.store_metadata(cid, metadata.clone()).await.unwrap();

        // Test get
        let retrieved = storage.get_metadata(cid).await.unwrap().unwrap();
        assert_eq!(retrieved.cid, metadata.cid);
        assert_eq!(retrieved.size, metadata.size);
        assert_eq!(retrieved.name, metadata.name);
        assert_eq!(retrieved.modified_at, metadata.modified_at);
        assert_eq!(retrieved.extra, metadata.extra);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let temp_dir = tempdir().unwrap();
        let config = StorageConfig {
            path: temp_dir.path().to_string_lossy().to_string(),
            max_size: 1024 * 1024,
            gc_enabled: true,
            gc_interval: Duration::from_secs(3600),
            cache_size: 1024 * 1024,
            retention_period: Duration::from_secs(3600),
        };

        let storage = IpfsStorage::new(config).await.unwrap();
        let cid = "test_cid";
        let now = SystemTime::now() - Duration::from_secs(2);
        let metadata = IpfsMetadata {
            cid: cid.to_string(),
            size: 1024,
            content_type: "text/plain".to_string(),
            created_at: now,
            expires_at: None,
            name: "test.txt".to_string(),
            modified_at: now,
            extra: serde_json::json!({"author": "arkSong"}),
        };

        storage.store_metadata(cid, metadata).await.unwrap();
        storage.cleanup().await.unwrap();
        assert!(storage.get_metadata(cid).await.unwrap().is_none());
    }
}
