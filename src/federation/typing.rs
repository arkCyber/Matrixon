//! Federation Typing Module
//! 
//! Handles typing notification exchange between federated servers,
//! including notification validation and propagation.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.11.0-alpha
//! Date: 2024-03-21

use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument};
use ruma::{
    api::federation::typing::send_typing::v1::Request as SendTypingRequest,
    events::{
        typing::{TypingEvent, TypingEventContent},
        AnyEphemeralRoomEvent,
        room::ephemeral::EphemeralRoomEvent,
    },
    identifiers::{OwnedRoomId, OwnedUserId, RoomId, ServerName, UserId, OwnedServerName},
};
use thiserror::Error;
use tokio_postgres::Client as DbClient;
use serde_json::Value;
use chrono::{DateTime, Utc};
use reqwest;

/// Typing-related errors
#[derive(Error, Debug)]
pub enum TypingError {
    #[error("Failed to update typing status: {0}")]
    UpdateFailed(String),
    
    #[error("Failed to send typing status: {0}")]
    SendFailed(String),
    
    #[error("Invalid typing data: {0}")]
    InvalidData(String),

    #[error("Database error: {0}")]
    Database(#[from] tokio_postgres::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Federation error: {0}")]
    Federation(String),
}

/// Federation typing service
#[derive(Debug, Clone)]
pub struct FederationTyping {
    db: DbClient,
    server_name: Box<ServerName>,
}

impl FederationTyping {
    /// Create a new federation typing service
    pub fn new(db: DbClient, server_name: Box<ServerName>) -> Self {
        Self { db, server_name }
    }
    
    /// Update a user's typing status
    #[instrument(skip(self), fields(room_id = %room_id, user_id = %user_id))]
    pub async fn update_typing(
        &self,
        room_id: &RoomId,
        user_id: &UserId,
        typing: TypingEvent,
    ) -> Result<(), TypingError> {
        let start = Instant::now();
        debug!("🔧 Updating typing status for user {} in room {}", user_id, room_id);

        // Store typing status in database
        let typing_json = serde_json::to_value(&typing)?;
        self.db.execute(
            "INSERT INTO typing_events (room_id, user_id, typing_data, timestamp) 
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (room_id, user_id) 
             DO UPDATE SET typing_data = $3, timestamp = $4",
            &[
                &room_id.to_string(),
                &user_id.to_string(),
                &typing_json,
                &Utc::now(),
            ],
        ).await?;

        info!("✅ Updated typing status in {:?}", start.elapsed());
        Ok(())
    }
    
    /// Send typing notifications to remote servers
    #[instrument(skip(self), fields(room_id = %room_id, user_id = %user_id))]
    pub async fn send_typing(
        &self,
        room_id: &RoomId,
        user_id: &UserId,
        typing: TypingEvent,
    ) -> Result<(), TypingError> {
        let start = Instant::now();
        debug!("🔧 Sending typing status to remote servers for user {} in room {}", user_id, room_id);

        // Get remote servers for the room
        let remote_servers = self.get_remote_servers(room_id).await?;

        // Send typing event to each remote server
        for server in remote_servers.iter() {
            if let Err(e) = self.send_to_server(server, room_id, user_id, &typing).await {
                error!("❌ Failed to send typing to server {}: {}", server, e);
                // Continue with other servers even if one fails
            }
        }

        info!("✅ Sent typing status to remote servers in {:?}", start.elapsed());
        Ok(())
    }
    
    /// Handle incoming typing notifications
    #[instrument(skip(self), fields(room_id = %room_id, user_id = %user_id))]
    pub async fn handle_incoming_typing(
        &self,
        room_id: &RoomId,
        user_id: &UserId,
        event: EphemeralRoomEvent<TypingEventContent>,
    ) -> Result<(), TypingError> {
        let start = Instant::now();
        debug!("🔧 Handling incoming typing notification from user {} in room {}", user_id, room_id);

        // Verify the sender's server
        if !self.verify_sender_server(user_id, &event).await? {
            return Err(TypingError::InvalidData("Invalid sender server".into()));
        }

        // Convert to TypingEvent for storage
        let typing = TypingEvent {
            content: event.content,
            room_id: event.room_id,
        };

        // Store the typing event
        self.update_typing(room_id, user_id, typing).await?;

        info!("✅ Handled incoming typing notification in {:?}", start.elapsed());
        Ok(())
    }
    
    /// Get typing status for a room
    #[instrument(skip(self), fields(room_id = %room_id))]
    pub async fn get_typing(
        &self,
        room_id: &RoomId,
    ) -> Result<Vec<TypingEvent>, TypingError> {
        let start = Instant::now();
        debug!("🔧 Getting typing status for room {}", room_id);

        let rows = self.db.query(
            "SELECT typing_data FROM typing_events 
             WHERE room_id = $1 
             AND timestamp > $2",
            &[
                &room_id.to_string(),
                &(Utc::now() - Duration::from_secs(30)).timestamp()
            ],
        ).await?;

        let typing_events: Result<Vec<TypingEvent>, TypingError> = rows
            .iter()
            .map(|row| {
                let typing_data: Value = row.get(0);
                serde_json::from_value(typing_data)
                    .map_err(|e| TypingError::Serialization(e))
            })
            .collect();

        info!("✅ Retrieved typing status in {:?}", start.elapsed());
        typing_events
    }

