//! Database models for Matrixon
//! 
//! This module defines the database models used throughout the Matrixon system.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use matrixon_core::types::{MatrixonUserId, MatrixonRoomId};

/// User model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: Uuid,
    
    /// Username
    pub username: String,
    
    /// Password hash
    pub password_hash: String,
    
    /// Email
    pub email: Option<String>,
    
    /// Created at
    pub created_at: DateTime<Utc>,
    
    /// Updated at
    pub updated_at: DateTime<Utc>,
}

/// Room model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// Room ID
    pub id: Uuid,
    
    /// Room alias
    pub alias: String,
    
    /// Room name
    pub name: String,
    
    /// Room topic
    pub topic: Option<String>,
    
    /// Room creator
    pub creator: MatrixonUserId,
    
    /// Created at
    pub created_at: DateTime<Utc>,
    
    /// Updated at
    pub updated_at: DateTime<Utc>,
}

/// Event model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event ID
    pub id: Uuid,
    
    /// Room ID
    pub room_id: MatrixonRoomId,
    
    /// Event type
    pub event_type: String,
    
    /// Event content
    pub content: serde_json::Value,
    
    /// Sender
    pub sender: MatrixonUserId,
    
    /// Origin server
    pub origin_server_ts: DateTime<Utc>,
    
    /// Created at
    pub created_at: DateTime<Utc>,
}

/// Device model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Device ID
    pub id: String,
    
    /// User ID
    pub user_id: MatrixonUserId,
    
    /// Display name
    pub display_name: Option<String>,
    
    /// Last seen IP
    pub last_seen_ip: Option<String>,
    
    /// Last seen timestamp
    pub last_seen_ts: Option<DateTime<Utc>>,
    
    /// Created at
    pub created_at: DateTime<Utc>,
    
    /// Updated at
    pub updated_at: DateTime<Utc>,
}

/// Test event model for benchmarks and tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEvent {
    /// Event ID
    pub id: Uuid,
    
    /// Room ID
    pub room_id: Uuid,
    
    /// Event content
    pub content: String,
    
    /// Created at
    pub created_at: DateTime<Utc>,
}

impl Default for TestEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl TestEvent {
    /// Create a new test event with random data
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            content: "Test event content".to_string(),
            created_at: Utc::now(),
        }
    }

    /// Create a new test event with specific content
    pub fn new_test_data() -> Self {
        Self {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            content: "Test event content for integration tests".to_string(),
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: Uuid::new_v4(),
            username: "test_user".to_string(),
            password_hash: "hash".to_string(),
            email: Some("test@example.com".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let serialized = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&serialized).unwrap();

        assert_eq!(user.username, deserialized.username);
        assert_eq!(user.email, deserialized.email);
    }

    #[test]
    fn test_room_serialization() {
        let room = Room {
            id: Uuid::new_v4(),
            alias: "test_room".to_string(),
            name: "Test Room".to_string(),
            topic: Some("Test Topic".to_string()),
            creator: MatrixonUserId {
                id: Uuid::new_v4(),
                username: "test_user".to_string(),
                domain: "matrixon.local".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let serialized = serde_json::to_string(&room).unwrap();
        let deserialized: Room = serde_json::from_str(&serialized).unwrap();

        assert_eq!(room.alias, deserialized.alias);
        assert_eq!(room.name, deserialized.name);
        assert_eq!(room.topic, deserialized.topic);
    }
}
