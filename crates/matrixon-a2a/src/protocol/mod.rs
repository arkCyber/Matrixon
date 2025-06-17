//! A2A Protocol Implementation
//! 
//! This module implements the core A2A protocol logic.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use crate::error::Error;
use crate::message::{Message, MessageType};
use tracing::{info, instrument};
use std::time::Instant;
use async_trait::async_trait;
use tokio::sync::RwLock;
use std::sync::Arc;
use std::time::Duration;

/// Protocol configuration
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    /// Protocol version
    pub version: String,
    /// Protocol timeout
    pub timeout: Duration,
    /// Maximum message size
    pub max_message_size: usize,
    /// Enable compression
    pub compression_enabled: bool,
    /// Enable encryption
    pub encryption_enabled: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry interval in milliseconds
    pub retry_interval: u64,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            timeout: Duration::from_secs(30),
            max_message_size: 1024 * 1024, // 1MB
            compression_enabled: true,
            encryption_enabled: true,
            max_retries: 3,
            retry_interval: 1000,
        }
    }
}

/// Protocol state
#[derive(Debug)]
pub enum ProtocolState {
    /// Initial state
    Initial,
    /// Connected state
    Connected,
    /// Error state
    Error(String),
}

/// Protocol implementation
#[derive(Debug)]
pub struct Protocol {
    /// Protocol configuration
    config: ProtocolConfig,
    /// Current state
    state: Arc<RwLock<ProtocolState>>,
}

impl Protocol {
    /// Create a new protocol instance
    #[instrument(level = "debug")]
    pub fn new(config: ProtocolConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ProtocolState::Initial)),
        }
    }

    /// Initialize protocol
    #[instrument(level = "debug", skip(self))]
    pub async fn init(&self) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Initializing protocol");

        let mut state = self.state.write().await;
        *state = ProtocolState::Connected;

        info!("✅ Protocol initialized in {:?}", start.elapsed());
        Ok(())
    }

    /// Process incoming message
    #[instrument(level = "debug", skip(self, message))]
    pub async fn process_message(&self, message: Message) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Processing protocol message");

        match message.message_type() {
            MessageType::Data => {
                info!("Processing data message");
            }
            MessageType::Control => {
                info!("Processing control message");
            }
            MessageType::Error => {
                let mut state = self.state.write().await;
                *state = ProtocolState::Error("Received error message".to_string());
            }
        }

        info!("✅ Message processed in {:?}", start.elapsed());
        Ok(())
    }
}

#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// Handle protocol message
    async fn handle_message(&self, message: Message) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_protocol_initialization() {
        let config = ProtocolConfig {
            version: "1.0".to_string(),
            timeout: Duration::from_millis(5000),
            max_message_size: 1024 * 1024,
            compression_enabled: true,
            encryption_enabled: true,
            max_retries: 3,
            retry_interval: 1000,
        };

        let protocol = Protocol::new(config);
        assert!(protocol.init().await.is_ok());

        let state = protocol.state.read().await;
        assert!(matches!(*state, ProtocolState::Connected));
    }

    #[tokio::test]
    async fn test_protocol() {
        let config = ProtocolConfig {
            version: "1.0".to_string(),
            timeout: Duration::from_secs(30),
            max_message_size: 1024 * 1024,
            compression_enabled: true,
            encryption_enabled: true,
            max_retries: 3,
            retry_interval: 1000,
        };
        let protocol = Protocol::new(config);

        let message = Message::new(
            MessageType::Data,
            "sender".to_string(),
            "receiver".to_string(),
            serde_json::json!({"data": "test"}),
            serde_json::json!({}),
        );

        let result = protocol.process_message(message).await;
        assert!(result.is_ok());
    }
}
