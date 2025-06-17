# matrixon Matrix Server Testing Guide

This document provides comprehensive instructions for running database functionality tests and performance tests for the matrixon Matrix Server.

## 📋 Overview

Our testing suite includes:
- **Database Functionality Tests**: Validate CRUD operations, concurrent access, and data integrity
- **Performance Tests**: Stress test concurrent connections, throughput, and latency
- **Integration Tests**: End-to-end functionality validation

## 🚀 Quick Start

### Basic Testing (No PostgreSQL Required)
```bash
# Run basic functionality tests
./run_tests.sh basic

# Check compilation and basic features
cargo test --lib --bins
```

### Full Testing with PostgreSQL
```bash
# 1. Start PostgreSQL (install if needed)
brew install postgresql
brew services start postgresql

# 2. Setup test environment
./tests/test_setup.sh setup

# 3. Run all tests
./run_tests.sh all
```

## 🗄️ Database Tests

### Prerequisites
- PostgreSQL 12+ installed and running
- User `matrixon` with password `matrixon` (created automatically by setup script)
- Database `matrixon_test` (created automatically by setup script)

### Running Database Tests
```bash
# Setup test database
./tests/test_setup.sh setup

# Run database functionality tests
./run_tests.sh database

# Or run specific test
cargo test --features backend_postgresql test_database_connection -- --test-threads=1
```

### Test Coverage
- ✅ **Connection Testing**: PostgreSQL database connection validation
- ✅ **CRUD Operations**: Insert, Get, Remove, Increment operations
- ✅ **Table Operations**: Multiple table handling and isolation
- ✅ **Batch Operations**: Bulk insert and update performance
- ✅ **Concurrent Access**: Multi-threaded database access patterns
- ✅ **Memory Management**: Memory usage tracking and optimization
- ✅ **Data Iteration**: Table scanning and prefix searches
- ✅ **Watch Operations**: Real-time data change notifications

## ⚡ Performance Tests

### Test Scenarios
1. **Concurrent Connections**: Test 1,000 simultaneous database connections
2. **Sustained Load**: Continuous operations for extended periods
3. **Batch Performance**: Bulk operation throughput testing
4. **Memory Limits**: Resource usage under high load
5. **Full System**: Comprehensive multi-workload testing

### Running Performance Tests
```bash
# Run all performance tests
./run_tests.sh performance

# Run specific performance test
cargo test --features backend_postgresql test_concurrent_connections -- --test-threads=1

# Run with monitoring
./scripts/performance_monitor.sh test concurrent
```

### Performance Metrics
- **Throughput**: Operations per second
- **Latency**: Average/Min/Max response times
- **Success Rate**: Percentage of successful operations
- **Resource Usage**: CPU, Memory, Disk utilization
- **Concurrent Connections**: Simultaneous connection handling

## 📊 Performance Monitoring

### Real-time Monitoring
```bash
# Start performance monitor
./scripts/performance_monitor.sh monitor

# Monitor with custom interval
./scripts/performance_monitor.sh monitor --interval 2

# Run test with monitoring
./scripts/performance_monitor.sh test all
```

### Generate Reports
```bash
# Generate HTML performance report
./scripts/performance_monitor.sh report
```

## 🛠️ Test Environment Setup

### Automatic Setup
```bash
# Full environment setup
./tests/test_setup.sh setup

# Verify setup
./tests/test_setup.sh verify

# Clean test data
./tests/test_setup.sh cleanup

# Reset database
./tests/test_setup.sh reset
```

### Manual Setup
```bash
# 1. Install PostgreSQL
brew install postgresql  # macOS
# or
sudo apt-get install postgresql postgresql-contrib  # Ubuntu

# 2. Start PostgreSQL service
brew services start postgresql  # macOS
# or
sudo systemctl start postgresql  # Ubuntu

# 3. Create test database and user
psql -U postgres -c "CREATE DATABASE matrixon_test;"
psql -U postgres -c "CREATE USER matrixon WITH PASSWORD 'matrixon';"
psql -U postgres -c "GRANT ALL PRIVILEGES ON DATABASE matrixon_test TO matrixon;"

# 4. Set environment variables
export TEST_DATABASE_URL="postgresql://matrixon:matrixon@localhost:5432/matrixon_test"
export matrixon_DATABASE_BACKEND="postgresql"
export RUST_LOG="matrixon=info"
```

