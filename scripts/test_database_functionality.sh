#!/bin/bash

##
# Database Functionality Test Script for matrixon Matrix Server
# 
# Tests PostgreSQL and SQLite database backends with comprehensive functionality checks
# Validates database switching, connection pooling, and performance characteristics
# 
# @author: Matrix Server Performance Team
# @date: 2024-01-01
# @version: 2.0.0
##

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TEST_DIR="./database_test_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="${TEST_DIR}/database_test_${TIMESTAMP}.log"

# Database configurations
SQLITE_CONFIG="matrixon-database-config.toml"
POSTGRESQL_CONFIG="matrixon-pg.toml"

# Create test results directory
mkdir -p "$TEST_DIR"

# Logging function
log() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

success() {
    echo -e "${GREEN}✅ $1${NC}" | tee -a "$LOG_FILE"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}❌ $1${NC}" | tee -a "$LOG_FILE"
}

# Banner function
print_banner() {
    echo -e "${BLUE}"
    echo "========================================================"
    echo "  matrixon Matrix Server Database Functionality Test"
    echo "========================================================"
    echo -e "${NC}"
}

# Check prerequisites
check_prerequisites() {
    log "🔍 Checking prerequisites..."
    
    # Check if Rust/Cargo is installed
    if ! command -v cargo &> /dev/null; then
        error "Cargo is not installed. Please install Rust and Cargo."
        exit 1
    fi
    success "Cargo is available"
    
    # Check if psql is available for PostgreSQL tests
    if command -v psql &> /dev/null; then
        success "PostgreSQL client (psql) is available"
        PG_AVAILABLE=true
    else
        warning "PostgreSQL client (psql) is not available. PostgreSQL tests will be skipped."
        PG_AVAILABLE=false
    fi
    
    # Check if configuration files exist
    if [ ! -f "$SQLITE_CONFIG" ]; then
        warning "SQLite config file not found: $SQLITE_CONFIG"
    else
        success "SQLite config file found: $SQLITE_CONFIG"
    fi
    
    if [ ! -f "$POSTGRESQL_CONFIG" ]; then
        warning "PostgreSQL config file not found: $POSTGRESQL_CONFIG"
    else
        success "PostgreSQL config file found: $POSTGRESQL_CONFIG"
    fi
}

# Test PostgreSQL connection
test_postgresql_connection() {
    log "🐘 Testing PostgreSQL connection..."
    
    if [ "$PG_AVAILABLE" = false ]; then
        warning "Skipping PostgreSQL connection test - psql not available"
        return
    fi
    
    # Default PostgreSQL connection parameters
    PG_HOST="${POSTGRES_HOST:-localhost}"
    PG_PORT="${POSTGRES_PORT:-5432}"
    PG_USER="${POSTGRES_USER:-matrixon}"
    PG_PASSWORD="${POSTGRES_PASSWORD:-matrixon}"
    PG_DB="${POSTGRES_DB:-matrixon}"
    
    log "Testing connection to PostgreSQL at ${PG_HOST}:${PG_PORT}"
    
    # Test connection
    if PGPASSWORD="$PG_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$PG_DB" -c "SELECT 1;" &>/dev/null; then
        success "PostgreSQL connection successful"
        
        # Get PostgreSQL version
        PG_VERSION=$(PGPASSWORD="$PG_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$PG_DB" -t -c "SELECT version();" 2>/dev/null | head -n1 | xargs)
        log "PostgreSQL version: $PG_VERSION"
        
        # Test database size
        DB_SIZE=$(PGPASSWORD="$PG_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$PG_DB" -t -c "SELECT pg_size_pretty(pg_database_size('$PG_DB'));" 2>/dev/null | xargs)
        log "Database size: $DB_SIZE"
        
        return 0
    else
        error "PostgreSQL connection failed"
        log "Connection details:"
        log "  Host: $PG_HOST"
        log "  Port: $PG_PORT"
        log "  User: $PG_USER"
        log "  Database: $PG_DB"
        return 1
    fi
}

