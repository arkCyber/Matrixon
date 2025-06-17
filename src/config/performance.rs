// =============================================================================
// Matrixon Matrix NextServer - Performance Module
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
//   Configuration management and validation. This module is part of the Matrixon Matrix NextServer
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
//   • Configuration parsing and validation
//   • Environment variable handling
//   • Default value management
//   • Type-safe configuration
//   • Runtime configuration updates
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

use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn, error, debug};
use std::time::{SystemTime, UNIX_EPOCH};

// Helper function for getting current timestamp
fn get_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

/// High-performance configuration settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    /// Server configuration
    pub server: ServerPerformanceConfig,
    
    /// Database performance settings
    pub database: DatabasePerformanceConfig,
    
    /// Testing configuration
    pub testing: Option<TestingConfig>,
    
    /// Performance optimization settings
    pub performance: OptimizationConfig,
    
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    
    /// Performance thresholds
    pub thresholds: Option<ThresholdConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerPerformanceConfig {
    pub max_concurrent_requests: u32,
    pub max_request_size: u32,
    pub worker_threads: Option<u16>,
    pub blocking_threads: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabasePerformanceConfig {
    pub backend: String,
    pub path: String,
    pub db_cache_capacity_mb: f64,
    pub matrixon_cache_capacity_modifier: f64,
    pub pdu_cache_capacity: u32,
    pub connection_pool_size: Option<u32>,
    pub connection_timeout_seconds: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestingConfig {
    pub concurrent_connections: u32,
    pub operations_per_connection: u32,
    pub test_duration_seconds: u64,
    pub batch_size: u32,
    pub max_latency_ms: u64,
    pub stress_test_connections: Option<u32>,
    pub load_test_duration: Option<u64>,
    pub memory_test_iterations: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OptimizationConfig {
    pub cleanup_second_interval: u32,
    pub allow_check_for_updates: bool,
    pub batch_timeout_ms: Option<u64>,
    pub max_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    pub enable_metrics: bool,
    pub metrics_port: Option<u16>,
    pub log_level: String,
    pub enable_profiling: Option<bool>,
    pub track_memory_usage: Option<bool>,
    pub memory_report_interval: Option<u64>,
    pub metrics_collection_interval: Option<u64>,
    pub metrics_retention_days: Option<u32>,
    pub enable_alerting: Option<bool>,
    pub alert_thresholds: Option<AlertThresholds>,
    pub enable_tracing: Option<bool>,
    pub tracing_sampling_rate: Option<f64>,
    pub enable_health_checks: Option<bool>,
    pub health_check_interval: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlertThresholds {
    pub cpu_usage_threshold: f64,
    pub memory_usage_threshold: f64,
    pub disk_usage_threshold: f64,
    pub error_rate_threshold: f64,
    pub latency_threshold_ms: u64,
    pub connection_count_threshold: u32,
    pub database_connection_threshold: u32,
    pub cache_hit_rate_threshold: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThresholdConfig {
    pub min_throughput_ops_per_sec: u32,
    pub max_average_latency_ms: u64,
    pub min_success_rate_percent: f64,
    pub max_memory_usage_mb: u32,
    pub max_cpu_usage_percent: f64,
    pub min_db_cache_hit_ratio: Option<f64>,
    pub max_db_connections: Option<u32>,
    pub max_query_time_ms: Option<u64>,
    pub max_error_rate_percent: Option<f64>,
    pub max_connection_count: Option<u32>,
    pub max_disk_usage_percent: Option<f64>,
    pub min_available_memory_mb: Option<u32>,
    pub max_background_tasks: Option<u32>,
    pub max_queue_size: Option<u32>,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        debug!("⏰ [{}] Creating default PerformanceConfig", timestamp);
        
        Self {
            server: ServerPerformanceConfig {
                max_concurrent_requests: 200000,
                max_request_size: 50_000_000,
                worker_threads: Some(128),
                blocking_threads: Some(64),
            },
            database: DatabasePerformanceConfig {
                backend: "postgresql".to_string(),
                path: "postgresql://matrixon:matrixon@localhost:5432/matrixon".to_string(),
                db_cache_capacity_mb: 32768.0,
                matrixon_cache_capacity_modifier: 3.0,
                pdu_cache_capacity: 20000000,
                connection_pool_size: Some(2000),
                connection_timeout_seconds: Some(15),
            },
            testing: Some(TestingConfig {
                concurrent_connections: 20000,
                operations_per_connection: 500,
                test_duration_seconds: 60,
                batch_size: 2000,
                max_latency_ms: 50,
                stress_test_connections: Some(200000),
                load_test_duration: Some(300),
                memory_test_iterations: Some(100000),
            }),
            performance: OptimizationConfig {
                cleanup_second_interval: 60,
                allow_check_for_updates: false,
                batch_timeout_ms: Some(50),
                max_batch_size: Some(5000),
            },
            monitoring: MonitoringConfig {
                enable_metrics: true,
                metrics_port: Some(9090),
                log_level: "info".to_string(),
                enable_profiling: Some(true),
                track_memory_usage: Some(true),
                memory_report_interval: Some(15),
                metrics_collection_interval: Some(60),
                metrics_retention_days: Some(7),
                enable_alerting: Some(true),
                alert_thresholds: Some(AlertThresholds {
                    cpu_usage_threshold: 85.0,
                    memory_usage_threshold: 90.0,
                    disk_usage_threshold: 80.0,
                    error_rate_threshold: 0.01,
                    latency_threshold_ms: 50,
                    connection_count_threshold: 2000,
                    database_connection_threshold: 1000,
                    cache_hit_rate_threshold: 95.0,
                }),
                enable_tracing: Some(true),
                tracing_sampling_rate: Some(0.1),
                enable_health_checks: Some(true),
                health_check_interval: Some(300),
            },
            thresholds: Some(ThresholdConfig {
                min_throughput_ops_per_sec: 50000,
                max_average_latency_ms: 50,
                min_success_rate_percent: 99.0,
                max_memory_usage_mb: 65536,
                max_cpu_usage_percent: 85.0,
                min_db_cache_hit_ratio: Some(95.0),
                max_db_connections: Some(2000),
                max_query_time_ms: Some(25),
                max_error_rate_percent: Some(0.01),
                max_connection_count: Some(2000),
                max_disk_usage_percent: Some(80.0),
                min_available_memory_mb: Some(1024),
                max_background_tasks: Some(100),
                max_queue_size: Some(1000),
            }),
        }
    }
}

impl PerformanceConfig {
    /// Load performance configuration from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let timestamp = get_timestamp();
        let path = path.as_ref();
        
        info!("⏰ [{}] 📋 Loading performance configuration from: {}", timestamp, path.display());
        
        // Validate file exists
        if !path.exists() {
            let error_msg = format!("Configuration file does not exist: {}", path.display());
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(error_msg.into());
        }
        
        // Validate file is readable
        if let Err(e) = std::fs::metadata(path) {
            let error_msg = format!("Cannot access configuration file {}: {}", path.display(), e);
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(error_msg.into());
        }
        
        let content = std::fs::read_to_string(path)
            .map_err(|e| {
                let error_msg = format!("Failed to read config file {}: {}", path.display(), e);
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error_msg)) as Box<dyn std::error::Error>
            })?;
        
        if content.trim().is_empty() {
            let error_msg = format!("Configuration file is empty: {}", path.display());
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(error_msg.into());
        }
        
        debug!("⏰ [{}] 📄 Read {} bytes from config file", timestamp, content.len());
        
        let config: PerformanceConfig = toml::from_str(&content)
            .map_err(|e| {
                let error_msg = format!("Failed to parse config file {}: {}", path.display(), e);
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                Box::new(e) as Box<dyn std::error::Error>
            })?;
        
        info!("⏰ [{}] ✅ Performance configuration loaded successfully", timestamp);
        
        // Validate the loaded configuration
        config.validate().map_err(|e| {
            let error_msg = format!("Configuration validation failed: {}", e);
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            e
        })?;
        
        Ok(config)
    }
    
    /// Load test configuration specifically
    pub fn load_test_config() -> Result<Self, Box<dyn std::error::Error>> {
        let timestamp = get_timestamp();
        let test_config_path = "configs/test_performance.toml";
        
        debug!("⏰ [{}] 🧪 Attempting to load test configuration", timestamp);
        
        if Path::new(test_config_path).exists() {
            info!("⏰ [{}] 📋 Found test config file, loading...", timestamp);
            Self::load_from_file(test_config_path)
        } else {
            warn!("⏰ [{}] ⚠️ Test config file not found, using defaults", timestamp);
            Ok(Self::default())
        }
    }
    
    /// Load production configuration
    pub fn load_production_config() -> Result<Self, Box<dyn std::error::Error>> {
        let timestamp = get_timestamp();
        let prod_config_path = "configs/high_performance.toml";
        
        debug!("⏰ [{}] 🏭 Attempting to load production configuration", timestamp);
        
        if Path::new(prod_config_path).exists() {
            info!("⏰ [{}] 📋 Found production config file, loading...", timestamp);
            Self::load_from_file(prod_config_path)
        } else {
            warn!("⏰ [{}] ⚠️ Production config file not found, using defaults", timestamp);
            Ok(Self::default())
        }
    }
    
    /// Validate configuration settings
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = get_timestamp();
        info!("⏰ [{}] 🔍 Validating performance configuration...", timestamp);
        
        // Validate server settings
        if self.server.max_concurrent_requests == 0 {
            let error_msg = "max_concurrent_requests must be greater than 0";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(error_msg.into());
        }
        
        if self.server.max_concurrent_requests > 250000 {
            warn!("⏰ [{}] ⚠️ Very high concurrent requests: {}", timestamp, self.server.max_concurrent_requests);
        }
        
        // Validate worker threads
        if let Some(threads) = self.server.worker_threads {
            if threads == 0 {
                let error_msg = "worker_threads must be greater than 0";
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                return Err(error_msg.into());
            }
            if threads > 512 {
                warn!("⏰ [{}] ⚠️ Very high worker thread count: {}", timestamp, threads);
            }
        }
        
        // Validate database settings
        if self.database.db_cache_capacity_mb <= 0.0 {
            let error_msg = "db_cache_capacity_mb must be greater than 0";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(error_msg.into());
        }
        
        if self.database.db_cache_capacity_mb > 131072.0 {
            warn!("⏰ [{}] ⚠️ Very high database cache: {:.1} MB", timestamp, self.database.db_cache_capacity_mb);
        }
        
        // Validate database backend
        let valid_backends = ["postgresql", "rocksdb", "sqlite"];
        if !valid_backends.contains(&self.database.backend.as_str()) {
            let error_msg = format!("Invalid database backend: {}. Must be one of: {:?}", self.database.backend, valid_backends);
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(error_msg.into());
        }
        
        // Validate connection pool settings
        if let Some(pool_size) = self.database.connection_pool_size {
            if pool_size == 0 {
                let error_msg = "connection_pool_size must be greater than 0";
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                return Err(error_msg.into());
            }
            if pool_size > 5000 {
                warn!("⏰ [{}] ⚠️ Very high connection pool size: {}", timestamp, pool_size);
            }
        }
        
        // Validate testing settings if present
        if let Some(ref testing) = self.testing {
            if testing.concurrent_connections == 0 {
                let error_msg = "testing concurrent_connections must be greater than 0";
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                return Err(error_msg.into());
            }
            
            if testing.concurrent_connections > 250000 {
                warn!("⏰ [{}] ⚠️ Very high test connections: {}", timestamp, testing.concurrent_connections);
            }
            
            if testing.test_duration_seconds == 0 {
                let error_msg = "test_duration_seconds must be greater than 0";
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                return Err(error_msg.into());
            }
            
            if testing.test_duration_seconds > 3600 {
                warn!("⏰ [{}] ⚠️ Very long test duration: {} seconds", timestamp, testing.test_duration_seconds);
            }
        }
        
        // Validate thresholds if present
        if let Some(ref thresholds) = self.thresholds {
            if thresholds.min_success_rate_percent < 0.0 || thresholds.min_success_rate_percent > 100.0 {
                let error_msg = "min_success_rate_percent must be between 0 and 100";
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                return Err(error_msg.into());
            }
            
            if thresholds.max_cpu_usage_percent > 100.0 {
                let error_msg = "max_cpu_usage_percent must not exceed 100";
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                return Err(error_msg.into());
            }
            
            if thresholds.max_cpu_usage_percent <= 0.0 {
                let error_msg = "max_cpu_usage_percent must be greater than 0";
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                return Err(error_msg.into());
            }
        }
        
        // Validate monitoring settings
        if let Some(port) = self.monitoring.metrics_port {
            if port < 1024 {
                warn!("⏰ [{}] ⚠️ Metrics port {} is in reserved range (< 1024)", timestamp, port);
            }
        }
        
        info!("⏰ [{}] ✅ Configuration validation passed", timestamp);
        Ok(())
    }
    
    /// Get testing configuration with fallback
    pub fn get_testing_config(&self) -> TestingConfig {
        let timestamp = get_timestamp();
        
        self.testing.clone().unwrap_or_else(|| {
            warn!("⏰ [{}] ⚠️ No testing config found, using defaults", timestamp);
            TestingConfig {
                concurrent_connections: 5000,
                operations_per_connection: 200,
                test_duration_seconds: 60,
                batch_size: 1000,
                max_latency_ms: 200,
                stress_test_connections: Some(10000),
                load_test_duration: Some(300),
                memory_test_iterations: Some(50000),
            }
        })
    }
    
    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = get_timestamp();
        let path = path.as_ref();
        
        info!("⏰ [{}] 💾 Saving performance configuration to: {}", timestamp, path.display());
        
        // Validate configuration before saving
        self.validate().map_err(|e| {
            let error_msg = format!("Cannot save invalid configuration: {}", e);
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            e
        })?;
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| {
                let error_msg = format!("Failed to serialize configuration: {}", e);
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                Box::new(e) as Box<dyn std::error::Error>
            })?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| {
                    let error_msg = format!("Failed to create directory {}: {}", parent.display(), e);
                    error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                    e
                })?;
        }
        
        std::fs::write(path, content)
            .map_err(|e| {
                let error_msg = format!("Failed to write config file {}: {}", path.display(), e);
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                e
            })?;
        
        info!("⏰ [{}] ✅ Configuration saved successfully", timestamp);
        Ok(())
    }
    
    /// Clone configuration with updated settings
    pub fn with_updated_server(&self, server_config: ServerPerformanceConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let timestamp = get_timestamp();
        debug!("⏰ [{}] 🔄 Updating server configuration", timestamp);
        
        let mut new_config = self.clone();
        new_config.server = server_config;
        
        // Validate the updated configuration
        new_config.validate().map_err(|e| {
            let error_msg = format!("Updated configuration is invalid: {}", e);
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            e
        })?;
        
        Ok(new_config)
    }
    
    /// Log configuration summary
    pub fn log_summary(&self) {
        let timestamp = get_timestamp();
        info!("⏰ [{}] 📊 ===== PERFORMANCE CONFIGURATION SUMMARY =====", timestamp);
        info!("⏰ [{}] 📊 Max Concurrent Requests: {}", timestamp, self.server.max_concurrent_requests);
        info!("⏰ [{}] 📊 Worker Threads: {:?}", timestamp, self.server.worker_threads);
        info!("⏰ [{}] 📊 Database Backend: {}", timestamp, self.database.backend);
        info!("⏰ [{}] 📊 Database Cache: {:.1} MB", timestamp, self.database.db_cache_capacity_mb);
        info!("⏰ [{}] 📊 PDU Cache Capacity: {}", timestamp, self.database.pdu_cache_capacity);
        
        if let Some(ref testing) = self.testing {
            info!("⏰ [{}] 📊 Test Concurrent Connections: {}", timestamp, testing.concurrent_connections);
            info!("⏰ [{}] 📊 Test Duration: {} seconds", timestamp, testing.test_duration_seconds);
        }
        
        if let Some(ref thresholds) = self.thresholds {
            info!("⏰ [{}] 📊 Min Throughput Target: {} ops/sec", timestamp, thresholds.min_throughput_ops_per_sec);
            info!("⏰ [{}] 📊 Max Latency Target: {} ms", timestamp, thresholds.max_average_latency_ms);
        }
        
        info!("⏰ [{}] 📊 ==============================================", timestamp);
    }
    
    /// Get memory usage estimation in MB
    pub fn estimate_memory_usage(&self) -> f64 {
        let timestamp = get_timestamp();
        debug!("⏰ [{}] 📊 Calculating memory usage estimation", timestamp);
        
        // Base memory usage for the application
        let mut total_memory = 512.0; // Base memory in MB
        
        // Add database cache
        total_memory += self.database.db_cache_capacity_mb;
        
        // Add PDU cache (estimate 1KB per PDU)
        let pdu_cache_mb = self.database.pdu_cache_capacity as f64 / 1024.0;
        total_memory += pdu_cache_mb;
        
        // Add worker thread overhead (estimate 8MB per thread)
        if let Some(threads) = self.server.worker_threads {
            total_memory += threads as f64 * 8.0;
        }
        
        // Add connection overhead (estimate 1MB per 100 connections)
        total_memory += self.server.max_concurrent_requests as f64 / 100.0;
        
        debug!("⏰ [{}] 📊 Estimated memory usage: {:.1} MB", timestamp, total_memory);
        total_memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Test default configuration creation
    #[test]
    fn test_default_config() {
        let config = PerformanceConfig::default();
        
        assert_eq!(config.server.max_concurrent_requests, 200000);
        assert_eq!(config.server.worker_threads, Some(128));
        assert_eq!(config.database.backend, "postgresql");
        assert!(config.testing.is_some());
        assert!(config.thresholds.is_some());
        
        // Validation should pass
        assert!(config.validate().is_ok());
    }

    /// Test configuration validation
    #[test]
    fn test_config_validation() {
        let mut config = PerformanceConfig::default();
        
        // Test invalid max_concurrent_requests
        config.server.max_concurrent_requests = 0;
        assert!(config.validate().is_err());
        
        // Reset to valid value
        config.server.max_concurrent_requests = 1000;
        assert!(config.validate().is_ok());
        
        // Test invalid database cache
        config.database.db_cache_capacity_mb = -1.0;
        assert!(config.validate().is_err());
        
        // Reset to valid value
        config.database.db_cache_capacity_mb = 1024.0;
        assert!(config.validate().is_ok());
        
        // Test invalid backend
        config.database.backend = "invalid_backend".to_string();
        assert!(config.validate().is_err());
    }

    /// Test file operations
    #[test]
    fn test_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let config = PerformanceConfig::default();
        
        // Test saving configuration
        assert!(config.save_to_file(&config_path).is_ok());
        assert!(config_path.exists());
        
        // Test loading configuration
        let loaded_config = PerformanceConfig::load_from_file(&config_path).unwrap();
        assert_eq!(loaded_config.server.max_concurrent_requests, config.server.max_concurrent_requests);
        assert_eq!(loaded_config.database.backend, config.database.backend);
    }

    /// Test loading non-existent file
    #[test]
    fn test_load_nonexistent_file() {
        let result = PerformanceConfig::load_from_file("non_existent_file.toml");
        assert!(result.is_err());
    }

    /// Test empty file handling
    #[test]
    fn test_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("empty_config.toml");
        
        // Create empty file
        fs::write(&config_path, "").unwrap();
        
        let result = PerformanceConfig::load_from_file(&config_path);
        assert!(result.is_err());
    }

    /// Test invalid TOML content
    #[test]
    fn test_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid_config.toml");
        
        // Create invalid TOML content
        fs::write(&config_path, "invalid toml content [[[").unwrap();
        
        let result = PerformanceConfig::load_from_file(&config_path);
        assert!(result.is_err());
    }

    /// Test memory usage estimation
    #[test]
    fn test_memory_estimation() {
        let config = PerformanceConfig::default();
        let memory = config.estimate_memory_usage();
        
        assert!(memory > 0.0);
        assert!(memory > config.database.db_cache_capacity_mb); // Should be at least cache size
    }

    /// Test configuration update
    #[test]
    fn test_config_update() {
        let config = PerformanceConfig::default();
        
        let new_server_config = ServerPerformanceConfig {
            max_concurrent_requests: 5000,
            max_request_size: 25_000_000,
            worker_threads: Some(32),
            blocking_threads: Some(16),
        };
        
        let updated_config = config.with_updated_server(new_server_config).unwrap();
        assert_eq!(updated_config.server.max_concurrent_requests, 5000);
        assert_eq!(updated_config.server.worker_threads, Some(32));
        
        // Original config should be unchanged
        assert_eq!(config.server.max_concurrent_requests, 200000);
    }

    /// Test invalid configuration update
    #[test]
    fn test_invalid_config_update() {
        let config = PerformanceConfig::default();
        
        let invalid_server_config = ServerPerformanceConfig {
            max_concurrent_requests: 0, // Invalid
            max_request_size: 25_000_000,
            worker_threads: Some(32),
            blocking_threads: Some(16),
        };
        
        let result = config.with_updated_server(invalid_server_config);
        assert!(result.is_err());
    }

    /// Test testing configuration fallback
    #[test]
    fn test_testing_config_fallback() {
        let mut config = PerformanceConfig::default();
        config.testing = None;
        
        let testing_config = config.get_testing_config();
        assert_eq!(testing_config.concurrent_connections, 5000);
        assert_eq!(testing_config.operations_per_connection, 200);
    }

    /// Test threshold validation
    #[test]
    fn test_threshold_validation() {
        let mut config = PerformanceConfig::default();
        
        // Test invalid success rate (too high)
        if let Some(ref mut thresholds) = config.thresholds {
            thresholds.min_success_rate_percent = 150.0;
        }
        assert!(config.validate().is_err());
        
        // Test invalid success rate (negative)
        if let Some(ref mut thresholds) = config.thresholds {
            thresholds.min_success_rate_percent = -10.0;
        }
        assert!(config.validate().is_err());
        
        // Test invalid CPU usage
        if let Some(ref mut thresholds) = config.thresholds {
            thresholds.min_success_rate_percent = 95.0; // Reset to valid
            thresholds.max_cpu_usage_percent = 150.0;
        }
        assert!(config.validate().is_err());
    }

    /// Test worker thread validation
    #[test]
    fn test_worker_thread_validation() {
        let mut config = PerformanceConfig::default();
        
        // Test zero worker threads
        config.server.worker_threads = Some(0);
        assert!(config.validate().is_err());
        
        // Test valid worker threads
        config.server.worker_threads = Some(16);
        assert!(config.validate().is_ok());
    }

    /// Test connection pool validation
    #[test]
    fn test_connection_pool_validation() {
        let mut config = PerformanceConfig::default();
        
        // Test zero connection pool size
        config.database.connection_pool_size = Some(0);
        assert!(config.validate().is_err());
        
        // Test valid connection pool size
        config.database.connection_pool_size = Some(100);
        assert!(config.validate().is_ok());
    }
} 
