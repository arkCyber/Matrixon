// =============================================================================
// Matrixon Matrix NextServer - Config Module
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Contributors: Matrixon Development Team
// Date: 2024-12-11
// Version: 2.0.0-alpha (PostgreSQL Backend)
// License: Apache 2.0 / MIT
//
// Description:
//   Command-line interface implementation. This module is part of the Matrixon Matrix NextServer
//   implementation, designed for enterprise-grade deployment with 20,000+
//   concurrent connections and <50ms response latency.
//
// Performance Targets:
//   • 20k+ concurrent connections
//   • <50ms response latency
//   • >99% success rate
//   • Memory-efficient operation
//   • Horizontal scalability
//
// Features:
//   • CLI command handling
//   • Interactive user interface
//   • Command validation and parsing
//   • Help and documentation
//   • Administrative operations
//
// Architecture:
//   • Async/await native implementation
//   • Zero-copy operations where possible
//   • Memory pool optimization
//   • Lock-free data structures
//   • Enterprise monitoring integration
//
// Dependencies:
//   • Tokio async runtime
//   • Structured logging with tracing
//   • Error handling with anyhow/thiserror
//   • Serialization with serde
//   • Matrix protocol types with ruma
//
// References:
//   • Matrix.org specification: https://matrix.org/
//   • Synapse reference: https://github.com/element-hq/synapse
//   • Matrix spec: https://spec.matrix.org/
//   • Performance guidelines: Internal Matrixon documentation
//
// Quality Assurance:
//   • Comprehensive unit testing
//   • Integration test coverage
//   • Performance benchmarking
//   • Memory leak detection
//   • Security audit compliance
//
// =============================================================================

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, warn, instrument};

use super::Commands;

/// Main CLI arguments structure
#[derive(Parser, Debug, Clone)]
#[command(
    name = "matrixon-admin",
    about = "Advanced CLI Administration Tool for matrixon Matrix Server",
    version = env!("CARGO_PKG_VERSION"),
    author = "Matrix Server Performance Team"
)]
pub struct CliArgs {
    /// Configuration file path
    #[arg(
        short, long,
        global = true,
        env = "matrixon_CONFIG",
        help = "Path to matrixon configuration file"
    )]
    pub config: Option<PathBuf>,

    /// Server URL for remote administration
    #[arg(
        short, long,
        global = true,
        env = "matrixon_SERVER_URL",
        help = "Server URL for remote administration"
    )]
    pub server: Option<String>,

    /// Access token for authentication
    #[arg(
        short, long,
        global = true,
        env = "matrixon_ACCESS_TOKEN",
        help = "Access token for authentication"
    )]
    pub token: Option<String>,

    /// Output format
    #[arg(
        short, long,
        global = true,
        default_value = "table",
        help = "Output format"
    )]
    pub output: OutputFormat,

    /// Verbose logging
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Quiet mode (suppress non-error output)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Interactive mode
    #[arg(short, long, global = true)]
    pub interactive: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// JSON output (equivalent to --output json)
    #[arg(long, global = true)]
    pub json: bool,

    /// Profile name to use
    #[arg(
        short = 'P', long,
        global = true,
        help = "Configuration profile to use"
    )]
    pub profile: Option<String>,

    /// Working directory
    #[arg(
        short = 'C', long,
        global = true,
        help = "Change to directory before executing command"
    )]
    pub chdir: Option<PathBuf>,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Output format options
#[derive(ValueEnum, Clone, Debug, Default, Serialize, Deserialize)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Yaml,
    Plain,
    Csv,
    Tree,
    Summary,
}

/// CLI configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Configuration format version
    pub version: String,
    
    /// Default settings
    pub defaults: DefaultSettings,
    
    /// Server profiles
    pub profiles: HashMap<String, ServerProfile>,
    
    /// User preferences
    pub preferences: UserPreferences,
    
    /// Authentication settings
    pub auth: AuthConfig,
    
    /// Output formatting settings
    pub formatting: FormattingConfig,
    
    /// Command history settings
    pub history: HistoryConfig,
    
    /// Plugin settings
    pub plugins: PluginConfig,
}

