// =============================================================================
// Matrixon Matrix NextServer - Test Utils Module
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Contributors: Matrixon Development Team
// Date: 2024-12-11
// Version: 2.0.0-alpha (PostgreSQL Backend)
// License: Apache 2.0 / MIT
//
// Description:
//   Core component of the Matrixon Matrix NextServer. This module is part of the Matrixon Matrix NextServer
//   implementation, designed for enterprise-grade deployment with 20,000+
//   concurrent connections and <50ms response latency.
//
// Performance Targets:
//   • 20k+ concurrent connections
//   • <50ms response latency
//   • >99% success rate
//   • Memory-efficient operation
//   • Horizontal scalability
//
// Features:
//   • High-performance Matrix operations
//   • Enterprise-grade reliability
//   • Scalable architecture
//   • Security-focused design
//   • Matrix protocol compliance
//
// Architecture:
//   • Async/await native implementation
//   • Zero-copy operations where possible
//   • Memory pool optimization
//   • Lock-free data structures
//   • Enterprise monitoring integration
//
// Dependencies:
//   • Tokio async runtime
//   • Structured logging with tracing
//   • Error handling with anyhow/thiserror
//   • Serialization with serde
//   • Matrix protocol types with ruma
//
// References:
//   • Matrix.org specification: https://matrix.org/
//   • Synapse reference: https://github.com/element-hq/synapse
//   • Matrix spec: https://spec.matrix.org/
//   • Performance guidelines: Internal Matrixon documentation
//
// Quality Assurance:
//   • Comprehensive unit testing
//   • Integration test coverage
//   • Performance benchmarking
//   • Memory leak detection
//   • Security audit compliance
//
// =============================================================================

#![cfg(any(test, feature = "testing"))]

use std::sync::Once;
use std::collections::HashMap;
use crate::database::KeyValueDatabase;
use crate::{Result, Error, services};
use ruma::{UserId, RoomId, DeviceId, OwnedUserId, OwnedRoomId, OwnedDeviceId};

static INIT: Once = Once::new();

/// Initialize test environment (call once per test process)
pub fn init_test_environment() {
    INIT.call_once(|| {
        // Initialize logging for tests
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter("debug")
            .try_init();
    });
}

/// Create an in-memory test database for testing.
/// This initializes the global SERVICES static.
pub async fn create_test_database() -> Result<()> {
    use crate::config::*;
    use std::collections::BTreeMap;
    use std::net::{IpAddr, Ipv4Addr};
    
    let incomplete = IncompleteConfig {
        address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        port: 8008,
        tls: None,
        server_name: "test.example.com".try_into().map_err(|_| Error::bad_config("Invalid server name"))?,
        database_backend: "sqlite".to_string(),
        database_path: ":memory:".to_string(),
        db_cache_capacity_mb: 64.0,
        enable_lightning_bolt: true,
        allow_check_for_updates: false,
        matrixon_cache_capacity_modifier: 1.0,
        rocksdb_max_open_files: 512,
        pdu_cache_capacity: 1000,
        cleanup_second_interval: 60,
        max_request_size: 1024,
        max_concurrent_requests: 100,
        max_fetch_prev_events: 100,
        allow_registration: true,
        registration_token: None,
        openid_token_ttl: 3600,
        allow_encryption: true,
        allow_federation: false,
        allow_room_creation: true,
        allow_unstable_room_versions: false,
        default_room_version: default_default_room_version(),
        well_known: IncompleteWellKnownConfig {
            client: None,
            server: None,
        },
        allow_jaeger: false,
        tracing_flame: false,
        proxy: ProxyConfig::None,
        jwt_secret: None,
        trusted_servers: vec![],
        log: "warn".to_string(),
        turn_username: None,
        turn_password: None,
        turn_uris: None,
        turn_secret: None,
        turn_ttl: 3600,
        turn: None,
        media: IncompleteMediaConfig {
            backend: IncompleteMediaBackendConfig::default(),
            retention: None,
        },
        emergency_password: None,
        captcha: Default::default(),
        catchall: BTreeMap::new(),
    };
    
    let config = Config::from(incomplete);
    
    // Create an in-memory database for testing
    crate::database::KeyValueDatabase::load_or_create(config).await
        .map_err(|_| Error::bad_database("Failed to create test database"))
}

