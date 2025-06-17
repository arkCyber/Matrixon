// =============================================================================
// Matrixon Matrix NextServer - Bot Configuration Module
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Date: 2024-03-19
// Version: 0.11.0-alpha
// License: Apache 2.0 / MIT
//
// Description:
//   Configuration management for Matrixon bot service
//   - Bot settings
//   - Command configuration
//   - Plugin settings
//   - Performance tuning
//
// =============================================================================

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use tracing::info;
use matrixon_core::error::{MatrixonError, Result};

/// Bot configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// Bot identity settings
    pub identity: IdentityConfig,
    /// Command settings
    pub commands: CommandConfig,
    /// Plugin settings
    pub plugins: PluginConfig,
    /// Performance settings
    pub performance: PerformanceConfig,
}

/// Bot identity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Bot username
    pub username: String,
    /// Bot password
    pub password: String,
    /// Bot display name
    pub display_name: String,
    /// Bot avatar URL (optional)
    pub avatar_url: Option<String>,
}

/// Command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    /// Command prefix (default: "!")
    pub prefix: String,
    /// List of enabled commands
    pub enabled_commands: Vec<String>,
    /// Command cooldown in seconds
    pub cooldown: u64,
    /// Maximum command length
    pub max_length: usize,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// List of enabled plugins
    pub enabled_plugins: Vec<String>,
    /// Plugin directory path
    pub plugin_dir: String,
    /// Plugin configuration
    pub plugin_config: serde_json::Value,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum concurrent commands
    pub max_concurrent_commands: usize,
    /// Command timeout in milliseconds
    pub command_timeout: u64,
    /// Message queue size
    pub message_queue_size: usize,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            identity: IdentityConfig {
                username: "matrixon-bot".to_string(),
                password: "".to_string(),
                display_name: "Matrixon Bot".to_string(),
                avatar_url: None,
            },
            commands: CommandConfig {
                prefix: "!".to_string(),
                enabled_commands: vec!["help".to_string(), "status".to_string(), "ping".to_string()],
                cooldown: 1,
                max_length: 1000,
            },
            plugins: PluginConfig {
                enabled_plugins: Vec::new(),
                plugin_dir: "plugins".to_string(),
                plugin_config: serde_json::json!({}),
            },
            performance: PerformanceConfig {
                max_concurrent_commands: 100,
                command_timeout: 5000,
                message_queue_size: 1000,
            },
        }
    }
}

impl BotConfig {
    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config_str = fs::read_to_string(path)
            .map_err(|e| MatrixonError::Config(format!("Failed to read config file: {}", e)))?;
        
        let config: BotConfig = serde_json::from_str(&config_str)
            .map_err(|e| MatrixonError::Config(format!("Failed to parse config file: {}", e)))?;
        
        info!("Successfully loaded bot configuration");
        Ok(config)
    }

    /// Save configuration to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let config_str = serde_json::to_string_pretty(self)
            .map_err(|e| MatrixonError::Config(format!("Failed to serialize config: {}", e)))?;
        
        fs::write(path, config_str)
            .map_err(|e| MatrixonError::Config(format!("Failed to write config file: {}", e)))?;
        
        info!("Successfully saved bot configuration");
        Ok(())
    }

    /// Create default configuration file if it doesn't exist
    pub fn create_default_config<P: AsRef<Path>>(path: P) -> Result<()> {
        if !path.as_ref().exists() {
            let default_config = Self::default();
            default_config.save_to_file(path)?;
            info!("Created default bot configuration file");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_serialization() {
        let config = BotConfig::default();
        let config_str = serde_json::to_string(&config).unwrap();
        let deserialized: BotConfig = serde_json::from_str(&config_str).unwrap();
        assert_eq!(config.identity.username, deserialized.identity.username);
    }

    #[test]
    fn test_config_file_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = BotConfig::default();
        
        // Test saving
        config.save_to_file(&temp_file).unwrap();
        
        // Test loading
        let loaded_config = BotConfig::from_file(&temp_file).unwrap();
        assert_eq!(config.identity.username, loaded_config.identity.username);
    }
} 
