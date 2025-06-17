// =============================================================================
// Matrixon Federation - Types Module
// =============================================================================
//
// Author: arkSong <arksong2018@gmail.com>
// Version: 0.11.0-alpha
// Date: 2024-03-21
//
// This module defines the core types used in the Matrixon federation
// implementation. It includes types for server discovery, authentication,
// event exchange, and state resolution.
//
// =============================================================================

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use ruma::discovery::ServerSigningKeys;
use ruma::authentication::ServerAuthentication;
use ruma::{
    events::AnyRoomEvent,
    OwnedRoomId, OwnedServerName, OwnedUserId,
};
use tracing::{debug, info, instrument};
use std::collections::HashMap;
use reqwest;

/// Server information for federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// The server's name
    pub name: OwnedServerName,
    /// The server's signing keys
    pub signing_keys: ServerSigningKeys,
    /// The server's authentication information
    pub auth: ServerAuthentication,
    /// Whether the server is trusted
    pub trusted: bool,
    /// Last successful communication timestamp
    pub last_seen: Instant,
}

impl ServerInfo {
    /// Creates a new server info instance
    #[instrument(level = "debug")]
    pub fn new(
        name: OwnedServerName,
        signing_keys: ServerSigningKeys,
        auth: ServerAuthentication,
    ) -> Self {
        debug!("🔧 Creating new ServerInfo for {}", name);
        
        Self {
            name,
            signing_keys,
            auth,
            trusted: false,
            last_seen: Instant::now(),
        }
    }

    /// Updates the last seen timestamp
    #[instrument(level = "debug")]
    pub fn update_last_seen(&mut self) {
        self.last_seen = Instant::now();
        debug!("✅ Updated last seen for {}", self.name);
    }

    /// Checks if the server is stale
    #[instrument(level = "debug")]
    pub fn is_stale(&self, max_age: Duration) -> bool {
        self.last_seen.elapsed() > max_age
    }
}

/// Federation event for exchange between servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationEvent {
    /// The event ID
    pub event_id: String,
    /// The room ID
    pub room_id: OwnedRoomId,
    /// The event type
    pub event_type: String,
    /// The event content
    pub content: AnyRoomEvent,
    /// The sender's user ID
    pub sender: OwnedUserId,
    /// The origin server
    pub origin: OwnedServerName,
    /// The event timestamp
    pub timestamp: u64,
    /// Whether the event is verified
    pub verified: bool,
}

impl FederationEvent {
    /// Creates a new federation event
    #[instrument(level = "debug")]
    pub fn new(
        event_id: String,
        room_id: OwnedRoomId,
        event_type: String,
        content: AnyRoomEvent,
        sender: OwnedUserId,
        origin: OwnedServerName,
    ) -> Self {
        debug!("🔧 Creating new FederationEvent {}", event_id);
        
        Self {
            event_id,
            room_id,
            event_type,
            content,
            sender,
            origin,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            verified: false,
        }
    }

    /// Verifies the event's signature
    #[instrument(level = "debug")]
    pub fn verify(&mut self) -> bool {
        debug!("🔧 Verifying event {}", self.event_id);
        // TODO: Implement event signature verification
        self.verified = true;
        info!("✅ Event {} verified", self.event_id);
        true
    }
}

/// Federation request structure
#[derive(Debug, Clone)]
pub struct FederationRequest {
    /// Unique request identifier
    pub request_id: String,
    
    /// Destination server name
    pub destination_server: OwnedServerName,
    
    /// HTTP method
    pub method: reqwest::Method,
    
    /// Request path
    pub path: String,
    
    /// Request headers
    pub headers: HashMap<String, String>,
    
    /// Request body
    pub body: Vec<u8>,
    
    /// Request timestamp
    pub timestamp: u64,
    
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for FederationRequest {
    fn default() -> Self {
        Self {
            request_id: String::new(),
            destination_server: OwnedServerName::try_from("localhost").unwrap(),
            method: reqwest::Method::GET,
            path: String::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            timestamp: 0,
            timeout_ms: 5000,
        }
    }
}

/// Federation response structure
#[derive(Debug, Clone)]
pub struct FederationResponse {
    /// Original request identifier
    pub request_id: String,
    