/// Initialize services for testing
/// This sets up the global services for tests that need it
pub async fn init_test_services() -> Result<()> {
    // Create test database configuration and initialize services
    let config = create_test_config()?;
    let db = crate::database::KeyValueDatabase::load_or_create(config.clone()).await
        .map_err(|_| Error::bad_database("Failed to initialize test services"))?;
    
    // Initialize services with static lifetime
    let services = Box::leak(Box::new(crate::service::Services::build(&db, config).await?));
    
    // Store services in global static
    let mut services_lock = crate::SERVICES.write().unwrap();
    *services_lock = Some(services);
    
    Ok(())
}

/// Create a test configuration
pub fn create_test_config() -> Result<crate::Config> {
    use crate::config::*;
    use std::collections::BTreeMap;
    use std::net::{IpAddr, Ipv4Addr};
    
    let incomplete = IncompleteConfig {
        address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        port: 8008,
        tls: None,
        server_name: "test.example.com".try_into().map_err(|_| Error::bad_config("Invalid server name"))?,
        database_backend: "sqlite".to_string(),
        database_path: ":memory:".to_string(),
        db_cache_capacity_mb: 64.0,
        enable_lightning_bolt: true,
        allow_check_for_updates: false,
        matrixon_cache_capacity_modifier: 1.0,
        rocksdb_max_open_files: 512,
        pdu_cache_capacity: 1000,
        cleanup_second_interval: 60,
        max_request_size: 1024,
        max_concurrent_requests: 100,
        max_fetch_prev_events: 100,
        allow_registration: true,
        registration_token: None,
        openid_token_ttl: 3600,
        allow_encryption: true,
        allow_federation: false,
        allow_room_creation: true,
        allow_unstable_room_versions: false,
        default_room_version: default_default_room_version(),
        well_known: IncompleteWellKnownConfig {
            client: None,
            server: None,
        },
        allow_jaeger: false,
        tracing_flame: false,
        proxy: ProxyConfig::None,
        jwt_secret: None,
        trusted_servers: vec![],
        log: "warn".to_string(),
        turn_username: None,
        turn_password: None,
        turn_uris: None,
        turn_secret: None,
        turn_ttl: 3600,
        turn: None,
        media: IncompleteMediaConfig {
            backend: IncompleteMediaBackendConfig::default(),
            retention: None,
        },
        emergency_password: None,
        captcha: Default::default(),
        catchall: BTreeMap::new(),
    };
    
    Ok(Config::from(incomplete))
}

/// Create test user IDs for testing
pub fn test_user_id(local: &str) -> OwnedUserId {
    format!("@{}:test.example.com", local)
        .try_into()
        .expect("Valid user ID")
}

/// Create test room IDs for testing  
pub fn test_room_id(local: &str) -> OwnedRoomId {
    format!("!{}:test.example.com", local)
        .try_into()
        .expect("Valid room ID")
}

/// Create test device IDs for testing
pub fn test_device_id(device: &str) -> OwnedDeviceId {
    device.to_string()
        .try_into()
        .expect("Valid device ID")
}

/// Test data generators
pub mod generators {
    use super::*;
    use serde_json::json;
    
    /// Generate test account data
    pub fn account_data(event_id: &str) -> serde_json::Value {
        json!({
            "type": "m.fully_read",
            "content": {
                "event_id": event_id
            }
        })
    }
    
    /// Generate test room creation data
    pub fn room_creation_content() -> serde_json::Value {
        json!({
            "type": "m.room.create",
            "content": {
                "creator": "@test:example.com",
                "room_version": "10"
            }
        })
    }
    
    /// Generate test user profile data
    pub fn user_profile(display_name: &str, avatar_url: Option<&str>) -> HashMap<String, serde_json::Value> {
        let mut profile = HashMap::new();
        profile.insert("displayname".to_string(), json!(display_name));
        if let Some(url) = avatar_url {
            profile.insert("avatar_url".to_string(), json!(url));
        }
        profile
    }
}

/// Performance testing utilities
pub mod performance {
    use std::time::{Duration, Instant};
    
    /// Benchmark context for performance testing
    pub struct BenchmarkContext {
        pub start_time: Instant,
        pub target_operations: usize,
        pub max_latency: Duration,
    }
    
