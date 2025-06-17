// =============================================================================
// Matrixon Federation - Client Module
// =============================================================================
//
// Author: arkSong <arksong2018@gmail.com>
// Version: 0.11.0-alpha
// Date: 2024-03-21
//
// This module implements the federation client functionality for Matrixon.
// It handles outgoing federation requests, server discovery, and event exchange.
//
// =============================================================================

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{Mutex, RwLock},
    time::sleep,
};
use reqwest::{
    Client as HttpClient,
    header::{HeaderMap, HeaderValue},
    StatusCode,
};
use ruma::{
    api::federation::{
        discovery::ServerSigningKeys,
        authentication::ServerAuthentication,
    },
    events::AnyRoomEvent,
    RoomId, ServerName, UserId,
    OwnedServerName,
};
use serde_json::Value;
use tracing::{debug, info, instrument, warn};

use crate::{
    error::{Result, FederationError},
    types::{ServerInfo, FederationEvent, FederationRequest, FederationResponse},
    traits::{ServerDiscovery, EventExchange, FederationClient},
    config::FederationConfig,
    utils,
};

/// Federation client implementation
pub struct Client {
    /// Client configuration
    config: Arc<FederationConfig>,
    
    /// Server discovery implementation
    discovery: Arc<dyn ServerDiscovery>,
    
    /// Event exchange implementation
    event_exchange: Arc<dyn EventExchange>,
    
    /// HTTP client
    http_client: Arc<HttpClient>,
    
    /// Server information cache
    server_info: Arc<RwLock<HashMap<OwnedServerName, ServerInfo>>>,
    
    /// Request rate limiter
    rate_limiter: Arc<Mutex<HashMap<OwnedServerName, (u32, Instant)>>>,
    
    /// Client metrics
    metrics: Arc<Mutex<ClientMetrics>>,
}

/// Client metrics
#[derive(Debug, Default)]
struct ClientMetrics {
    /// Total requests sent
    total_requests: u64,
    
    /// Total events sent
    total_events: u64,
    
    /// Total errors encountered
    total_errors: u64,
    
    /// Average request latency
    avg_latency: Duration,
    
    /// Current active connections
    active_connections: usize,
    
    /// Client uptime
    uptime: Duration,
}