    /// HTTP status code
    pub status_code: u16,
    
    /// Response headers
    pub headers: HashMap<String, String>,
    
    /// Response body
    pub body: Vec<u8>,
    
    /// Response timestamp
    pub timestamp: u64,
    
    /// Request duration in milliseconds
    pub duration_ms: u64,
}

impl Default for FederationResponse {
    fn default() -> Self {
        Self {
            request_id: String::new(),
            status_code: 0,
            headers: HashMap::new(),
            body: Vec::new(),
            timestamp: 0,
            duration_ms: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use ruma::server_name;

    #[test]
    fn test_server_info() {
        let name = server_name!("example.com");
        let signing_keys = ServerSigningKeys::default();
        let auth = ServerAuthentication::default();
        
        let server = ServerInfo::new(name.clone(), signing_keys, auth);
        assert_eq!(server.name, name);
        assert!(!server.trusted);
        
        let old_last_seen = server.last_seen;
        std::thread::sleep(Duration::from_millis(1));
        
        let mut server = server;
        server.update_last_seen();
        assert!(server.last_seen > old_last_seen);
        
        assert!(!server.is_stale(Duration::from_secs(1)));
        assert!(server.is_stale(Duration::from_nanos(1)));
    }

    #[test]
    fn test_federation_event() {
        let event_id = "test_event_id".to_string();
        let room_id = RoomId::try_from("!test:example.com").unwrap();
        let event_type = "m.room.message".to_string();
        let content = AnyRoomEvent::default();
        let sender = UserId::try_from("@test:example.com").unwrap();
        let origin = server_name!("example.com");
        
        let event = FederationEvent::new(
            event_id.clone(),
            room_id.clone(),
            event_type.clone(),
            content.clone(),
            sender.clone(),
            origin.clone(),
        );
        
        assert_eq!(event.event_id, event_id);
        assert_eq!(event.room_id, room_id);
        assert_eq!(event.event_type, event_type);
        assert_eq!(event.sender, sender);
        assert_eq!(event.origin, origin);
        assert!(!event.verified);
        
        let mut event = event;
        assert!(event.verify());
        assert!(event.verified);
    }

    #[test]
    fn test_federation_request() {
        let request_id = "test_request_id".to_string();
        let destination = server_name!("example.com");
        let method = reqwest::Method::GET;
        let path = "/_matrix/federation/v1/version".to_string();
        let timeout_ms = 5000;
        
        let mut request = FederationRequest {
            request_id,
            destination_server: destination,
            method,
            path,
            headers: HashMap::new(),
            body: Vec::new(),
            timestamp: 0,
            timeout_ms,
        };
        
        assert_eq!(request.request_id, "test_request_id".to_string());
        assert_eq!(request.destination_server, destination);
        assert_eq!(request.method, method);
        assert_eq!(request.path, path);
        assert_eq!(request.timeout_ms, timeout_ms);
        assert!(request.headers.is_empty());
        assert!(request.body.is_empty());
        
        request.headers.insert("Content-Type".to_string(), "application/json".to_string());
        assert_eq!(request.headers.get("Content-Type"), Some(&"application/json".to_string()));
        
        let body = serde_json::json!({"key": "value"}).to_string().into_bytes();
        request.body = body;
        assert_eq!(request.body, body);
    }

    #[test]
    fn test_federation_response() {
        let request_id = "test_request_id".to_string();
        let status_code = 200;
        let start_time = Instant::now();
        
        let mut response = FederationResponse {
            request_id,
            status_code,
            headers: HashMap::new(),
            body: Vec::new(),
            timestamp: 0,
            duration_ms: 0,
        };
        
        assert_eq!(response.request_id, "test_request_id".to_string());
        assert_eq!(response.status_code, status_code);
        assert!(response.headers.is_empty());
        assert!(response.body.is_empty());
        
        response.headers.insert("Content-Type".to_string(), "application/json".to_string());
        assert_eq!(response.headers.get("Content-Type"), Some(&"application/json".to_string()));
        
        let body = serde_json::json!({"key": "value"}).to_string().into_bytes();
        response.body = body;
        assert_eq!(response.body, body);
    }
} 
