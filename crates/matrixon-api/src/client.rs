//! Client API module for Matrixon
//! 
//! This module provides client-side API functionality for interacting with
//! Matrixon servers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Client API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Server URL
    pub server_url: String,
    
    /// Access token
    pub access_token: Option<String>,
    
    /// Device ID
    pub device_id: Option<String>,
}

/// Client API trait
#[async_trait]
pub trait ClientApi {
    /// Get the client configuration
    fn config(&self) -> &ClientConfig;
    
    /// Set the access token
    fn set_access_token(&mut self, token: String);
    
    /// Get the access token
    fn access_token(&self) -> Option<&str>;
    
    /// Get the device ID
    fn device_id(&self) -> Option<&str>;
}

/// Client API implementation
#[derive(Debug)]
pub struct Client {
    config: ClientConfig,
}

impl Client {
    /// Create a new client
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }
    
    /// Create a new client with default configuration
    pub fn new_default(server_url: String) -> Self {
        Self {
            config: ClientConfig {
                server_url,
                access_token: None,
                device_id: None,
            },
        }
    }
}

#[async_trait]
impl ClientApi for Client {
    fn config(&self) -> &ClientConfig {
        &self.config
    }
    
    fn set_access_token(&mut self, token: String) {
        self.config.access_token = Some(token);
    }
    
    fn access_token(&self) -> Option<&str> {
        self.config.access_token.as_deref()
    }
    
    fn device_id(&self) -> Option<&str> {
        self.config.device_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = ClientConfig {
            server_url: "https://matrixon.local".to_string(),
            access_token: Some("test_token".to_string()),
            device_id: Some("test_device".to_string()),
        };
        
        let client = Client::new(config);
        assert_eq!(client.config().server_url, "https://matrixon.local");
        assert_eq!(client.access_token(), Some("test_token"));
        assert_eq!(client.device_id(), Some("test_device"));
    }

    #[test]
    fn test_client_default() {
        let client = Client::new_default("https://matrixon.local".to_string());
        assert_eq!(client.config().server_url, "https://matrixon.local");
        assert!(client.access_token().is_none());
        assert!(client.device_id().is_none());
    }
} 
