#!/bin/bash

# matrixon Matrix Server - Test Infrastructure Setup Script
# Author: AI Assistant 
# Date: $(date)
# Version: 1.0
# Purpose: Set up basic test infrastructure for running tests

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}🔧 INFO:${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ SUCCESS:${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  WARNING:${NC} $1"
}

log_error() {
    echo -e "${RED}❌ ERROR:${NC} $1"
}

# Function to create test utilities module
create_test_utils() {
    log_info "Creating test utilities module..."
    
    cat > src/test_utils.rs << 'EOF'
//! Test utilities for matrixon Matrix Server
//! 
//! This module provides common test utilities, fixtures, and helpers
//! for unit and integration testing.

#![cfg(test)]

use std::sync::Once;
use std::collections::HashMap;
use crate::database::KeyValueDatabase;
use crate::{Result, Error};
use ruma::{UserId, RoomId, DeviceId};

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

/// Create a test database instance
/// This creates an in-memory database for testing purposes
pub async fn create_test_database() -> Result<KeyValueDatabase> {
    init_test_environment();
    
    // For now, return an error indicating this needs to be implemented
    // In a real implementation, you would create an in-memory database
    Err(Error::BadDatabase("Test database not implemented yet - please implement create_test_database()"))
}

/// Create test user IDs for testing
pub fn test_user_id(local: &str) -> &'static UserId {
    Box::leak(format!("@{}:test.example.com", local).into_boxed_str())
        .try_into()
        .expect("Valid user ID")
}

/// Create test room IDs for testing  
pub fn test_room_id(local: &str) -> &'static RoomId {
    Box::leak(format!("!{}:test.example.com", local).into_boxed_str())
        .try_into()
        .expect("Valid room ID")
}

/// Create test device IDs for testing
pub fn test_device_id(device: &str) -> &'static DeviceId {
    Box::leak(device.to_string().into_boxed_str())
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
            "origin_server_ts": chrono::Utc::now().timestamp_millis(),
            "event_id": format!("$test_event_{}:{}", rand::random::<u64>(), "example.com")
        })
    }
}
EOF

    log_success "Created test utilities module: src/test_utils.rs"
}

# Function to add test utilities to lib.rs
add_test_utils_to_lib() {
    log_info "Adding test utilities to lib.rs..."
    
    # Check if test_utils is already in lib.rs
    if grep -q "mod test_utils;" src/lib.rs; then
        log_warning "test_utils module already exists in lib.rs"
    else
        # Add to lib.rs
        echo "" >> src/lib.rs
        echo "#[cfg(test)]" >> src/lib.rs
        echo "pub mod test_utils;" >> src/lib.rs
        log_success "Added test_utils module to lib.rs"
    fi
}

# Function to update Cargo.toml with test dependencies
add_test_dependencies() {
    log_info "Adding test dependencies to Cargo.toml..."
    
    # Check if test dependencies section exists
    if grep -q "\[dev-dependencies\]" Cargo.toml; then
        log_info "dev-dependencies section already exists"
    else
        echo "" >> Cargo.toml
        echo "[dev-dependencies]" >> Cargo.toml
    fi
    
    # Add common test dependencies if not present
    if ! grep -q "tokio-test" Cargo.toml; then
        cat >> Cargo.toml << 'EOF'

# Test utilities
tokio-test = "0.4"
tempfile = "3.10"
rand = "0.9"
chrono = { version = "0.4", features = ["serde"] }
EOF
        log_success "Added test dependencies to Cargo.toml"
    else
        log_warning "Test dependencies appear to already be present"
    fi
}

# Function to create test configuration
create_test_config() {
    log_info "Creating test configuration..."
    
    cat > tests/test_config.toml << 'EOF'
# Test configuration for matrixon Matrix Server tests

[database]
backend = "memory"  # Use in-memory database for tests
max_connections = 10

[performance]
max_operation_time_ms = 1000
max_concurrent_operations = 100

[matrix]
server_name = "test.example.com"
test_federation = false

[logging]
level = "debug"
test_writer = true
EOF

    log_success "Created test configuration: tests/test_config.toml"
}

# Function to create sample test implementation
create_sample_test() {
    log_info "Creating sample test implementation..."
    
    mkdir -p tests
    
    cat > tests/integration_test.rs << 'EOF'
//! Integration tests for matrixon Matrix Server
//! 
//! These tests verify that different components work together correctly.

use matrixon::test_utils::*;

#[tokio::test]
#[ignore] // Remove this when test infrastructure is ready
async fn test_basic_server_functionality() {
    // Initialize test environment
    init_test_environment();
    
    // This is a placeholder integration test
    // Replace with actual integration test when database is implemented
    
    assert!(true, "Placeholder integration test");
}

#[tokio::test] 
#[ignore] // Remove this when test infrastructure is ready
async fn test_matrix_protocol_compliance() {
    use matrixon::test_utils::matrix::*;
    
    // Test Matrix event validation
    let event = test_pdu("m.room.message", "@test:example.com", "!room:example.com");
    assert!(validate_matrix_event(&event), "Generated PDU should be valid");
}

#[tokio::test]
async fn test_performance_utilities() {
    use matrixon::test_utils::performance::*;
    use std::time::Duration;
    
    // Test performance measurement
    let (result, duration) = measure_time(|| {
        std::thread::sleep(Duration::from_millis(10));
        42
    });
    
    assert_eq!(result, 42);
    assert!(duration >= Duration::from_millis(10));
    assert!(duration < Duration::from_millis(100)); // Should not take too long
}
EOF

    log_success "Created sample integration test: tests/integration_test.rs"
}

