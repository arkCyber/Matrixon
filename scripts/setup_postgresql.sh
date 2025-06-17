#!/bin/bash

##
# PostgreSQL Setup Script for matrixon Matrix Server
# 
# This script sets up PostgreSQL database for matrixon testing and development
# Handles database creation, user setup, and optimization configuration
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
NC='\033[0m'

# PostgreSQL configuration
PG_HOST="${POSTGRES_HOST:-localhost}"
PG_PORT="${POSTGRES_PORT:-5432}"
PG_ADMIN_USER="${POSTGRES_ADMIN_USER:-postgres}"
PG_USER="${POSTGRES_USER:-matrixon}"
PG_PASSWORD="${POSTGRES_PASSWORD:-matrixon}"
PG_DB="${POSTGRES_DB:-matrixon}"
PG_TEST_DB="${POSTGRES_TEST_DB:-matrixon_test}"

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
    echo "  PostgreSQL Setup for matrixon Matrix Server"
    echo "========================================================"
    echo -e "${NC}"
}

# Check if PostgreSQL is running
check_postgresql() {
    log "🔍 Checking PostgreSQL availability..."
    
    if ! command -v psql &> /dev/null; then
        error "PostgreSQL client (psql) is not installed"
        echo ""
        echo "Please install PostgreSQL:"
        echo "  macOS: brew install postgresql"
        echo "  Ubuntu/Debian: sudo apt-get install postgresql-client"
        echo "  CentOS/RHEL: sudo yum install postgresql"
        exit 1
    fi
    
    success "PostgreSQL client is available"
    
    # Test connection to PostgreSQL
    if psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" -c "SELECT 1;" &>/dev/null; then
        success "PostgreSQL server is running and accessible"
    else
        error "Cannot connect to PostgreSQL server"
        echo ""
        echo "Please ensure PostgreSQL is running:"
        echo "  macOS: brew services start postgresql"
        echo "  Ubuntu/Debian: sudo systemctl start postgresql"
        echo "  Docker: docker run -d --name postgres -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:15"
        exit 1
    fi
    
    # Get PostgreSQL version
    local pg_version=$(psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" -t -c "SELECT version();" 2>/dev/null | head -n1 | xargs)
    log "PostgreSQL version: $pg_version"
}

# Create database and user
setup_database() {
    log "🗄️  Setting up matrixon database..."
    
    # Check if database exists
    if psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" -lqt | cut -d \| -f 1 | grep -qw "$PG_DB"; then
        warning "Database '$PG_DB' already exists"
        read -p "Do you want to drop and recreate it? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            log "Dropping existing database..."
            dropdb -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" "$PG_DB"
        else
            log "Using existing database"
            return 0
        fi
    fi
    
    # Create database
    log "Creating database '$PG_DB'..."
    createdb -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" "$PG_DB"
    success "Database '$PG_DB' created"
    
    # Create user if not exists
    log "Setting up user '$PG_USER'..."
    psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" -d "$PG_DB" << EOF
DO \$\$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = '$PG_USER') THEN
        CREATE USER $PG_USER WITH PASSWORD '$PG_PASSWORD';
    END IF;
END
\$\$;

GRANT ALL PRIVILEGES ON DATABASE $PG_DB TO $PG_USER;
GRANT ALL ON SCHEMA public TO $PG_USER;
ALTER USER $PG_USER CREATEDB;
ALTER DATABASE $PG_DB OWNER TO $PG_USER;
EOF
    
    success "User '$PG_USER' configured with database access"
}

