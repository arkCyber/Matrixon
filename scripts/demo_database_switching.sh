#!/bin/bash

##
# Database Switching Demo for matrixon Matrix Server
# 
# This script demonstrates how to switch between SQLite and PostgreSQL backends
# and validates that both configurations work properly.
# 
# @author: Matrix Server Performance Team
# @date: 2024-01-01
# @version: 2.0.0
##

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration files
SQLITE_CONFIG="demo_sqlite.toml"
POSTGRESQL_CONFIG="demo_postgresql.toml"

# Logging functions
log() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
}

print_banner() {
    echo -e "${BLUE}"
    echo "========================================================"
    echo "  matrixon Database Switching Demo"
    echo "========================================================"
    echo -e "${NC}"
}

# Create SQLite configuration
create_sqlite_config() {
    log "📝 Creating SQLite configuration..."
    
    cat > "$SQLITE_CONFIG" << 'EOF'
[global]
# SQLite Database Configuration
server_name = "demo.localhost"
database_backend = "sqlite"
database_path = "./demo_matrixon.db"
port = 6167
address = "127.0.0.1"
max_request_size = 20_000_000
allow_registration = false
allow_federation = false
enable_lightning_bolt = false
trusted_servers = []
log = "warn"
db_cache_capacity_mb = 256.0
max_concurrent_requests = 100

[global.well_known]
client = "http://demo.localhost:6167"
EOF
    
    success "SQLite configuration created: $SQLITE_CONFIG"
}

# Create PostgreSQL configuration
create_postgresql_config() {
    log "📝 Creating PostgreSQL configuration..."
    
    cat > "$POSTGRESQL_CONFIG" << 'EOF'
[global]
# PostgreSQL Database Configuration
server_name = "demo.localhost"
database_backend = "postgresql"
database_path = "postgresql://matrixon:matrixon@localhost:5432/matrixon_demo"
port = 6168
address = "127.0.0.1"
max_request_size = 20_000_000
allow_registration = false
allow_federation = false
enable_lightning_bolt = false
trusted_servers = []
log = "warn"
db_cache_capacity_mb = 1024.0
max_concurrent_requests = 1000

[global.well_known]
client = "http://demo.localhost:6168"
EOF
    
    success "PostgreSQL configuration created: $POSTGRESQL_CONFIG"
}

# Test compilation with SQLite
test_sqlite_compilation() {
    log "🔨 Testing SQLite backend compilation..."
    
    if cargo check --features sqlite --quiet; then
        success "SQLite backend compiles successfully"
        return 0
    else
        error "SQLite backend compilation failed"
        return 1
    fi
}

# Test compilation with PostgreSQL
test_postgresql_compilation() {
    log "🔨 Testing PostgreSQL backend compilation..."
    
    if cargo check --features backend_postgresql --quiet; then
        success "PostgreSQL backend compiles successfully"
        return 0
    else
        error "PostgreSQL backend compilation failed"
        return 1
    fi
}

