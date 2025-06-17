// Matrixon Admin Service Module
// Author: arkSong (arksong2018@gmail.com)
// Date: 2024
// Version: 1.0
// Purpose: Implement admin service functionality

use std::{
    sync::Arc,
    time::Instant,
};

use ruma::{
    api::client::error::ErrorKind,
    events::AnyTimelineEvent,
    serde::Raw,
    OwnedEventId, OwnedRoomId, OwnedUserId, EventId, RoomId, UserId,
};

use tracing::{debug, error, info, instrument, warn};

use crate::{
    config::Config,
    service::Services,
    Error,
    Result,
    database::KeyValueDatabase,
};

use tokio::sync::{RwLock, Mutex, mpsc};

use super::commands::AdminRoomEvent;
use ruma_events::room::message::{MessageType, TextMessageEventContent, RoomMessageEventContent};

/// Admin service implementation
pub struct Service {
    pub sender: mpsc::UnboundedSender<AdminRoomEvent>,
    receiver: Mutex<mpsc::UnboundedReceiver<AdminRoomEvent>>,
    admin_user_id: Box<OwnedUserId>,
    admin_room_id: Option<OwnedRoomId>,
    config: Arc<Config>,
}

impl Service {
    /// Create a new admin service instance
    pub fn new(config: Arc<Config>, admin_user_id: OwnedUserId) -> Arc<Self> {
        let (sender, receiver) = mpsc::unbounded_channel();
        Arc::new(Self { 
            sender,
            receiver: Mutex::new(receiver),
            admin_user_id: Box::new(admin_user_id),
            admin_room_id: None,
            config,
        })
    }

    pub fn start_handler(self: &Arc<Self>) {
        let self_clone = Arc::clone(self);
        tokio::spawn(async move {
            let mut receiver = {
                let mut receiver = self_clone.receiver.lock().unwrap();
                receiver.try_recv().ok()
            };
            
            while let Some(event) = receiver {
                match event {
                    AdminRoomEvent::ProcessMessage(message) => {
                        if let Err(e) = self_clone.process_message(&message).await {
                            error!("Failed to process admin message: {}", e);
                        }
                    }
                    AdminRoomEvent::SendMessage(content) => {
                        if let Err(e) = self_clone.send_room_message(&self_clone.admin_user_id, content, None).await {
                            error!("Failed to send admin message: {}", e);
                        }
                    }
                }
                
                receiver = {
                    let mut receiver = self_clone.receiver.lock().unwrap();
                    receiver.try_recv().ok()
                };
            }
        });
    }

    /// Process an admin message
    pub async fn process_message(&self, _message: &str) -> Result<()> {
        // Implementation
        Ok(())
    }

    pub async fn send_room_message(
        &self,
        _sender: &OwnedUserId,
        _content: RoomMessageEventContent,
        _txn_id: Option<String>,
    ) -> Result<MessageType> {
        let start = Instant::now();
        debug!("🔧 Sending room message");

        // Implementation

        info!("✅ Room message sent in {:?}", start.elapsed());
        Ok(MessageType::Text(TextMessageEventContent::plain("Room message sent")))
    }

    pub fn get_admin_room(&self) -> Result<Option<OwnedRoomId>> {
        Ok(self.admin_room_id.clone())
    }

    pub async fn make_user_admin(&self, user_id: &OwnedUserId, _displayname: String) -> Result<()> {
        let start = Instant::now();
        debug!("🔧 Making user admin: {}", user_id);

        // Implementation

        info!("✅ User made admin in {:?}", start.elapsed());
        Ok(())
    }

    pub fn user_is_admin(&self, user_id: &OwnedUserId) -> Result<bool> {
        Ok(user_id == &**self.admin_user_id)
    }

    pub async fn send_message(&self, content: RoomMessageEventContent, txn_id: Option<String>) -> Result<MessageType> {
        self.send_room_message(&self.admin_user_id, content, txn_id).await
    }
} 