    /// Get remote servers for a room
    async fn get_remote_servers(&self, room_id: &RoomId) -> Result<Vec<Box<ServerName>>, TypingError> {
        let rows = self.db.query(
            "SELECT DISTINCT server_name FROM room_servers WHERE room_id = $1",
            &[&room_id.to_string()],
        ).await?;

        let servers: Result<Vec<Box<ServerName>>, TypingError> = rows
            .iter()
            .map(|row| {
                let server_name: String = row.get(0);
                ServerName::try_from(server_name.as_str())
                    .map(Box::new)
                    .map_err(|e| TypingError::InvalidData(format!("Invalid server name: {}", e)))
            })
            .collect();

        servers
    }

    /// Send typing event to a specific server
    #[instrument(skip(self), fields(server = %server, room_id = %room_id, user_id = %user_id))]
    async fn send_to_server(
        &self,
        server: &Box<ServerName>,
        room_id: &RoomId,
        user_id: &UserId,
        typing: &TypingEvent,
    ) -> Result<(), TypingError> {
        let start = Instant::now();
        debug!("🔧 Sending typing event to server {} for room {}", server, room_id);

        // Create the federation request
        let request = SendTypingRequest {
            room_id: room_id.clone(),
            user_id: user_id.clone(),
            typing: typing.clone(),
            origin: self.server_name.clone(),
            origin_server_ts: Utc::now(),
        };

        // Convert request to JSON
        let request_json = serde_json::to_value(&request)
            .map_err(|e| TypingError::Serialization(e))?;

        // Create HTTP request
        let client = reqwest::Client::new();
        let url = format!("https://{}/_matrix/federation/v1/send_typing", server);
        
        let response = client
            .post(&url)
            .json(&request_json)
            .send()
            .await
            .map_err(|e| TypingError::Federation(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TypingError::Federation(format!(
                "Server returned error: {} - {}",
                response.status(),
                error_text
            )));
        }

        info!("✅ Sent typing event to server {} in {:?}", server, start.elapsed());
        Ok(())
    }

    /// Verify that the sender's server is valid
    #[instrument(skip(self), fields(user_id = %user_id))]
    async fn verify_sender_server(
        &self,
        user_id: &UserId,
        event: &EphemeralRoomEvent<TypingEventContent>,
    ) -> Result<bool, TypingError> {
        let start = Instant::now();
        debug!("🔧 Verifying sender server for user {}", user_id);

        // Extract server name from user ID
        let sender_server = user_id.server_name();

        // Check if server is in our trusted servers list
        let is_trusted = self.db.query_one(
            "SELECT EXISTS (
                SELECT 1 FROM trusted_servers 
                WHERE server_name = $1 
                AND is_active = true
            )",
            &[&sender_server.to_string()],
        ).await
        .map(|row| row.get::<_, bool>(0))
        .map_err(|e| TypingError::Database(e))?;

        if !is_trusted {
            error!("❌ Untrusted server {} attempted to send typing notification", sender_server);
            return Ok(false);
        }

        // Verify server signature if present
        if let Some(signatures) = &event.signatures {
            if let Some(signature) = signatures.get(sender_server) {
                // TODO: Implement signature verification
                // This would verify the server's signature using their public key
                // For now, we'll just check if the signature exists
                if signature.is_empty() {
                    error!("❌ Invalid signature from server {}", sender_server);
                    return Ok(false);
                }
            }
        }

        info!("✅ Verified sender server in {:?}", start.elapsed());
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_postgres::NoTls;
    use std::str::FromStr;
    
    async fn setup_test_db() -> DbClient {
        let (client, connection) = tokio_postgres::connect(
            "host=localhost user=postgres password=postgres dbname=matrixon_test",
            NoTls,
        ).await.unwrap();
        
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        
        client
    }
    
    #[tokio::test]
    async fn test_federation_typing_service() {
        let db = setup_test_db().await;
        let server_name = Box::new(ServerName::from_str("test.server").unwrap());
        let service = FederationTyping::new(db, server_name);
        
        let room_id = RoomId::from_str("!test:test.server").unwrap();
        let user_id = UserId::from_str("@test:test.server").unwrap();
        let typing = TypingEvent {
            content: ruma::events::typing::TypingEventContent {
                user_ids: vec![user_id.clone()],
            },
            room_id: Some(room_id.clone()),
        };
        
        // Test updating typing status
        assert!(service.update_typing(&room_id, &user_id, typing.clone()).await.is_ok());
        
        // Test getting typing status
        let typing_events = service.get_typing(&room_id).await.unwrap();
        assert!(!typing_events.is_empty());
    }
}
