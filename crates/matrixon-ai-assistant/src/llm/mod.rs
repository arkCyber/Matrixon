use serde::{Deserialize, Serialize};

/// Message type for LLM interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// LLM Service for handling AI assistant interactions
#[derive(Debug)]
pub struct LlmService {
    // TODO: Add necessary fields
}

impl Default for LlmService {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmService {
    /// Create a new LLM service instance
    pub fn new() -> Self {
        Self {
            // Initialize fields
        }
    }

    // TODO: Implement service methods
}