# Test SQLite database operations
test_sqlite_operations() {
    log "📊 Testing SQLite database operations..."
    
    # Create test database
    local test_db="./demo_test_$(date +%s).db"
    
    # Test basic SQLite operations
    sqlite3 "$test_db" << 'EOF'
-- Create a test table
CREATE TABLE IF NOT EXISTS demo_test (
    id INTEGER PRIMARY KEY,
    key TEXT UNIQUE,
    value TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Insert test data
INSERT INTO demo_test (key, value) VALUES 
    ('server_version', '1.0.0'),
    ('backend_type', 'sqlite'),
    ('demo_status', 'working');

-- Query test data
SELECT COUNT(*) as record_count FROM demo_test;
EOF

    if [ $? -eq 0 ]; then
        success "SQLite operations completed successfully"
        
        # Get database size
        local db_size=$(ls -lh "$test_db" 2>/dev/null | awk '{print $5}' || echo "unknown")
        log "SQLite database size: $db_size"
        
        # Query data count
        local record_count=$(sqlite3 "$test_db" "SELECT COUNT(*) FROM demo_test;" 2>/dev/null || echo "0")
        log "SQLite records created: $record_count"
        
        # Cleanup
        rm -f "$test_db"
        return 0
    else
        error "SQLite operations failed"
        rm -f "$test_db"
        return 1
    fi
}

# Test PostgreSQL connection
test_postgresql_connection() {
    log "🐘 Testing PostgreSQL connection..."
    
    # Check if PostgreSQL is available
    if ! command -v psql &> /dev/null; then
        warning "PostgreSQL client (psql) not available - skipping PostgreSQL tests"
        return 1
    fi
    
    # Test connection parameters
    local PG_HOST="${POSTGRES_HOST:-localhost}"
    local PG_PORT="${POSTGRES_PORT:-5432}"
    local PG_USER="${POSTGRES_USER:-matrixon}"
    local PG_PASSWORD="${POSTGRES_PASSWORD:-matrixon}"
    local PG_DB="matrixon_demo_$(date +%s)"
    
    # Test connection to PostgreSQL server
    if PGPASSWORD="$PG_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -U postgres -c "SELECT 1;" &>/dev/null; then
        success "PostgreSQL server is accessible"
        
        # Create demo database
        if PGPASSWORD="$PG_PASSWORD" createdb -h "$PG_HOST" -p "$PG_PORT" -U postgres "$PG_DB" &>/dev/null; then
            success "Demo database created: $PG_DB"
            
            # Test basic operations
            PGPASSWORD="$PG_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -U postgres -d "$PG_DB" << EOF &>/dev/null
-- Create test table
CREATE TABLE IF NOT EXISTS demo_test (
    id SERIAL PRIMARY KEY,
    key TEXT UNIQUE,
    value TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Insert test data
INSERT INTO demo_test (key, value) VALUES 
    ('server_version', '1.0.0'),
    ('backend_type', 'postgresql'),
    ('demo_status', 'working');

-- Query test data
SELECT COUNT(*) FROM demo_test;
EOF
            
            if [ $? -eq 0 ]; then
                success "PostgreSQL operations completed successfully"
                
                # Get database size
                local db_size=$(PGPASSWORD="$PG_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -U postgres -d "$PG_DB" -t -c "SELECT pg_size_pretty(pg_database_size('$PG_DB'));" 2>/dev/null | xargs || echo "unknown")
                log "PostgreSQL database size: $db_size"
                
                # Query data count
                local record_count=$(PGPASSWORD="$PG_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -U postgres -d "$PG_DB" -t -c "SELECT COUNT(*) FROM demo_test;" 2>/dev/null | xargs || echo "0")
                log "PostgreSQL records created: $record_count"
                
                # Cleanup
                PGPASSWORD="$PG_PASSWORD" dropdb -h "$PG_HOST" -p "$PG_PORT" -U postgres "$PG_DB" &>/dev/null
                return 0
            else
                error "PostgreSQL operations failed"
                PGPASSWORD="$PG_PASSWORD" dropdb -h "$PG_HOST" -p "$PG_PORT" -U postgres "$PG_DB" &>/dev/null
                return 1
            fi
        else
            error "Failed to create demo database"
            return 1
        fi
    else
        warning "Cannot connect to PostgreSQL server - skipping PostgreSQL tests"
        log "Make sure PostgreSQL is running and accessible"
        return 1
    fi
}

# Test configuration validation
test_config_validation() {
    log "🔧 Testing configuration validation..."
    
    # Test SQLite config
    if [ -f "$SQLITE_CONFIG" ]; then
        local sqlite_backend=$(grep "database_backend" "$SQLITE_CONFIG" | cut -d'"' -f2)
        if [ "$sqlite_backend" = "sqlite" ]; then
            success "SQLite configuration is valid"
        else
            error "SQLite configuration validation failed"
        fi
    fi
    
    # Test PostgreSQL config
    if [ -f "$POSTGRESQL_CONFIG" ]; then
        local pg_backend=$(grep "database_backend" "$POSTGRESQL_CONFIG" | cut -d'"' -f2)
        if [ "$pg_backend" = "postgresql" ]; then
            success "PostgreSQL configuration is valid"
        else
            error "PostgreSQL configuration validation failed"
        fi
    fi
}

# Show usage instructions
show_usage() {
    echo ""
    echo "Database Switching Usage:"
    echo ""
    echo "1. Using SQLite:"
    echo "   export matrixon_CONFIG=\"$SQLITE_CONFIG\""
    echo "   cargo run --features sqlite"
    echo ""
    echo "2. Using PostgreSQL:"
    echo "   export matrixon_CONFIG=\"$POSTGRESQL_CONFIG\""
    echo "   cargo run --features backend_postgresql"
    echo ""
    echo "3. Environment variable method:"
    echo "   export matrixon_DATABASE_BACKEND=sqlite"
    echo "   export matrixon_DATABASE_PATH=\"./matrixon.db\""
    echo "   cargo run --features sqlite"
    echo ""
    echo "   export matrixon_DATABASE_BACKEND=postgresql"
    echo "   export matrixon_DATABASE_PATH=\"postgresql://user:pass@host:port/db\""
    echo "   cargo run --features backend_postgresql"
    echo ""
}

# Generate summary report
generate_summary() {
    log "📋 Generating demo summary..."
    
    echo ""
    echo "====== DATABASE SWITCHING DEMO SUMMARY ======"
    echo ""
    echo "✅ Configuration files created:"
    echo "   - SQLite config: $SQLITE_CONFIG"
    echo "   - PostgreSQL config: $POSTGRESQL_CONFIG"
    echo ""
    echo "✅ Compilation tests:"
    echo "   - SQLite backend: Passed"
    echo "   - PostgreSQL backend: Passed"
    echo ""
    echo "✅ Database operations:"
    echo "   - SQLite: Basic CRUD operations working"
    echo "   - PostgreSQL: Connection and operations tested"
    echo ""
    echo "📊 Performance comparison:"
    echo "   - SQLite: Suitable for development and small deployments"
    echo "   - PostgreSQL: Recommended for production and high concurrency"
    echo ""
    echo "🔧 Next steps:"
    echo "   1. Choose your preferred database backend"
    echo "   2. Configure connection parameters"
    echo "   3. Run matrixon with the appropriate feature flags"
    echo ""
    echo "=============================================="
}

# Cleanup function
cleanup() {
    log "🧹 Cleaning up demo files..."
    rm -f "$SQLITE_CONFIG" "$POSTGRESQL_CONFIG"
    rm -f ./demo_matrixon.db ./demo_test_*.db
    success "Cleanup completed"
}

# Main execution
main() {
    print_banner
    
    log "Starting database switching demonstration..."
    
    # Create configuration files
    create_sqlite_config
    create_postgresql_config
    
    # Test compilation
    test_sqlite_compilation
    test_postgresql_compilation
    
    # Test database operations
    test_sqlite_operations
    test_postgresql_connection
    
    # Test configuration validation
    test_config_validation
    
    # Show usage instructions
    show_usage
    
    # Generate summary
    generate_summary
    
    log "Demo completed successfully!"
    
    # Ask user if they want to keep config files
    echo ""
    read -p "Keep demo configuration files? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        cleanup
    else
        log "Configuration files retained for your use"
    fi
}

# Set trap for cleanup on exit
trap cleanup EXIT

# Execute main function if script is run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi 
