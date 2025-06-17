//! Database migrations for Matrixon
//! 
//! This module provides database migration functionality for the Matrixon system.

use sqlx::{
    postgres::PgPool,
    migrate::MigrateDatabase,
    Postgres,
};
use matrixon_core::{Result, MatrixonError};
use tracing::{debug, info, instrument};

/// Migration version
pub const MIGRATION_VERSION: &str = "20240321000000";

/// Run database migrations
#[instrument(level = "debug")]
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    debug!("🔧 Starting database migrations");
    
    // Create migrations table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            version TEXT PRIMARY KEY,
            applied_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    // Run migrations
    let migrations = vec![
        // Users table
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            email TEXT UNIQUE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        )
        "#,
        
        // Rooms table
        r#"
        CREATE TABLE IF NOT EXISTS rooms (
            id UUID PRIMARY KEY,
            alias TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            topic TEXT,
            creator_id UUID NOT NULL REFERENCES users(id),
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        )
        "#,
        
        // Events table
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id UUID PRIMARY KEY,
            room_id UUID NOT NULL REFERENCES rooms(id),
            event_type TEXT NOT NULL,
            content JSONB NOT NULL,
            sender_id UUID NOT NULL REFERENCES users(id),
            origin_server_ts TIMESTAMP WITH TIME ZONE NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        )
        "#,
        
        // Devices table
        r#"
        CREATE TABLE IF NOT EXISTS devices (
            id TEXT PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id),
            display_name TEXT,
            last_seen_ip TEXT,
            last_seen_ts TIMESTAMP WITH TIME ZONE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        )
        "#,
    ];
    
    for migration in migrations {
        sqlx::query(migration)
            .execute(pool)
            .await
            .map_err(|e| MatrixonError::Database(e.to_string()))?;
    }
    
    // Record migration version
    sqlx::query(
        r#"
        INSERT INTO _migrations (version)
        VALUES ($1)
        ON CONFLICT (version) DO NOTHING
        "#,
    )
    .bind(MIGRATION_VERSION)
    .execute(pool)
    .await
    .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    info!("✅ Database migrations completed");
    Ok(())
}

/// Check if database exists
pub async fn database_exists(url: &str) -> Result<bool> {
    Postgres::database_exists(url).await.map_err(|e| MatrixonError::Database(e.to_string()))
}

/// Create database if it doesn't exist
pub async fn create_database(url: &str) -> Result<()> {
    if !database_exists(url).await? {
        Postgres::create_database(url).await.map_err(|e| MatrixonError::Database(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    async fn test_database_exists() {
        let url = "postgres://matrixon:matrixon@localhost/matrixon_test";
        let exists = database_exists(url).await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_create_database() {
        let url = "postgres://matrixon:matrixon@localhost/matrixon_test";
        create_database(url).await.unwrap();
        
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await
            .unwrap();
            
        run_migrations(&pool).await.unwrap();
    }
} 
