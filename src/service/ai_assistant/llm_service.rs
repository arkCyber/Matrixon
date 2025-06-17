use crate::error::Error;
use crate::service::ai_assistant::llm_integration::{LLMIntegration, LLMProvider};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

/// Message type for LLM interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// LLM Service for handling AI assistant interactions 
