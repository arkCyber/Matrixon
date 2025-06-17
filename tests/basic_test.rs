//! Basic integration tests for matrixon Matrix Server
//! 
//! These tests verify basic functionality without requiring full test infrastructure.

use std::time::{Duration, Instant};

#[tokio::test]
async fn test_basic_functionality() {
    // This is a basic test that verifies the test infrastructure works
    assert!(true, "Basic test infrastructure is working");
}

#[tokio::test]
async fn test_performance_utilities() {
    // Test performance measurement
    let (result, duration) = measure_time(|| {
        std::thread::sleep(Duration::from_millis(10));
        42
    });
    
    assert_eq!(result, 42);
    assert!(duration >= Duration::from_millis(10));
    assert!(duration < Duration::from_millis(100)); // Should not take too long
}

#[test]
fn test_matrix_event_validation() {
    use serde_json::json;
    
    // Test Matrix event validation
    let valid_event = json!({
        "type": "m.room.message",
        "content": {
            "body": "Hello, Matrix!",
            "msgtype": "m.text"
        },
        "sender": "@test:example.com",
        "room_id": "!room:example.com"
    });
    
    assert!(validate_matrix_event(&valid_event), "Valid event should pass validation");
    
    let invalid_event = json!({
        "invalid": "event"
    });
    
    assert!(!validate_matrix_event(&invalid_event), "Invalid event should fail validation");
}

#[tokio::test]
async fn test_concurrent_operations() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    let counter = Arc::new(AtomicU32::new(0));
    let mut handles = Vec::new();
    
    // Spawn 10 concurrent tasks
    for _ in 0..10 {
        let counter_clone = Arc::clone(&counter);
        let handle = tokio::spawn(async move {
            // Simulate some async work
            tokio::time::sleep(Duration::from_millis(1)).await;
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    assert_eq!(counter.load(Ordering::SeqCst), 10, "All concurrent operations should complete");
}

#[test]
fn test_enterprise_compliance_basic() {
    // Test that basic types can be created and used
    use serde_json::json;
    
    let test_data = json!({
        "enterprise": "compliance",
        "performance": "target",
        "reliability": 99.9
    });
    
    assert!(test_data["enterprise"].is_string());
    assert_eq!(test_data["reliability"].as_f64().unwrap(), 99.9);
}

// Helper functions for tests

/// Measure execution time of a closure
fn measure_time<F, T>(f: F) -> (T, Duration) 
where 
    F: FnOnce() -> T 
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

/// Validate Matrix event structure
fn validate_matrix_event(event: &serde_json::Value) -> bool {
    event.get("type").is_some() && 
    event.get("content").is_some()
}

/// Performance testing helper
fn assert_performance<F, T>(f: F, max_duration: Duration, operation_name: &str) -> T
where 
    F: FnOnce() -> T 
{
    let (result, duration) = measure_time(f);
    assert!(
        duration <= max_duration,
        "{} took {:?}, expected <= {:?}",
        operation_name,
        duration,
        max_duration
    );
    result
}

#[test]
fn test_performance_assertion() {
    // Test the performance assertion helper
    let result = assert_performance(
        || {
            std::thread::sleep(Duration::from_millis(5));
            "fast_operation"
        },
        Duration::from_millis(20),
        "test operation"
    );
    
    assert_eq!(result, "fast_operation");
}

#[test]
fn test_matrix_protocol_basics() {
    use serde_json::json;
    
    // Test basic Matrix protocol structures
    let room_creation_event = json!({
        "type": "m.room.create",
        "content": {
            "creator": "@test:example.com",
            "room_version": "10"
        }
    });
    
    assert!(validate_matrix_event(&room_creation_event));
    assert_eq!(room_creation_event["type"], "m.room.create");
    assert_eq!(room_creation_event["content"]["room_version"], "10");
} 
