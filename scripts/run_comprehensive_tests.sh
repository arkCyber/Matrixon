#!/bin/bash

##
# Comprehensive Testing Suite for matrixon Matrix Server
# 
# Runs all test types: unit, integration, performance, and load testing
# Validates 20,000+ concurrent connection capability
# 
# @author: Matrix Server Performance Team
# @date: 2024-01-01
# @version: 2.0.0
##

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
PURPLE='\033[0;35m'
NC='\033[0m'

# Test configuration
TEST_RESULTS_DIR="test_results/$(date +%Y%m%d_%H%M%S)"
LOG_DIR="$TEST_RESULTS_DIR/logs"
REPORTS_DIR="$TEST_RESULTS_DIR/reports"
CARGO_TARGET_DIR="target"
PERFORMANCE_TARGET_MS=50
SUCCESS_RATE_TARGET=99.0

# Test categories
TEST_CATEGORIES=(
    "unit:cargo test --lib --release"
    "integration:cargo test --test integration_test --release"
    "stress:cargo test --test stress_tests --release"
    "database:cargo test --test database_tests --release"
    "performance:cargo test --release -- test_performance"
    "benchmark:cargo bench"
)

# Function definitions
log_info() {
    mkdir -p "$LOG_DIR" 2>/dev/null || true
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_DIR/comprehensive_test.log"
}

log_success() {
    mkdir -p "$LOG_DIR" 2>/dev/null || true
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_DIR/comprehensive_test.log"
}

log_warning() {
    mkdir -p "$LOG_DIR" 2>/dev/null || true
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_DIR/comprehensive_test.log"
}

log_error() {
    mkdir -p "$LOG_DIR" 2>/dev/null || true
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_DIR/comprehensive_test.log"
}

log_section() {
    mkdir -p "$LOG_DIR" 2>/dev/null || true
    echo -e "${CYAN}[SECTION]${NC} $1" | tee -a "$LOG_DIR/comprehensive_test.log"
}

# Setup test environment
setup_test_environment() {
    log_info "Setting up comprehensive test environment..."
    
    # Create directories
    mkdir -p "$TEST_RESULTS_DIR" "$LOG_DIR" "$REPORTS_DIR"
    
    # Set environment variables for testing
    export RUST_LOG=debug
    export RUST_BACKTRACE=1
    export CARGO_TERM_COLOR=always
    export RUSTFLAGS="-C target-cpu=native -C opt-level=3"
    
    # Increase system limits for testing
    ulimit -n 2097152
    ulimit -u 32768
    
    log_success "Test environment setup completed"
}

# Validate system requirements
validate_system_for_testing() {
    log_info "Validating system requirements for comprehensive testing..."
    
    # Check Rust toolchain
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Cargo not found. Please install Rust toolchain."
        exit 1
    fi
    
    local rust_version=$(rustc --version)
    log_info "Rust version: $rust_version"
    
    # Check available memory (minimum 8GB for full testing)
    local memory_gb=$(free -g | awk 'NR==2{printf "%.0f", $2}')
    if [ "$memory_gb" -lt 8 ]; then
        log_warning "Limited memory: ${memory_gb}GB (recommended: 8GB+)"
    fi
    
    # Check CPU cores
    local cores=$(nproc)
    log_info "CPU cores: $cores"
    
    # Check required tools for testing
    local required_tools=("jq" "curl" "git" "ps" "nc")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            log_warning "Optional tool not found: $tool"
        fi
    done
    
    log_success "System validation completed"
}

# Clean previous build artifacts
clean_build_artifacts() {
    log_info "Cleaning previous build artifacts..."
    
    cargo clean
    rm -rf "$CARGO_TARGET_DIR/criterion" 2>/dev/null || true
    
    log_success "Build artifacts cleaned"
}

