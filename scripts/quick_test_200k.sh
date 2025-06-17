#!/bin/bash

##
# Quick 200k Infrastructure Test for matrixon Matrix Server
# 
# Validates 200k infrastructure setup without full load testing
# Tests configurations, connections, and basic performance
# 
# @author: Matrix Server Performance Team
# @date: 2024-01-01
# @version: 1.0.0
##

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
RESULTS_DIR="quick_test_results/$(date +%Y%m%d_%H%M%S)"
LOG_FILE="$RESULTS_DIR/quick_test.log"

# Function definitions
log_info() {
    mkdir -p "$(dirname "$LOG_FILE")" 2>/dev/null || true
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_FILE"
}

log_success() {
    mkdir -p "$(dirname "$LOG_FILE")" 2>/dev/null || true
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"
}

log_warning() {
    mkdir -p "$(dirname "$LOG_FILE")" 2>/dev/null || true
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

log_error() {
    mkdir -p "$(dirname "$LOG_FILE")" 2>/dev/null || true
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

# Setup test environment
setup_environment() {
    log_info "Setting up quick test environment..."
    mkdir -p "$RESULTS_DIR"
    log_success "Environment setup completed"
}

# Test 1: Validate configuration files
test_configuration_files() {
    log_info "Testing configuration files..."
    local errors=0
    
    # Check nginx configuration
    if [ -f "configs/nginx-load-balancer.conf" ]; then
        if nginx -t -c "$(pwd)/configs/nginx-load-balancer.conf" 2>/dev/null; then
            log_success "Nginx configuration is valid"
        else
            log_error "Nginx configuration has errors"
            ((errors++))
        fi
    else
        log_warning "Nginx configuration not found"
    fi
    
    # Check Redis configuration
    if [ -f "configs/redis-cluster.conf" ]; then
        log_success "Redis cluster configuration found"
    else
        log_warning "Redis cluster configuration not found"
    fi
    
    # Check Docker Compose
    if [ -f "docker-compose.200k.yml" ]; then
        if docker-compose -f docker-compose.200k.yml config >/dev/null 2>&1; then
            log_success "Docker Compose configuration is valid"
        else
            log_error "Docker Compose configuration has errors"
            ((errors++))
        fi
    else
        log_warning "Docker Compose 200k configuration not found"
    fi
    
    return $errors
}

# Test 2: Build and compile tests
test_build() {
    log_info "Testing build process..."
    
    # Clean build
    cargo clean >/dev/null 2>&1
    
    # Test build
    if cargo check --release --features backend_postgresql 2>/dev/null; then
        log_success "Build check passed"
        return 0
    else
        log_error "Build check failed"
        return 1
    fi
}

# Test 3: Run unit tests (quick subset)
test_unit_tests() {
    log_info "Running quick unit tests..."
    
    # Run only performance-related tests
    if cargo test --lib --release -- test_performance 2>/dev/null; then
        log_success "Performance unit tests passed"
        return 0
    else
        log_error "Performance unit tests failed"
        return 1
    fi
}

# Test 4: System requirements validation
test_system_requirements() {
    log_info "Validating system requirements for 200k connections..."
    local warnings=0
    
    # Check memory
    local memory_gb=$(free -g | awk 'NR==2{printf "%.0f", $2}')
    if [ "$memory_gb" -ge 64 ]; then
        log_success "Memory: ${memory_gb}GB (sufficient for 200k)"
    else
        log_warning "Memory: ${memory_gb}GB (recommended: 64GB+ for 200k)"
        ((warnings++))
    fi
    
    # Check CPU cores
    local cores=$(nproc)
    if [ "$cores" -ge 16 ]; then
        log_success "CPU cores: $cores (sufficient for 200k)"
    else
        log_warning "CPU cores: $cores (recommended: 16+ for 200k)"
        ((warnings++))
    fi
    
    # Check file descriptor limits
    local ulimit_files=$(ulimit -n)
    if [ "$ulimit_files" -ge 1048576 ]; then
        log_success "File descriptor limit: $ulimit_files (sufficient)"
    else
        log_warning "File descriptor limit: $ulimit_files (recommended: 2097152)"
        ((warnings++))
    fi
    
    # Check network settings
    if [ -f "/proc/sys/net/core/somaxconn" ]; then
        local somaxconn=$(cat /proc/sys/net/core/somaxconn)
        if [ "$somaxconn" -ge 65536 ]; then
            log_success "Network queue: $somaxconn (sufficient)"
        else
            log_warning "Network queue: $somaxconn (recommended: 131072)"
            ((warnings++))
        fi
    fi
    
    return $warnings
}

# Test 5: Port availability for multi-instance
test_port_availability() {
    log_info "Testing port availability for multi-instance deployment..."
    local errors=0
    
    # Test ports 8001-8008 for matrixon instances
    for port in {8001..8008}; do
        if nc -z localhost "$port" 2>/dev/null; then
            log_warning "Port $port is already in use"
            ((errors++))
        else
            log_info "Port $port is available"
        fi
    done
    
    # Test load balancer ports
    for port in 80 443; do
        if nc -z localhost "$port" 2>/dev/null; then
            log_warning "Load balancer port $port is already in use"
            ((errors++))
        else
            log_info "Load balancer port $port is available"
        fi
    done
    
    return $errors
}

# Test 6: Database connectivity
test_database_connectivity() {
    log_info "Testing database connectivity..."
    
    # Test PostgreSQL
    if command -v psql >/dev/null 2>&1; then
        if psql -h localhost -U postgres -c "SELECT 1;" >/dev/null 2>&1; then
            log_success "PostgreSQL connection successful"
        else
            log_warning "PostgreSQL connection failed (may need setup)"
        fi
    else
        log_warning "PostgreSQL client not found"
    fi
    
    # Test Redis
    if command -v redis-cli >/dev/null 2>&1; then
        if redis-cli ping >/dev/null 2>&1; then
            log_success "Redis connection successful"
        else
            log_warning "Redis connection failed (may need setup)"
        fi
    else
        log_warning "Redis client not found"
    fi
}

# Test 7: Docker environment
test_docker_environment() {
    log_info "Testing Docker environment..."
    
    if command -v docker >/dev/null 2>&1; then
        if docker --version >/dev/null 2>&1; then
            log_success "Docker is available"
            
            # Test Docker Compose
            if command -v docker-compose >/dev/null 2>&1; then
                log_success "Docker Compose is available"
                return 0
            else
                log_warning "Docker Compose not found"
                return 1
            fi
        else
            log_error "Docker is not working properly"
            return 1
        fi
    else
        log_warning "Docker not found"
        return 1
    fi
}

# Test 8: Basic performance validation
test_basic_performance() {
    log_info "Running basic performance validation..."
    
    # Build test binary
    if ! cargo build --release --features backend_postgresql >/dev/null 2>&1; then
        log_error "Failed to build for performance test"
        return 1
    fi
    
    # Test startup time
    local start_time=$(date +%s%N)
    timeout 30s ./target/release/matrixon --help >/dev/null 2>&1 || true
    local end_time=$(date +%s%N)
    local startup_ms=$(( (end_time - start_time) / 1000000 ))
    
    if [ "$startup_ms" -lt 5000 ]; then
        log_success "Binary startup time: ${startup_ms}ms (good)"
    else
        log_warning "Binary startup time: ${startup_ms}ms (may be slow)"
    fi
    
    return 0
}

# Generate quick test report
generate_report() {
    local report_file="$RESULTS_DIR/quick_test_report.html"
    
    log_info "Generating quick test report..."
    
    cat > "$report_file" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>matrixon 200k Quick Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background: #f0f0f0; padding: 20px; border-radius: 5px; margin-bottom: 20px; }
        .section { margin: 10px 0; padding: 10px; border: 1px solid #ddd; border-radius: 5px; }
        .success { color: #4CAF50; font-weight: bold; }
        .error { color: #f44336; font-weight: bold; }
        .warning { color: #ff9800; font-weight: bold; }
        .info { color: #2196F3; font-weight: bold; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🚀 matrixon 200k Infrastructure Quick Test</h1>
        <p><strong>Test Date:</strong> $(date)</p>
        <p><strong>Purpose:</strong> Validate 200k concurrent connection infrastructure</p>
    </div>
    
    <div class="section">
        <h2>📋 Test Results</h2>
        <pre>$(cat "$LOG_FILE")</pre>
    </div>
    
    <div class="section">
        <h2>🎯 Next Steps</h2>
        <p>If all tests pass with minimal warnings:</p>
        <ul>
            <li>Run comprehensive tests: <code>./scripts/run_comprehensive_tests.sh</code></li>
            <li>Deploy multi-instance: <code>./scripts/deploy_multi_instance.sh</code></li>
            <li>Run 200k load test: <code>./scripts/load_test_200k.sh</code></li>
        </ul>
    </div>
</body>
</html>
EOF
    
    log_success "Quick test report generated: $report_file"
}

# Main execution
main() {
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║        matrixon 200K INFRASTRUCTURE QUICK TEST               ║${NC}"
    echo -e "${CYAN}║            Validating 200k Connection Readiness             ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    local start_time=$(date +%s)
    local total_errors=0
    local total_warnings=0
    
    setup_environment
    
    # Run all tests
    echo "Running infrastructure validation tests..."
    echo ""
    
    test_configuration_files; total_errors=$((total_errors + $?))
    test_build; total_errors=$((total_errors + $?))
    test_unit_tests; total_errors=$((total_errors + $?))
    test_system_requirements; total_warnings=$((total_warnings + $?))
    test_port_availability; total_warnings=$((total_warnings + $?))
    test_database_connectivity
    test_docker_environment
    test_basic_performance
    
    generate_report
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                QUICK TEST COMPLETED!                         ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    log_success "Quick test completed in ${duration}s"
    
    # Display summary
    echo -e "${YELLOW}Results Summary:${NC}"
    echo "- Errors: $total_errors"
    echo "- Warnings: $total_warnings"
    echo "- Duration: ${duration}s"
    echo "- Report: $RESULTS_DIR/quick_test_report.html"
    
    if [ "$total_errors" -eq 0 ]; then
        echo ""
        echo -e "${GREEN}✅ Infrastructure is ready for 200k testing!${NC}"
        echo ""
        echo -e "${CYAN}Recommended next steps:${NC}"
        echo "1. Run comprehensive tests: ./scripts/run_comprehensive_tests.sh --unit-only"
        echo "2. Deploy multi-instance setup: ./scripts/deploy_multi_instance.sh"
        echo "3. Run full load test: ./scripts/load_test_200k.sh"
    else
        echo ""
        echo -e "${RED}❌ Please fix $total_errors error(s) before proceeding${NC}"
    fi
    
    exit $total_errors
}

# Run quick test
main "$@" 
