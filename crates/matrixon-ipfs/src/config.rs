//! IPFS configuration
//! 
//! This module defines the configuration types for IPFS integration.
//! It provides settings for node configuration, storage, and network parameters.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// IPFS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsConfig {
    /// IPFS node address
    #[serde(default = "default_node_address")]
    pub node_address: String,

    /// IPFS API port
    #[serde(default = "default_api_port")]
    pub api_port: u16,

    /// IPFS gateway port
    #[serde(default = "default_gateway_port")]
    pub gateway_port: u16,

    /// IPFS swarm port
    #[serde(default = "default_swarm_port")]
    pub swarm_port: u16,

    /// Storage configuration
    #[serde(default)]
    pub storage: StorageConfig,

    /// Network configuration
    #[serde(default)]
    pub network: NetworkConfig,

    /// Timeout configuration
    #[serde(default)]
    pub timeout: TimeoutConfig,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage path
    #[serde(default = "default_storage_path")]
    pub path: String,

    /// Maximum storage size in bytes
    #[serde(default = "default_max_storage_size")]
    pub max_size: u64,

    /// Enable garbage collection
    #[serde(default = "default_gc_enabled")]
    pub gc_enabled: bool,

    /// Garbage collection interval
    #[serde(default = "default_gc_interval")]
    pub gc_interval: Duration,

    /// Cache size in bytes for frequently accessed content
    #[serde(default = "default_cache_size")]
    pub cache_size: u64,

    /// Retention period for cached content
    #[serde(default = "default_retention_period")]
    pub retention_period: Duration,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Bootstrap nodes
    #[serde(default = "default_bootstrap_nodes")]
    pub bootstrap_nodes: Vec<String>,

    /// Enable DHT
    #[serde(default = "default_dht_enabled")]
    pub dht_enabled: bool,

    /// Enable pubsub
    #[serde(default = "default_pubsub_enabled")]
    pub pubsub_enabled: bool,

    /// Enable relay
    #[serde(default = "default_relay_enabled")]
    pub relay_enabled: bool,
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// API request timeout
    #[serde(default = "default_api_timeout")]
    pub api_timeout: Duration,

    /// DHT query timeout
    #[serde(default = "default_dht_timeout")]
    pub dht_timeout: Duration,

    /// Pin operation timeout
    #[serde(default = "default_pin_timeout")]
    pub pin_timeout: Duration,
}

// Default values
fn default_node_address() -> String {
    "127.0.0.1".to_string()
}

fn default_api_port() -> u16 {
    5001
}

fn default_gateway_port() -> u16 {
    8080
}

fn default_swarm_port() -> u16 {
    4001
}

fn default_storage_path() -> String {
    ".ipfs".to_string()
}

fn default_max_storage_size() -> u64 {
    10 * 1024 * 1024 * 1024 // 10GB
}

fn default_gc_enabled() -> bool {
    true
}

fn default_gc_interval() -> Duration {
    Duration::from_secs(3600) // 1 hour
}

fn default_bootstrap_nodes() -> Vec<String> {
    vec![
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN".to_string(),
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa".to_string(),
    ]
}

fn default_dht_enabled() -> bool {
    true
}

fn default_pubsub_enabled() -> bool {
    true
}

fn default_relay_enabled() -> bool {
    true
}

fn default_api_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_dht_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_pin_timeout() -> Duration {
    Duration::from_secs(300)
}

fn default_cache_size() -> u64 {
    1024 * 1024 * 1024 // 1GB
}

fn default_retention_period() -> Duration {
    Duration::from_secs(86400) // 24 hours
}

impl Default for IpfsConfig {
    fn default() -> Self {
        Self {
            node_address: default_node_address(),
            api_port: default_api_port(),
            gateway_port: default_gateway_port(),
            swarm_port: default_swarm_port(),
            storage: StorageConfig::default(),
            network: NetworkConfig::default(),
            timeout: TimeoutConfig::default(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            path: default_storage_path(),
            max_size: default_max_storage_size(),
            gc_enabled: default_gc_enabled(),
            gc_interval: default_gc_interval(),
            cache_size: default_cache_size(),
            retention_period: default_retention_period(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bootstrap_nodes: default_bootstrap_nodes(),
            dht_enabled: default_dht_enabled(),
            pubsub_enabled: default_pubsub_enabled(),
            relay_enabled: default_relay_enabled(),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            api_timeout: default_api_timeout(),
            dht_timeout: default_dht_timeout(),
            pin_timeout: default_pin_timeout(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IpfsConfig::default();
        assert_eq!(config.node_address, "127.0.0.1");
        assert_eq!(config.api_port, 5001);
        assert_eq!(config.gateway_port, 8080);
        assert_eq!(config.swarm_port, 4001);
    }

    #[test]
    fn test_config_serialization() {
        let config = IpfsConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: IpfsConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config.node_address, deserialized.node_address);
    }
} 