## 🎯 Test Configuration

### Database Configuration
The tests use optimized settings for PostgreSQL:
- **Max Connections**: 500
- **Shared Buffers**: 256MB
- **Effective Cache Size**: 1GB
- **Work Memory**: 4MB
- **Cache Hit Ratio Target**: >90%

### Performance Configuration
```rust
PerformanceConfig {
    concurrent_connections: 1000,
    operations_per_connection: 100,
    test_duration_seconds: 30,
    batch_size: 50,
    max_latency_ms: 100,
}
```

## 📈 Expected Performance Metrics

### Database Operations
- **Insert Operations**: >10,000 ops/sec
- **Read Operations**: >50,000 ops/sec
- **Batch Operations**: >1,000 batches/sec
- **Concurrent Connections**: 1,000+ simultaneous
- **Success Rate**: >95%

### Latency Targets
- **Average Latency**: <50ms
- **95th Percentile**: <100ms
- **99th Percentile**: <200ms

## 🐛 Troubleshooting

### Common Issues

#### PostgreSQL Connection Failed
```bash
# Check if PostgreSQL is running
pg_isready -h localhost -p 5432

# Start PostgreSQL service
brew services start postgresql  # macOS
sudo systemctl start postgresql  # Ubuntu
```

#### Database Permission Denied
```bash
# Grant permissions to matrixon user
psql -U postgres -c "ALTER USER matrixon CREATEDB;"
psql -U postgres -c "GRANT ALL ON SCHEMA public TO matrixon;"
```

#### Tests Filtered Out
This happens when PostgreSQL feature is not enabled:
```bash
# Make sure to use the backend_postgresql feature
cargo test --features backend_postgresql

# Check if database is accessible
./tests/test_setup.sh verify
```

#### High Memory Usage
```bash
# Monitor memory during tests
./scripts/performance_monitor.sh monitor --interval 1

# Reduce test load
# Edit tests/performance_tests.rs and reduce:
# - concurrent_connections
# - operations_per_connection
# - test_duration_seconds
```

### Performance Issues
- **Low Throughput**: Check PostgreSQL configuration and ensure SSD storage
- **High Latency**: Verify network connectivity and database index usage
- **Connection Limits**: Increase PostgreSQL max_connections setting
- **Memory Usage**: Monitor and tune PostgreSQL shared_buffers

## 🔧 Advanced Configuration

### PostgreSQL Optimization
Edit PostgreSQL configuration file (`postgresql.conf`):
```ini
# Performance settings for testing
max_connections = 500
shared_buffers = 256MB
effective_cache_size = 1GB
maintenance_work_mem = 64MB
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1
effective_io_concurrency = 200
work_mem = 4MB
```

### Test Customization
Edit test configurations in:
- `tests/database_tests.rs` - Database test parameters
- `tests/performance_tests.rs` - Performance test settings
- `tests/test_setup.sh` - Environment configuration

## 📚 Additional Resources

- [PostgreSQL Performance Tuning](https://www.postgresql.org/docs/current/performance-tips.html)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tokio Async Testing](https://tokio.rs/tokio/topics/testing)
- [matrixon Configuration](../README.md)

## 🤝 Contributing

When adding new tests:
1. Follow the existing test structure and naming conventions
2. Add comprehensive error handling and logging
3. Include performance metrics and assertions
4. Update this documentation
5. Ensure tests can run both with and without PostgreSQL

## 📝 Test Results Example

```
📊 ===== PERFORMANCE TEST RESULTS =====
📊 Test Duration: 30.2s
📊 Total Operations: 150,000
📊 Successful Operations: 149,850
📊 Failed Operations: 150
📊 Success Rate: 99.90%
📊 Throughput: 4,967.55 ops/sec
📊 Average Latency: 45.23 ms
📊 Min Latency: 2 ms
📊 Max Latency: 234 ms
📊 =====================================
```

---

**Note**: These tests are designed to validate the enhanced high-performance capabilities of matrixon with PostgreSQL backend support for 100,000+ concurrent connections. 
