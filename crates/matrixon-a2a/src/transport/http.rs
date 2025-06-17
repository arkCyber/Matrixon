//! HTTP Transport Implementation
//! 
//! This module implements HTTP transport for A2A protocol.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use super::{Transport, TransportConfig};
use crate::error::Error;
use crate::message::Message;
use tracing::{info, instrument};
use std::time::Instant;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use reqwest::Client;
use serde_json;

/// HTTP transport implementation
#[derive(Debug)]
pub struct HttpTransport {
    /// Transport configuration
    config: TransportConfig,
    /// HTTP client
    client: Arc<Client>,
    /// Message queue
    message_queue: Arc<RwLock<Vec<Message>>>,
}

impl HttpTransport {
    /// Create a new HTTP transport
    #[instrument(level = "debug")]
    pub async fn new(config: TransportConfig) -> Result<Self, Error> {
        let start = Instant::now();
        info!("🔧 Creating HTTP transport");

        let client = Client::new();
        let transport = Self {
            config,
            client: Arc::new(client),
            message_queue: Arc::new(RwLock::new(Vec::new())),
        };

        info!("✅ HTTP transport created in {:?}", start.elapsed());
        Ok(transport)
    }

    /// Get base URL
    fn base_url(&self) -> String {
        format!("http://{}:{}", self.config.host, self.config.port)
    }
}

#[async_trait]
impl Transport for HttpTransport {
    /// Start the HTTP transport
    #[instrument(level = "debug", skip(self))]
    async fn start(&self) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Starting HTTP transport");

        // HTTP transport doesn't need explicit start
        info!("✅ HTTP transport started in {:?}", start.elapsed());
        Ok(())
    }

    /// Stop the HTTP transport
    #[instrument(level = "debug", skip(self))]
    async fn stop(&self) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Stopping HTTP transport");

        // HTTP transport doesn't need explicit stop
        info!("✅ HTTP transport stopped in {:?}", start.elapsed());
        Ok(())
    }

    /// Send a message through HTTP
    #[instrument(level = "debug", skip(self, message))]
    async fn send(&self, message: Message) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Sending HTTP message");

        let url = format!("{}/message", self.base_url());
        let response = self.client
            .post(&url)
            .json(&message)
            .send()
            .await
            .map_err(|e| Error::Connection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::Protocol(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        info!("✅ HTTP message sent in {:?}", start.elapsed());
        Ok(())
    }

    /// Receive a message from HTTP
    #[instrument(level = "debug", skip(self))]
    async fn receive(&self) -> Result<Message, Error> {
        let start = Instant::now();
        info!("🔧 Receiving HTTP message");

        let url = format!("{}/message", self.base_url());
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Connection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::Protocol(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        let message = response
            .json::<Message>()
            .await
            .map_err(|e| Error::Serialization(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("HTTP deserialization error: {}", e)))))?;

        info!("✅ HTTP message received in {:?}", start.elapsed());
        Ok(message)
    }
}
