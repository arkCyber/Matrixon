//! Configuration Management Module
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Date: 2024-03-21
//! Version: 0.1.0
//!
//! Purpose: Defines configuration structures for the Matrixon monitoring system, including metrics, system, health, alert, performance, and logging settings.
//!
//! All code is documented in English, with detailed struct and field documentation, error handling, and performance characteristics.
//! 
//! This module defines configuration structures for the Matrixon monitoring system.
//! It includes settings for metrics collection, logging, health checks, and more.

use serde::{Deserialize, Serialize};

/// Main monitoring configuration
///
/// The MonitorConfig struct aggregates all configuration options for the monitoring system.
/// Includes metrics, system, health, alert, performance, and logging configurations.
///
/// # Example
/// ```rust
/// let config = MonitorConfig::default();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct MonitorConfig {
    /// Metrics configuration
    pub metrics: MetricsConfig,
    /// System monitoring configuration
    pub system: SystemConfig,
    /// Health check configuration
    pub health: HealthConfig,
    /// Alert configuration
    pub alert: AlertConfig,
    /// Performance monitoring configuration
    pub performance: PerformanceConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Metrics configuration
///
/// The MetricsConfig struct defines settings for metrics collection, retention, and export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    /// Metrics collection interval in seconds
    pub collection_interval_seconds: u64,
    /// Metrics retention period in days
    pub retention_days: u32,
    /// Enable Prometheus metrics export
    pub prometheus_enabled: bool,
    /// Prometheus metrics endpoint
    pub prometheus_endpoint: String,
    /// Custom metrics
    pub custom_metrics: Vec<CustomMetric>,
}

/// Custom metric configuration
///
/// The CustomMetric struct defines a user-defined metric for collection and export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetric {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Metric description
    pub description: String,
    /// Metric labels
    pub labels: Vec<String>,
}

/// Metric type
///
/// The MetricType enum specifies the type of metric (counter, gauge, histogram, summary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    /// Counter metric
    Counter,
    /// Gauge metric
    Gauge,
    /// Histogram metric
    Histogram,
    /// Summary metric
    Summary,
}

/// System monitoring configuration
///
/// The SystemConfig struct defines settings for system-level metrics collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Collection interval in seconds
    pub collection_interval_secs: u64,
    /// Whether to monitor CPU usage
    pub monitor_cpu: bool,
    /// Whether to monitor memory usage
    pub monitor_memory: bool,
    /// Whether to monitor disk usage
    pub monitor_disk: bool,
    /// Whether to monitor network usage
    pub monitor_network: bool,
}

/// Health check configuration
///
/// The HealthConfig struct defines settings for health check endpoints and status tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Enable health checks
    pub enabled: bool,
    /// Health check interval in seconds
    pub interval_seconds: u64,
    /// Health check timeout in seconds
    pub timeout_seconds: u64,
    /// Health check endpoints
    pub endpoints: Vec<HealthEndpoint>,
}

/// Health check endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthEndpoint {
    /// Endpoint name
    pub name: String,
    /// Endpoint path
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Expected status code
    pub expected_status: u16,
    /// Timeout in seconds
    pub timeout_seconds: u64,
}

/// Alert configuration
///
/// The AlertConfig struct defines settings for alert rule management and notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Enable alerting
    pub enabled: bool,
    /// Alert rules
    pub rules: Vec<AlertRule>,
    /// Notification channels
    pub channels: Vec<NotificationChannel>,
    /// Alert cooldown period in minutes
    pub cooldown_minutes: u64,
}

/// Alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name
    pub name: String,
    /// Alert condition
    pub condition: AlertCondition,
    /// Alert threshold
    pub threshold: f64,
    /// Alert duration in minutes
    pub duration_minutes: u64,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Notification channels
    pub channels: Vec<String>,
    /// Enable rule
    pub enabled: bool,
}

/// Alert condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// Memory usage percentage
    MemoryUsagePercent,
    /// CPU usage percentage
    CpuUsagePercent,
    /// Disk usage percentage
    DiskUsagePercent,
    /// Response time in milliseconds
    ResponseTimeMs,
    /// Error rate
    ErrorRate,
    /// Federation failures
    FederationFailures,
    /// Database connections
    DatabaseConnections,
    /// Active users
    ActiveUsers,
}

