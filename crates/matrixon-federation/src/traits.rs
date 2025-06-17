// =============================================================================
// Matrixon Federation - Traits Module
// =============================================================================
//
// Author: arkSong <arksong2018@gmail.com>
// Version: 0.11.0-alpha
// Date: 2024-03-21
//
// This module defines the core traits used in the Matrixon federation
// implementation. It includes traits for server discovery, authentication,
// event exchange, and state resolution.
//
// =============================================================================

use std::time::Duration;
use async_trait::async_trait;
use ruma::{
    api::federation::{
        discovery::ServerSigningKeys,
        authentication::ServerAuthentication,
    },
    events::AnyRoomEvent,
    RoomId, OwnedServerName, UserId,
};
use tracing::{debug, info, instrument};

use crate::{
    error::Result,
    types::{ServerInfo, FederationEvent, FederationRequest, FederationResponse},
};

/// Trait for server discovery and key management
#[async_trait]
pub trait ServerDiscovery: Send + Sync {
    /// Discovers a server's information
    #[instrument(level = "debug")]
    async fn discover_server(&self, server_name: &OwnedServerName) -> Result<ServerInfo>;

    /// Gets a server's signing keys
    #[instrument(level = "debug")]
    async fn get_server_keys(&self, server_name: &OwnedServerName) -> Result<ServerSigningKeys>;

    /// Verifies a server's authentication
    #[instrument(level = "debug")]
    async fn verify_server_auth(&self, server_name: &OwnedServerName) -> Result<ServerAuthentication>;

    /// Updates server information
    #[instrument(level = "debug")]
    async fn update_server_info(&self, server_info: ServerInfo) -> Result<()>;

    /// Removes stale server information
    #[instrument(level = "debug")]
    async fn remove_stale_servers(&self, max_age: Duration) -> Result<()>;
}

/// Trait for event exchange between servers
#[async_trait]
pub trait EventExchange: Send + Sync {
    /// Sends an event to a remote server
    #[instrument(level = "debug")]
    async fn send_event(&self, event: FederationEvent) -> Result<()>;

    /// Receives an event from a remote server
    #[instrument(level = "debug")]
    async fn receive_event(&self, room_id: &RoomId) -> Result<FederationEvent>;

    /// Verifies an event's signature
    #[instrument(level = "debug")]
    async fn verify_event(&self, event: &FederationEvent) -> Result<bool>;

    /// Gets missing events for a room
    #[instrument(level = "debug")]
    async fn get_missing_events(&self, room_id: &RoomId) -> Result<Vec<FederationEvent>>;
}

/// Trait for state resolution between servers
#[async_trait]
pub trait StateResolution: Send + Sync {
    /// Gets the current state of a room
    #[instrument(level = "debug")]
    async fn get_room_state(&self, room_id: &RoomId) -> Result<Vec<FederationEvent>>;

    /// Resolves state conflicts between servers
    #[instrument(level = "debug")]
    async fn resolve_state_conflicts(&self, room_id: &RoomId) -> Result<Vec<FederationEvent>>;

    /// Applies resolved state to a room
    #[instrument(level = "debug")]
    async fn apply_resolved_state(&self, room_id: &RoomId, events: Vec<FederationEvent>) -> Result<()>;
}

/// Trait for federation request handling
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Handles an incoming federation request
    #[instrument(level = "debug")]
    async fn handle_request(&self, request: FederationRequest) -> Result<FederationResponse>;

    /// Validates a federation request
    #[instrument(level = "debug")]
    async fn validate_request(&self, request: &FederationRequest) -> Result<()>;

    /// Processes a federation request
    #[instrument(level = "debug")]
    async fn process_request(&self, request: FederationRequest) -> Result<FederationResponse>;
}

/// Trait for federation client operations
#[async_trait]
pub trait FederationClient: Send + Sync {
    /// Sends a request to a remote server
    #[instrument(level = "debug")]
    async fn send_request(&self, request: FederationRequest) -> Result<FederationResponse>;

    /// Gets server version information
    #[instrument(level = "debug")]
    async fn get_server_version(&self, server_name: &OwnedServerName) -> Result<String>;

    /// Gets server capabilities
    #[instrument(level = "debug")]
    async fn get_server_capabilities(&self, server_name: &OwnedServerName) -> Result<serde_json::Value>;

    /// Gets user profile information
    #[instrument(level = "debug")]
    async fn get_user_profile(&self, server_name: &OwnedServerName, user_id: &UserId) -> Result<serde_json::Value>;
}

/// Trait for federation server operations
#[async_trait]
pub trait FederationServer: Send + Sync {
    /// Starts the federation server
    #[instrument(level = "debug")]
    async fn start(&self) -> Result<()>;

    /// Stops the federation server
    #[instrument(level = "debug")]
    async fn stop(&self) -> Result<()>;

    /// Gets server statistics
    #[instrument(level = "debug")]
    async fn get_stats(&self) -> Result<serde_json::Value>;

    /// Gets server health status
    #[instrument(level = "debug")]
    async fn get_health(&self) -> Result<serde_json::Value>;
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
            async fn discover_server(&self, server_name: &OwnedServerName) -> Result<ServerInfo>;
            async fn get_server_keys(&self, server_name: &OwnedServerName) -> Result<ServerSigningKeys>;
            async fn verify_server_auth(&self, server_name: &OwnedServerName) -> Result<ServerAuthentication>;
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

