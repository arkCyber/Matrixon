//! System Monitoring Module
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Date: 2024-03-21
//! Version: 0.1.0
//!
//! Purpose: Implements system-level metrics collection for the Matrixon monitoring system, including CPU, memory, disk, and network statistics.
//!
//! All code is documented in English, with detailed function documentation, error handling, and performance characteristics.
//! 
//! This module handles the collection of system-level metrics such as CPU usage,
//! memory usage, disk I/O, and network statistics.

use std::sync::Arc;
use tracing::{info, instrument};
use sysinfo::{System, Disks, Networks};
use serde::{Serialize, Deserialize};
use crate::metrics::MetricsManager;
use crate::config::SystemConfig;
use super::error::Result;
use tokio::sync::RwLock;

/// System metrics data structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
}

/// System monitor for collecting system metrics
#[derive(Debug)]
pub struct SystemMonitor {
    config: SystemConfig,
    metrics: Arc<MetricsManager>,
    system: Arc<RwLock<System>>,
    disks: Arc<RwLock<Disks>>,
    networks: Arc<RwLock<Networks>>,
}

impl Clone for SystemMonitor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            system: self.system.clone(),
            disks: self.disks.clone(),
            networks: self.networks.clone(),
        }
    }
}

impl SystemMonitor {
    /// Create a new system monitor instance
    pub fn new(config: SystemConfig, metrics: Arc<MetricsManager>) -> Self {
        Self {
            config,
            metrics,
            system: Arc::new(RwLock::new(System::new_all())),
            disks: Arc::new(RwLock::new(Disks::new_with_refreshed_list())),
            networks: Arc::new(RwLock::new(Networks::new_with_refreshed_list())),
        }
    }

    /// Collect system metrics
    #[instrument(skip(self))]
    pub async fn collect_metrics(&self) -> Result<()> {
        let mut sys = self.system.write().await;
        let system = &mut *sys;
        system.refresh_all();

        // CPU metrics
        let cpu_usage = system.global_cpu_info().cpu_usage();
        self.metrics.record_cpu_usage(cpu_usage as f64);

        // Memory metrics
        let memory_usage = (system.used_memory() as f64 / system.total_memory() as f64) * 100.0;
        self.metrics.record_memory_usage(system.used_memory());

        // Disk metrics
        let mut disks = self.disks.write().await;
        disks.refresh();
        let mut total_disk_usage = 0.0;
        let mut disk_count = 0;
        for disk in disks.list() {
            let usage = (disk.available_space() as f64 / disk.total_space() as f64) * 100.0;
            total_disk_usage += usage;
            disk_count += 1;
        }
        let avg_disk_usage = if disk_count > 0 { total_disk_usage / disk_count as f64 } else { 0.0 };

        // Network metrics
        let mut networks = self.networks.write().await;
        networks.refresh();
        let mut total_network_usage = 0.0;
        for network in networks.list().values() {
            total_network_usage += (network.received() + network.transmitted()) as f64;
        }

        info!(
            "System metrics collected - CPU: {:.1}%, Memory: {:.1}%, Disk: {:.1}%, Network: {:.1} bytes",
            cpu_usage, memory_usage, avg_disk_usage, total_network_usage
        );

        Ok(())
    }

    /// Get current system metrics as a HashMap
    #[instrument(skip(self))]
    pub async fn get_metrics(&self) -> Result<SystemMetrics> {
        let mut sys = self.system.write().await;
        let system = &mut *sys;
        system.refresh_all();

        let cpu_usage = system.global_cpu_info().cpu_usage();
        let memory_usage = (system.used_memory() as f32 / system.total_memory() as f32) * 100.0;

        // Disk metrics
        let mut disks = self.disks.write().await;
        disks.refresh();
        let mut total_disk_usage = 0.0;
        let mut disk_count = 0;
        for disk in disks.list() {
            let usage = (disk.available_space() as f32 / disk.total_space() as f32) * 100.0;
            total_disk_usage += usage;
            disk_count += 1;
        }
        let avg_disk_usage = if disk_count > 0 { total_disk_usage / disk_count as f32 } else { 0.0 };

        // Network metrics
        let mut networks = self.networks.write().await;
        networks.refresh();
        let mut total_network_usage = 0.0;
        for network in networks.list().values() {
            total_network_usage += (network.received() + network.transmitted()) as f32;
        }

        Ok(SystemMetrics {
            cpu_usage,
            memory_usage,
            disk_usage: avg_disk_usage,
            network_usage: total_network_usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MetricsConfig;

    #[tokio::test]
    async fn test_system_monitor() -> Result<()> {
        let config = SystemConfig::default();
        let metrics_config = MetricsConfig::default();
        let metrics = Arc::new(MetricsManager::new(metrics_config)?);
        let monitor = SystemMonitor::new(config, metrics);

        monitor.collect_metrics().await?;
        let metrics = monitor.get_metrics().await?;
        assert!(metrics.cpu_usage >= 0.0);
        assert!(metrics.memory_usage >= 0.0);
        assert!(metrics.disk_usage >= 0.0);
        assert!(metrics.network_usage >= 0.0);
        Ok(())
    }
}
