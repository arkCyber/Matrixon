//! Matrixon AI Assistant - Configuration Module
//! 
//! This module provides configuration types for various AI assistant features.
//! It includes settings for translation, Q&A, summarization, and recommendation services.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Translation service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    pub enabled: bool,
    pub default_source_language: String,
    pub default_target_language: String,
    pub supported_languages: Vec<String>,
    pub cache_size: usize,
    pub cache_ttl_seconds: u64,
    pub max_text_length: usize,
    pub rate_limit_per_minute: u32,
    pub auto_detect_language: bool,
    pub translation_provider: String,
}

/// Q&A bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaBotConfig {
    pub enabled: bool,
    pub model_name: String,
    pub max_context_length: usize,
    pub max_response_length: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub cache_size: usize,
    pub cache_ttl_seconds: u64,
    pub rate_limit_per_minute: u32,
    pub knowledge_base_path: Option<String>,
    pub supported_languages: Vec<String>,
    pub confidence_threshold: f32,
}

/// Summarization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationConfig {
    pub enabled: bool,
    pub model_name: String,
    pub max_input_length: usize,
    pub max_output_length: usize,
    pub compression_ratio: f32,
    pub cache_size: usize,
    pub cache_ttl_seconds: u64,
    pub rate_limit_per_minute: u32,
}

/// Recommendation algorithm types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RecommendationAlgorithm {
    CollaborativeFiltering,
    ContentBased,
    Hybrid,
    MatrixFactorization,
    NeuralNetwork,
}

/// Recommendation service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationConfig {
    pub enabled: bool,
    pub algorithm: RecommendationAlgorithm,
    pub model_name: String,
    pub max_recommendations: usize,
    pub min_confidence: f32,
    pub cache_size: usize,
    pub cache_ttl_seconds: u64,
    pub rate_limit_per_minute: u32,
    pub feature_weights: HashMap<String, f32>,
    pub user_interaction_weight: f32,
    pub similarity_threshold: f32,
}

/// Global AI assistant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAssistantConfig {
    pub translation: TranslationConfig,
    pub qa_bot: QaBotConfig,
    pub summarization: SummarizationConfig,
    pub recommendation: RecommendationConfig,
    pub log_level: String,
    pub metrics_enabled: bool,
    pub metrics_port: u16,
    pub health_check_interval_seconds: u64,
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_source_language: "en".to_string(),
            default_target_language: "en".to_string(),
            supported_languages: vec!["en".to_string(), "es".to_string(), "fr".to_string()],
            cache_size: 1000,
            cache_ttl_seconds: 3600,
            max_text_length: 5000,
            rate_limit_per_minute: 60,
            auto_detect_language: true,
            translation_provider: "openai".to_string(),
        }
    }
}

impl Default for QaBotConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_name: "gpt-3.5-turbo".to_string(),
            max_context_length: 4000,
            max_response_length: 1000,
            temperature: 0.7,
            top_p: 1.0,
            cache_size: 1000,
            cache_ttl_seconds: 3600,
            rate_limit_per_minute: 60,
            knowledge_base_path: None,
            supported_languages: vec!["en".to_string()],
            confidence_threshold: 0.7,
        }
    }
}

impl Default for SummarizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_name: "gpt-3.5-turbo".to_string(),
            max_input_length: 8000,
            max_output_length: 1000,
            compression_ratio: 0.3,
            cache_size: 1000,
            cache_ttl_seconds: 3600,
            rate_limit_per_minute: 60,
        }
    }
}

impl Default for RecommendationConfig {
    fn default() -> Self {
        let mut feature_weights = HashMap::new();
        feature_weights.insert("relevance".to_string(), 0.4);
        feature_weights.insert("popularity".to_string(), 0.3);
        feature_weights.insert("recency".to_string(), 0.3);

        Self {
            enabled: true,
            algorithm: RecommendationAlgorithm::Hybrid,
            model_name: "hybrid-recommender".to_string(),
            max_recommendations: 10,
            min_confidence: 0.7,
            cache_size: 1000,
            cache_ttl_seconds: 3600,
            rate_limit_per_minute: 60,
            feature_weights,
            user_interaction_weight: 0.5,
            similarity_threshold: 0.6,
        }
    }
}

impl Default for AiAssistantConfig {
    fn default() -> Self {
        Self {
            translation: TranslationConfig::default(),
            qa_bot: QaBotConfig::default(),
            summarization: SummarizationConfig::default(),
            recommendation: RecommendationConfig::default(),
            log_level: "info".to_string(),
            metrics_enabled: true,
            metrics_port: 9090,
            health_check_interval_seconds: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_config_default() {
        let config = TranslationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_source_language, "en");
        assert_eq!(config.default_target_language, "en");
        assert!(config.supported_languages.contains(&"en".to_string()));
        assert!(config.auto_detect_language);
        assert_eq!(config.translation_provider, "openai");
    }

    #[test]
    fn test_qa_bot_config_default() {
        let config = QaBotConfig::default();
        assert!(config.enabled);
        assert_eq!(config.model_name, "gpt-3.5-turbo");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.confidence_threshold, 0.7);
        assert_eq!(config.supported_languages, vec!["en".to_string()]);
    }

    #[test]
    fn test_summarization_config_default() {
        let config = SummarizationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.model_name, "gpt-3.5-turbo");
        assert_eq!(config.compression_ratio, 0.3);
    }

    #[test]
    fn test_recommendation_config_default() {
        let config = RecommendationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.algorithm, RecommendationAlgorithm::Hybrid);
        assert_eq!(config.feature_weights.get("relevance"), Some(&0.4));
        assert_eq!(config.user_interaction_weight, 0.5);
        assert_eq!(config.similarity_threshold, 0.6);
    }

    #[test]
    fn test_ai_assistant_config_default() {
        let config = AiAssistantConfig::default();
        assert_eq!(config.log_level, "info");
        assert!(config.metrics_enabled);
        assert_eq!(config.metrics_port, 9090);
    }
}