    mock! {
        StateResolution {}
        #[async_trait]
        impl StateResolution for StateResolution {
            async fn get_room_state(&self, room_id: &RoomId) -> Result<Vec<FederationEvent>>;
            async fn resolve_state_conflicts(&self, room_id: &RoomId) -> Result<Vec<FederationEvent>>;
            async fn apply_resolved_state(&self, room_id: &RoomId, events: Vec<FederationEvent>) -> Result<()>;
        }
    }

    mock! {
        RequestHandler {}
        #[async_trait]
        impl RequestHandler for RequestHandler {
            async fn handle_request(&self, request: FederationRequest) -> Result<FederationResponse>;
            async fn validate_request(&self, request: &FederationRequest) -> Result<()>;
            async fn process_request(&self, request: FederationRequest) -> Result<FederationResponse>;
        }
    }

    mock! {
        FederationClient {}
        #[async_trait]
        impl FederationClient for FederationClient {
            async fn send_request(&self, request: FederationRequest) -> Result<FederationResponse>;
            async fn get_server_version(&self, server_name: &OwnedServerName) -> Result<String>;
            async fn get_server_capabilities(&self, server_name: &OwnedServerName) -> Result<serde_json::Value>;
            async fn get_user_profile(&self, server_name: &OwnedServerName, user_id: &UserId) -> Result<serde_json::Value>;
        }
    }

    mock! {
        FederationServer {}
        #[async_trait]
        impl FederationServer for FederationServer {
            async fn start(&self) -> Result<()>;
            async fn stop(&self) -> Result<()>;
            async fn get_stats(&self) -> Result<serde_json::Value>;
            async fn get_health(&self) -> Result<serde_json::Value>;
        }
    }

    #[test]
    fn test_server_discovery_trait() {
        let mut mock = MockServerDiscovery::new();
        let server_name = OwnedServerName::try_from("example.com").unwrap();
        
        mock.expect_discover_server()
            .with(eq(server_name.clone()))
            .returning(|_| Ok(ServerInfo::default()));
        
        mock.expect_get_server_keys()
            .with(eq(server_name.clone()))
            .returning(|_| Ok(ServerSigningKeys::default()));
        
        mock.expect_verify_server_auth()
            .with(eq(server_name.clone()))
            .returning(|_| Ok(ServerAuthentication::default()));
        
        mock.expect_update_server_info()
            .returning(|_| Ok(()));
        
        mock.expect_remove_stale_servers()
            .returning(|_| Ok(()));
    }

    #[test]
    fn test_event_exchange_trait() {
        let mut mock = MockEventExchange::new();
        let room_id = RoomId::try_from("!test:example.com").unwrap();
        
        mock.expect_send_event()
            .returning(|_| Ok(()));
        
        mock.expect_receive_event()
            .with(eq(room_id.clone()))
            .returning(|_| Ok(FederationEvent::default()));
        
        mock.expect_verify_event()
            .returning(|_| Ok(true));
        
        mock.expect_get_missing_events()
            .with(eq(room_id.clone()))
            .returning(|_| Ok(vec![]));
    }

    #[test]
    fn test_state_resolution_trait() {
        let mut mock = MockStateResolution::new();
        let room_id = RoomId::try_from("!test:example.com").unwrap();
        
        mock.expect_get_room_state()
            .with(eq(room_id.clone()))
            .returning(|_| Ok(vec![]));
        
        mock.expect_resolve_state_conflicts()
            .with(eq(room_id.clone()))
            .returning(|_| Ok(vec![]));
        
        mock.expect_apply_resolved_state()
            .returning(|_, _| Ok(()));
    }

    #[test]
    fn test_request_handler_trait() {
        let mut mock = MockRequestHandler::new();
        let request = FederationRequest::default();
        
        mock.expect_handle_request()
            .returning(|_| Ok(FederationResponse::default()));
        
        mock.expect_validate_request()
            .returning(|_| Ok(()));
        
        mock.expect_process_request()
            .returning(|_| Ok(FederationResponse::default()));
    }

    #[test]
    fn test_federation_client_trait() {
        let mut mock = MockFederationClient::new();
        let server_name = OwnedServerName::try_from("example.com").unwrap();
        let user_id = UserId::try_from("@test:example.com").unwrap();
        
        mock.expect_send_request()
            .returning(|_| Ok(FederationResponse::default()));
        
        mock.expect_get_server_version()
            .with(eq(server_name.clone()))
            .returning(|_| Ok("1.0.0".to_string()));
        
        mock.expect_get_server_capabilities()
            .with(eq(server_name.clone()))
            .returning(|_| Ok(serde_json::json!({})));
        
        mock.expect_get_user_profile()
            .with(eq(server_name.clone()), eq(user_id.clone()))
            .returning(|_, _| Ok(serde_json::json!({})));
    }

    #[test]
    fn test_federation_server_trait() {
        let mut mock = MockFederationServer::new();
        
        mock.expect_start()
            .returning(|| Ok(()));
        
        mock.expect_stop()
            .returning(|| Ok(()));
        
        mock.expect_get_stats()
            .returning(|| Ok(serde_json::json!({})));
        
        mock.expect_get_health()
            .returning(|| Ok(serde_json::json!({})));
    }
}
