//! A2A State Management
//! 
//! This module implements state management for A2A protocol.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use crate::error::Error;
use tracing::{info, instrument};
use std::time::Instant;
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// State configuration
#[derive(Debug, Clone, Default)]
pub struct StateConfig {
    /// Maximum message history
    pub max_history: usize,
    /// Message timeout in seconds
    pub message_timeout: u64,
    /// Cleanup interval in seconds
    pub cleanup_interval: u64,
}

/// Message state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageState {
    /// Message ID
    pub id: Uuid,
    /// Message type
    pub message_type: String,
    /// Sender ID
    pub sender: String,
    /// Receiver ID
    pub receiver: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Status
    pub status: MessageStatus,
}

/// Message status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(PartialEq)]
pub enum MessageStatus {
    /// Message sent
    Sent,
    /// Message received
    Received,
    /// Message acknowledged
    Acknowledged,
    /// Message failed
    Failed,
}

/// Connection state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Error state
    Error(String),
}

/// State implementation
#[derive(Debug)]
pub struct State {
    /// State configuration
    config: StateConfig,
    /// Message history
    messages: Arc<DashMap<Uuid, MessageState>>,
    /// Connection state
    connection_state: Arc<RwLock<ConnectionState>>,
}

/// State errors
#[derive(Error, Debug)]
pub enum StateError {
    #[error("Message not found: {0}")]
    MessageNotFound(Uuid),
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
    #[error("State initialization failed: {0}")]
    InitializationError(String),
}

impl State {
    /// Create a new state instance
    #[instrument(level = "debug")]
    pub fn new(config: StateConfig) -> Self {
        Self {
            config,
            messages: Arc::new(DashMap::new()),
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
        }
    }

    /// Initialize state
    #[instrument(level = "debug", skip(self))]
    pub async fn init(&self) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Initializing state");

        // Start cleanup task
        self.start_cleanup_task().await?;

        info!("✅ State initialized in {:?}", start.elapsed());
        Ok(())
    }

    /// Cleanup state
    #[instrument(level = "debug", skip(self))]
    pub async fn cleanup(&self) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Cleaning up state");

        // Remove old messages
        let now = Utc::now();
        self.messages.retain(|_, state| {
            let elapsed = (now - state.timestamp).num_seconds();
            elapsed < self.config.message_timeout as i64 && elapsed >= 0
        });

        info!("✅ State cleaned up in {:?}", start.elapsed());
        Ok(())
    }

    /// Update sent message state
    #[instrument(level = "debug", skip(self))]
    pub async fn update_sent(&self, message_id: Uuid) -> Result<(), Error> {
        let _start = Instant::now();
        info!("🔧 Updating sent message state");

        self.messages.entry(message_id)
            .or_insert_with(|| MessageState {
                id: message_id,
                message_type: "unknown".to_string(),
                sender: "unknown".to_string(),
                receiver: "unknown".to_string(),
                timestamp: Utc::now(),
                status: MessageStatus::Sent,
            })
            .value_mut()
            .status = MessageStatus::Sent;

        Ok(())
    }

    /// Update received message state
    #[instrument(level = "debug", skip(self))]
    pub async fn update_received(&self, message_id: Uuid) -> Result<(), Error> {
        let _start = Instant::now();
        info!("🔧 Updating received message state");

        if let Some(mut state) = self.messages.get_mut(&message_id) {
            state.status = MessageStatus::Received;
            Ok(())
        } else {
            Err(StateError::MessageNotFound(message_id).into())
        }
    }

    /// Update connection state
    #[instrument(level = "debug", skip(self))]
    pub async fn update_connection_state(&self, state: ConnectionState) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Updating connection state");

        let mut connection_state = self.connection_state.write().await;
        *connection_state = state;

        info!("✅ Connection state updated in {:?}", start.elapsed());
        Ok(())
    }

    /// Start cleanup task
    #[instrument(level = "debug", skip(self))]
    async fn start_cleanup_task(&self) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Starting cleanup task");

        let messages = self.messages.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(config.cleanup_interval)).await;
                
                let now = Utc::now();
                messages.retain(|_, state| {
                    let elapsed = (now - state.timestamp).num_seconds();
                    elapsed < config.message_timeout as i64 && elapsed >= 0
                });
            }
        });

        info!("✅ Cleanup task started in {:?}", start.elapsed());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_state_initialization() {
        let config = StateConfig {
            max_history: 100,
            message_timeout: 60,
            cleanup_interval: 300,
        };

        let state = State::new(config);
        assert_eq!(state.config.max_history, 100);
        assert_eq!(state.config.message_timeout, 60);
        assert_eq!(state.config.cleanup_interval, 300);
    }

    #[tokio::test]
    async fn test_message_state_updates() {
        let config = StateConfig {
            max_history: 100,
            message_timeout: 60,
            cleanup_interval: 300,
        };

        let state = State::new(config);
        let message_id = Uuid::new_v4();

        // Test initial state
        state.update_sent(message_id).await.unwrap();
        let message_state = state.messages.get(&message_id).unwrap();
        assert_eq!(message_state.status, MessageStatus::Sent);

        // Test state transitions
        state.update_received(message_id).await.unwrap();
        let message_state = state.messages.get(&message_id).unwrap();
        assert_eq!(message_state.status, MessageStatus::Received);

        state.update_connection_state(ConnectionState::Connected).await.unwrap();
        let connection_state = state.connection_state.read().await;
        assert!(matches!(*connection_state, ConnectionState::Connected));
    }

    #[tokio::test]
    async fn test_cleanup_old_messages() {
        let config = StateConfig {
            max_history: 100,
            message_timeout: 1, // Short timeout for testing (1 second)
            cleanup_interval: 1, // Short interval for testing (1 second)
        };

        let state = State::new(config.clone());
        state.init().await.unwrap(); // Ensure cleanup task is running
        let message_id = Uuid::new_v4();

        // Add a message and wait for it to expire
        state.update_sent(message_id).await.unwrap();
        
        // Wait for cleanup task to run at least once
        sleep(Duration::from_secs(config.cleanup_interval + 1)).await;

        // Verify message is cleaned up
        assert!(!state.messages.contains_key(&message_id));
    }

    #[tokio::test]
    async fn test_connection_state() {
        let state = State::new(StateConfig::default());
        
        // Test initial state
        assert!(matches!(*state.connection_state.read().await, ConnectionState::Disconnected));

        // Test state transitions
        state.update_connection_state(ConnectionState::Connecting).await.unwrap();
        assert!(matches!(*state.connection_state.read().await, ConnectionState::Connecting));

        state.update_connection_state(ConnectionState::Connected).await.unwrap();
        assert!(matches!(*state.connection_state.read().await, ConnectionState::Connected));

        state.update_connection_state(ConnectionState::Disconnected).await.unwrap();
        assert!(matches!(*state.connection_state.read().await, ConnectionState::Disconnected));
    }

    #[tokio::test]
    async fn test_error_handling() {
        let state = State::new(StateConfig::default());
        let non_existent_id = Uuid::new_v4();

        // Test updating non-existent message
        let result = state.update_received(non_existent_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::State(StateError::MessageNotFound(_))));
    }
}
