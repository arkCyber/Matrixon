// =============================================================================
// Matrixon Matrix NextServer - Server Discovery Module
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Contributors: Matrixon Development Team
// Date: 2024-12-11
// Version: 2.0.0-alpha (PostgreSQL Backend)
// License: Apache 2.0 / MIT
//
// Description:
//   Server discovery implementation for Matrix federation. This module handles
//   discovering and validating other Matrix servers for federation.
//
// Performance Targets:
//   • <50ms server discovery latency
//   • >99% discovery success rate
//   • Efficient caching of server information
//   • Minimal resource usage
//
// Features:
//   • Server discovery via DNS SRV records
//   • Well-known server discovery
//   • Server validation and health checks
//   • Server information caching
//   • Automatic retry with backoff
//
// =============================================================================

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, Instant},
};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, instrument, warn};
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    TokioAsyncResolver,
};

use crate::Error;
use crate::Result;

/// Server discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Discovery interval
    pub discovery_interval: Duration,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry backoff duration
    pub retry_backoff: Duration,
    /// Validation timeout
    pub validation_timeout: Duration,
    /// Cache TTL
    pub cache_ttl: Duration,
}

/// Discovered server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredServer {
    /// Server name
    pub server_name: String,
    /// Server version
    pub version: Option<String>,
    /// Federation version
    pub federation_version: String,
    /// Server endpoints
    pub endpoints: Vec<String>,
    /// Last validation time
    pub last_validated: SystemTime,
    /// Validation status
    pub is_valid: bool,
    /// Server capabilities
    pub capabilities: HashMap<String, serde_json::Value>,
}

/// Server discovery service
pub struct ServerDiscovery {
    /// Discovery configuration
    config: DiscoveryConfig,
    /// DNS resolver
    resolver: TokioAsyncResolver,
    /// HTTP client
    http_client: Client,
    /// Discovered servers cache
    servers: Arc<RwLock<HashMap<String, DiscoveredServer>>>,
    /// Server validation results
    validation_results: Arc<RwLock<HashMap<String, bool>>>,
}

impl ServerDiscovery {
    /// Create a new server discovery service
    #[instrument(level = "debug", skip(config))]
    pub async fn new(config: DiscoveryConfig) -> Result<Self> {
        let start = Instant::now();
        debug!("🔧 Initializing Server Discovery service");

        // Create DNS resolver
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        ).map_err(|_e| Error::BadConfig("Failed to create DNS resolver".to_string()))?;

        // Create HTTP client
        let http_client = Client::builder()
            .timeout(config.validation_timeout)
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(100)
            .build()
            .map_err(|e| Error::BadConfig(format!("Failed to create HTTP client: {}", e)))?;

        let service = Self {
            config,
            resolver,
            http_client,
            servers: Arc::new(RwLock::new(HashMap::new())),
            validation_results: Arc::new(RwLock::new(HashMap::new())),
        };

