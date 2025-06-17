//! Scheduled backup functionality for Matrixon
//!
//! Handles scheduling and managing automated backups
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2025-06-15

use std::time::Duration;
use tokio::time;
use tracing::{info, error, instrument};
use super::{BackupConfig, error::BackupError};

/// Manages scheduled backup operations
#[derive(Debug)]
pub struct BackupScheduler {
    config: BackupConfig,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl BackupScheduler {
    /// Create a new BackupScheduler instance
    pub fn new(config: BackupConfig) -> Self {
        Self {
            config,
            handle: None,
        }
    }

    /// Start the backup scheduler
    #[instrument]
    pub fn start(&mut self) -> Result<(), BackupError> {
        if self.handle.is_some() {
            return Err(BackupError::other("Scheduler already running"));
        }

        let config = self.config.clone();
        self.handle = Some(tokio::spawn(async move {
            Self::run_scheduled_backups(config).await;
        }));

        info!("⏰ Backup scheduler started");
        Ok(())
    }

    /// Stop the backup scheduler
    #[instrument]
    pub async fn stop(&mut self) -> Result<(), BackupError> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            let _ = handle.await;
            info!("🛑 Backup scheduler stopped");
        }
        Ok(())
    }

    /// Main backup scheduling loop
    #[instrument]
    async fn run_scheduled_backups(config: BackupConfig) {
        let interval = Duration::from_secs(
            config.schedule.as_ref().unwrap().interval_hours as u64 * 3600
        );

        let mut interval = time::interval(interval);
        interval.tick().await; // Skip immediate first tick

        loop {
            interval.tick().await;
            
            info!("⏰ Running scheduled backup");
            if let Err(e) = super::perform_backup(&config).await {
                error!("❌ Scheduled backup failed: {}", e);
            }
        }
    }
}

impl Drop for BackupScheduler {
    fn drop(&mut self) {
        if self.handle.is_some() {
            let _ = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(self.stop());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_scheduler_lifecycle() {
        let temp_dir = tempdir().unwrap();
        let config = BackupConfig {
            enabled: true,
            base_dir: temp_dir.path().to_path_buf(),
            database_url: "postgres://matrixon:password@localhost/matrixon".to_string(),
            max_backups: 5,
            compression_level: 6,
            schedule: Some(crate::BackupScheduleConfig {
                enabled: true,
                interval_hours: 1,
            }),
        };

        let mut scheduler = BackupScheduler::new(config);
        assert!(scheduler.start().is_ok());
        assert!(scheduler.stop().await.is_ok());
    }
}