impl AlertCondition {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertCondition::MemoryUsagePercent => "memory_usage_percent",
            AlertCondition::CpuUsagePercent => "cpu_usage_percent",
            AlertCondition::DiskUsagePercent => "disk_usage_percent",
            AlertCondition::ResponseTimeMs => "response_time_ms",
            AlertCondition::ErrorRate => "error_rate",
            AlertCondition::FederationFailures => "federation_failures",
            AlertCondition::DatabaseConnections => "database_connections",
            AlertCondition::ActiveUsers => "active_users",
        }
    }
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Critical severity - immediate attention required
    Critical,
    /// High severity - urgent attention required
    High,
    /// Medium severity - attention required
    Medium,
    /// Low severity - informational
    Low,
    /// Warning severity - potential issue
    Warning,
    /// Info severity - informational only
    Info,
}

/// Notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Channel name
    pub name: String,
    /// Channel type
    pub channel_type: ChannelType,
    /// Channel configuration
    pub config: ChannelConfig,
    /// Enable channel
    pub enabled: bool,
}

/// Channel type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelType {
    /// Email channel
    Email,
    /// Slack channel
    Slack,
    /// Webhook channel
    Webhook,
    /// Matrix channel
    Matrix,
    /// Discord channel
    Discord,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::Email => write!(f, "Email"),
            ChannelType::Slack => write!(f, "Slack"),
            ChannelType::Webhook => write!(f, "Webhook"),
            ChannelType::Matrix => write!(f, "Matrix"),
            ChannelType::Discord => write!(f, "Discord"),
        }
    }
}

/// Channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Channel URL
    pub url: Option<String>,
    /// Channel token
    pub token: Option<String>,
    /// Channel recipients
    pub recipients: Vec<String>,
    /// Custom fields
    pub custom_fields: std::collections::HashMap<String, String>,
}

/// Performance monitoring configuration
///
/// The PerformanceConfig struct defines settings for performance metrics collection and analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable performance monitoring
    pub enabled: bool,
    /// Monitoring interval in seconds
    pub monitoring_interval_seconds: u64,
    /// Metrics retention period in days
    pub metrics_retention_days: u32,
    /// Enable profiling
    pub enable_profiling: bool,
    /// Track memory usage
    pub track_memory_usage: bool,
    /// Memory report interval in minutes
    pub memory_report_interval: u64,
}

/// Logging configuration
///
/// The LoggingConfig struct defines settings for structured logging, log levels, and output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub log_level: String,
    /// Enable file logging
    pub enable_file_logging: bool,
    /// Log directory
    pub log_directory: String,
    /// Maximum file size in bytes
    pub max_file_size: u64,
    /// Maximum number of files
    pub max_files: u32,
    /// Enable JSON format
    pub enable_json_format: bool,
}


impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval_seconds: 30,
            retention_days: 7,
            prometheus_enabled: true,
            prometheus_endpoint: "/metrics".to_string(),
            custom_metrics: Vec::new(),
        }
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            collection_interval_secs: 60,
            monitor_cpu: true,
            monitor_memory: true,
            monitor_disk: true,
            monitor_network: true,
        }
    }
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 30,
            timeout_seconds: 5,
            endpoints: vec![
                HealthEndpoint {
                    name: "health".to_string(),
                    path: "/health".to_string(),
                    method: "GET".to_string(),
                    expected_status: 200,
                    timeout_seconds: 5,
                },
            ],
        }
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: Vec::new(),
            channels: Vec::new(),
            cooldown_minutes: 5,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monitoring_interval_seconds: 60,
            metrics_retention_days: 7,
            enable_profiling: true,
            track_memory_usage: true,
            memory_report_interval: 15,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            enable_file_logging: true,
            log_directory: "logs".to_string(),
            max_file_size: 1024 * 1024 * 10, // 10MB
            max_files: 5,
            enable_json_format: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MonitorConfig::default();
        assert!(config.metrics.enabled);
        assert!(config.system.monitor_cpu);
        assert!(config.system.monitor_memory);
        assert!(config.system.monitor_disk);
        assert!(config.system.monitor_network);
        assert_eq!(config.logging.log_level, "info");
    }

    #[test]
    fn test_config_serialization() {
        let config = MonitorConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: MonitorConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config.metrics.enabled, deserialized.metrics.enabled);
        assert_eq!(config.system.collection_interval_secs, deserialized.system.collection_interval_secs);
        assert_eq!(config.health.enabled, deserialized.health.enabled);
        assert_eq!(config.alert.enabled, deserialized.alert.enabled);
        assert_eq!(config.performance.enabled, deserialized.performance.enabled);
        assert_eq!(config.logging.log_level, deserialized.logging.log_level);
    }
}