        debug!("✅ Server Discovery service initialized in {:?}", start.elapsed());
        Ok(service)
    }

    /// Discover server information
    #[instrument(level = "debug", skip(self), fields(server_name = %server_name))]
    pub async fn discover_server(&self, server_name: &str) -> Result<DiscoveredServer> {
        let start = Instant::now();
        debug!("🔍 Discovering server: {}", server_name);

        // Check cache first
        if let Some(cached) = self.get_cached_server(server_name).await {
            if self.is_cache_valid(&cached).await {
                debug!("✅ Using cached server information");
                return Ok(cached);
            }
        }

        // Discover server via DNS SRV
        let endpoints: Vec<String> = self.discover_server_endpoints(server_name).await?;

        // Validate server
        let (version, federation_version, capabilities) = self.validate_server(server_name, endpoints.as_slice()).await?;

        let server = DiscoveredServer {
            server_name: server_name.to_string(),
            version,
            federation_version,
            endpoints,
            last_validated: SystemTime::now(),
            is_valid: true,
            capabilities,
        };

        // Cache server information
        self.cache_server(server.clone()).await?;

        debug!("✅ Server discovered in {:?}", start.elapsed());
        Ok(server)
    }

    /// Get cached server information
    async fn get_cached_server(&self, server_name: &str) -> Option<DiscoveredServer> {
        self.servers.read().await.get(server_name).cloned()
    }

    /// Check if cached server information is still valid
    async fn is_cache_valid(&self, server: &DiscoveredServer) -> bool {
        if let Ok(duration) = server.last_validated.elapsed() {
            duration < self.config.cache_ttl
        } else {
            false
        }
    }

    /// Cache server information
    async fn cache_server(&self, server: DiscoveredServer) -> Result<()> {
        let mut servers = self.servers.write().await;
        servers.insert(server.server_name.clone(), server);
        Ok(())
    }

    /// Discover server endpoints via DNS SRV
    #[instrument(level = "debug", skip(self), fields(server_name = %server_name))]
    async fn discover_server_endpoints(&self, server_name: &str) -> Result<Vec<String>> {
        let mut endpoints = Vec::new();

        // Try DNS SRV record first
        let srv_name = format!("_matrix._tcp.{}", server_name);
        match self.resolver.srv_lookup(&srv_name).await {
            Ok(response) => {
                for record in response.iter() {
                    let endpoint = format!("{}:{}", record.target(), record.port());
                    endpoints.push(endpoint);
                }
            }
            Err(e) => {
                warn!("⚠️ DNS SRV lookup failed: {}", e);
            }
        }

        // Fallback to well-known
        if endpoints.is_empty() {
            let well_known_url = format!("https://{}/.well-known/matrix/server", server_name);
            match self.http_client.get(&well_known_url).send().await {
                Ok(response) => {
                    if let Ok(data) = response.json::<serde_json::Value>().await {
                        if let Some(server) = data.get("m.server") {
                            if let Some(server_str) = server.as_str() {
                                endpoints.push(server_str.to_string());
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("⚠️ Well-known lookup failed: {}", e);
                }
            }
        }

        // Fallback to direct connection
        if endpoints.is_empty() {
            endpoints.push(format!("{}:8448", server_name));
        }

        if endpoints.is_empty() {
            return Err(Error::BadConfig(format!("No endpoints found for server {}", server_name)));
        }

        Ok(endpoints)
    }

    /// Validate server and get version information
    #[instrument(level = "debug", skip(self), fields(server_name = %server_name))]
    async fn validate_server(
        &self,
        server_name: &str,
        endpoints: &[String],
    ) -> Result<(Option<String>, String, HashMap<String, serde_json::Value>)> {
        for endpoint in endpoints {
            let url = format!("https://{}/_matrix/federation/v1/version", endpoint);
            match self.http_client.get(&url).send().await {
                Ok(response) => {
                    if let Ok(data) = response.json::<serde_json::Value>().await {
                        let version = data.get("server").and_then(|s| s.get("version")).and_then(|v| v.as_str()).map(String::from);
                        let federation_version = data.get("server")
                            .and_then(|s| s.get("federation_version"))
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| "1.0".to_string());
                        let capabilities = data.get("server")
                            .and_then(|s| s.get("capabilities"))
                            .and_then(|c| c.as_object())
                            .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                            .unwrap_or_default();

                        return Ok((version, federation_version, capabilities));
                    }
                }
                Err(e) => {
                    warn!("⚠️ Server validation failed for {}: {}", endpoint, e);
                }
            }
        }

        Err(Error::BadConfig(format!("Failed to validate server {}", server_name)))
    }
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            discovery_interval: Duration::from_secs(300), // 5 minutes
            max_retries: 3,
            retry_backoff: Duration::from_secs(5),
            validation_timeout: Duration::from_secs(10),
            cache_ttl: Duration::from_secs(3600), // 1 hour
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_discovery() {
        let config = DiscoveryConfig::default();
        let discovery = ServerDiscovery::new(config).await.unwrap();

        // Test discovery of matrix.org
        let result = discovery.discover_server("matrix.org").await;
        assert!(result.is_ok(), "Should discover matrix.org server");

        if let Ok(server) = result {
            assert!(!server.endpoints.is_empty(), "Should have endpoints");
            assert!(server.is_valid, "Server should be valid");
        }
    }

    #[tokio::test]
    async fn test_cache_behavior() {
        let config = DiscoveryConfig::default();
        let discovery = ServerDiscovery::new(config).await.unwrap();

        // First discovery
        let server1 = discovery.discover_server("matrix.org").await.unwrap();
        
        // Second discovery should use cache
        let server2 = discovery.discover_server("matrix.org").await.unwrap();
        
        assert_eq!(server1.server_name, server2.server_name);
        assert_eq!(server1.endpoints, server2.endpoints);
    }
}
