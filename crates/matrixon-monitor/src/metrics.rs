//! Metrics Collection System
//! 
//! This module handles the collection and export of Prometheus metrics for the Matrixon server.
//! It includes application metrics, system metrics, and custom business metrics.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2024-03-21
//!
//! Purpose: Implements metrics collection, export, and management for the Matrixon monitoring system. 

use std::collections::HashMap;
use std::time::Duration;
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use tracing::instrument;
use crate::config::MetricsConfig;
use super::error::{Result, MonitorError};

/// Metrics manager for collecting and exposing metrics
pub struct MetricsManager {
    config: MetricsConfig,
    handle: PrometheusHandle,
}

impl MetricsManager {
    /// Create a new metrics manager instance
    pub fn new(config: MetricsConfig) -> Result<Self> {
        let builder = PrometheusBuilder::new();
        let handle = builder
            .install_recorder()
            .map_err(|e| MonitorError::MetricsError(format!("Failed to install metrics recorder: {}", e)))?;

        Ok(Self { 
            config, 
            handle
        })
    }

    /// Get current metrics as HashMap
    #[instrument(level = "debug", skip(self))]
    pub async fn get_metrics(&self) -> Result<HashMap<String, f64>> {
        let prometheus_output = self.handle.render();
        let mut metrics = HashMap::new();
        
        for line in prometheus_output.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(value) = parts[1].parse::<f64>() {
                    metrics.insert(parts[0].to_string(), value);
                }
            }
        }
        
        Ok(metrics)
    }

    /// Record a custom metric
    #[instrument(skip(self), level = "debug")]
    pub async fn record_custom_metric(&self, name: &str, value: f64) -> Result<()> {
        gauge!("matrixon_custom_metric", value, "name" => name.to_string());
        Ok(())
    }

    /// Record request duration
    #[instrument(skip(self), level = "debug")]
    pub fn record_request_duration(&self, path: &str, method: &str, duration: Duration) {
        histogram!("matrixon_request_duration_seconds", duration.as_secs_f64(), 
            "path" => path.to_string(),
            "method" => method.to_string()
        );
    }

    /// Record request count
    #[instrument(skip(self), level = "debug")]
    pub fn record_request_count(&self, path: &str, method: &str, status: u16) {
        counter!("matrixon_requests_total", 1,
            "path" => path.to_string(),
            "method" => method.to_string(),
            "status" => status.to_string()
        );
    }

    /// Record error count
    #[instrument(skip(self), level = "debug")]
    pub fn record_error_count(&self, error_type: &str) {
        counter!("matrixon_errors_total", 1, "type" => error_type.to_string());
    }

    /// Record memory usage
    #[instrument(skip(self), level = "debug")]
    pub fn record_memory_usage(&self, bytes: u64) {
        gauge!("matrixon_memory_usage_bytes", bytes as f64);
    }

    /// Record CPU usage
    #[instrument(skip(self), level = "debug")]
    pub fn record_cpu_usage(&self, percentage: f64) {
        gauge!("matrixon_cpu_usage_percent", percentage);
    }

    /// Record active connections
    #[instrument(skip(self), level = "debug")]
    pub fn record_active_connections(&self, count: u64) {
        gauge!("matrixon_active_connections", count as f64);
    }

    /// Record database query duration
    #[instrument(skip(self), level = "debug")]
    pub fn record_db_query_duration(&self, query_type: &str, duration: Duration) {
        histogram!("matrixon_db_query_duration_seconds", duration.as_secs_f64(),
            "query_type" => query_type.to_string()
        );
    }

    /// Record cache operations
    #[instrument(skip(self), level = "debug")]
    pub fn record_cache_operation(&self, operation: &str, result: &str) {
        counter!("matrixon_cache_operations_total", 1,
            "operation" => operation.to_string(),
            "result" => result.to_string()
        );
    }
}

impl std::fmt::Debug for MetricsManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsManager")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::OnceCell;
    use std::sync::Mutex;

    static TEST_MANAGER: OnceCell<Mutex<MetricsManager>> = OnceCell::new();

    fn get_test_manager() -> Result<&'static Mutex<MetricsManager>> {
        TEST_MANAGER.get_or_try_init(|| {
            let config = MetricsConfig::default();
            let manager = MetricsManager::new(config)?;
            Ok(Mutex::new(manager))
        })
    }

    #[tokio::test]
    async fn test_metrics_manager() -> Result<()> {
        let manager = get_test_manager()?.lock().unwrap();
        manager.record_custom_metric("test_metric", 42.0).await?;
        let metrics = manager.get_metrics().await?;
        assert!(metrics.contains_key("matrixon_custom_metric"));
        Ok(())
    }
}