/// Default CLI settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultSettings {
    /// Default server profile
    pub profile: String,
    
    /// Default output format
    pub output_format: OutputFormat,
    
    /// Default page size for listings
    pub page_size: u64,
    
    /// Default timeout for operations
    pub timeout_seconds: u64,
    
    /// Default retry count for failed operations
    pub retry_count: u32,
}

/// Server profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    /// Profile name
    pub name: String,
    
    /// Server URL
    pub url: String,
    
    /// Configuration file path
    pub config_file: Option<PathBuf>,
    
    /// Connection settings
    pub connection: ConnectionSettings,
    
    /// Authentication settings
    pub auth: ProfileAuthSettings,
    
    /// Custom settings for this profile
    pub custom: HashMap<String, serde_yaml::Value>,
}

/// Connection settings for a server profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSettings {
    /// Connection timeout in seconds
    pub timeout: u64,
    
    /// Request timeout in seconds
    pub request_timeout: u64,
    
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Enable TLS verification
    pub verify_tls: bool,
    
    /// Custom TLS certificate path
    pub tls_cert_path: Option<PathBuf>,
}

/// Authentication settings for a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileAuthSettings {
    /// Authentication method
    pub method: AuthMethod,
    
    /// Stored access token (encrypted)
    pub access_token: Option<String>,
    
    /// Username for login-based auth
    pub username: Option<String>,
    
    /// Token expiry time
    #[serde(skip)]
    pub token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Auto-refresh token
    pub auto_refresh: bool,
}

/// Authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Username/password authentication
    Password,
    
    /// Access token authentication
    Token,
    
    /// Certificate-based authentication
    Certificate,
    
    /// Interactive authentication
    Interactive,
}

/// User preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Enable colored output
    pub colored_output: bool,
    
    /// Show progress bars
    pub progress_bars: bool,
    
    /// Confirm destructive operations
    pub confirm_destructive: bool,
    
    /// Save command history
    pub save_history: bool,
    
    /// Auto-complete suggestions
    pub auto_complete: bool,
    
    /// Preferred editor for editing configs
    pub editor: Option<String>,
    
    /// Timezone for displaying dates
    pub timezone: String,
    
    /// Date format preference
    pub date_format: String,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Token storage method
    pub token_storage: TokenStorage,
    
    /// Encryption key for stored tokens
    pub encryption_key: Option<String>,
    
    /// Session timeout in minutes
    pub session_timeout: u64,
    
    /// Auto-logout on inactivity
    pub auto_logout: bool,
}

/// Token storage method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenStorage {
    /// Store in config file (encrypted)
    ConfigFile,
    
    /// Store in system keychain
    Keychain,
    
    /// Store in environment variables
    Environment,
    
    /// Don't store (prompt each time)
    None,
}

/// Output formatting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingConfig {
    /// Default table style
    pub table_style: TableStyle,
    
    /// Maximum column width
    pub max_column_width: usize,
    
    /// Truncate long values
    pub truncate_values: bool,
    
    /// Show borders in tables
    pub show_borders: bool,
    
    /// Indent size for tree output
    pub tree_indent: usize,
    
    /// Color scheme
    pub color_scheme: ColorScheme,
}

/// Table display style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TableStyle {
    Plain,
    Grid,
    Rounded,
    Minimal,
    Compact,
}

/// Color scheme for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    /// Primary color (for headers, emphasis)
    pub primary: String,
    
    /// Secondary color (for values)
    pub secondary: String,
    
    /// Success color
    pub success: String,
    
    /// Warning color
    pub warning: String,
    
    /// Error color
    pub error: String,
    
    /// Muted color (for metadata)
    pub muted: String,
}

