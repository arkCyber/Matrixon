//! Core traits for Matrixon
//! 
//! This module defines the fundamental traits used throughout the Matrixon system.
//! These traits provide the core interfaces for implementing Matrix protocol
//! functionality in a modular and extensible way.

use async_trait::async_trait;
use crate::{Result, types::{MatrixonUserId, MatrixonRoomId, EventId, MessageContent}};
use std::time::Duration;

/// Trait for services that can be started and stopped
#[async_trait]
pub trait Service {
    /// Start the service
    async fn start(&self) -> Result<()> {
        Ok(())
    }
    
    /// Stop the service
    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

/// Trait for components that need initialization
#[async_trait]
pub trait Initializable {
    /// Initialize the component
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }
}

/// Trait for components that can be configured
#[async_trait]
pub trait Configurable {
    type Config: Send + Sync;
    
    /// Configure the component
    async fn configure(&mut self, _config: Self::Config) -> Result<()> {
        Ok(())
    }
}

/// Trait for components that can be monitored
#[async_trait]
pub trait Monitorable {
    /// Get the current status of the component
    async fn status(&self) -> Result<String> {
        Ok("ok".to_string())
    }
    
    /// Get metrics for the component
    async fn metrics(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

/// Trait for user management
#[async_trait]
pub trait UserManager {
    /// Create a new user
    async fn create_user(&self, _username: &str, _password: &str) -> Result<MatrixonUserId> {
        unimplemented!()
    }
    
    /// Get a user by ID
    async fn get_user(&self, _user_id: &MatrixonUserId) -> Result<MatrixonUserId> {
        unimplemented!()
    }
    
    /// Update a user
    async fn update_user(&self, _user_id: &MatrixonUserId) -> Result<()> {
        unimplemented!()
    }
    
    /// Delete a user
    async fn delete_user(&self, _user_id: &MatrixonUserId) -> Result<()> {
        unimplemented!()
    }
}

/// Trait for room management
#[async_trait]
pub trait RoomManager {
    /// Create a new room
    async fn create_room(&self, _creator: &MatrixonUserId, _name: &str) -> Result<MatrixonRoomId> {
        unimplemented!()
    }
    
    /// Get a room by ID
    async fn get_room(&self, _room_id: &MatrixonRoomId) -> Result<MatrixonRoomId> {
        unimplemented!()
    }
    
    /// Update a room
    async fn update_room(&self, _room_id: &MatrixonRoomId) -> Result<()> {
        unimplemented!()
    }
    
    /// Delete a room
    async fn delete_room(&self, _room_id: &MatrixonRoomId) -> Result<()> {
        unimplemented!()
    }
}

/// Trait for event handling
#[async_trait]
pub trait EventHandler {
    /// Send an event
    async fn send_event(&self, _room_id: &MatrixonRoomId, _event_type: &str, _content: serde_json::Value) -> Result<EventId> {
        unimplemented!()
    }
    
    /// Get an event by ID
    async fn get_event(&self, _event_id: &EventId) -> Result<serde_json::Value> {
        unimplemented!()
    }
    
    /// Get events for a room
    async fn get_room_events(&self, _room_id: &MatrixonRoomId, _limit: usize) -> Result<Vec<EventId>> {
        unimplemented!()
    }
}

/// Trait for message handling
#[async_trait]
pub trait MessageHandler {
    /// Send a message
    async fn send_message(&self, _room_id: &MatrixonRoomId, _sender: &MatrixonUserId, _content: MessageContent) -> Result<EventId> {
        unimplemented!()
    }
    
    /// Get messages for a room
    async fn get_room_messages(&self, _room_id: &MatrixonRoomId, _limit: usize) -> Result<Vec<MessageContent>> {
        unimplemented!()
    }
}

/// Trait for federation
#[async_trait]
pub trait FederationHandler {
    /// Send a PDU to a remote server
    async fn send_pdu(&self, _server_name: &str, _pdu: serde_json::Value) -> Result<()> {
        unimplemented!()
    }
    
    /// Get PDUs from a remote server
    async fn get_pdus(&self, _server_name: &str, _since: Duration) -> Result<Vec<serde_json::Value>> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use mockall::predicate::*;
    use mockall::mock;

    mock! {
        Service {}
        #[async_trait]
        impl Service for Service {
            async fn start(&self) -> Result<()>;
            async fn stop(&self) -> Result<()>;
        }
    }

    #[test]
    fn test_service_trait() {
        let mut mock = MockService::new();
        mock.expect_start()
            .times(1)
            .returning(|| Ok(()));
        mock.expect_stop()
            .times(1)
            .returning(|| Ok(()));
    }
} 