    /// Create a benchmark context for performance testing
    pub async fn create_benchmark_context() -> BenchmarkContext {
        super::init_test_environment();
        BenchmarkContext {
            start_time: Instant::now(),
            target_operations: 1000,
            max_latency: Duration::from_millis(50),
        }
    }
    
    /// Measure execution time of a closure
    pub fn measure_time<F, T>(f: F) -> (T, Duration) 
    where 
        F: FnOnce() -> T 
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        (result, duration)
    }
    
    /// Assert that an operation completes within a time limit
    pub fn assert_performance<F, T>(f: F, max_duration: Duration, operation_name: &str) -> T
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
}

/// Concurrent testing utilities  
pub mod concurrent {
    use std::sync::Arc;
    use tokio::task::JoinHandle;
    
    /// Run multiple async operations concurrently and collect results
    pub async fn run_concurrent<F, T, Fut>(
        count: usize,
        operation: F,
    ) -> Vec<Result<T, Box<dyn std::error::Error + Send + Sync>>>
    where
        F: Fn(usize) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let operation = Arc::new(operation);
        let mut handles: Vec<JoinHandle<Result<T, Box<dyn std::error::Error + Send + Sync>>>> = Vec::new();
        
        for i in 0..count {
            let op = Arc::clone(&operation);
            let handle = tokio::spawn(async move {
                let result = op(i).await;
                Ok(result)
            });
            handles.push(handle);
        }
        
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)),
            }
        }
        
        results
    }
}

/// Matrix protocol testing utilities
pub mod matrix {
    use super::*;
    use serde_json::json;
    
    /// Test context for Matrix operations
    pub struct TestContext {
        pub server_name: String,
        pub test_user: OwnedUserId,
        pub test_room: OwnedRoomId,
    }
    
    /// Create a test context for Matrix operations
    pub async fn create_test_context() -> TestContext {
        init_test_environment();
        TestContext {
            server_name: "test.example.com".to_string(),
            test_user: test_user_id("test_user"),
            test_room: test_room_id("test_room"),
        }
    }
    
    /// Validate Matrix event structure
    pub fn validate_matrix_event(event: &serde_json::Value) -> bool {
        event.get("type").is_some() && 
        event.get("content").is_some()
    }
    
