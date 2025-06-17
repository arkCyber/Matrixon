//! Core types for Matrixon
//! 
//! This module defines the fundamental types used throughout the Matrixon ecosystem.
//! These types are designed to be shared across all crates in the project.
//! 
//! The types in this module are based on the Matrix protocol specification
//! and provide a Rust-native interface for working with Matrix concepts.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use ruma::{
    OwnedRoomId,
    OwnedUserId,
    RoomId,
    UserId,
    EventId as RumaEventId,
};
use std::time::{Duration, SystemTime};
use tracing::instrument;
use std::fmt;

/// A unique identifier for a PDU (Protocol Data Unit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PduId(pub u64);

/// A count of PDUs, used for pagination and ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PduCount(pub u64);

/// A timestamp for events and PDUs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EventTimestamp(pub SystemTime);

/// A duration for timeouts and retries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryAfter(pub Duration);

impl RetryAfter {
    /// Create a new RetryAfter with the specified duration
    #[instrument(level = "debug")]
    pub fn new(duration: Duration) -> Self {
        Self(duration)
    }

    /// Get the duration as seconds
    pub fn as_secs(&self) -> u64 {
        self.0.as_secs()
    }
}

/// A room version identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomVersion(pub String);

impl RoomVersion {
    /// Create a new room version
    #[instrument(level = "debug")]
    pub fn new<T: Into<String> + std::fmt::Debug>(version: T) -> Self {
        Self(version.into())
    }

    /// Get the version string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Matrixon user identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MatrixonUserId {
    /// Unique identifier
    pub id: Uuid,
    
    /// Username
    pub username: String,
    
    /// Domain
    pub domain: String,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl From<OwnedUserId> for MatrixonUserId {
    fn from(ruma_id: OwnedUserId) -> Self {
        let id_str = ruma_id.as_str();
        let parts: Vec<&str> = id_str.split(':').collect();
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            username: parts[0].to_string(),
            domain: parts[1].to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl fmt::Display for MatrixonUserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}:{}", self.username, self.domain)
    }
}

/// Matrixon room identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MatrixonRoomId {
    /// Unique identifier
    pub id: Uuid,
    
    /// Room alias
    pub alias: String,
    
    /// Domain
    pub domain: String,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl From<OwnedRoomId> for MatrixonRoomId {
    fn from(ruma_id: OwnedRoomId) -> Self {
        let id_str = ruma_id.as_str();
        let parts: Vec<&str> = id_str.split(':').collect();
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            alias: parts[0].to_string(),
            domain: parts[1].to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl fmt::Display for MatrixonRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "!{}:{}", self.alias, self.domain)
    }
}

/// Event identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EventId {
    /// Unique identifier
    pub id: Uuid,
    
    /// Room ID
    pub room_id: MatrixonRoomId,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Event type
    pub event_type: String,
    
    /// Sender
    pub sender: MatrixonUserId,
}

impl From<&RumaEventId> for EventId {
    #[instrument(level = "debug")]
    fn from(ruma_id: &RumaEventId) -> Self {
        Self {
            id: Uuid::new_v4(),
            room_id: MatrixonRoomId {
                id: Uuid::new_v4(),
                alias: "dummy".to_string(),
                domain: "matrixon.local".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            timestamp: Utc::now(),
            event_type: "m.room.message".to_string(),
            sender: MatrixonUserId {
                id: Uuid::new_v4(),
                username: "dummy".to_string(),
                domain: "matrixon.local".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        }
    }
}

/// Message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    /// Message type
    pub msgtype: String,
    
    /// Message body
    pub body: String,
    
    /// Format (optional)
    pub format: Option<String>,
    
    /// Formatted body (optional)
    pub formatted_body: Option<String>,
    
    /// Relates to (optional)
    pub relates_to: Option<RelatesTo>,
    
    /// Mentions (optional)
    pub mentions: Option<Mentions>,
}

/// Relates to information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatesTo {
    /// Event ID
    pub event_id: EventId,
    
    /// Relation type
    pub rel_type: String,
}

/// Mentions information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mentions {
    /// User IDs
    pub user_ids: Vec<MatrixonUserId>,
    
    /// Room flag
    pub room: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use test_log::test;

    #[test]
    fn test_pdu_id_serialization() {
        let pdu_id = PduId(123);
        let serialized = serde_json::to_string(&pdu_id).unwrap();
        let deserialized: PduId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(pdu_id, deserialized);
    }

    #[test]
    fn test_retry_after() {
        let duration = Duration::from_secs(60);
        let retry_after = RetryAfter::new(duration);
        assert_eq!(retry_after.0, duration);
        assert_eq!(retry_after.as_secs(), 60);
    }

    #[test]
    fn test_room_version() {
        let version = RoomVersion::new("1");
        let serialized = serde_json::to_string(&version).unwrap();
        let deserialized: RoomVersion = serde_json::from_str(&serialized).unwrap();
        assert_eq!(version, deserialized);
        assert_eq!(version.as_str(), "1");
    }

    #[test]
    fn test_user_id_conversion() {
        let ruma_id = UserId::parse("@test_user:matrixon.local").unwrap().to_owned();
        let matrixon_id = MatrixonUserId::from(ruma_id.clone());
        assert_eq!(matrixon_id.to_string(), ruma_id.to_string());
    }

    #[test]
    fn test_room_id_conversion() {
        let ruma_id = RoomId::parse("!test_room:matrixon.local").unwrap().to_owned();
        let matrixon_id = MatrixonRoomId::from(ruma_id.clone());
        assert_eq!(matrixon_id.to_string(), ruma_id.to_string());
    }

    #[test]
    fn test_message_content() {
        let content = MessageContent {
            msgtype: "m.text".to_string(),
            body: "Hello, world!".to_string(),
            format: Some("org.matrix.custom.html".to_string()),
            formatted_body: Some("<p>Hello, world!</p>".to_string()),
            relates_to: None,
            mentions: None,
        };
        
        let serialized = serde_json::to_string(&content).unwrap();
        let deserialized: MessageContent = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(content.msgtype, deserialized.msgtype);
        assert_eq!(content.body, deserialized.body);
        assert_eq!(content.format, deserialized.format);
        assert_eq!(content.formatted_body, deserialized.formatted_body);
    }

}
