//! Matrix event handlers for AI services
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2025-06-14

use ruma::events::room::message::RoomMessageEventContent;
use tracing::{info, instrument};

use crate::{error::Error, models::NlpModel, services::AIService};

/// Handler for Matrix room messages
pub struct MessageHandler {
    nlp_model: Box<dyn NlpModel + Send + Sync>,
    ai_service: Box<dyn AIService + Send + Sync>,
}

impl MessageHandler {
    /// Create a new MessageHandler
    pub fn new(
        nlp_model: Box<dyn NlpModel + Send + Sync>,
        ai_service: Box<dyn AIService + Send + Sync>,
    ) -> Self {
        Self {
            nlp_model,
            ai_service,
        }
    }

    /// Process incoming room message
    #[instrument(skip(self))]
    pub async fn handle_message(&self, message: &RoomMessageEventContent) -> Result<(), Error> {
        info!("Handling message");

        let text = message.body();
        let processed_text = self.nlp_model.process_text(text).await?;

        let processed_msg = RoomMessageEventContent::text_plain(processed_text);
        self.ai_service.process_message(&processed_msg).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{models::DefaultNlpModel, services::AIServiceImpl};
    use test_log::test;

    #[test(tokio::test)]
    async fn test_message_handler() {
        let nlp_model = Box::new(DefaultNlpModel);
        let ai_service = Box::new(AIServiceImpl);
        let handler = MessageHandler::new(nlp_model, ai_service);

        let message = RoomMessageEventContent::text_plain("test message");
        let result = handler.handle_message(&message).await;
        assert!(result.is_ok());
    }
}
