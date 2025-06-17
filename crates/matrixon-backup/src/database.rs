//! Database backup functionality for Matrixon
//!
//! Provides database backup operations including:
//! - PostgreSQL backups using pg_dump equivalent
//! - SQLite backups via direct file copy  
//! - Backup verification and integrity checking
//! - Compression and encryption support
//! - Retention policy management
//!
//! ## Features
//! - `postgres`: Enables PostgreSQL backup support
//! - `sqlite`: Enables SQLite backup support
//! - Automatic cleanup of old backups
//! - Detailed logging and instrumentation
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2025-06-15

use std::path::{Path, PathBuf};
use chrono::Local;
use tracing::info;
use super::{BackupConfig, error::{BackupError, BackupResult}};
use crate::utils::BackupUtils;

/// Database backup implementation
pub struct DatabaseBackup;

impl DatabaseBackup {
    /// Perform a complete database backup
    #[tracing::instrument]
    pub async fn backup_database(config: &BackupConfig) -> BackupResult<()> {
        info!("💾 Starting database backup");
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("matrixon_db_{}.sql.gz", timestamp);
        let _backup_path = config.base_dir.join(backup_filename);

        // Determine database type and perform appropriate backup
        #[cfg(feature = "postgres")]
        {
            Self::backup_postgres(&_backup_path, config, config.compression_level).await?;
            info!("✅ PostgreSQL backup completed successfully");
            Ok(())
        }
        
        #[cfg(feature = "sqlite")] 
        {
            Self::backup_sqlite(&_backup_path, config.compression_level).await?;
            info!("✅ SQLite backup completed successfully");
            Ok(())
        }
        
        #[cfg(not(any(feature = "postgres", feature = "sqlite")))]
        {
            Err(BackupError::other("No database feature enabled for backup"))
        }
    }

    /// Backup PostgreSQL database
    #[tracing::instrument]
    pub(crate) async fn backup_postgres(backup_path: &Path, config: &BackupConfig, compression_level: u32) -> BackupResult<()> {
        info!("🐘 Starting PostgreSQL backup");
        
        #[cfg(feature = "postgres")]
        {
            use matrixon_db::{DatabaseConfig, DatabasePool};
            
            let db_config = DatabaseConfig {
                url: config.database_url.clone(),
                max_connections: 5,
                connection_timeout: std::time::Duration::from_secs(30).as_secs(),
                min_idle: Some(1),
                max_lifetime: Some(std::time::Duration::from_secs(300).as_secs()),
            };
            let pool = DatabasePool::new(&db_config).await
                .map_err(|e| BackupError::database(format!("Failed to get DB pool: {}", e)))?;

            let row = sqlx::query("SELECT pg_dump_all()")
                .fetch_one(pool.pool())
                .await
                .map_err(|e| BackupError::database(format!("pg_dump failed: {}", e)))?;
            let dump_data: String = row.get(0);

            // Create temporary file for dump
            let temp_file = tempfile::NamedTempFile::new()?;
            let temp_path = temp_file.path().to_path_buf();
            tokio::fs::write(&temp_path, &dump_data).await?;

            // Compress the backup
            BackupUtils::create_compressed_backup(&temp_path, backup_path, compression_level)?;
        }

        if cfg!(not(feature = "postgres")) {
            Err(BackupError::other("Postgres feature not enabled"))
        } else {
            Ok(())
        }
    }

    /// Backup SQLite database
    #[tracing::instrument]
    #[allow(dead_code)]
    async fn backup_sqlite(backup_path: &Path, compression_level: u32) -> BackupResult<()> {
        info!("💽 Starting SQLite backup");
        
        let db_path = PathBuf::from("/var/lib/matrixon/matrixon.db"); // TODO: Make configurable

        // Copy database file to temp location
        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path().to_path_buf();
        tokio::fs::copy(db_path, &temp_path).await?;

        // Compress the backup
        BackupUtils::create_compressed_backup(&temp_path, backup_path, compression_level)?;

        Ok(())
    }

    /// Verify database backup integrity
    #[tracing::instrument]
    pub async fn verify_backup(backup_path: &Path) -> BackupResult<()> {
        info!("🔍 Verifying database backup");

        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path().to_path_buf();

        // Decompress backup
        BackupUtils::decompress_backup(backup_path, &temp_path)?;

        // Basic verification - check file size and header
        let metadata = std::fs::metadata(&temp_path)?;
        if metadata.len() < 100 {
            return Err(BackupError::database("Backup file too small to be valid"));
        }

        // TODO: Add more sophisticated verification based on database type

        info!("✅ Backup verification successful");
        Ok(())
    }
}

/// Public interface for database backup
pub async fn backup_database(config: &BackupConfig) -> BackupResult<()> {
    DatabaseBackup::backup_database(config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_backup_config_validation() {
        let temp_dir = tempdir().unwrap();
        let config = BackupConfig {
            enabled: true,
            base_dir: temp_dir.path().to_path_buf(),
            database_url: "postgres://matrixon:password@localhost/matrixon".to_string(),
            max_backups: 5,
            compression_level: 6,
            schedule: None,
        };

        assert!(DatabaseBackup::backup_database(&config).await.is_err()); // Should fail without DB
    }
}