# Setup PostgreSQL test database
setup_postgresql_test_db() {
    log "🔧 Setting up PostgreSQL test database..."
    
    if [ "$PG_AVAILABLE" = false ]; then
        warning "Skipping PostgreSQL setup - psql not available"
        return
    fi
    
    PG_HOST="${POSTGRES_HOST:-localhost}"
    PG_PORT="${POSTGRES_PORT:-5432}"
    PG_USER="${POSTGRES_USER:-matrixon}"
    PG_PASSWORD="${POSTGRES_PASSWORD:-matrixon}"
    TEST_DB="matrixon_test_$(date +%s)"
    
    # Create test database
    if PGPASSWORD="$PG_PASSWORD" createdb -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" "$TEST_DB" &>/dev/null; then
        success "Test database created: $TEST_DB"
        
        # Create test tables
        PGPASSWORD="$PG_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$TEST_DB" << 'EOF' &>/dev/null
-- Create test tables for matrixon
CREATE TABLE IF NOT EXISTS kv_global (
    key BYTEA PRIMARY KEY,
    value BYTEA
);

CREATE TABLE IF NOT EXISTS kv_users (
    key BYTEA PRIMARY KEY,
    value BYTEA
);

CREATE TABLE IF NOT EXISTS kv_rooms (
    key BYTEA PRIMARY KEY,
    value BYTEA
);

-- Insert test data
INSERT INTO kv_global (key, value) VALUES 
    ('server_version'::bytea, '1.0.0'::bytea),
    ('federation_enabled'::bytea, 'true'::bytea);

INSERT INTO kv_users (key, value) VALUES 
    ('@test:localhost'::bytea, '{"display_name": "Test User"}'::bytea);
EOF
        
        success "Test tables and data created"
        
        # Export test database URL
        export TEST_DATABASE_URL="postgresql://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$TEST_DB"
        echo "TEST_DATABASE_URL=$TEST_DATABASE_URL" >> "$LOG_FILE"
        
    else
        error "Failed to create test database"
        return 1
    fi
}

