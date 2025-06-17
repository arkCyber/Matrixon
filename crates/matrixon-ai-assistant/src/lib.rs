//! Matrixon AI Assistant
//! 
//! This crate provides AI assistant functionality for the Matrixon Matrix Server.
//! It includes features like conversation summarization, user behavior analysis,
//! translation services, and LLM integration.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use matrixon_common::error::{MatrixonError, Result};

pub mod analyzer;
pub mod config;
pub mod integration;
pub mod llm;
pub mod llm_integration;
pub mod qa;
pub mod recommendation;
pub mod summarizer;
pub mod translation;

/// Re-exports commonly used types
pub mod prelude {
    pub use super::translation::TranslationService;
    pub use super::llm::LlmService;
    pub use super::qa::QaBot;
    pub use super::analyzer::{RoomAnalyzer, UserBehaviorAnalyzer};
    pub use super::summarizer::ConversationSummarizer;
    pub use super::recommendation::RecommendationEngine;
    pub use crate::LlmIntegration;
}

/// Simplified error type for this crate
pub type Error = MatrixonError;

/// New error conversion module
pub mod error_conversion {
    use super::*;

    /// Convert external error to MatrixonError
    pub fn to_matrixon_error<E: std::error::Error>(error: E) -> MatrixonError {
        MatrixonError::Other(error.to_string())
    }
}

/// Re-export conversion functions for convenience
pub use error_conversion::to_matrixon_error;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let matrixon_error = to_matrixon_error(io_error);
        assert_eq!(matrixon_error.to_string(), "test");
    }
}

// Re-export key config types for use in submodules
pub use config::{TranslationConfig, QaBotConfig, SummarizationConfig, RecommendationConfig, RecommendationAlgorithm};
pub use llm_integration::{LlmIntegration, LlmRequest, LlmResponse, LlmProvider, LlmProviderConfig};