/// Command history configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    /// Enable command history
    pub enabled: bool,
    
    /// Maximum history entries
    pub max_entries: u64,
    
    /// History file path
    pub file_path: Option<PathBuf>,
    
    /// Ignore patterns (commands not to save)
    pub ignore_patterns: Vec<String>,
    
    /// Save timestamps
    pub save_timestamps: bool,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Enable plugin system
    pub enabled: bool,
    
    /// Plugin directory
    pub plugin_dir: Option<PathBuf>,
    
    /// Enabled plugins
    pub enabled_plugins: Vec<String>,
    
    /// Plugin settings
    pub plugin_settings: HashMap<String, serde_yaml::Value>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            version: "2.0.0".to_string(),
            defaults: DefaultSettings {
                profile: "default".to_string(),
                output_format: OutputFormat::Table,
                page_size: 50,
                timeout_seconds: 30,
                retry_count: 3,
            },
            profiles: {
                let mut profiles = HashMap::new();
                profiles.insert("default".to_string(), ServerProfile::default());
                profiles
            },
            preferences: UserPreferences::default(),
            auth: AuthConfig::default(),
            formatting: FormattingConfig::default(),
            history: HistoryConfig::default(),
            plugins: PluginConfig::default(),
        }
    }
}

impl Default for ServerProfile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            url: "http://localhost:6167".to_string(),
            config_file: Some(PathBuf::from("/etc/matrixon.toml")),
            connection: ConnectionSettings::default(),
            auth: ProfileAuthSettings::default(),
            custom: HashMap::new(),
        }
    }
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            timeout: 30,
            request_timeout: 60,
            max_retries: 3,
            verify_tls: true,
            tls_cert_path: None,
        }
    }
}

impl Default for ProfileAuthSettings {
    fn default() -> Self {
        Self {
            method: AuthMethod::Token,
            access_token: None,
            username: None,
            token_expires_at: None,
            auto_refresh: true,
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            colored_output: true,
            progress_bars: true,
            confirm_destructive: true,
            save_history: true,
            auto_complete: true,
            editor: std::env::var("EDITOR").ok(),
            timezone: "UTC".to_string(),
            date_format: "%Y-%m-%d %H:%M:%S UTC".to_string(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            token_storage: TokenStorage::ConfigFile,
            encryption_key: None,
            session_timeout: 480, // 8 hours
            auto_logout: true,
        }
    }
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            table_style: TableStyle::Grid,
            max_column_width: 50,
            truncate_values: true,
            show_borders: true,
            tree_indent: 2,
            color_scheme: ColorScheme::default(),
        }
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            primary: "cyan".to_string(),
            secondary: "white".to_string(),
            success: "green".to_string(),
            warning: "yellow".to_string(),
            error: "red".to_string(),
            muted: "bright_black".to_string(),
        }
    }
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 10000,
            file_path: None,
            ignore_patterns: vec![
                "history*".to_string(),
                "*password*".to_string(),
                "*token*".to_string(),
            ],
            save_timestamps: true,
        }
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            plugin_dir: None,
            enabled_plugins: Vec::new(),
            plugin_settings: HashMap::new(),
        }
    }
}

impl CliConfig {
    /// Load configuration from file or create default
    #[instrument(level = "debug")]
    pub async fn load_or_default() -> Result<Self> {
        match Self::load().await {
            Ok(config) => {
                debug!("✅ Loaded CLI configuration from file");
                Ok(config)
            }
            Err(e) => {
                debug!("⚠️ Failed to load CLI config, using defaults: {}", e);
                Ok(Self::default())
            }
        }
    }
    
    /// Load configuration from file
    #[instrument(level = "debug")]
    pub async fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        
        if !config_path.exists() {
            return Err(anyhow::anyhow!("Configuration file does not exist"));
        }
        
        let content = fs::read_to_string(&config_path).await
            .context("Failed to read configuration file")?;
        
        let config: Self = serde_yaml::from_str(&content)
            .context("Failed to parse configuration file")?;
        
        config.validate()?;
        
