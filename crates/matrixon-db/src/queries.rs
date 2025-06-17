//! Database queries for Matrixon
//! 
//! This module provides database query functions for the Matrixon system.

use sqlx::{postgres::PgPool, Row};
use matrixon_core::{Result, MatrixonError};
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::models::{User, Room, Event, TestEvent};

/// Create a new user
#[instrument(level = "debug")]
pub async fn create_user(pool: &PgPool, user: &User) -> Result<()> {
    debug!("🔧 Creating user: {}", user.username);
    
    sqlx::query(
        r#"
        INSERT INTO matrixon_userid_password (key, value, created_at, updated_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user.id.as_bytes())
    .bind(serde_json::to_vec(&user).map_err(|e| MatrixonError::Serialization(e.to_string()))?)
    .bind(user.created_at)
    .bind(user.updated_at)
    .execute(pool)
    .await
    .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    info!("✅ Created user: {}", user.username);
    Ok(())
}

/// Get a user by username
#[instrument(level = "debug")]
pub async fn get_user_by_username(pool: &PgPool, username: &str) -> Result<Option<User>> {
    debug!("🔧 Getting user by username: {}", username);
    
    let user = sqlx::query(
        r#"
        SELECT value
        FROM matrixon_userid_password
        WHERE value::jsonb->>'username' = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(|e| MatrixonError::Database(e.to_string()))?
    .map(|row: sqlx::postgres::PgRow| {
        let value: Vec<u8> = row.get("value");
        serde_json::from_slice(&value).map_err(|e| MatrixonError::Deserialization(e.to_string()))
    })
    .transpose()?;
    
    Ok(user)
}

/// Create a new room
#[instrument(level = "debug")]
pub async fn create_room(pool: &PgPool, room: &Room) -> Result<()> {
    debug!("🔧 Creating room: {}", room.alias);
    
    sqlx::query(
        r#"
        INSERT INTO matrixon_pduid_pdu (key, value, created_at, updated_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(room.id.as_bytes())
    .bind(serde_json::to_vec(&room).map_err(|e| MatrixonError::Serialization(e.to_string()))?)
    .bind(room.created_at)
    .bind(room.updated_at)
    .execute(pool)
    .await
    .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    info!("✅ Created room: {}", room.alias);
    Ok(())
}

/// Get a room by alias
#[instrument(level = "debug")]
pub async fn get_room_by_alias(pool: &PgPool, alias: &str) -> Result<Option<Room>> {
    debug!("🔧 Getting room by alias: {}", alias);
    
    let room = sqlx::query(
        r#"
        SELECT value
        FROM matrixon_pduid_pdu
        WHERE value::jsonb->>'alias' = $1
        "#,
    )
    .bind(alias)
    .fetch_optional(pool)
    .await
    .map_err(|e| MatrixonError::Database(e.to_string()))?
    .map(|row: sqlx::postgres::PgRow| {
        let value: Vec<u8> = row.get("value");
        serde_json::from_slice(&value).map_err(|e| MatrixonError::Deserialization(e.to_string()))
    })
    .transpose()?;
    
    Ok(room)
}

/// Create a new event
#[instrument(level = "debug")]
pub async fn create_event(pool: &PgPool, event: &Event) -> Result<()> {
    debug!("🔧 Creating event: {}", event.id);
    
    sqlx::query(
        r#"
        INSERT INTO matrixon_pduid_pdu (key, value, created_at, updated_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(event.id.as_bytes())
    .bind(serde_json::to_vec(&event).map_err(|e| MatrixonError::Serialization(e.to_string()))?)
    .bind(event.created_at)
    .bind(event.created_at)
    .execute(pool)
    .await
    .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    // Create event ID to PDU ID mapping
    sqlx::query(
        r#"
        INSERT INTO matrixon_eventid_pduid (key, value, created_at, updated_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(event.id.as_bytes())
    .bind(event.id.as_bytes())
    .bind(event.created_at)
    .bind(event.created_at)
    .execute(pool)
    .await
    .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    info!("✅ Created event: {}", event.id);
    Ok(())
}

/// Get events for a room
#[instrument(level = "debug")]
pub async fn get_room_events(
    pool: &PgPool,
    room_id: &uuid::Uuid,
    limit: i64,
    before: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Vec<Event>> {
    debug!("🔧 Getting events for room: {}", room_id);
    
    let events = if let Some(before) = before {
        sqlx::query(
            r#"
            SELECT value
            FROM matrixon_pduid_pdu
            WHERE value::jsonb->>'room_id' = $1::text
            AND created_at < $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(room_id.to_string())
        .bind(before)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(
            r#"
            SELECT value
            FROM matrixon_pduid_pdu
            WHERE value::jsonb->>'room_id' = $1::text
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(room_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| MatrixonError::Database(e.to_string()))?
    .into_iter()
    .map(|row: sqlx::postgres::PgRow| {
        let value: Vec<u8> = row.get("value");
        serde_json::from_slice(&value).map_err(|e| MatrixonError::Deserialization(e.to_string()))
    })
    .collect::<Result<Vec<Event>>>()?;
    
    Ok(events)
}

/// Insert a test event
#[instrument(level = "debug")]
pub async fn insert_event(pool: &PgPool, event: &TestEvent) -> Result<Uuid> {
    debug!("🔧 Inserting test event: {}", event.id);
    
    sqlx::query(
        r#"
        INSERT INTO matrixon_test_events (id, room_id, content, created_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(event.id)
    .bind(event.room_id)
    .bind(&event.content)
    .bind(event.created_at)
    .fetch_one(pool)
    .await
    .map(|row: sqlx::postgres::PgRow| row.get::<Uuid, _>("id"))
    .map_err(|e| MatrixonError::Database(e.to_string()))
}

/// Get a test event by ID
#[instrument(level = "debug")]
pub async fn get_event(pool: &PgPool, id: Uuid) -> Result<TestEvent> {
    debug!("🔧 Getting test event: {}", id);
    
    let event = sqlx::query(
        r#"
        SELECT id, room_id, content, created_at
        FROM matrixon_test_events
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map(|row: sqlx::postgres::PgRow| TestEvent {
        id: row.get("id"),
        room_id: row.get("room_id"),
        content: row.get("content"),
        created_at: row.get("created_at"),
    })
    .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    Ok(event)
}

/// Update test event content
#[instrument(level = "debug")]
pub async fn update_event_content(pool: &PgPool, id: Uuid, content: &str) -> Result<()> {
    debug!("🔧 Updating test event content: {}", id);
    
    sqlx::query(
        r#"
        UPDATE matrixon_test_events
        SET content = $1
        WHERE id = $2
        "#,
    )
    .bind(content)
    .bind(id)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    info!("✅ Updated test event content: {}", id);
    Ok(())
}

/// Delete a test event
#[instrument(level = "debug")]
pub async fn delete_event(pool: &PgPool, id: Uuid) -> Result<()> {
    debug!("🔧 Deleting test event: {}", id);
    
    sqlx::query(
        r#"
        DELETE FROM matrixon_test_events
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| MatrixonError::Database(e.to_string()))?;
    
    info!("✅ Deleted test event: {}", id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;
    use chrono::Utc;

    #[tokio::test]
    async fn test_user_operations() {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect("postgres://matrixon:matrixon@localhost/matrixon_test")
            .await
            .unwrap();
            
        let user = User {
            id: Uuid::new_v4(),
            username: "test_user".to_string(),
            password_hash: "hash".to_string(),
            email: Some("test@example.com".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        create_user(&pool, &user).await.unwrap();
        let retrieved = get_user_by_username(&pool, &user.username).await.unwrap().unwrap();
        
        assert_eq!(user.username, retrieved.username);
        assert_eq!(user.email, retrieved.email);
    }
}
