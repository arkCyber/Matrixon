#!/bin/bash

##
# Test Environment Setup Script for matrixon Matrix Server
# 
# Automates the setup of testing environment including PostgreSQL,
# database initialization, and test data preparation
# 
# @author: Matrix Server Performance Team
# @date: 2024-01-01
# @version: 1.0.0
##

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
POSTGRES_USER="matrixon"
POSTGRES_PASSWORD="matrixon"
POSTGRES_DB="matrixon_test"
POSTGRES_HOST="localhost"
POSTGRES_PORT="5432"
TEST_DATABASE_URL="postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@${POSTGRES_HOST}:${POSTGRES_PORT}/${POSTGRES_DB}"

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️ $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
    exit 1
}

# Check if PostgreSQL is running
check_postgres() {
    log "Checking PostgreSQL connection..."
    
    if pg_isready -h $POSTGRES_HOST -p $POSTGRES_PORT > /dev/null 2>&1; then
        success "PostgreSQL is running"
    else
        error "PostgreSQL is not running. Please start PostgreSQL first."
    fi
}

# Setup test database
setup_database() {
    log "Setting up test database..."
    
    # Check if database exists
    if psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U postgres -lqt | cut -d \| -f 1 | grep -qw $POSTGRES_DB; then
        warning "Database $POSTGRES_DB already exists. Dropping and recreating..."
        dropdb -h $POSTGRES_HOST -p $POSTGRES_PORT -U postgres $POSTGRES_DB
    fi
    
    # Create database
    createdb -h $POSTGRES_HOST -p $POSTGRES_PORT -U postgres $POSTGRES_DB
    success "Database $POSTGRES_DB created"
    
    # Create user if not exists
    psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U postgres -d $POSTGRES_DB -c "
        DO \$\$
        BEGIN
            IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = '$POSTGRES_USER') THEN
                CREATE USER $POSTGRES_USER WITH PASSWORD '$POSTGRES_PASSWORD';
            END IF;
        END
        \$\$;
        
        GRANT ALL PRIVILEGES ON DATABASE $POSTGRES_DB TO $POSTGRES_USER;
        GRANT ALL ON SCHEMA public TO $POSTGRES_USER;
        ALTER USER $POSTGRES_USER CREATEDB;
    " > /dev/null 2>&1
    
    success "User $POSTGRES_USER configured"
}

# Run database migrations
run_migrations() {
    log "Running database migrations..."
    
    if [ -f "migrations/001_initial_schema.sql" ]; then
        psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -f migrations/001_initial_schema.sql > /dev/null 2>&1
        success "Database schema initialized"
    else
        warning "No migration files found, skipping..."
    fi
}

# Optimize PostgreSQL for testing
optimize_postgres() {
    log "Checking PostgreSQL configuration for testing..."
    
    # Check if we can access postgresql.conf
    POSTGRES_CONF_PATH="/usr/local/var/postgresql@14/postgresql.conf"
    if [ ! -f "$POSTGRES_CONF_PATH" ]; then
        # Try alternative paths
        POSTGRES_CONF_PATH="/opt/homebrew/var/postgresql@14/postgresql.conf"
        if [ ! -f "$POSTGRES_CONF_PATH" ]; then
            warning "PostgreSQL config file not found at expected paths. Skipping optimization."
            return
        fi
    fi
    
    log "Found PostgreSQL config at: $POSTGRES_CONF_PATH"
    
    # Backup original config
    if [ ! -f "${POSTGRES_CONF_PATH}.backup" ]; then
        cp "$POSTGRES_CONF_PATH" "${POSTGRES_CONF_PATH}.backup"
        success "PostgreSQL config backed up"
    fi
    
    # Apply test optimizations
    log "Applying test optimizations to PostgreSQL..."
    
    # These settings optimize for testing, not production
    cat >> "$POSTGRES_CONF_PATH" << EOF

# Test optimizations added by matrixon test setup
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
min_wal_size = 1GB
max_wal_size = 4GB
max_worker_processes = 16
max_parallel_workers_per_gather = 4
max_parallel_workers = 16
max_parallel_maintenance_workers = 4
EOF
    
    success "PostgreSQL optimizations applied"
    warning "Please restart PostgreSQL for changes to take effect"
}

