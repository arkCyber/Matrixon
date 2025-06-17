//! Integration tests for Matrixon backup functionality
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2025-06-15

use matrixon_backup::{BackupConfig, BackupError};
use tempfile::tempdir;
use std::path::PathBuf;

#[tokio::test]
async fn test_backup_disabled() {
    let temp_dir = tempdir().unwrap();
    let config = BackupConfig {
        enabled: false,
        base_dir: temp_dir.path().to_path_buf(),
        max_backups: 5,
        compression_level: 6,
        schedule: None,
    };

    let result = matrixon_backup::perform_backup(&config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_invalid_backup_dir() {
    let invalid_path = PathBuf::from("/invalid/path");
    let config = BackupConfig {
        enabled: true,
        base_dir: invalid_path,
        max_backups: 5,
        compression_level: 6,
        schedule: None,
    };

    let result = matrixon_backup::perform_backup(&config).await;
    assert!(matches!(result, Err(BackupError::Io(_)));
}

#[tokio::test]
async fn test_scheduler_lifecycle() {
    let temp_dir = tempdir().unwrap();
    let config = BackupConfig {
        enabled: true,
        base_dir: temp_dir.path().to_path_buf(),
        max_backups: 5,
        compression_level: 6,
        schedule: Some(matrixon_backup::BackupScheduleConfig {
            enabled: true,
            interval_hours: 1,
        }),
    };

    let mut scheduler = matrixon_backup::scheduler::BackupScheduler::new(config);
    assert!(scheduler.start().is_ok());
    assert!(scheduler.stop().await.is_ok());
}

#[tokio::test]
async fn test_utils_validation() {
    let temp_dir = tempdir().unwrap();
    let valid_path = temp_dir.path().join("backups");
    
    let result = matrixon_backup::utils::BackupUtils::validate_backup_dir(&valid_path);
    assert!(result.is_ok());
    assert!(valid_path.exists());
}
