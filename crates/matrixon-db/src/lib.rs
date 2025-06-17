//! Matrixon Database Library
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.11.0-alpha
//! Date: 2024-03-21
//! 
//! This library provides database functionality for Matrixon, implementing
//! efficient storage and retrieval of Matrix data.

use std::time::Instant;
use tracing::{debug, info, instrument};
use matrixon_core::{Result, MatrixonError};
use sqlx::postgres::PgPool;

pub mod models;
pub mod migrations;
pub mod queries;
pub mod pool;

// Re-exports
pub use pool::DatabasePool;
pub use models::{TestEvent, Event, User, Room, Device};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    
    /// Maximum number of connections
    pub max_connections: u32,
    
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    
    /// Minimum number of idle connections
    pub min_idle: Option<u32>,
    
    /// Maximum lifetime of connections in seconds
    pub max_lifetime: Option<u64>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://matrixon:matrixon@localhost/matrixon".to_string(),
            max_connections: 100,
            connection_timeout: 30,
            min_idle: Some(10),
            max_lifetime: Some(1800),
        }
    }
}

/// Database implementation
#[derive(Debug)]
pub struct Database {
    config: DatabaseConfig,
    pool: Option<PgPool>,
}

impl Database {
    /// Create a new database instance
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            config,
            pool: None,
        }
    }
    
    /// Get the database configuration
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }
    
    /// Get the database connection pool
    pub fn pool(&self) -> Option<&PgPool> {
        self.pool.as_ref()
    }
    
    /// Initialize the database
    #[instrument(level = "debug")]
    pub async fn initialize(&mut self) -> Result<()> {
        debug!("🔧 Initializing database");
        let start = Instant::now();
        
        // Create connection pool
        self.pool = Some(pool::create_pool(&self.config).await?);
        
        // Run migrations
        self.migrate().await?;
        
        info!("✅ Database initialized in {:?}", start.elapsed());
        Ok(())
    }
    
    /// Run database migrations
    #[instrument(level = "debug")]
    pub async fn migrate(&self) -> Result<()> {
        debug!("🔧 Running database migrations");
        let start = Instant::now();
        
        if let Some(pool) = &self.pool {
            migrations::run_migrations(pool).await?;
        } else {
            return Err(MatrixonError::Database("Database not initialized".to_string()));
        }
        
        info!("✅ Database migrations completed in {:?}", start.elapsed());
        Ok(())
    }
    
    /// Check database health
    #[instrument(level = "debug")]
    pub async fn health_check(&self) -> Result<bool> {
        debug!("🔧 Checking database health");
        
        if let Some(pool) = &self.pool {
            pool::check_pool_health(pool).await
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.url, "postgres://matrixon:matrixon@localhost/matrixon");
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.connection_timeout, 30);
        assert_eq!(config.min_idle, Some(10));
        assert_eq!(config.max_lifetime, Some(1800));
    }
    
    #[tokio::test]
    async fn test_database_initialization() {
        let mut db = Database::new(DatabaseConfig::default());
        assert!(db.initialize().await.is_ok());
        assert!(db.health_check().await.unwrap());
    }
}
