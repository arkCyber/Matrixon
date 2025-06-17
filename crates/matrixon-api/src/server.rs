//! Server API module for Matrixon
//! 
//! This module provides server-side API functionality for the Matrixon server.

use async_trait::async_trait;
use axum::{
    Router,
    routing::get,
    http::StatusCode,
};
use matrixon_core::Result;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::ApiConfig;

/// Server API trait
#[async_trait]
pub trait ServerApi {
    /// Get the server configuration
    fn config(&self) -> &ApiConfig;
    
    /// Start the server
    async fn start(&self) -> Result<()>;
    
    /// Stop the server
    async fn stop(&self) -> Result<()>;
}

/// Server API implementation
#[derive(Debug)]
pub struct Server {
    config: ApiConfig,
    router: Router,
}

impl Server {
    /// Create a new server
    pub fn new(config: ApiConfig) -> Self {
        let router = Router::new()
            .route("/_matrix/client/versions", get(|| async { StatusCode::OK }))
            .layer(TraceLayer::new_for_http());
            
        Self { config, router }
    }
    
    /// Create a new server with default configuration
    pub fn new_default() -> Self {
        Self::new(ApiConfig::default())
    }
}

#[async_trait]
impl ServerApi for Server {
    fn config(&self) -> &ApiConfig {
        &self.config
    }
    
    async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        info!("Starting server on {}", addr);
        
        // TODO: Implement actual server startup
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        info!("Stopping server");
        
        // TODO: Implement actual server shutdown
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let config = ApiConfig::default();
        let server = Server::new(config);
        assert_eq!(server.config().host, "0.0.0.0");
        assert_eq!(server.config().port, 8448);
    }

    #[test]
    fn test_server_default() {
        let server = Server::new_default();
        assert_eq!(server.config().host, "0.0.0.0");
        assert_eq!(server.config().port, 8448);
    }
} 