# Build optimized binary for testing
build_for_testing() {
    log_info "Building optimized matrixon binary for testing..."
    
    export RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=thin"
    
    if ! cargo build --release --features backend_postgresql; then
        log_error "Build failed"
        exit 1
    fi
    
    # Build test binaries
    if ! cargo test --no-run --release; then
        log_error "Test build failed"
        exit 1
    fi
    
    log_success "Build completed successfully"
}

# Run unit tests
run_unit_tests() {
    log_section "Running Unit Tests"
    local output_file="$REPORTS_DIR/unit_tests.txt"
    local start_time=$(date +%s)
    
    log_info "Executing unit tests..."
    
    if cargo test --lib --release --verbose 2>&1 | tee "$output_file"; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_success "Unit tests completed in ${duration}s"
        
        # Extract test statistics
        local passed=$(grep -c "test result: ok" "$output_file" || echo "0")
        local failed=$(grep -c "FAILED" "$output_file" || echo "0")
        
        echo "Unit Test Results:" >> "$REPORTS_DIR/summary.txt"
        echo "- Passed: $passed" >> "$REPORTS_DIR/summary.txt"
        echo "- Failed: $failed" >> "$REPORTS_DIR/summary.txt"
        echo "- Duration: ${duration}s" >> "$REPORTS_DIR/summary.txt"
        echo "" >> "$REPORTS_DIR/summary.txt"
        
        return 0
    else
        log_error "Unit tests failed"
        return 1
    fi
}

# Run integration tests
run_integration_tests() {
    log_section "Running Integration Tests"
    local output_file="$REPORTS_DIR/integration_tests.txt"
    local start_time=$(date +%s)
    
    log_info "Executing integration tests..."
    
    if cargo test --test integration_test --release --verbose 2>&1 | tee "$output_file"; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_success "Integration tests completed in ${duration}s"
        
        echo "Integration Test Results:" >> "$REPORTS_DIR/summary.txt"
        echo "- Status: PASSED" >> "$REPORTS_DIR/summary.txt"
        echo "- Duration: ${duration}s" >> "$REPORTS_DIR/summary.txt"
        echo "" >> "$REPORTS_DIR/summary.txt"
        
        return 0
    else
        log_error "Integration tests failed"
        return 1
    fi
}

# Run performance benchmarks
run_performance_benchmarks() {
    log_section "Running Performance Benchmarks"
    local output_file="$REPORTS_DIR/performance_benchmarks.txt"
    local start_time=$(date +%s)
    
    log_info "Executing performance benchmarks..."
    
    # Run performance tests
    if cargo test --release -- test_performance 2>&1 | tee "$output_file"; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_success "Performance benchmarks completed in ${duration}s"
        
        # Analyze performance results
        analyze_performance_results "$output_file"
        
        return 0
    else
        log_error "Performance benchmarks failed"
        return 1
    fi
}

# Analyze performance results
analyze_performance_results() {
    local input_file="$1"
    local analysis_file="$REPORTS_DIR/performance_analysis.txt"
    
    log_info "Analyzing performance benchmark results..."
    
    {
        echo "=== Performance Benchmark Analysis ==="
        echo "Timestamp: $(date)"
        echo ""
        
        # Extract timing information
        echo "=== Key Performance Metrics ==="
        grep -E "(should be|was:|took:|completed in)" "$input_file" | head -20
        echo ""
        
        # Check for performance failures
        local failures=$(grep -c "FAILED" "$input_file" || echo "0")
        if [ "$failures" -gt 0 ]; then
            echo "⚠️  Performance test failures detected: $failures"
            grep -A 2 -B 2 "FAILED" "$input_file"
        else
            echo "✅ All performance benchmarks passed"
        fi
        
        echo ""
        echo "=== Performance Summary ==="
        echo "- Target Latency: <${PERFORMANCE_TARGET_MS}ms"
        echo "- Success Rate Target: >${SUCCESS_RATE_TARGET}%"
        echo "- Test Failures: $failures"
        
    } > "$analysis_file"
    
    echo "Performance Benchmark Results:" >> "$REPORTS_DIR/summary.txt"
    echo "- Status: $([ "$failures" -eq 0 ] && echo "PASSED" || echo "FAILED")" >> "$REPORTS_DIR/summary.txt"
    echo "- Failures: $failures" >> "$REPORTS_DIR/summary.txt"
    echo "" >> "$REPORTS_DIR/summary.txt"
    
    log_success "Performance analysis completed"
}