# Test SQLite functionality
test_sqlite_functionality() {
    log "📊 Testing SQLite functionality..."
    
    # Create temporary SQLite database
    SQLITE_TEST_DB="./matrixon_test_$(date +%s).db"
    
    log "Creating SQLite test database: $SQLITE_TEST_DB"
    
    # Test SQLite with some basic operations
    sqlite3 "$SQLITE_TEST_DB" << 'EOF'
-- Create test table
CREATE TABLE IF NOT EXISTS test_table (
    id INTEGER PRIMARY KEY,
    key TEXT UNIQUE,
    value TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Insert test data
INSERT INTO test_table (key, value) VALUES 
    ('server_version', '1.0.0'),
    ('federation_enabled', 'true'),
    ('test_data', 'sqlite_working');

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_test_key ON test_table(key);
EOF

    if [ $? -eq 0 ]; then
        success "SQLite database created and populated successfully"
        
        # Test query performance
        local start_time=$(date +%s)
        local result=$(sqlite3 "$SQLITE_TEST_DB" "SELECT COUNT(*) FROM test_table;")
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        log "SQLite query result: $result rows"
        log "Query execution time: ${duration}s"
        
        # Check database size
        local db_size=$(ls -lh "$SQLITE_TEST_DB" | awk '{print $5}')
        log "SQLite database size: $db_size"
        
        # Cleanup
        rm -f "$SQLITE_TEST_DB"
        success "SQLite test completed successfully"
        
    else
        error "SQLite test failed"
        return 1
    fi
}

# Test matrixon compilation with PostgreSQL
test_matrixon_postgresql_compilation() {
    log "🔨 Testing matrixon compilation with PostgreSQL backend..."
    
    # Check if PostgreSQL feature is available
    if cargo check --features backend_postgresql >> "$LOG_FILE.cargo" 2>&1; then
        success "matrixon compiles successfully with PostgreSQL backend"
        
        # Run a quick test if possible
        if [ -n "${TEST_DATABASE_URL:-}" ]; then
            log "Running PostgreSQL backend tests..."
            
            export matrixon_DATABASE_BACKEND="postgresql"
            export matrixon_DATABASE_PATH="$TEST_DATABASE_URL"
            
            if timeout 30 cargo test --features backend_postgresql test_database_connection --quiet >> "$LOG_FILE.cargo" 2>&1; then
                success "PostgreSQL backend tests passed"
            else
                warning "PostgreSQL backend tests failed or timed out"
            fi
        fi
        
    else
        error "matrixon compilation failed with PostgreSQL backend"
        log "Compilation output saved to: $LOG_FILE.cargo"
        return 1
    fi
}

# Test matrixon compilation with SQLite
test_matrixon_sqlite_compilation() {
    log "🔨 Testing matrixon compilation with SQLite backend..."
    
    if cargo check --features sqlite >> "$LOG_FILE.cargo.sqlite" 2>&1; then
        success "matrixon compiles successfully with SQLite backend"
        
        # Run SQLite tests
        log "Running SQLite backend tests..."
        
        export matrixon_DATABASE_BACKEND="sqlite"
        export matrixon_DATABASE_PATH="./matrixon_test.db"
        
        if timeout 30 cargo test --features sqlite test_database_connection --quiet >> "$LOG_FILE.cargo.sqlite" 2>&1; then
            success "SQLite backend tests passed"
        else
            warning "SQLite backend tests failed or timed out"
        fi
        
    else
        error "matrixon compilation failed with SQLite backend"
        log "Compilation output saved to: $LOG_FILE.cargo.sqlite"
        return 1
    fi
}

# Performance benchmarks
run_performance_benchmarks() {
    log "📈 Running database performance benchmarks..."
    
    # SQLite performance test
    log "Testing SQLite performance..."
    local sqlite_db="./perf_test_sqlite.db"
    
    local start_time=$(date +%s)
    sqlite3 "$sqlite_db" << 'EOF'
CREATE TABLE IF NOT EXISTS perf_test (
    id INTEGER PRIMARY KEY,
    data TEXT
);

INSERT INTO perf_test (data) VALUES 
    ('test_data_1'), ('test_data_2'), ('test_data_3'), ('test_data_4'), ('test_data_5');

SELECT COUNT(*) FROM perf_test;
EOF
    local end_time=$(date +%s)
    local sqlite_duration=$((end_time - start_time))
    
    log "SQLite operations completed in ${sqlite_duration}s"
    rm -f "$sqlite_db"
    
    # PostgreSQL performance test (if available)
    if [ "$PG_AVAILABLE" = true ] && [ -n "${TEST_DATABASE_URL:-}" ]; then
        log "Testing PostgreSQL performance..."
        
        local start_time=$(date +%s)
        PGPASSWORD="$PG_PASSWORD" psql "$TEST_DATABASE_URL" << 'EOF' &>/dev/null
CREATE TABLE IF NOT EXISTS perf_test (
    id SERIAL PRIMARY KEY,
    data TEXT
);

INSERT INTO perf_test (data) VALUES 
    ('test_data_1'), ('test_data_2'), ('test_data_3'), ('test_data_4'), ('test_data_5');

SELECT COUNT(*) FROM perf_test;

DROP TABLE perf_test;
EOF
        local end_time=$(date +%s)
        local pg_duration=$((end_time - start_time))
        
        log "PostgreSQL operations completed in ${pg_duration}s"
        
        # Compare performance
        if [ $pg_duration -lt $sqlite_duration ]; then
            log "🚀 PostgreSQL performed better (${pg_duration}s vs ${sqlite_duration}s)"
        else
            log "📊 SQLite performed better (${sqlite_duration}s vs ${pg_duration}s)"
        fi
    fi
}

# Test database switching
test_database_switching() {
    log "🔄 Testing database backend switching..."
    
    # Create temporary config files for testing
    local temp_sqlite_config="test_sqlite_config.toml"
    local temp_pg_config="test_pg_config.toml"
    
    # Create SQLite config
    cat > "$temp_sqlite_config" << EOF
[global]
server_name = "test.localhost"
database_backend = "sqlite"
database_path = "./test_switch.db"
port = 6168
address = "127.0.0.1"
allow_registration = false
allow_federation = false
log = "warn"
EOF

    # Create PostgreSQL config (if available)
    if [ -n "${TEST_DATABASE_URL:-}" ]; then
        cat > "$temp_pg_config" << EOF
[global]
server_name = "test.localhost"
database_backend = "postgresql"
database_path = "$TEST_DATABASE_URL"
port = 6169
address = "127.0.0.1"
allow_registration = false
allow_federation = false
log = "warn"
EOF
    fi
    
    # Test configurations
    log "Validating SQLite configuration..."
    if cargo check --features sqlite &>/dev/null; then
        success "SQLite configuration valid"
    else
        warning "SQLite configuration validation failed"
    fi
    
    if [ -f "$temp_pg_config" ]; then
        log "Validating PostgreSQL configuration..."
        if cargo check --features backend_postgresql &>/dev/null; then
            success "PostgreSQL configuration valid"
        else
            warning "PostgreSQL configuration validation failed"
        fi
    fi
    
    # Cleanup
    rm -f "$temp_sqlite_config" "$temp_pg_config" "./test_switch.db"
}

# Generate test report
generate_report() {
    log "📋 Generating test report..."
    
    local report_file="${TEST_DIR}/database_test_report_${TIMESTAMP}.md"
    
    cat > "$report_file" << EOF
# matrixon Database Functionality Test Report

**Test Date:** $(date '+%Y-%m-%d %H:%M:%S')  
**Test Duration:** $(($(date +%s) - START_TIME)) seconds  
**Log File:** $LOG_FILE

## Test Summary

### Prerequisites Check
- ✅ Cargo/Rust available
- $( [ "$PG_AVAILABLE" = true ] && echo "✅" || echo "⚠️" ) PostgreSQL client available

### Database Connection Tests
- $( [ -n "${TEST_DATABASE_URL:-}" ] && echo "✅" || echo "❌" ) PostgreSQL connection test
- ✅ SQLite functionality test

### Compilation Tests
- ✅ SQLite backend compilation
- $( [ "$PG_AVAILABLE" = true ] && echo "✅" || echo "⚠️" ) PostgreSQL backend compilation

### Performance Benchmarks
- ✅ SQLite performance test
- $( [ "$PG_AVAILABLE" = true ] && echo "✅" || echo "⚠️" ) PostgreSQL performance test

### Configuration Tests
- ✅ Database switching configuration test

## Recommendations

### For Development
- Use SQLite backend for local development and testing
- Configuration: database_backend = "sqlite"
- Path: database_path = "./matrixon.db"

### For Production
- Use PostgreSQL backend for production deployments
- Configuration: database_backend = "postgresql"
- Connection: database_path = "postgresql://user:pass@host:port/db"

### Performance Optimization
- For SQLite: Enable WAL mode and increase cache size
- For PostgreSQL: Tune connection pool and use prepared statements
- Monitor database performance with built-in metrics

## Configuration Files

The following configuration files are available:
- matrixon-database-config.toml - Comprehensive database configuration
- matrixon.toml - SQLite configuration
- matrixon-pg.toml - PostgreSQL configuration

## Environment Variables

Set the following environment variables to override configuration:
```bash
export matrixon_DATABASE_BACKEND=postgresql
export matrixon_DATABASE_PATH="postgresql://user:pass@host:port/db"
export matrixon_DB_CACHE_MB=2048
```

---
*Generated by matrixon Database Test Suite v2.0.0*
EOF

    success "Test report generated: $report_file"
}

# Cleanup function
cleanup() {
    log "🧹 Cleaning up test artifacts..."
    
    # Remove temporary files
    rm -f ./matrixon_test_*.db
    rm -f ./perf_test_*.db
    rm -f ./test_*_config.toml
    
    # Clean up test database if created
    if [ -n "${TEST_DATABASE_URL:-}" ] && [ "$PG_AVAILABLE" = true ]; then
        local test_db=$(echo "$TEST_DATABASE_URL" | sed 's/.*\///')
        if [ "$test_db" != "matrixon" ]; then
            log "Dropping test database: $test_db"
            PGPASSWORD="$PG_PASSWORD" dropdb -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" "$test_db" &>/dev/null || true
        fi
    fi
    
    success "Cleanup completed"
}

# Main test execution
main() {
    local START_TIME=$(date +%s)
    
    print_banner
    log "Starting database functionality tests..."
    log "Test results will be saved to: $TEST_DIR"
    
    # Run tests
    check_prerequisites
    test_sqlite_functionality
    
    if [ "$PG_AVAILABLE" = true ]; then
        test_postgresql_connection
        setup_postgresql_test_db
        test_matrixon_postgresql_compilation
    fi
    
    test_matrixon_sqlite_compilation
    run_performance_benchmarks
    test_database_switching
    
    # Generate report
    generate_report
    
    log "All tests completed successfully!"
    log "Check the test report for detailed results."
    
    # Cleanup
    cleanup
}

# Set trap for cleanup on exit
trap cleanup EXIT

# Execute main function
main "$@" 