# Create test database
setup_test_database() {
    log "🧪 Setting up test database..."
    
    # Drop test database if exists
    if psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" -lqt | cut -d \| -f 1 | grep -qw "$PG_TEST_DB"; then
        log "Dropping existing test database..."
        dropdb -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" "$PG_TEST_DB"
    fi
    
    # Create test database
    createdb -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" "$PG_TEST_DB"
    
    # Grant access to matrixon user
    psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" -d "$PG_TEST_DB" << EOF
GRANT ALL PRIVILEGES ON DATABASE $PG_TEST_DB TO $PG_USER;
GRANT ALL ON SCHEMA public TO $PG_USER;
ALTER DATABASE $PG_TEST_DB OWNER TO $PG_USER;
EOF
    
    success "Test database '$PG_TEST_DB' created"
}

# Install PostgreSQL extensions
install_extensions() {
    log "🔧 Installing PostgreSQL extensions..."
    
    psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$PG_DB" << 'EOF'
-- Install extensions for better performance
CREATE EXTENSION IF NOT EXISTS btree_gin;
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Create indexes for common query patterns
-- (These will be used by matrixon's KV tables)
EOF
    
    success "PostgreSQL extensions installed"
}

# Create sample matrixon tables
create_sample_tables() {
    log "📋 Creating sample matrixon tables..."
    
    psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$PG_DB" << 'EOF'
-- Sample tables that mimic matrixon's key-value structure
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

CREATE TABLE IF NOT EXISTS kv_events (
    key BYTEA PRIMARY KEY,
    value BYTEA
);

CREATE TABLE IF NOT EXISTS kv_state (
    key BYTEA PRIMARY KEY,
    value BYTEA
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_kv_global_key ON kv_global USING HASH (key);
CREATE INDEX IF NOT EXISTS idx_kv_users_key ON kv_users USING HASH (key);
CREATE INDEX IF NOT EXISTS idx_kv_rooms_key ON kv_rooms USING HASH (key);
CREATE INDEX IF NOT EXISTS idx_kv_events_key ON kv_events USING HASH (key);
CREATE INDEX IF NOT EXISTS idx_kv_state_key ON kv_state USING HASH (key);

-- Insert some initial data
INSERT INTO kv_global (key, value) VALUES 
    ('server_version'::bytea, '"1.0.0"'::bytea),
    ('federation_enabled'::bytea, 'true'::bytea),
    ('registration_enabled'::bytea, 'true'::bytea)
ON CONFLICT (key) DO NOTHING;

-- Create a test user
INSERT INTO kv_users (key, value) VALUES 
    ('@test:localhost'::bytea, '{"display_name": "Test User", "created_at": "2024-01-01T00:00:00Z"}'::bytea)
ON CONFLICT (key) DO NOTHING;
EOF
    
    success "Sample matrixon tables created with test data"
}

# Test database performance
test_performance() {
    log "📊 Testing database performance..."
    
    # Run some performance tests
    local start_time=$(date +%s%N)
    
    psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$PG_DB" << 'EOF' > /dev/null
-- Performance test queries
SELECT COUNT(*) FROM kv_global;
SELECT COUNT(*) FROM kv_users;
SELECT COUNT(*) FROM kv_rooms;

-- Test INSERT performance
INSERT INTO kv_global (key, value) 
SELECT 
    ('test_key_' || generate_series(1, 1000))::bytea,
    ('test_value_' || generate_series(1, 1000))::bytea
ON CONFLICT (key) DO NOTHING;

-- Test SELECT performance
SELECT * FROM kv_global WHERE key = 'server_version'::bytea;

-- Cleanup test data
DELETE FROM kv_global WHERE key LIKE 'test_key_%'::bytea;
EOF
    
    local end_time=$(date +%s%N)
    local duration=$((($end_time - $start_time) / 1000000))
    
    log "Performance test completed in ${duration}ms"
    
    # Get database statistics
    local db_size=$(psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$PG_DB" -t -c "SELECT pg_size_pretty(pg_database_size('$PG_DB'));" 2>/dev/null | xargs)
    log "Database size: $db_size"
    
    local table_count=$(psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$PG_DB" -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';" 2>/dev/null | xargs)
    log "Tables created: $table_count"
    
    success "Database performance test completed"
}

# Generate configuration
generate_config() {
    log "📝 Generating PostgreSQL configuration for matrixon..."
    
    local pg_config_file="matrixon-pg-generated.toml"
    
    cat > "$pg_config_file" << EOF
##
# matrixon PostgreSQL Configuration
# Generated by setup_postgresql.sh on $(date)
##

[global]
# Server configuration
server_name = "localhost"
address = "127.0.0.1"
port = 6167

# PostgreSQL database configuration
database_backend = "postgresql"
database_path = "postgresql://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$PG_DB"

# Performance settings
db_cache_capacity_mb = 1024.0
max_concurrent_requests = 1000
pdu_cache_capacity = 100000
cleanup_second_interval = 300

# Basic Matrix server settings
max_request_size = 20_000_000
allow_registration = true
allow_federation = true
enable_lightning_bolt = true
trusted_servers = ["matrix.org"]
log = "info"

[global.well_known]
client = "http://localhost:6167"
EOF
    
    success "PostgreSQL configuration generated: $pg_config_file"
    
    # Generate environment file
    local env_file=".env.postgresql"
    
    cat > "$env_file" << EOF
# PostgreSQL Environment Variables for matrixon
# Source this file: source .env.postgresql

export matrixon_DATABASE_BACKEND=postgresql
export matrixon_DATABASE_PATH="postgresql://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$PG_DB"
export TEST_DATABASE_URL="postgresql://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$PG_TEST_DB"

# PostgreSQL connection settings
export POSTGRES_HOST=$PG_HOST
export POSTGRES_PORT=$PG_PORT
export POSTGRES_USER=$PG_USER
export POSTGRES_PASSWORD=$PG_PASSWORD
export POSTGRES_DB=$PG_DB

# Rust settings
export RUST_LOG=matrixon=info
export RUST_BACKTRACE=1
EOF
    
    success "Environment file generated: $env_file"
    
    echo ""
    echo "To use PostgreSQL with matrixon:"
    echo "  1. Source the environment file: source $env_file"
    echo "  2. Run matrixon with: cargo run --features backend_postgresql -- -c $pg_config_file"
    echo "  3. Or run tests with: cargo test --features backend_postgresql"
}

# Cleanup function
cleanup() {
    read -p "Do you want to remove test databases? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        log "🧹 Cleaning up test databases..."
        dropdb -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" "$PG_TEST_DB" 2>/dev/null || true
        success "Test databases cleaned up"
    fi
}

# Show help
show_help() {
    echo "PostgreSQL Setup Script for matrixon Matrix Server"
    echo ""
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  setup     - Full setup (create database, user, tables)"
    echo "  test      - Test database connection and performance"
    echo "  config    - Generate matrixon configuration files"
    echo "  cleanup   - Remove test databases"
    echo "  help      - Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  POSTGRES_HOST         - PostgreSQL host (default: localhost)"
    echo "  POSTGRES_PORT         - PostgreSQL port (default: 5432)"
    echo "  POSTGRES_ADMIN_USER   - Admin user (default: postgres)"
    echo "  POSTGRES_USER         - matrixon user (default: matrixon)"
    echo "  POSTGRES_PASSWORD     - matrixon password (default: matrixon)"
    echo "  POSTGRES_DB           - Main database (default: matrixon)"
    echo "  POSTGRES_TEST_DB      - Test database (default: matrixon_test)"
}

# Main function
main() {
    local command="${1:-setup}"
    
    case "$command" in
        "setup")
            print_banner
            check_postgresql
            setup_database
            setup_test_database
            install_extensions
            create_sample_tables
            test_performance
            generate_config
            ;;
        "test")
            check_postgresql
            test_performance
            ;;
        "config")
            generate_config
            ;;
        "cleanup")
            cleanup
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            error "Unknown command: $command"
            show_help
            exit 1
            ;;
    esac
}

# Execute main function
main "$@" 