# Run stress tests
run_stress_tests() {
    log_section "Running Stress Tests"
    local output_file="$REPORTS_DIR/stress_tests.txt"
    local start_time=$(date +%s)
    
    log_info "Executing stress tests..."
    
    if cargo test --test stress_tests --release --verbose 2>&1 | tee "$output_file"; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_success "Stress tests completed in ${duration}s"
        
        echo "Stress Test Results:" >> "$REPORTS_DIR/summary.txt"
        echo "- Status: PASSED" >> "$REPORTS_DIR/summary.txt"
        echo "- Duration: ${duration}s" >> "$REPORTS_DIR/summary.txt"
        echo "" >> "$REPORTS_DIR/summary.txt"
        
        return 0
    else
        log_error "Stress tests failed"
        return 1
    fi
}

# Run database tests
run_database_tests() {
    log_section "Running Database Tests"
    local output_file="$REPORTS_DIR/database_tests.txt"
    local start_time=$(date +%s)
    
    log_info "Executing database tests..."
    
    if cargo test --test database_tests --release --verbose 2>&1 | tee "$output_file"; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_success "Database tests completed in ${duration}s"
        
        echo "Database Test Results:" >> "$REPORTS_DIR/summary.txt"
        echo "- Status: PASSED" >> "$REPORTS_DIR/summary.txt"
        echo "- Duration: ${duration}s" >> "$REPORTS_DIR/summary.txt"
        echo "" >> "$REPORTS_DIR/summary.txt"
        
        return 0
    else
        log_error "Database tests failed"
        return 1
    fi
}

# Run cargo benchmark tests
run_cargo_benchmarks() {
    log_section "Running Cargo Benchmarks"
    local output_file="$REPORTS_DIR/cargo_benchmarks.txt"
    local start_time=$(date +%s)
    
    log_info "Executing cargo benchmarks..."
    
    # Check if criterion is available
    if cargo bench --help 2>/dev/null | grep -q "criterion"; then
        if cargo bench 2>&1 | tee "$output_file"; then
            local end_time=$(date +%s)
            local duration=$((end_time - start_time))
            log_success "Cargo benchmarks completed in ${duration}s"
            
            echo "Cargo Benchmark Results:" >> "$REPORTS_DIR/summary.txt"
            echo "- Status: PASSED" >> "$REPORTS_DIR/summary.txt"
            echo "- Duration: ${duration}s" >> "$REPORTS_DIR/summary.txt"
            echo "" >> "$REPORTS_DIR/summary.txt"
            
            return 0
        else
            log_error "Cargo benchmarks failed"
            return 1
        fi
    else
        log_warning "Criterion benchmarking not available, skipping cargo bench"
        echo "Cargo Benchmark Results:" >> "$REPORTS_DIR/summary.txt"
        echo "- Status: SKIPPED (Criterion not available)" >> "$REPORTS_DIR/summary.txt"
        echo "" >> "$REPORTS_DIR/summary.txt"
        return 0
    fi
}

# Run 200k load test
run_200k_load_test() {
    log_section "Running 200k Concurrent Connection Load Test"
    
    if [ -f "scripts/load_test_200k.sh" ]; then
        log_info "Starting 200k load test (this may take 30+ minutes)..."
        
        # Run load test script
        if bash scripts/load_test_200k.sh 2>&1 | tee "$REPORTS_DIR/load_test_200k.txt"; then
            log_success "200k load test completed"
            
            echo "200k Load Test Results:" >> "$REPORTS_DIR/summary.txt"
            echo "- Status: COMPLETED" >> "$REPORTS_DIR/summary.txt"
            echo "- Target: 200,000 concurrent connections" >> "$REPORTS_DIR/summary.txt"
            echo "" >> "$REPORTS_DIR/summary.txt"
        else
            log_error "200k load test failed"
            echo "200k Load Test Results:" >> "$REPORTS_DIR/summary.txt"
            echo "- Status: FAILED" >> "$REPORTS_DIR/summary.txt"
            echo "" >> "$REPORTS_DIR/summary.txt"
        fi
    else
        log_warning "Load test script not found, skipping 200k test"
        echo "200k Load Test Results:" >> "$REPORTS_DIR/summary.txt"
        echo "- Status: SKIPPED (Script not found)" >> "$REPORTS_DIR/summary.txt"
        echo "" >> "$REPORTS_DIR/summary.txt"
    fi
}

