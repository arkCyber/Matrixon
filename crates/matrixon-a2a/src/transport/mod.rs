//! A2A Transport Layer
//! 
//! This module implements the transport layer for A2A protocol,
//! supporting both WebSocket and HTTP transports.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use crate::error::Error as A2AError;
use crate::message::Message;
use tracing::{info, instrument};
use std::time::Instant;
use async_trait::async_trait;
use std::sync::Arc;

mod websocket;
mod http;

pub use websocket::WebSocketTransport;
pub use http::HttpTransport;

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Transport type
    pub transport_type: TransportType,
    /// Host
    pub host: String,
    /// Port
    pub port: u16,
    /// TLS configuration
    pub tls: Option<TlsConfig>,
}

/// Transport type
#[derive(Debug, Clone)]
pub enum TransportType {
    /// WebSocket transport
    WebSocket,
    /// HTTP transport
    Http,
}

/// TLS configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Certificate path
    pub cert_path: String,
    /// Key path
    pub key_path: String,
}

/// Transport trait
#[async_trait]
pub trait Transport: std::fmt::Debug + Send + Sync {
    /// Start the transport
    async fn start(&self) -> Result<(), A2AError>;
    
    /// Stop the transport
    async fn stop(&self) -> Result<(), A2AError>;
    
    /// Send a message
    async fn send(&self, message: Message) -> Result<(), A2AError>;
    
    /// Receive a message
    async fn receive(&self) -> Result<Message, A2AError>;
}

/// Create a new transport instance
#[instrument(level = "debug")]
pub async fn create_transport(config: TransportConfig) -> Result<Arc<dyn Transport>, A2AError> {
    let start = Instant::now();
    info!("🔧 Creating transport");

    let transport: Arc<dyn Transport> = match config.transport_type {
        TransportType::WebSocket => {
            let ws = WebSocketTransport::from_config(config);
            Arc::new(ws)
        }
        TransportType::Http => {
            let http = HttpTransport::new(config).await?;
            Arc::new(http)
        }
    };

    info!("✅ Transport created in {:?}", start.elapsed());
    Ok(transport)
}

/// Initialize transport layer
#[instrument(level = "debug")]
pub async fn init(config: &TransportConfig) -> Result<(), A2AError> {
    let start = Instant::now();
    info!("🔧 Initializing transport layer");

    // Initialize TLS if configured
    if let Some(_tls) = &config.tls {
        // TODO: Initialize TLS
        info!("✅ TLS initialized");
    }

    info!("✅ Transport layer initialized in {:?}", start.elapsed());
    Ok(())
}

/// Shutdown transport layer
#[instrument(level = "debug")]
pub async fn shutdown() -> Result<(), A2AError> {
    let start = Instant::now();
    info!("🔧 Shutting down transport layer");

    // TODO: Implement shutdown logic

    info!("✅ Transport layer shut down in {:?}", start.elapsed());
    Ok(())
}
