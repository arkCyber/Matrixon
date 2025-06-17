//! IPFS types
//! 
//! This module defines the core types used in the IPFS integration.
//! It includes types for content addressing, data storage, and metadata.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Content Identifier (CID)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cid {
    /// The CID string
    pub value: String,
    /// The CID version
    pub version: u64,
    /// The codec
    pub codec: String,
}

impl Cid {
    /// Create a new CID
    pub fn new(value: String, version: u64, codec: String) -> Self {
        Self {
            value,
            version,
            codec,
        }
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        self.value.clone()
    }
}

/// IPFS data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsData {
    /// Content identifier
    pub cid: String,
    /// Raw data
    pub data: Vec<u8>,
    /// Content type
    pub content_type: String,
}

impl IpfsData {
    /// Create new data
    pub fn new(cid: String, data: Vec<u8>, content_type: String) -> Self {
        Self {
            cid,
            data,
            content_type,
        }
    }
}

/// IPFS metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsMetadata {
    /// Content identifier
    pub cid: String,
    /// File size in bytes
    pub size: usize,
    /// Content type
    pub content_type: String,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Expiration timestamp (optional)
    pub expires_at: Option<SystemTime>,
    /// File name
    pub name: String,
    /// Last modification timestamp
    pub modified_at: SystemTime,
    /// Additional metadata
    pub extra: serde_json::Value,
}

impl IpfsMetadata {
    /// Create new metadata
    pub fn new(
        cid: String,
        name: String,
        size: usize,
        content_type: String,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            cid,
            name,
            size,
            content_type,
            created_at: now,
            modified_at: now,
            expires_at: None,
            extra: serde_json::Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid() {
        let cid = Cid::new("QmTest".to_string(), 1, "dag-pb".to_string());
        assert_eq!(cid.version, 1);
        assert_eq!(cid.codec, "dag-pb");
    }

    #[test]
    fn test_metadata_creation() {
        let metadata = IpfsMetadata::new(
            "test_cid".to_string(),
            "test.txt".to_string(),
            1024,
            "text/plain".to_string(),
        );

        assert_eq!(metadata.cid, "test_cid");
        assert_eq!(metadata.name, "test.txt");
        assert_eq!(metadata.size, 1024);
        assert_eq!(metadata.content_type, "text/plain");
        assert!(metadata.expires_at.is_none());
        assert!(metadata.extra.is_null());
    }

    #[test]
    fn test_data_creation() {
        let data = IpfsData::new(
            "test_cid".to_string(),
            vec![1, 2, 3],
            "application/octet-stream".to_string(),
        );

        assert_eq!(data.cid, "test_cid");
        assert_eq!(data.data, vec![1, 2, 3]);
        assert_eq!(data.content_type, "application/octet-stream");
    }
} 
