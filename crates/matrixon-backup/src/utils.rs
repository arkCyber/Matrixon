//! Utility functions for Matrixon backup operations
//!
//! Provides helper functions for backup compression, validation, and file operations
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2025-06-15

use std::path::{Path, PathBuf};
use std::fs;
use flate2::{Compression, write::GzEncoder};
use flate2::read::GzDecoder;
use std::io::{Read, Write};
use tracing::info;
use super::error::BackupError;

/// Utility functions for backup operations
pub struct BackupUtils;

impl BackupUtils {
    /// Validate and create backup directory if needed
    pub fn validate_backup_dir(path: &Path) -> Result<(), BackupError> {
        if !path.exists() {
            info!("📂 Creating backup directory at {:?}", path);
            fs::create_dir_all(path)?;
        } else if !path.is_dir() {
            return Err(BackupError::config(format!(
                "Backup directory does not exist: {}",
                path.display()
            )));
        }

        // Check write permissions
        let test_file = path.join(".permission_test");
        fs::write(&test_file, "test")?;
        fs::remove_file(&test_file)?;

        Ok(())
    }

    /// Create a compressed backup file
    pub fn create_compressed_backup(
        src_path: &Path,
        dest_path: &Path,
        compression_level: u32
    ) -> Result<(), BackupError> {
        info!("🗜 Compressing backup from {:?} to {:?}", src_path, dest_path);
        
        let src_file = fs::File::open(src_path)?;
        let dest_file = fs::File::create(dest_path)?;
        
        let mut encoder = GzEncoder::new(dest_file, Compression::new(compression_level));
        let mut buffer = [0; 1024];
        
        let mut reader = std::io::BufReader::new(src_file);
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            encoder.write_all(&buffer[..bytes_read])?;
        }
        
        encoder.finish()?;
        Ok(())
    }

    /// Decompress a backup file
    pub fn decompress_backup(
        src_path: &Path,
        dest_path: &Path
    ) -> Result<(), BackupError> {
        info!("🗜 Decompressing backup from {:?} to {:?}", src_path, dest_path);
        
        let src_file = fs::File::open(src_path)?;
        let dest_file = fs::File::create(dest_path)?;
        
        let mut decoder = GzDecoder::new(src_file);
        let mut writer = std::io::BufWriter::new(dest_file);
        
        let mut buffer = [0; 1024];
        loop {
            let bytes_read = decoder.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            writer.write_all(&buffer[..bytes_read])?;
        }
        
        writer.flush()?;
        Ok(())
    }

    /// Clean up old backups according to retention policy
    pub fn cleanup_old_backups(
        backup_dir: &Path,
        max_backups: usize
    ) -> Result<(), BackupError> {
        let mut backup_files = Self::list_backup_files(backup_dir)?;
        
        if backup_files.len() <= max_backups {
            return Ok(());
        }

        // Sort by modification time (oldest first)
        backup_files.sort_by_key(|f| f.modified_time);

        // Delete oldest backups beyond max_backups limit
        for file in backup_files.iter().take(backup_files.len() - max_backups) {
            info!("🗑 Removing old backup: {:?}", file.path);
            fs::remove_file(&file.path)?;
        }

        Ok(())
    }

    /// List all backup files in directory
    fn list_backup_files(backup_dir: &Path) -> Result<Vec<BackupFileInfo>, BackupError> {
        let mut backups = Vec::new();
        
        for entry in fs::read_dir(backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().is_some_and(|ext| ext == "gz") {
                let metadata = fs::metadata(&path)?;
                backups.push(BackupFileInfo {
                    path,
                    modified_time: metadata.modified()?,
                    size: metadata.len(),
                });
            }
        }
        
        Ok(backups)
    }
}

/// Information about a backup file
#[derive(Clone)]
struct BackupFileInfo {
    path: PathBuf,
    modified_time: std::time::SystemTime,
    #[allow(dead_code)]
    size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_validate_backup_dir() {
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path().join("backups");
        
        assert!(BackupUtils::validate_backup_dir(&test_dir).is_ok());
        assert!(test_dir.exists());
    }

    #[test]
    fn test_compression_roundtrip() {
        let temp_dir = tempdir().unwrap();
        let src_path = temp_dir.path().join("test.txt");
        let compressed_path = temp_dir.path().join("test.gz");
        let decompressed_path = temp_dir.path().join("test_decompressed.txt");
        
        fs::write(&src_path, "test content").unwrap();
        
        assert!(BackupUtils::create_compressed_backup(&src_path, &compressed_path, 6).is_ok());
        assert!(BackupUtils::decompress_backup(&compressed_path, &decompressed_path).is_ok());
        
        let content = fs::read_to_string(&decompressed_path).unwrap();
        assert_eq!(content, "test content");
    }
}
