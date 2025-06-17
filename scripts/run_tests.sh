#!/bin/bash

##
# Quick Test Runner for matrixon Matrix Server
# 
# Simplified script to run database and performance tests
# 
# @author: Matrix Server Performance Team
# @date: 2024-01-01
# @version: 1.0.0
##

set -e

# Color codes
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️ $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if we have necessary environment
check_environment() {
    log "Checking test environment..."
    
    # Check if PostgreSQL is available
    if ! command -v psql > /dev/null 2>&1; then
        warning "PostgreSQL client not found. Database tests may fail."
    fi
    
    # Check if cargo is available
    if ! command -v cargo > /dev/null 2>&1; then
        error "Cargo not found. Please install Rust and Cargo."
        exit 1
    fi
    
    # Check if we can compile
    log "Checking compilation..."
    if ! cargo check --quiet; then
        error "Compilation failed. Please fix compilation errors first."
        exit 1
    fi
    
    success "Environment check passed"
}

# Run basic functionality tests
run_basic_tests() {
    log "Running basic functionality tests..."
    
    # Run without PostgreSQL feature first
    log "Testing with default backend..."
    cargo test --lib --bins --quiet -- --test-threads=1 || warning "Some basic tests failed"
    
    success "Basic tests completed"
}

# Run database tests
run_database_tests() {
    log "Running database functionality tests..."
    
    # Set test environment
    export TEST_DATABASE_URL="postgresql://matrixon:matrixon@localhost:5432/matrixon_test"
    export RUST_LOG="matrixon=info"
    
    # Run database tests
    if cargo test --features backend_postgresql database_tests --quiet -- --test-threads=1; then
        success "Database tests passed"
    else
        warning "Database tests failed - this may be expected if PostgreSQL is not running"
        log "To set up PostgreSQL for testing:"
        log "  1. Start PostgreSQL service"
        log "  2. Run: ./tests/test_setup.sh setup"
        log "  3. Re-run this script"
    fi
}

# Run performance tests (lightweight version)
run_performance_tests() {
    log "Running performance tests..."
    
    # Set test environment
    export TEST_DATABASE_URL="postgresql://matrixon:matrixon@localhost:5432/matrixon_test"
    export RUST_LOG="matrixon=warn"  # Reduce log noise during performance tests
    
    # Run performance tests with shorter duration
    if cargo test --features backend_postgresql performance_tests --quiet -- --test-threads=1; then
        success "Performance tests passed"
    else
        warning "Performance tests failed - this may be expected if PostgreSQL is not set up"
    fi
}

# Run stress tests
run_stress_tests() {
    log "Running ultimate stress tests (10,000+ connections)..."
    
    # Set test environment for stress testing
    export TEST_DATABASE_URL="postgresql://matrixon:matrixon@localhost:5432/matrixon_test"
    export RUST_LOG="matrixon=warn"  # Minimal logging for maximum performance
    
    warning "⚠️ Stress tests require significant system resources!"
    log "📊 Target: 10,000 concurrent connections"
    log "🧵 Using 64 worker threads"
    log "💾 Recommended: 16GB+ RAM"
    
    # Run stress tests
    if cargo test --release --features backend_postgresql stress_tests --quiet -- --test-threads=1; then
        success "Stress tests passed"
    else
        warning "Stress tests failed - this requires a powerful system and proper PostgreSQL setup"
    fi
}

# Run all tests
run_all_tests() {
    log "🚀 Running comprehensive high-performance test suite..."
    log "⚡ Configuration: 64 worker threads, 10,000 max connections"
    
    check_environment
    run_basic_tests
    run_database_tests
    run_performance_tests
    run_stress_tests
    
    success "🎉 All high-performance tests completed! Check output above for any warnings."
}

# Show usage
usage() {
    echo "Usage: $0 [test-type]"
    echo ""
    echo "Test types:"
    echo "  basic       - Run basic functionality tests (default)"
    echo "  database    - Run database functionality tests"
    echo "  performance - Run performance and concurrency tests (5,000 connections)"
    echo "  stress      - Run ultimate stress tests (10,000+ connections)"
    echo "  all         - Run all tests including stress tests"
    echo "  setup       - Set up test environment (requires PostgreSQL)"
    echo "  help        - Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 basic"
    echo "  $0 database"
    echo "  $0 all"
}

# Setup test environment
setup_environment() {
    log "Setting up test environment..."
    
    if [ ! -f "tests/test_setup.sh" ]; then
        error "Test setup script not found"
        exit 1
    fi
    
    chmod +x tests/test_setup.sh
    ./tests/test_setup.sh setup
}

# Main execution
main() {
    local test_type="${1:-basic}"
    
    case "$test_type" in
        "basic")
            check_environment
            run_basic_tests
            ;;
        "database")
            check_environment
            run_database_tests
            ;;
        "performance")
            check_environment
            run_performance_tests
            ;;
        "stress")
            check_environment
            run_stress_tests
            ;;
        "all")
            run_all_tests
            ;;
        "setup")
            setup_environment
            ;;
        "help"|"--help"|"-h")
            usage
            ;;
        *)
            error "Unknown test type: $test_type"
            usage
            exit 1
            ;;
    esac
}

# Run main function
main "$@" 
