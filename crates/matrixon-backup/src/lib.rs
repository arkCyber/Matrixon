//! Matrixon Backup Module
//!
//! Provides comprehensive backup functionality for Matrixon server, including:
//! - Database backups
//! - Configuration backups
//! - Scheduled backups
//! - Compression and encryption
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2025-06-15

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use tracing::info;

pub mod error;
pub mod database;
pub mod utils;
pub mod scheduler;

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Whether backups are enabled
    pub enabled: bool,
    /// Base directory for storing backups
    pub base_dir: PathBuf,
    /// Database connection URL (e.g. postgres://user:pass@host/db)
    pub database_url: String,
    /// Maximum number of backups to retain
    pub max_backups: usize,
    /// Compression level (0-9)
    pub compression_level: u32,
    /// Schedule configuration
    pub schedule: Option<BackupScheduleConfig>,
}

/// Backup schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupScheduleConfig {
    /// Whether scheduled backups are enabled
    pub enabled: bool,
    /// Interval between backups in hours
    pub interval_hours: u32,
}

/// Perform a complete backup operation
#[tracing::instrument]
pub async fn perform_backup(config: &BackupConfig) -> Result<(), error::BackupError> {
    if !config.enabled {
        info!("🔕 Backups are disabled in configuration");
        return Ok(());
    }

    info!("💾 Starting backup operation");
    
    // Create backup directory if needed
    utils::BackupUtils::validate_backup_dir(&config.base_dir)?;

    // Perform database backup
    database::backup_database(config).await?;

    info!("✅ Backup completed successfully");
    Ok(())
}

/// Initialize backup system
pub async fn init_backup_system(config: BackupConfig) -> Result<scheduler::BackupScheduler, error::BackupError> {
    let mut scheduler = scheduler::BackupScheduler::new(config);
    scheduler.start()?;
    Ok(scheduler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_backup_disabled() {
        let temp_dir = tempdir().unwrap();
        let config = BackupConfig {
            enabled: false,
            base_dir: temp_dir.path().to_path_buf(),
            database_url: "postgres://matrixon:password@localhost/matrixon".to_string(),
            max_backups: 5,
            compression_level: 6,
            schedule: None,
        };

        assert!(perform_backup(&config).await.is_ok());
    }
}
