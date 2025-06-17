//! Core AI services implementation
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2025-06-14

use async_trait::async_trait;
use ruma::events::room::message::RoomMessageEventContent;
use tracing::{info, instrument};

use crate::error::Error;

#[async_trait]
pub trait AIService: Send + Sync {
    async fn process_message(&self, message: &RoomMessageEventContent) -> Result<(), Error>;
}

/// Core AI service trait implementation
pub struct AIServiceImpl;

#[async_trait]
impl AIService for AIServiceImpl {
    #[instrument(skip(self))]
    async fn process_message(&self, message: &RoomMessageEventContent) -> Result<(), Error> {
        info!("Processing message with AI");
        // TODO: Implement actual AI processing logic
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test(tokio::test)]
    async fn test_process_message() {
        let service = AIServiceImpl;
        let message = RoomMessageEventContent::text_plain("Test message");

        let result = service.process_message(&message).await;
        assert!(result.is_ok());
    }
}