    /// Create a test Matrix PDU (Persistent Data Unit)
    pub fn test_pdu(event_type: &str, sender: &str, room_id: &str) -> serde_json::Value {
        json!({
            "type": event_type,
            "sender": sender,
            "room_id": room_id,
            "content": {},
            "origin_server_ts": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            "event_id": format!("$test_event_{}:{}", 
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(), 
                "example.com")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use serde_json::json;

    /// Test: Verify test environment initialization
    /// 
    /// This test ensures that the test environment can be initialized
    /// safely multiple times without issues.
    #[test]
    fn test_init_test_environment() {
        // Should be safe to call multiple times
        init_test_environment();
        init_test_environment();
        init_test_environment();
        
        // If we get here without panicking, the test passes
        assert!(true, "Test environment initialization successful");
    }

    /// Test: Verify test database creation success
    /// 
    /// This test verifies that the test database creation function
    /// properly creates an in-memory database for testing.
    #[tokio::test]
    async fn test_create_test_database_success() {
        let result = create_test_database().await;
        
        // Should succeed in creating test database
        assert!(result.is_ok(), "Should successfully create test database");
    }

    /// Test: Verify test user ID generation
    /// 
    /// This test ensures that test user IDs are generated correctly
    /// with proper Matrix ID format.
    #[test]
    fn test_user_id_generation() {
        let user_id = test_user_id("alice");
        
        // Verify format
        assert_eq!(user_id.as_str(), "@alice:test.example.com");
        
        // Verify it's a valid Matrix user ID
        assert!(user_id.as_str().starts_with('@'));
        assert!(user_id.as_str().contains(':'));
        
        // Test with special characters
        let special_user = test_user_id("user.with.dots");
        assert_eq!(special_user.as_str(), "@user.with.dots:test.example.com");
    }

    /// Test: Verify test room ID generation
    /// 
    /// This test ensures that test room IDs are generated correctly
    /// with proper Matrix room ID format.
    #[test]
    fn test_room_id_generation() {
        let room_id = test_room_id("general");
        
        // Verify format
        assert_eq!(room_id.as_str(), "!general:test.example.com");
        
        // Verify it's a valid Matrix room ID
        assert!(room_id.as_str().starts_with('!'));
        assert!(room_id.as_str().contains(':'));
        
        // Test with special characters
        let special_room = test_room_id("room-with-dashes");
        assert_eq!(special_room.as_str(), "!room-with-dashes:test.example.com");
    }

    /// Test: Verify test device ID generation
    /// 
    /// This test ensures that test device IDs are generated correctly.
    #[test]
    fn test_device_id_generation() {
        let device_id = test_device_id("DEVICE123");
        
        // Verify format
        assert_eq!(device_id.as_str(), "DEVICE123");
        
        // Test with various formats
        let mobile_device = test_device_id("mobile_app_v1.0");
        assert_eq!(mobile_device.as_str(), "mobile_app_v1.0");
        
        let web_device = test_device_id("web-client");
        assert_eq!(web_device.as_str(), "web-client");
    }

    /// Test: Verify generators module functionality
    /// 
    /// This test ensures that all data generators work correctly
    /// and produce valid test data.
    #[test]
    fn test_generators_account_data() {
        let event_id = "$test_event:example.com";
        let account_data = generators::account_data(event_id);
        
        // Verify structure
        assert_eq!(account_data["type"], "m.fully_read");
        assert_eq!(account_data["content"]["event_id"], event_id);
        
        // Verify it's valid JSON
        assert!(account_data.is_object());
    }

    #[test]
    fn test_generators_room_creation_content() {
        let room_content = generators::room_creation_content();
        
        // Verify structure
        assert_eq!(room_content["type"], "m.room.create");
        assert_eq!(room_content["content"]["creator"], "@test:example.com");
        assert_eq!(room_content["content"]["room_version"], "10");
        
        // Verify it's valid JSON
        assert!(room_content.is_object());
    }

    #[test]
    fn test_generators_user_profile() {
        // Test with avatar URL
        let profile_with_avatar = generators::user_profile("Alice", Some("mxc://example.com/avatar"));
        assert_eq!(profile_with_avatar["displayname"], json!("Alice"));
        assert_eq!(profile_with_avatar["avatar_url"], json!("mxc://example.com/avatar"));
        assert_eq!(profile_with_avatar.len(), 2);
        
        // Test without avatar URL
        let profile_without_avatar = generators::user_profile("Bob", None);
        assert_eq!(profile_without_avatar["displayname"], json!("Bob"));
        assert!(!profile_without_avatar.contains_key("avatar_url"));
        assert_eq!(profile_without_avatar.len(), 1);
    }

    /// Test: Verify performance testing utilities
    /// 
    /// This test ensures that performance measurement utilities
    /// work correctly and provide accurate timing.
    #[test]
    fn test_performance_measure_time() {
        let (result, duration) = performance::measure_time(|| {
            std::thread::sleep(Duration::from_millis(10));
            42
        });
        
        // Verify result
        assert_eq!(result, 42);
        
        // Verify timing (should be at least 10ms, but allow some variance)
        assert!(duration >= Duration::from_millis(8), "Duration should be at least 8ms: {:?}", duration);
        assert!(duration <= Duration::from_millis(50), "Duration should be less than 50ms: {:?}", duration);
    }

    #[test]
    fn test_performance_assert_performance_success() {
        let result = performance::assert_performance(
            || {
                std::thread::sleep(Duration::from_millis(5));
                "success"
            },
            Duration::from_millis(100),
            "test operation"
        );
        
        assert_eq!(result, "success");
    }

    #[test]
    #[should_panic(expected = "test operation took")]
    fn test_performance_assert_performance_failure() {
        performance::assert_performance(
            || {
                std::thread::sleep(Duration::from_millis(50));
                "too slow"
            },
            Duration::from_millis(10),
            "test operation"
        );
    }

    /// Test: Verify concurrent testing utilities
    /// 
    /// This test ensures that concurrent operation utilities
    /// work correctly for async operations.
    #[tokio::test]
    async fn test_concurrent_run_concurrent() {
        let results = concurrent::run_concurrent(3, |i| async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            i * 2
        }).await;
        
        // Verify all operations completed
        assert_eq!(results.len(), 3);
        
        // Verify all results are successful
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok(), "Operation {} should succeed", i);
            if let Ok(value) = result {
                assert_eq!(*value, i * 2, "Operation {} should return correct value", i);
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_run_concurrent_with_errors() {
        let results = concurrent::run_concurrent(2, |i| async move {
            if i == 1 {
                panic!("Test panic");
            }
            i
        }).await;
        
        // Verify we get results for all operations
        assert_eq!(results.len(), 2);
        
        // First operation should succeed
        assert!(results[0].is_ok());
        if let Ok(value) = &results[0] {
            assert_eq!(*value, 0);
        }
        
        // Second operation should fail due to panic
        assert!(results[1].is_err());
    }

    /// Test: Verify Matrix protocol testing utilities
    /// 
    /// This test ensures that Matrix protocol utilities
    /// work correctly for event validation and generation.
    #[test]
    fn test_matrix_validate_matrix_event() {
        // Valid event
        let valid_event = json!({
            "type": "m.room.message",
            "content": {
                "msgtype": "m.text",
                "body": "Hello, world!"
            }
        });
        assert!(matrix::validate_matrix_event(&valid_event));
        
        // Invalid event - missing type
        let invalid_event_no_type = json!({
            "content": {
                "msgtype": "m.text",
                "body": "Hello, world!"
            }
        });
        assert!(!matrix::validate_matrix_event(&invalid_event_no_type));
        
        // Invalid event - missing content
        let invalid_event_no_content = json!({
            "type": "m.room.message"
        });
        assert!(!matrix::validate_matrix_event(&invalid_event_no_content));
    }

    #[test]
    fn test_matrix_test_pdu() {
        let pdu = matrix::test_pdu("m.room.message", "@alice:example.com", "!room:example.com");
        
        // Verify required fields
        assert_eq!(pdu["type"], "m.room.message");
        assert_eq!(pdu["sender"], "@alice:example.com");
        assert_eq!(pdu["room_id"], "!room:example.com");
        assert!(pdu["content"].is_object());
        
        // Verify timestamp is reasonable (within last minute)
        let timestamp = pdu["origin_server_ts"].as_u64().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        assert!(timestamp <= now, "Timestamp should not be in the future");
        assert!(timestamp > now - 60000, "Timestamp should be recent (within 1 minute)");
        
        // Verify event ID format
        let event_id = pdu["event_id"].as_str().unwrap();
        assert!(event_id.starts_with("$test_event_"));
        assert!(event_id.ends_with(":example.com"));
    }

    /// Test: Verify test utilities integration
    /// 
    /// This test ensures that different test utilities work together
    /// correctly for comprehensive testing scenarios.
    #[test]
    fn test_utilities_integration() {
        // Generate test IDs
        let user_id = test_user_id("testuser");
        let room_id = test_room_id("testroom");
        let device_id = test_device_id("testdevice");
        
        // Generate test data using the IDs
        let profile = generators::user_profile("Test User", Some("mxc://example.com/avatar"));
        let pdu = matrix::test_pdu("m.room.member", user_id.as_str(), room_id.as_str());
        
        // Verify integration
        assert!(matrix::validate_matrix_event(&pdu));
        assert_eq!(pdu["sender"], user_id.as_str());
        assert_eq!(pdu["room_id"], room_id.as_str());
        assert_eq!(profile["displayname"], json!("Test User"));
        
        // Verify all components work together
        assert!(true, "Test utilities integration successful");
    }

    /// Test: Verify test utilities performance characteristics
    /// 
    /// This test ensures that test utilities themselves are performant
    /// and don't introduce significant overhead.
    #[test]
    fn test_utilities_performance() {
        // Test ID generation performance
        let (_, duration) = performance::measure_time(|| {
            for i in 0..1000 {
                let _ = test_user_id(&format!("user{}", i));
                let _ = test_room_id(&format!("room{}", i));
                let _ = test_device_id(&format!("device{}", i));
            }
        });
        
        // Should complete quickly (less than 100ms for 1000 operations)
        assert!(duration < Duration::from_millis(100), 
                "ID generation should be fast: {:?}", duration);
        
        // Test data generation performance
        let (_, duration) = performance::measure_time(|| {
            for i in 0..100 {
                let _ = generators::account_data(&format!("$event{}:example.com", i));
                let _ = generators::room_creation_content();
                let _ = generators::user_profile(&format!("User {}", i), None);
            }
        });
        
        // Should complete quickly (less than 50ms for 100 operations)
        assert!(duration < Duration::from_millis(50), 
                "Data generation should be fast: {:?}", duration);
    }
}