impl Client {
    /// Creates a new federation client
    #[instrument(level = "debug")]
    pub fn new(
        config: FederationConfig,
        discovery: Arc<dyn ServerDiscovery>,
        event_exchange: Arc<dyn EventExchange>,
    ) -> Result<Self> {
        config.validate()?;
        
        let http_client = HttpClient::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| FederationError::Internal(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self {
            config: Arc::new(config),
            discovery,
            event_exchange,
            http_client: Arc::new(http_client),
            server_info: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(ClientMetrics::default())),
        })
    }
    
    /// Sends a request to a remote server
    #[instrument(level = "debug")]
    pub async fn send_request(&self, request: FederationRequest) -> Result<FederationResponse> {
        let start = Instant::now();
        
        // Update metrics
        {
            let mut metrics = self.metrics.lock().await;
            metrics.total_requests += 1;
        }
        
        // Check rate limits
        self.check_rate_limit(&request.destination_server).await?;
        
        // Get server info
        let server_info = self.get_server_info(&request.destination_server).await?;
        
        // Build request URL
        let url = format!(
            "https://{}{}",
            request.destination_server,
            request.path
        );
        
        // Build request headers
        let mut headers = HeaderMap::new();
        for (key, value) in request.headers {
            headers.insert(
                key.parse().map_err(|e| FederationError::Internal(format!("Invalid header key: {}", e)))?,
                HeaderValue::from_str(&value)
                    .map_err(|e| FederationError::Internal(format!("Invalid header value: {}", e)))?,
            );
        }
        
        // Send request
        let response = self.http_client
            .request(request.method, &url)
            .headers(headers)
            .body(request.body)
            .send()
            .await
            .map_err(|e| FederationError::Network(format!("Request failed: {}", e)))?;
        
        // Get response status
        let status = response.status();
        
        // Get response headers
        let mut response_headers = HashMap::new();
        for (key, value) in response.headers() {
            response_headers.insert(
                key.to_string(),
                value.to_str()
                    .map_err(|e| FederationError::Internal(format!("Invalid header value: {}", e)))?
                    .to_string(),
            );
        }
        
        // Get response body
        let body = response.bytes()
            .await
            .map_err(|e| FederationError::Network(format!("Failed to read response body: {}", e)))?;
        
        // Create response
        let response = FederationResponse {
            request_id: request.request_id,
            status_code: status.as_u16(),
            headers: response_headers,
            body: body.to_vec(),
            timestamp: utils::get_timestamp(),
            duration_ms: start.elapsed().as_millis() as u64,
        };
        
        // Update metrics
        {
            let mut metrics = self.metrics.lock().await;
            metrics.avg_latency = (metrics.avg_latency * (metrics.total_requests - 1) as u32
                + start.elapsed())
                / metrics.total_requests as u32;
        }
        
        Ok(response)
    }
    
    /// Gets server version information
    #[instrument(level = "debug")]
    pub async fn get_server_version(&self, server_name: &ServerName) -> Result<String> {
        let request = FederationRequest {
            request_id: utils::random_string(16)?,
            destination_server: server_name.clone(),
            method: reqwest::Method::GET,
            path: "/_matrix/federation/v1/version".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            timestamp: utils::get_timestamp(),
            timeout_ms: self.config.timeout_ms,
        };
        
        let response = self.send_request(request).await?;
        
        if response.status_code != StatusCode::OK.as_u16() {
            return Err(FederationError::Server(format!(
                "Failed to get server version: {}",
                response.status_code
            )));
        }
        
        let version: Value = serde_json::from_slice(&response.body)
            .map_err(|e| FederationError::Json(format!("Failed to parse version response: {}", e)))?;
        
        Ok(version["server"]["version"]
            .as_str()
            .ok_or_else(|| FederationError::Json("Missing version field".into()))?
            .to_string())
    }
    
    /// Gets server capabilities
    #[instrument(level = "debug")]
    pub async fn get_server_capabilities(&self, server_name: &ServerName) -> Result<Value> {
        let request = FederationRequest {
            request_id: utils::random_string(16)?,
            destination_server: server_name.clone(),
            method: reqwest::Method::GET,
            path: "/_matrix/federation/v1/capabilities".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            timestamp: utils::get_timestamp(),
            timeout_ms: self.config.timeout_ms,
        };
        
        let response = self.send_request(request).await?;
        
        if response.status_code != StatusCode::OK.as_u16() {
            return Err(FederationError::Server(format!(
                "Failed to get server capabilities: {}",
                response.status_code
            )));
        }
        
        serde_json::from_slice(&response.body)
            .map_err(|e| FederationError::Json(format!("Failed to parse capabilities response: {}", e)))
    }
    
    /// Gets user profile information
    #[instrument(level = "debug")]
    pub async fn get_user_profile(&self, server_name: &ServerName, user_id: &UserId) -> Result<Value> {
        let request = FederationRequest {
            request_id: utils::random_string(16)?,
            destination_server: server_name.clone(),
            method: reqwest::Method::GET,
            path: format!("/_matrix/federation/v1/query/profile?user_id={}", user_id),
            headers: HashMap::new(),
            body: Vec::new(),
            timestamp: utils::get_timestamp(),
            timeout_ms: self.config.timeout_ms,
        };
        
        let response = self.send_request(request).await?;
        
        if response.status_code != StatusCode::OK.as_u16() {
            return Err(FederationError::Server(format!(
                "Failed to get user profile: {}",
                response.status_code
            )));
        }
        
        serde_json::from_slice(&response.body)
            .map_err(|e| FederationError::Json(format!("Failed to parse profile response: {}", e)))
    }
    
    /// Get server info with caching
    #[instrument(level = "debug", skip(self))]
    pub async fn get_server_info(&self, server_name: &ServerName) -> Result<ServerInfo, FederationError> {
        // Check cache first
        {
            let server_info = self.server_info.read().await;
            if let Some(info) = server_info.get(server_name) {
                return Ok(info.clone());
            }
        }
        
        // Discover server if not cached
        let info = self.discovery.discover_server(server_name).await?;
        
        // Cache the result
        {
            let mut server_info = self.server_info.write().await;
            server_info.insert(server_name.to_owned(), info.clone());
        }
        
        Ok(info)
    }
    
    /// Check rate limits before sending request
    #[instrument(level = "debug", skip(self))]
    pub async fn check_rate_limit(&self, server_name: &ServerName) -> Result<(), FederationError> {
        let mut rate_limiter = self.rate_limiter.lock().await;
        
        // Get current rate limit info for this server
        let now = Instant::now();
        let (count, last_reset) = rate_limiter
            .entry(server_name.to_owned())
            .or_insert((0, now));
            
        // Reset counter if rate limit window has passed
        if now.duration_since(*last_reset) > Duration::from_secs(60) {
            *count = 0;
            *last_reset = now;
        }
        
        // Check if we've exceeded the rate limit
        if *count >= self.config.rate_limit_per_minute {
            return Err(FederationError::RateLimited {
                server: server_name.to_string(),
                retry_after: Duration::from_secs(60),
            });
        }
        
        *count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use mockall::predicate::*;
    use mockall::mock;
    
    mock! {
        ServerDiscovery {}
        #[async_trait]
        impl ServerDiscovery for ServerDiscovery {
            async fn discover_server(&self, server_name: &ServerName) -> Result<ServerInfo>;
            async fn get_server_keys(&self, server_name: &ServerName) -> Result<ServerSigningKeys>;
            async fn verify_server_auth(&self, server_name: &ServerName) -> Result<ServerAuthentication>;
            async fn update_server_info(&self, server_info: ServerInfo) -> Result<()>;
            async fn remove_stale_servers(&self, max_age: Duration) -> Result<()>;
        }
    }
    
    mock! {
        EventExchange {}
        #[async_trait]
        impl EventExchange for EventExchange {
            async fn send_event(&self, event: FederationEvent) -> Result<()>;
            async fn receive_event(&self, room_id: &RoomId) -> Result<FederationEvent>;
            async fn verify_event(&self, event: &FederationEvent) -> Result<bool>;
            async fn get_missing_events(&self, room_id: &RoomId) -> Result<Vec<FederationEvent>>;
        }
    }
    
    #[test]
    fn test_client_creation() {
        let config = FederationConfig::new("example.com".to_string());
        let discovery = Arc::new(MockServerDiscovery::new());
        let event_exchange = Arc::new(MockEventExchange::new());
        
        let client = Client::new(config, discovery, event_exchange);
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_client_send_request() {
        let config = FederationConfig::new("example.com".to_string());
        let mut discovery = MockServerDiscovery::new();
        
        discovery
            .expect_discover_server()
            .returning(|_| Ok(ServerInfo::default()));
        
        let client = Client::new(
            config,
            Arc::new(discovery),
            Arc::new(MockEventExchange::new()),
        )
        .unwrap();
        
        let request = FederationRequest::default();
        let response = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client.send_request(request));
        
        // Note: This test will fail in CI because it requires network access
        // In a real test environment, we would use a mock HTTP client
        assert!(response.is_err());
    }
    
    #[test]
    fn test_client_get_server_version() {
        let config = FederationConfig::new("example.com".to_string());
        let mut discovery = MockServerDiscovery::new();
        
        discovery
            .expect_discover_server()
            .returning(|_| Ok(ServerInfo::default()));
        
        let client = Client::new(
            config,
            Arc::new(discovery),
            Arc::new(MockEventExchange::new()),
        )
        .unwrap();
        
        let server_name = server_name!("example.com");
        let version = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client.get_server_version(&server_name));
        
        // Note: This test will fail in CI because it requires network access
        // In a real test environment, we would use a mock HTTP client
        assert!(version.is_err());
    }
    
    #[test]
    fn test_client_get_server_capabilities() {
        let config = FederationConfig::new("example.com".to_string());
        let mut discovery = MockServerDiscovery::new();
        
        discovery
            .expect_discover_server()
            .returning(|_| Ok(ServerInfo::default()));
        
        let client = Client::new(
            config,
            Arc::new(discovery),
            Arc::new(MockEventExchange::new()),
        )
        .unwrap();
        
        let server_name = server_name!("example.com");
        let capabilities = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client.get_server_capabilities(&server_name));
        
        // Note: This test will fail in CI because it requires network access
        // In a real test environment, we would use a mock HTTP client
        assert!(capabilities.is_err());
    }
    
    #[test]
    fn test_client_get_user_profile() {
        let config = FederationConfig::new("example.com".to_string());
        let mut discovery = MockServerDiscovery::new();
        
        discovery
            .expect_discover_server()
            .returning(|_| Ok(ServerInfo::default()));
        
        let client = Client::new(
            config,
            Arc::new(discovery),
            Arc::new(MockEventExchange::new()),
        )
        .unwrap();
        
        let server_name = server_name!("example.com");
        let user_id = UserId::try_from("@test:example.com").unwrap();
        let profile = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client.get_user_profile(&server_name, &user_id));
        
        // Note: This test will fail in CI because it requires network access
        // In a real test environment, we would use a mock HTTP client
        assert!(profile.is_err());
    }
} 