# Prepare test data
prepare_test_data() {
    log "Preparing test data..."
    
    # Create test tables for performance testing
    psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB << EOF > /dev/null 2>&1
CREATE TABLE IF NOT EXISTS test_performance (
    id SERIAL PRIMARY KEY,
    key_name VARCHAR(255) NOT NULL,
    value_data BYTEA,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_test_performance_key ON test_performance(key_name);
CREATE INDEX IF NOT EXISTS idx_test_performance_created ON test_performance(created_at);

-- Insert some initial test data
INSERT INTO test_performance (key_name, value_data) 
SELECT 
    'test_key_' || generate_series(1, 1000),
    ('test_value_' || generate_series(1, 1000))::bytea;

-- Create test schema tables that mimic matrixon's structure
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

-- Add some initial data
INSERT INTO kv_global (key, value) VALUES 
    ('server_version'::bytea, '1.0.0'::bytea),
    ('federation_enabled'::bytea, 'true'::bytea);
EOF
    
    success "Test data prepared"
}

# Set environment variables
setup_environment() {
    log "Setting up environment variables..."
    
    export TEST_DATABASE_URL="$TEST_DATABASE_URL"
    export matrixon_DATABASE_BACKEND="postgresql"
    export matrixon_DATABASE_PATH="$TEST_DATABASE_URL"
    export RUST_LOG="matrixon=debug,matrixon::database=trace"
    export RUST_BACKTRACE=1
    
    # Write to .env file for persistence
    cat > .env.test << EOF
TEST_DATABASE_URL=$TEST_DATABASE_URL
matrixon_DATABASE_BACKEND=postgresql
matrixon_DATABASE_PATH=$TEST_DATABASE_URL
RUST_LOG=matrixon=debug,matrixon::database=trace
RUST_BACKTRACE=1
EOF
    
    success "Environment variables configured"
    log "You can source .env.test to load these variables"
}

# Verify test setup
verify_setup() {
    log "Verifying test setup..."
    
    # Test database connection
    if psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -c "SELECT 1;" > /dev/null 2>&1; then
        success "Database connection verified"
    else
        error "Database connection failed"
    fi
    
    # Check table creation
    TABLE_COUNT=$(psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='public';" | xargs)
    log "Found $TABLE_COUNT test tables"
    
    # Test basic operations
    psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -c "
        INSERT INTO kv_global (key, value) VALUES ('test_setup'::bytea, 'success'::bytea);
        SELECT value FROM kv_global WHERE key = 'test_setup'::bytea;
        DELETE FROM kv_global WHERE key = 'test_setup'::bytea;
    " > /dev/null 2>&1
    
    success "Basic database operations verified"
}

# Clean up function
cleanup() {
    log "Cleaning up test environment..."
    
    if [ "$1" = "full" ]; then
        dropdb -h $POSTGRES_HOST -p $POSTGRES_PORT -U postgres $POSTGRES_DB
        success "Test database dropped"
    else
        psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -c "
            TRUNCATE TABLE test_performance;
            TRUNCATE TABLE kv_global, kv_users, kv_rooms, kv_events;
        " > /dev/null 2>&1
        success "Test data cleaned"
    fi
}

# Show usage
usage() {
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  setup     - Full test environment setup (default)"
    echo "  cleanup   - Clean test data"
    echo "  reset     - Drop and recreate database"
    echo "  verify    - Verify test setup"
    echo "  help      - Show this help"
    echo ""
    echo "Environment variables:"
    echo "  POSTGRES_HOST     - PostgreSQL host (default: localhost)"
    echo "  POSTGRES_PORT     - PostgreSQL port (default: 5432)"
    echo "  POSTGRES_USER     - PostgreSQL user (default: matrixon)"
    echo "  POSTGRES_PASSWORD - PostgreSQL password (default: matrixon)"
    echo "  POSTGRES_DB       - PostgreSQL database (default: matrixon_test)"
}

# Main execution
main() {
    case "${1:-setup}" in
        "setup")
            log "🚀 Starting test environment setup..."
            check_postgres
            setup_database
            run_migrations
            prepare_test_data
            setup_environment
            verify_setup
            success "✅ Test environment setup complete!"
            echo ""
            log "To run tests with PostgreSQL backend:"
            echo "  source .env.test"
            echo "  cargo test --features backend_postgresql -- --test-threads=1"
            ;;
        "cleanup")
            cleanup
            ;;
        "reset")
            cleanup full
            setup_database
            run_migrations
            prepare_test_data
            verify_setup
            success "Test environment reset complete!"
            ;;
        "verify")
            verify_setup
            ;;
        "optimize")
            optimize_postgres
            ;;
        "help"|"--help"|"-h")
            usage
            ;;
        *)
            error "Unknown command: $1"
            usage
            ;;
    esac
}

# Check dependencies
check_dependencies() {
    for cmd in psql pg_isready createdb dropdb; do
        if ! command -v $cmd > /dev/null 2>&1; then
            error "Required command '$cmd' not found. Please install PostgreSQL client tools."
        fi
    done
}

# Initialize
check_dependencies
main "$@" 