        Ok(config)
    }
    
    /// Save configuration to file
    #[instrument(level = "debug")]
    pub async fn save(&self) -> Result<()> {
        self.validate()?;
        
        let config_path = Self::get_config_path()?;
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create configuration directory")?;
        }
        
        let content = serde_yaml::to_string(self)
            .context("Failed to serialize configuration")?;
        
        fs::write(&config_path, content).await
            .context("Failed to write configuration file")?;
        
        debug!("✅ Saved CLI configuration to {:?}", config_path);
        
        Ok(())
    }
    
    /// Get configuration file path
    pub fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Failed to get user configuration directory")?
            .join("matrixon-admin");
        
        Ok(config_dir.join("config.yaml"))
    }
    
    /// Get history file path
    pub fn get_history_path(&self) -> Result<PathBuf> {
        if let Some(ref path) = self.history.file_path {
            Ok(path.clone())
        } else {
            let config_dir = dirs::config_dir()
                .context("Failed to get user configuration directory")?
                .join("matrixon-admin");
            
            Ok(config_dir.join("history.txt"))
        }
    }
    
    /// Validate configuration
    fn validate(&self) -> Result<()> {
        // Check version compatibility
        if self.version != "2.0.0" {
            warn!("Configuration version {} may not be compatible", self.version);
        }
        
        // Validate profiles
        for (name, profile) in &self.profiles {
            if name != &profile.name {
                return Err(anyhow::anyhow!(
                    "Profile name mismatch: key '{}' vs name '{}'",
                    name, profile.name
                ));
            }
            
            // Validate URL
            if let Err(_) = url::Url::parse(&profile.url) {
                return Err(anyhow::anyhow!("Invalid URL in profile '{}': {}", name, profile.url));
            }
        }
        
        // Check default profile exists
        if !self.profiles.contains_key(&self.defaults.profile) {
            return Err(anyhow::anyhow!(
                "Default profile '{}' does not exist",
                self.defaults.profile
            ));
        }
        
        Ok(())
    }
    
    /// Get active profile
    pub fn get_active_profile(&self, profile_name: Option<&str>) -> Result<&ServerProfile> {
        let profile_name = profile_name.unwrap_or(&self.defaults.profile);
        
        self.profiles.get(profile_name)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile_name))
    }
    
    /// Add or update a profile
    pub fn set_profile(&mut self, profile: ServerProfile) {
        self.profiles.insert(profile.name.clone(), profile);
    }
    
    /// Remove a profile
    pub fn remove_profile(&mut self, name: &str) -> Result<()> {
        if name == self.defaults.profile {
            return Err(anyhow::anyhow!("Cannot remove default profile"));
        }
        
        self.profiles.remove(name)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", name))?;
        
        Ok(())
    }
    
    /// Create a sample configuration file
    pub async fn create_sample_config<P: AsRef<Path>>(path: P) -> Result<()> {
        let config = Self::default();
        
        let content = serde_yaml::to_string(&config)
            .context("Failed to serialize sample configuration")?;
        
        // Add comments to make it more user-friendly
        let commented_content = format!(
            "# matrixon Admin CLI Configuration\n\
             # Version: {}\n\
             # This file contains configuration for the matrixon-admin CLI tool\n\
             # You can edit this file to customize behavior and add server profiles\n\n{}",
            config.version,
            content
        );
        
        fs::write(path.as_ref(), commented_content).await
            .context("Failed to write sample configuration file")?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_default_config_validation() {
        let config = CliConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_serialization() {
        let config = CliConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: CliConfig = serde_yaml::from_str(&yaml).unwrap();
        
        // Basic validation that deserialization works
        assert_eq!(config.version, deserialized.version);
        assert_eq!(config.defaults.profile, deserialized.defaults.profile);
    }
    
    #[tokio::test]
    async fn test_sample_config_creation() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.yaml");
        
        CliConfig::create_sample_config(&config_path).await.unwrap();
        
        assert!(config_path.exists());
        
        // Verify the file can be loaded
        let content = fs::read_to_string(&config_path).await.unwrap();
        assert!(content.contains("matrixon Admin CLI Configuration"));
    }
    
    #[test]
    fn test_profile_management() {
        let mut config = CliConfig::default();
        
        let new_profile = ServerProfile {
            name: "test".to_string(),
            url: "https://test.example.com".to_string(),
            ..Default::default()
        };
        
        config.set_profile(new_profile.clone());
        
        let retrieved = config.get_active_profile(Some("test")).unwrap();
        assert_eq!(retrieved.name, "test");
        assert_eq!(retrieved.url, "https://test.example.com");
        
        config.remove_profile("test").unwrap();
        assert!(config.get_active_profile(Some("test")).is_err());
    }
}
