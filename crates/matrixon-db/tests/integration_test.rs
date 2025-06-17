//! Integration tests for Matrixon Database Layer
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Date: 2025-06-15
//! Version: 0.1.0
//!
//! This module contains integration tests for database operations.

use matrixon_db::{pool::DatabasePool, models::TestEvent, queries};
use serial_test::serial;

#[tokio::test]
async fn test_db_connection() {
    let pool = DatabasePool::new_test_pool().await.unwrap();
    assert!(pool.is_connected().await);
}

#[tokio::test]
async fn test_event_crud() {
    let pool = DatabasePool::new_test_pool().await.unwrap();
    let event = TestEvent::new_test_data();
    
    // Create
    let id = queries::insert_event(pool.pool(), &event).await.unwrap();
    
    // Read
    let retrieved = queries::get_event(pool.pool(), id).await.unwrap();
    assert_eq!(retrieved.content, event.content);
    
    // Update
    let updated_content = "Updated content".to_string();
    queries::update_event_content(pool.pool(), id, &updated_content).await.unwrap();
    
    // Verify update
    let updated = queries::get_event(pool.pool(), id).await.unwrap();
    assert_eq!(updated.content, updated_content);
    
    // Delete
    queries::delete_event(pool.pool(), id).await.unwrap();
    assert!(queries::get_event(pool.pool(), id).await.is_err());
}

#[tokio::test]
#[serial]
async fn test_concurrent_access() {
    let pool = DatabasePool::new_test_pool().await.unwrap();
    let event = TestEvent::new_test_data();
    
    let handles: Vec<_> = (0..10).map(|i| {
        let pool = pool.clone();
        let mut event = event.clone();
        event.content = format!("Content {}", i);
        tokio::spawn(async move {
            queries::insert_event(pool.pool(), &event).await
        })
    }).collect();
    
    let results = futures::future::join_all(handles).await;
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(result.unwrap().is_ok());
    }
}
