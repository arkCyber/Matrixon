//! Database connection pool management for Matrixon
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Date: 2025-06-15
//! Version: 0.1.0
//!
//! This module provides database connection pool functionality for the Matrixon system.

use std::time::{Duration, Instant};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use matrixon_core::{Result, MatrixonError};
use tracing::{debug, info, instrument};
use metrics::{counter, histogram};

use crate::DatabaseConfig;

/// Database connection pool with metrics
#[derive(Debug, Clone)]
pub struct DatabasePool {
    pool: PgPool,
}

impl DatabasePool {
    /// Create a new database connection pool
    #[instrument(level = "debug")]
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        debug!("🔧 Creating database connection pool");
        let start = Instant::now();

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(config.connection_timeout))
            .min_connections(config.min_idle.unwrap_or(0))
            .max_lifetime(config.max_lifetime.map(Duration::from_secs))
            .connect(&config.url)
            .await
            .map_err(|e| MatrixonError::Database(e.to_string()))?;

        histogram!("db.pool.create.time", start.elapsed());
        info!("✅ Created database connection pool with {} max connections", config.max_connections);
        
        Ok(Self { pool })
    }

    /// Create a test pool (for benchmarks and tests)
    #[instrument(level = "debug")]
    pub async fn new_test_pool() -> Result<Self> {
        let config = DatabaseConfig::default();
        Self::new(&config).await
    }

    /// Get the inner SQLx pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Check if the pool is connected
    #[instrument(level = "debug")]
    pub async fn is_connected(&self) -> bool {
        check_pool_health(&self.pool).await.unwrap_or(false)
    }

    /// Get a connection from the pool
    #[instrument(level = "debug")]
    pub async fn get_conn(&self) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>> {
        let start = Instant::now();
        counter!("db.pool.connections.checked_out", 1);

        let conn = self.pool.acquire().await
            .map_err(|e| MatrixonError::Database(e.to_string()))?;

        histogram!("db.pool.acquire.time", start.elapsed());
        Ok(conn)
    }
}

/// Create a raw SQLx connection pool (without metrics)
#[instrument(level = "debug")]
pub async fn create_pool(config: &DatabaseConfig) -> Result<PgPool> {
    debug!("🔧 Creating database connection pool");
    
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(Duration::from_secs(config.connection_timeout))
        .min_connections(config.min_idle.unwrap_or(0))
        .max_lifetime(config.max_lifetime.map(Duration::from_secs))
        .connect(&config.url)
        .await
        .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    info!("✅ Created database connection pool with {} max connections", config.max_connections);
    Ok(pool)
}

/// Check if the database connection pool is healthy
#[instrument(level = "debug")]
pub async fn check_pool_health(pool: &PgPool) -> Result<bool> {
    debug!("🔧 Checking database connection pool health");
    
    let result = sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    let is_healthy = result.rows_affected() == 1;
    info!("✅ Database connection pool health check: {}", is_healthy);
    
    Ok(is_healthy)
}

/// Get the number of active connections in the pool
#[instrument(level = "debug")]
pub async fn get_active_connections(pool: &PgPool) -> Result<u32> {
    debug!("🔧 Getting number of active database connections");
    
    let count = sqlx::query("SELECT count(*) FROM pg_stat_activity WHERE datname = current_database()")
        .fetch_one(pool)
        .await
        .map_err(|e| MatrixonError::Database(e.to_string()))?
        .get::<i64, _>(0) as u32;
    
    info!("✅ Active database connections: {}", count);
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_pool_creation() {
        let config = DatabaseConfig {
            url: "postgres://matrixon:matrixon@localhost/matrixon_test".to_string(),
            max_connections: 5,
            connection_timeout: 30,
            min_idle: Some(1),
            max_lifetime: Some(3600),
        };
        
        let pool = create_pool(&config).await.unwrap();
        assert!(check_pool_health(&pool).await.unwrap());
        
        let active_connections = get_active_connections(&pool).await.unwrap();
        assert!(active_connections > 0);
    }
}