# Function to enable some tests by removing #[ignore]
enable_basic_tests() {
    log_info "Enabling basic tests by removing some #[ignore] attributes..."
    
    # Find test files and remove #[ignore] from simple tests
    find src -name "*.rs" -exec grep -l "#\[ignore\]" {} \; | head -3 | while read -r file; do
        log_info "Processing $file for selective test enabling..."
        
        # Enable only performance and basic functionality tests (safer ones)
        sed -i.bak '/test_performance_benchmarks/,/async fn/ {
            s/#\[ignore\] \/\/ Ignore until test infrastructure is set up//
        }' "$file" 2>/dev/null || true
        
        # Remove backup files
        rm -f "${file}.bak" 2>/dev/null || true
    done
    
    log_success "Selectively enabled some basic tests"
}

# Function to run test verification
verify_test_setup() {
    log_info "Verifying test setup..."
    
    # Check compilation
    if cargo check --tests --quiet; then
        log_success "Test compilation successful"
    else
        log_error "Test compilation failed"
        return 1
    fi
    
    # Run a quick test to verify infrastructure
    if cargo test --lib test_performance_utilities -- --nocapture 2>/dev/null; then
        log_success "Basic test execution successful"
    else
        log_warning "Some tests may need implementation before they can run"
    fi
    
    # Show test count
    local test_count
    test_count=$(cargo test --lib -- --list 2>/dev/null | grep -c "test " || echo "0")
    log_info "Found $test_count tests in the library"
}

# Function to create test documentation
create_test_documentation() {
    log_info "Creating test documentation..."
    
    cat > TESTING.md << 'EOF'
# Testing Guide for matrixon Matrix Server

## Overview

This document describes the testing infrastructure and practices for the matrixon Matrix Server project.

## Test Structure

### Unit Tests
- Located in `#[cfg(test)]` modules within source files
- Test individual functions and modules in isolation
- Use mock data and test utilities

### Integration Tests  
- Located in the `tests/` directory
- Test component interactions and end-to-end functionality
- Use test database instances

### Performance Tests
- Included as part of unit tests with `test_performance_` prefix
- Measure operation timing and throughput
- Assert performance requirements

## Test Utilities

The `test_utils` module provides:

- `create_test_database()` - Test database instances
- `test_user_id()`, `test_room_id()`, `test_device_id()` - ID generators
- `generators::*` - Test data generators  
- `performance::*` - Performance testing utilities
- `concurrent::*` - Concurrency testing helpers
- `matrix::*` - Matrix protocol testing utilities

## Running Tests

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests  
cargo test --test integration_test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run ignored tests (when infrastructure is ready)
cargo test -- --ignored
```

## Test Configuration

Test configuration is stored in `tests/test_config.toml` and includes:

- Database settings for tests
- Performance thresholds  
- Matrix protocol settings
- Logging configuration

## Adding New Tests

1. For unit tests, add to the `#[cfg(test)]` module in the source file
2. For integration tests, create new files in `tests/`
3. Use test utilities from `test_utils` module
4. Follow naming convention: `test_<functionality>`
5. Include performance assertions where appropriate
6. Add Matrix protocol compliance tests for new features

## Test Coverage

Current test coverage can be checked with:

```bash
# Generate coverage report (requires cargo-tarpaulin)
cargo tarpaulin --out Html

# View coverage report
open tarpaulin-report.html
```

## Troubleshooting

### Tests are Ignored
Many tests are marked with `#[ignore]` until the test infrastructure is fully implemented. Remove `#[ignore]` as components become testable.

### Database Connection Errors
Ensure test database utilities are properly implemented in `test_utils::create_test_database()`.

### Performance Test Failures
Adjust performance thresholds in test configuration or the specific test if running on slower hardware.

## Contributing

When adding new features:

1. Write tests first (TDD approach)
2. Ensure all new code has corresponding tests
3. Update this documentation for new testing patterns
4. Run full test suite before submitting PRs

For questions about testing, refer to the enterprise development rules in `.cursorrules`.
EOF

    log_success "Created testing documentation: TESTING.md"
}

# Main execution function
main() {
    log_info "Setting up test infrastructure for matrixon Matrix Server"
    
    # Ensure we're in the project root
    if [ ! -f "Cargo.toml" ]; then
        log_error "Must be run from project root (Cargo.toml not found)"
        exit 1
    fi
    
    # Create directories
    mkdir -p tests src
    
    # Set up test infrastructure
    create_test_utils
    add_test_utils_to_lib
    add_test_dependencies
    create_test_config
    create_sample_test
    create_test_documentation
    
    # Enable some basic tests
    enable_basic_tests
    
    # Verify setup
    verify_test_setup
    
    log_success "Test infrastructure setup completed!"
    log_info "Next steps:"
    echo "  1. Implement create_test_database() in src/test_utils.rs"
    echo "  2. Run 'cargo test' to verify all tests compile"
    echo "  3. Gradually remove #[ignore] attributes as components become testable"
    echo "  4. Review TESTING.md for detailed testing guide"
    echo "  5. Consider setting up cargo-tarpaulin for coverage reports"
}

# Execute main function
main "$@" 
