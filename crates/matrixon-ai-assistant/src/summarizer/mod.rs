// =============================================================================
// Matrixon Matrix NextServer - Conversation Summarizer Module
// =============================================================================
// [保留之前的文件头注释...]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{debug, info, instrument};
use serde::{Deserialize, Serialize};

use matrixon_common::error::Result;
use super::llm_integration::{LlmIntegration, LlmRequest};

/// Summarization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationConfig {
    pub enabled: bool,
    pub model_name: String,
    pub max_input_length: usize,
    pub max_output_length: usize,
    pub compression_ratio: f32,
    pub cache_size: usize,
    pub timeout: Duration,
    pub min_messages: usize,
    pub max_messages: usize,
    pub target_length: usize,
}

impl Default for SummarizationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_name: "gpt-3.5-turbo".to_string(),
            max_input_length: 4000,
            max_output_length: 1000,
            compression_ratio: 0.3,
            cache_size: 1000,
            timeout: Duration::from_secs(30),
            min_messages: 5,
            max_messages: 100,
            target_length: 500,
        }
    }
}

/// Summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryStats {
    pub total_summaries: u64,
    pub successful_summaries: u64,
    pub failed_summaries: u64,
    pub average_processing_time_ms: f64,
    pub average_message_count: f64,
    pub cache_hit_rate: f32,
}

/// Summary request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryRequest {
    pub room_id: String,
    pub event_ids: Option<Vec<String>>,
    pub user_id: String,
    pub max_messages: Option<usize>,
    pub summary_length: Option<usize>,
    pub include_metadata: bool,
}

/// Summary response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResponse {
    pub summary: String,
    pub metadata: HashMap<String, String>,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub message_count: usize,
    pub participants: Vec<String>,
    pub topics: Vec<String>,
    pub key_decisions: Vec<String>,
    pub time_range: (String, String),
}

/// Conversation analysis result
#[derive(Debug, Clone)]
pub struct ConversationAnalysis {
    pub participants: Vec<String>,
    pub topics: Vec<String>,
    pub key_decisions: Vec<String>,
    pub time_range: (String, String),
    pub metadata: HashMap<String, String>,
    pub confidence: f32,
}

/// Conversation Summarizer Service
#[derive(Debug)]
pub struct ConversationSummarizer {
    config: SummarizationConfig,
    llm_integration: Arc<LlmIntegration>,
    summary_cache: Arc<RwLock<HashMap<String, (SummaryResponse, Instant)>>>,
    stats: Arc<RwLock<SummaryStats>>,
}

impl ConversationSummarizer {
    /// Create a new ConversationSummarizer instance
    pub fn new(config: SummarizationConfig, llm_integration: Arc<LlmIntegration>) -> Self {
        Self {
            config,
            llm_integration,
            summary_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SummaryStats {
                total_summaries: 0,
                successful_summaries: 0,
                failed_summaries: 0,
                average_processing_time_ms: 0.0,
                average_message_count: 0.0,
                cache_hit_rate: 0.0,
            })),
        }
    }

    /// Generate a summary for the conversation
    #[instrument(level = "debug")]
    pub async fn summarize(&self, request: SummaryRequest) -> Result<SummaryResponse> {
        let start = Instant::now();
        debug!("Starting conversation summarization");

        // Check if cached summary exists
        let cache_key = format!("{}-{}", request.room_id, request.user_id);
        if let Some(cached) = self.get_cached_summary(&cache_key).await {
            debug!("Returning cached summary");
            return Ok(cached);
        }

        // Generate summary using LLM
        let mut messages = HashMap::new();
        messages.insert("content".to_string(), 
            format!("Summarize this conversation in {} words: ...", 
                   request.summary_length.unwrap_or(self.config.target_length)));

        let llm_request = LlmRequest {
            model: self.config.model_name.clone(),
            messages: vec![messages],
            max_tokens: Some(self.config.max_output_length),
            temperature: Some(0.7),
            top_p: None,
            user_id: Some(request.user_id.clone()),
        };

        let llm_response = self.llm_integration.generate_response(&llm_request).await?;

        let response = SummaryResponse {
            summary: llm_response.content,
            metadata: HashMap::new(),
            confidence: 0.9,
            processing_time_ms: start.elapsed().as_millis() as u64,
            message_count: request.max_messages.unwrap_or(0),
            participants: vec![],
            topics: vec![],
            key_decisions: vec![],
            time_range: ("".to_string(), "".to_string()),
        };

        // Update cache and stats
        self.cache_summary(&cache_key, &response).await;
        let mut stats = self.stats.write().await;
        stats.total_summaries += 1;
        stats.successful_summaries += 1;
        stats.average_processing_time_ms = 
            (stats.average_processing_time_ms * (stats.total_summaries - 1) as f64 
            + response.processing_time_ms as f64) / stats.total_summaries as f64;

        info!("Completed summarization in {:?}", start.elapsed());
        Ok(response)
    }

    /// Get cached summary
    async fn get_cached_summary(&self, cache_key: &str) -> Option<SummaryResponse> {
        let cache = self.summary_cache.read().await;
        if let Some((summary, timestamp)) = cache.get(cache_key) {
            if timestamp.elapsed().as_secs() < 1800 {
                return Some(summary.clone());
            }
        }
        None
    }

    /// Cache summary response
    async fn cache_summary(&self, cache_key: &str, response: &SummaryResponse) {
        let mut cache = self.summary_cache.write().await;
        cache.insert(cache_key.to_string(), (response.clone(), Instant::now()));
        
        // Update cache hit rate stats
        let mut stats = self.stats.write().await;
        let total = stats.total_summaries;
        let hits = total - stats.failed_summaries;
        stats.cache_hit_rate = hits as f32 / total as f32;
    }

    /// Format timestamp for display
    fn format_timestamp(timestamp: u64) -> String {
        let secs = timestamp / 1000;
        let mins = secs / 60;
        let hours = mins / 60;
        format!("{:02}:{:02}:{:02}", hours % 24, mins % 60, secs % 60)
    }

    /// Format username for display
    fn format_username(username: &str) -> String {
        username.split('@').next().unwrap_or(username).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_summarize() {
        let config = SummarizationConfig::default();
        let llm = Arc::new(LlmIntegration::new_test());
        let summarizer = ConversationSummarizer::new(config, llm);

        let request = SummaryRequest {
            room_id: "!test:example.com".to_string(),
            event_ids: None,
            user_id: "@user:example.com".to_string(),
            max_messages: Some(10),
            summary_length: Some(200),
            include_metadata: false,
        };

        let result = summarizer.summarize(request).await;
        assert!(result.is_ok());
        let summary = result.unwrap();
        assert!(!summary.summary.is_empty());
    }
}