# Generate comprehensive test report
generate_test_report() {
    log_info "Generating comprehensive test report..."
    local report_file="$REPORTS_DIR/comprehensive_test_report.html"
    
    cat > "$report_file" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>matrixon Matrix Server - Comprehensive Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background: #f0f0f0; padding: 20px; border-radius: 5px; margin-bottom: 20px; }
        .section { margin: 20px 0; padding: 15px; border: 1px solid #ddd; border-radius: 5px; }
        .success { color: #4CAF50; font-weight: bold; }
        .error { color: #f44336; font-weight: bold; }
        .warning { color: #ff9800; font-weight: bold; }
        .info { color: #2196F3; font-weight: bold; }
        table { border-collapse: collapse; width: 100%; margin: 10px 0; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; }
        .metric { background: #e8f5e8; padding: 10px; border-radius: 5px; margin: 5px 0; }
        .performance-target { background: #fff3cd; padding: 10px; border-radius: 5px; margin: 5px 0; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🚀 matrixon Matrix Server - Comprehensive Test Report</h1>
        <p><strong>Test Date:</strong> $(date)</p>
        <p><strong>Target Performance:</strong> 20,000+ concurrent connections, &lt;50ms latency, &gt;99% success rate</p>
        <p><strong>Test Environment:</strong> $(uname -a)</p>
    </div>
EOF
    
    # Add test results summary
    if [ -f "$REPORTS_DIR/summary.txt" ]; then
        cat >> "$report_file" << EOF
    <div class="section">
        <h2>📊 Test Results Summary</h2>
        <pre>$(cat "$REPORTS_DIR/summary.txt")</pre>
    </div>
EOF
    fi
    
    # Add performance analysis if available
    if [ -f "$REPORTS_DIR/performance_analysis.txt" ]; then
        cat >> "$report_file" << EOF
    <div class="section">
        <h2>⚡ Performance Analysis</h2>
        <pre>$(cat "$REPORTS_DIR/performance_analysis.txt")</pre>
    </div>
EOF
    fi
    
    # Add test file links
    cat >> "$report_file" << 'EOF'
    <div class="section">
        <h2>📁 Detailed Test Results</h2>
        <ul>
            <li><a href="unit_tests.txt">Unit Test Results</a></li>
            <li><a href="integration_tests.txt">Integration Test Results</a></li>
            <li><a href="performance_benchmarks.txt">Performance Benchmark Results</a></li>
            <li><a href="stress_tests.txt">Stress Test Results</a></li>
            <li><a href="database_tests.txt">Database Test Results</a></li>
            <li><a href="cargo_benchmarks.txt">Cargo Benchmark Results</a></li>
            <li><a href="load_test_200k.txt">200k Load Test Results</a></li>
        </ul>
    </div>
    
    <div class="section">
        <h2>🎯 Performance Targets</h2>
        <div class="performance-target">
            <h3>Enterprise-Grade Performance Goals</h3>
            <ul>
                <li><strong>Concurrent Connections:</strong> 20,000+</li>
                <li><strong>Response Latency:</strong> &lt; 50ms average</li>
                <li><strong>Success Rate:</strong> &gt; 99%</li>
                <li><strong>Throughput:</strong> 50,000+ requests/second</li>
                <li><strong>Memory Usage:</strong> Efficient and bounded</li>
                <li><strong>CPU Utilization:</strong> Optimized for multi-core</li>
            </ul>
        </div>
    </div>
    
    <div class="section">
        <h2>🏗️ Test Architecture</h2>
        <p>This comprehensive test suite validates matrixon's capability as a high-performance Matrix NextServer:</p>
        <ul>
            <li><strong>Unit Tests:</strong> Component-level functionality and performance</li>
            <li><strong>Integration Tests:</strong> End-to-end workflow validation</li>
            <li><strong>Performance Benchmarks:</strong> Microsecond-level timing analysis</li>
            <li><strong>Stress Tests:</strong> High-concurrency and edge-case scenarios</li>
            <li><strong>Database Tests:</strong> Data persistence and consistency</li>
            <li><strong>Load Tests:</strong> Real-world 200k connection simulation</li>
        </ul>
    </div>
</body>
</html>
EOF
    
    log_success "Comprehensive test report generated: $report_file"
}

# Monitor system resources during tests
monitor_system_resources() {
    local duration="$1"
    local phase="$2"
    local output_file="$LOG_DIR/system_resources_${phase}.csv"
    
    {
        echo "Timestamp,CPU_Usage,Memory_Usage,Load_Avg,Disk_IO,Network_RX,Network_TX"
        
        for ((i=0; i<duration; i+=10)); do
            timestamp=$(date '+%Y-%m-%d %H:%M:%S')
            cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//' || echo "0")
            memory_usage=$(free | grep Mem | awk '{printf "%.1f", $3/$2 * 100.0}' || echo "0")
            load_avg=$(uptime | awk '{print $10}' | sed 's/,//' || echo "0")
            disk_io=$(iostat 1 1 | tail -1 | awk '{print $4}' || echo "0")
            network_stats=$(cat /proc/net/dev | grep -E "(eth0|en)" | head -1 || echo "0 0 0 0 0 0 0 0 0 0")
            network_rx=$(echo $network_stats | awk '{print $2}' || echo "0")
            network_tx=$(echo $network_stats | awk '{print $10}' || echo "0")
            
            echo "$timestamp,$cpu_usage,$memory_usage,$load_avg,$disk_io,$network_rx,$network_tx"
            sleep 10
        done
    } > "$output_file" &
    
    echo $! > "$LOG_DIR/monitor_${phase}.pid"
}

# Stop system monitoring
stop_system_monitoring() {
    local phase="$1"
    local pid_file="$LOG_DIR/monitor_${phase}.pid"
    
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            log_info "Stopped system monitoring for phase: $phase"
        fi
        rm -f "$pid_file"
    fi
}

# Main test execution function
main() {
    echo -e "${PURPLE}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${PURPLE}║          matrixon MATRIX SERVER COMPREHENSIVE TESTING         ║${NC}"
    echo -e "${PURPLE}║              Enterprise-Grade Performance Validation         ║${NC}"
    echo -e "${PURPLE}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    local start_time=$(date +%s)
    local test_results=()
    
    # Initialize test environment
    setup_test_environment
    validate_system_for_testing
    clean_build_artifacts
    build_for_testing
    
    # Start system monitoring
    monitor_system_resources 7200 "comprehensive_testing"  # 2 hours max
    
    log_info "Starting comprehensive test suite..."
    log_info "Results will be saved to: $TEST_RESULTS_DIR"
    
    # Initialize summary file
    echo "=== matrixon Matrix Server - Test Results Summary ===" > "$REPORTS_DIR/summary.txt"
    echo "Test Date: $(date)" >> "$REPORTS_DIR/summary.txt"
    echo "" >> "$REPORTS_DIR/summary.txt"
    
    # Run test categories
    log_section "Phase 1: Unit and Integration Tests"
    if run_unit_tests; then
        test_results+=("Unit Tests: PASSED")
    else
        test_results+=("Unit Tests: FAILED")
    fi
    
    if run_integration_tests; then
        test_results+=("Integration Tests: PASSED")
    else
        test_results+=("Integration Tests: FAILED")
    fi
    
    log_section "Phase 2: Performance and Benchmark Tests"
    if run_performance_benchmarks; then
        test_results+=("Performance Benchmarks: PASSED")
    else
        test_results+=("Performance Benchmarks: FAILED")
    fi
    
    if run_cargo_benchmarks; then
        test_results+=("Cargo Benchmarks: PASSED")
    else
        test_results+=("Cargo Benchmarks: FAILED")
    fi
    
    log_section "Phase 3: Stress and Database Tests"
    if run_stress_tests; then
        test_results+=("Stress Tests: PASSED")
    else
        test_results+=("Stress Tests: FAILED")
    fi
    
    if run_database_tests; then
        test_results+=("Database Tests: PASSED")
    else
        test_results+=("Database Tests: FAILED")
    fi
    
    log_section "Phase 4: 200k Concurrent Connection Load Test"
    run_200k_load_test
    
    # Stop system monitoring
    stop_system_monitoring "comprehensive_testing"
    
    # Generate final report
    generate_test_report
    
    local end_time=$(date +%s)
    local total_duration=$((end_time - start_time))
    
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                COMPREHENSIVE TESTING COMPLETED!              ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    log_success "Comprehensive testing completed in ${total_duration}s"
    
    # Display results summary
    echo -e "${YELLOW}Test Results Summary:${NC}"
    for result in "${test_results[@]}"; do
        if [[ $result == *"PASSED"* ]]; then
            echo -e "  ${GREEN}✅ $result${NC}"
        else
            echo -e "  ${RED}❌ $result${NC}"
        fi
    done
    
    echo ""
    echo -e "${CYAN}Detailed Results:${NC}"
    echo "- Results Directory: $TEST_RESULTS_DIR"
    echo "- Comprehensive Report: $REPORTS_DIR/comprehensive_test_report.html"
    echo "- Log Files: $LOG_DIR/"
    echo "- Test Duration: ${total_duration}s"
    
    echo ""
    echo -e "${PURPLE}Next Steps:${NC}"
    echo "1. Review detailed test results in the reports directory"
    echo "2. Address any test failures or performance issues"
    echo "3. Run production deployment if all tests pass"
    echo "4. Set up continuous monitoring and alerting"
}

# Handle cleanup on exit
cleanup() {
    log_info "Cleaning up test processes..."
    
    # Stop all monitoring processes
    for pid_file in "$LOG_DIR"/monitor_*.pid; do
        if [ -f "$pid_file" ]; then
            local pid=$(cat "$pid_file")
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid"
            fi
            rm -f "$pid_file"
        fi
    done
    
    # Kill any background test processes
    jobs -p | xargs -r kill 2>/dev/null || true
    
    log_info "Cleanup completed"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Check command line arguments
case "${1:-}" in
    --help|-h)
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Comprehensive testing suite for matrixon Matrix Server"
        echo ""
        echo "Options:"
        echo "  --unit-only       Run only unit tests"
        echo "  --perf-only       Run only performance tests"  
        echo "  --load-only       Run only load tests"
        echo "  --skip-load       Skip 200k load test"
        echo "  --help            Show this help"
        echo ""
        echo "Example:"
        echo "  $0                # Run all tests"
        echo "  $0 --unit-only    # Run only unit tests"
        echo "  $0 --skip-load    # Run all except load test"
        exit 0
        ;;
    --unit-only)
        setup_test_environment
        validate_system_for_testing
        build_for_testing
        run_unit_tests
        exit 0
        ;;
    --perf-only)
        setup_test_environment
        validate_system_for_testing
        build_for_testing
        run_performance_benchmarks
        exit 0
        ;;
    --load-only)
        setup_test_environment
        run_200k_load_test
        exit 0
        ;;
    --skip-load)
        # Set flag to skip load test (implement in main function)
        SKIP_LOAD_TEST=true
        ;;
esac

# Run comprehensive testing
main "$@" 
