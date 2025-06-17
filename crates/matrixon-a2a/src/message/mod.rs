//! A2A Message Implementation
//! 
//! This module implements the core A2A message format.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use crate::error::Error;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, instrument};
use std::time::Instant;

/// Message type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// Data message
    Data,
    /// Control message
    Control,
    /// Error message
    Error,
}

/// Message implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID
    id: Uuid,
    /// Message type
    message_type: MessageType,
    /// Sender ID
    sender: String,
    /// Receiver ID
    receiver: String,
    /// Message content
    content: serde_json::Value,
    /// Metadata
    metadata: serde_json::Value,
    /// Timestamp
    timestamp: DateTime<Utc>,
}

impl Message {
    /// Create a new message
    #[instrument(level = "debug")]
    pub fn new(
        message_type: MessageType,
        sender: String,
        receiver: String,
        content: serde_json::Value,
        metadata: serde_json::Value,
    ) -> Self {
        let start = Instant::now();
        info!("🔧 Creating new message");

        Self {
            id: Uuid::new_v4(),
            message_type,
            sender,
            receiver,
            content,
            metadata,
            timestamp: Utc::now(),
        }
    }

    /// Get message ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get message type
    pub fn message_type(&self) -> &MessageType {
        &self.message_type
    }

    /// Get sender ID
    pub fn sender(&self) -> &str {
        &self.sender
    }

    /// Get receiver ID
    pub fn receiver(&self) -> &str {
        &self.receiver
    }

    /// Get content
    pub fn content(&self) -> &serde_json::Value {
        &self.content
    }

    /// Get metadata
    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    /// Get timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Convert message to bytes
    #[instrument(level = "debug", skip(self))]
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let start = Instant::now();
        info!("🔧 Converting message to bytes");

        serde_json::to_vec(self)
            .map_err(|e| Error::Serialization(e))
    }

    /// Create message from bytes
    #[instrument(level = "debug")]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let start = Instant::now();
        info!("🔧 Creating message from bytes");

        serde_json::from_slice(&bytes)
            .map_err(|e| Error::Serialization(e))
    }
}

/// Message handler trait
#[async_trait::async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle incoming message
    async fn handle_message(&self, message: Message) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let message = Message::new(
            MessageType::Data,
            "sender".to_string(),
            "receiver".to_string(),
            serde_json::json!({"data": "test"}),
            serde_json::json!({}),
        );

        assert_eq!(message.sender(), "sender");
        assert_eq!(message.receiver(), "receiver");
        assert_eq!(message.message_type(), &MessageType::Data);
    }

    #[test]
    fn test_message_serialization() {
        let message = Message::new(
            MessageType::Data,
            "sender".to_string(),
            "receiver".to_string(),
            serde_json::json!({"data": "test"}),
            serde_json::json!({}),
        );

        let bytes = message.to_bytes().unwrap();
        let deserialized = Message::from_bytes(bytes).unwrap();

        assert_eq!(message.id(), deserialized.id());
        assert_eq!(message.sender(), deserialized.sender());
        assert_eq!(message.receiver(), deserialized.receiver());
    }
}
