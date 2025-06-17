//! Matrixon AI Services
//!
//! This crate provides AI services for the Matrixon Matrix server, including:
//! - AI assistant integration
//! - Model Context Protocol (MCP) integration
//! - AI-powered features and utilities
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod error;
pub mod handlers;
pub mod mcp;
pub mod models;
pub mod services;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

// Re-export commonly used types
pub use mcp::client::McpClient;
pub use mcp::server::McpServer;

use ruma::events::room::message::RoomMessageEventContent;

/// Core AI service trait
#[async_trait::async_trait]
pub trait AIService {
    /// Process Matrix room message with AI
    async fn process_message(
        &self,
        message: &RoomMessageEventContent,
    ) -> Result<RoomMessageEventContent>;

    /// Get service health status
    async fn health_check(&self) -> Result<()>;
}

/// Basic AI service implementation
pub struct BasicAIService;

#[async_trait::async_trait]
impl AIService for BasicAIService {
    async fn process_message(
        &self,
        message: &RoomMessageEventContent,
    ) -> Result<RoomMessageEventContent> {
        // TODO: Implement actual AI processing
        Ok(message.clone())
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use test_log::test;

    #[test(tokio::test)]
    async fn test_process_message() {
        let service = BasicAIService;
        let message = RoomMessageEventContent::text_plain("Hello");

        let result: Result<_> = service.process_message(&message).await;
        assert!(result.is_ok());
    }

    #[test(tokio::test)]
    async fn test_health_check() {
        let service = BasicAIService;
        let result: Result<()> = service.health_check().await;
        assert!(result.is_ok());
    }
}
