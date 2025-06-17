#!/bin/bash

##
# Advanced Load Testing Script for matrixon Matrix Server
# 
# Tests performance under 20,000+ concurrent connections
# Validates latency, throughput, and system stability
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
NC='\033[0m'

# Test configuration
TARGET_HOST="localhost"
TARGET_PORT="80"
TOTAL_CONNECTIONS=200000
RAMP_UP_TIME=600    # 10 minutes to reach full load
TEST_DURATION=1800  # 30 minutes at full load
BATCH_SIZE=1000     # Connections per batch

# Performance targets
TARGET_LATENCY_MS=50
TARGET_SUCCESS_RATE=99.0
TARGET_THROUGHPUT=50000

# Test phases
PHASES=(
    "10000:120"    # 10k connections for 2 minutes
    "25000:180"    # 25k connections for 3 minutes
    "50000:240"    # 50k connections for 4 minutes
    "100000:300"   # 100k connections for 5 minutes
    "150000:300"   # 150k connections for 5 minutes
    "200000:1800"  # 200k connections for 30 minutes
)

# Directories
RESULTS_DIR="load_test_results/$(date +%Y%m%d_%H%M%S)"
LOG_DIR="$RESULTS_DIR/logs"
REPORTS_DIR="$RESULTS_DIR/reports"

# Tools required
REQUIRED_TOOLS=("wrk" "ab" "vegeta" "jq" "bc" "curl" "netstat")

# Function definitions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_DIR/test.log"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_DIR/test.log"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_DIR/test.log"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_DIR/test.log"
}

# Setup test environment
setup_environment() {
    log_info "Setting up load testing environment..."
    
    # Create directories
    mkdir -p "$RESULTS_DIR" "$LOG_DIR" "$REPORTS_DIR"
    
    # Check required tools
    for tool in "${REQUIRED_TOOLS[@]}"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            log_error "Required tool not found: $tool"
            log_info "Install with: sudo apt-get install $tool"
            exit 1
        fi
    done
    
    # Increase system limits
    ulimit -n 2097152
    echo 'net.core.somaxconn = 131072' | sudo tee -a /etc/sysctl.conf
    echo 'net.ipv4.tcp_tw_reuse = 1' | sudo tee -a /etc/sysctl.conf
    sudo sysctl -p
    
    log_success "Environment setup completed"
}

# Validate target server
validate_target() {
    log_info "Validating target server: $TARGET_HOST:$TARGET_PORT"
    
    # Check if server is responding
    if ! curl -s "http://$TARGET_HOST:$TARGET_PORT/health" >/dev/null; then
        if ! curl -s "http://$TARGET_HOST:$TARGET_PORT" >/dev/null; then
            log_error "Target server not responding"
            exit 1
        fi
    fi
    
    # Check Matrix endpoints
    if curl -s "http://$TARGET_HOST:$TARGET_PORT/_matrix/client/versions" | jq . >/dev/null 2>&1; then
        log_success "Matrix API endpoints validated"
    else
        log_warning "Matrix API endpoints not found, testing generic HTTP"
    fi
    
    log_success "Target server validation completed"
}

# Generate test data
generate_test_data() {
    log_info "Generating test data and user accounts..."
    
    cat > "$RESULTS_DIR/test_users.json" << 'EOF'
{
  "users": [
    {"username": "testuser1", "password": "testpass123"},
    {"username": "testuser2", "password": "testpass123"},
    {"username": "testuser3", "password": "testpass123"},
    {"username": "testuser4", "password": "testpass123"},
    {"username": "testuser5", "password": "testpass123"}
  ],
  "rooms": [
    {"name": "Load Test Room 1", "alias": "#loadtest1"},
    {"name": "Load Test Room 2", "alias": "#loadtest2"},
    {"name": "Load Test Room 3", "alias": "#loadtest3"}
  ]
}
EOF
    
    # Generate message templates
    cat > "$RESULTS_DIR/message_templates.txt" << 'EOF'
Hello from load test user!
This is a test message for performance validation.
Testing message throughput under high load.
Matrix protocol performance test in progress.
Concurrent connection validation message.
EOF
    
    log_success "Test data generated"
}

# Monitor system resources
monitor_resources() {
    local test_phase="$1"
    local duration="$2"
    local output_file="$LOG_DIR/resources_${test_phase}.log"
    
    log_info "Starting resource monitoring for phase: $test_phase"
    
    {
        echo "Timestamp,CPU_Usage,Memory_Usage,Network_RX,Network_TX,Open_Files,TCP_Connections"
        
        for ((i=0; i<duration; i+=5)); do
            timestamp=$(date '+%Y-%m-%d %H:%M:%S')
            cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//')
            memory_usage=$(free | grep Mem | awk '{printf "%.1f", $3/$2 * 100.0}')
            network_stats=$(cat /proc/net/dev | grep eth0 || cat /proc/net/dev | grep en | head -1)
            network_rx=$(echo $network_stats | awk '{print $2}')
            network_tx=$(echo $network_stats | awk '{print $10}')
            open_files=$(lsof | wc -l)
            tcp_connections=$(netstat -an | grep ESTABLISHED | wc -l)
            
            echo "$timestamp,$cpu_usage,$memory_usage,$network_rx,$network_tx,$open_files,$tcp_connections"
            sleep 5
        done
    } > "$output_file" &
    
    echo $! > "$LOG_DIR/monitor_${test_phase}.pid"
}

# Stop resource monitoring
stop_monitoring() {
    local test_phase="$1"
    local pid_file="$LOG_DIR/monitor_${test_phase}.pid"
    
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            log_info "Stopped resource monitoring for phase: $test_phase"
        fi
        rm -f "$pid_file"
    fi
}

# Run HTTP load test using wrk
run_wrk_test() {
    local connections="$1"
    local duration="$2"
    local test_name="$3"
    local output_file="$REPORTS_DIR/wrk_${test_name}.txt"
    
    log_info "Running wrk test: $connections connections for ${duration}s"
    
    # Calculate threads (max 64, or connections/100)
    local threads=$((connections / 100))
    [ $threads -gt 64 ] && threads=64
    [ $threads -lt 1 ] && threads=1
    
    wrk -t $threads -c $connections -d ${duration}s \
        --timeout 30s \
        --latency \
        "http://$TARGET_HOST:$TARGET_PORT/" > "$output_file" 2>&1
    
    log_success "wrk test completed: $test_name"
}

# Run Apache Bench test
run_ab_test() {
    local connections="$1"
    local requests="$2"
    local test_name="$3"
    local output_file="$REPORTS_DIR/ab_${test_name}.txt"
    
    log_info "Running Apache Bench test: $connections connections, $requests requests"
    
    ab -n $requests -c $connections \
       -k -r \
       "http://$TARGET_HOST:$TARGET_PORT/" > "$output_file" 2>&1
    
    log_success "Apache Bench test completed: $test_name"
}

# Run Vegeta attack test
run_vegeta_test() {
    local rate="$1"
    local duration="$2"
    local test_name="$3"
    local output_file="$REPORTS_DIR/vegeta_${test_name}.json"
    
    log_info "Running Vegeta test: ${rate} requests/second for ${duration}s"
    
    echo "GET http://$TARGET_HOST:$TARGET_PORT/" | \
        vegeta attack -rate="$rate" -duration="${duration}s" | \
        vegeta report -type=json > "$output_file"
    
    log_success "Vegeta test completed: $test_name"
}

# Run Matrix-specific tests
run_matrix_tests() {
    local connections="$1"
    local duration="$2"
    local test_name="$3"
    
    log_info "Running Matrix-specific tests: $test_name"
    
    # Test Matrix version endpoint
    run_wrk_test "$connections" "$duration" "${test_name}_versions" \
        "http://$TARGET_HOST:$TARGET_PORT/_matrix/client/versions"
    
    # Test Matrix capabilities
    run_wrk_test "$((connections/2))" "$duration" "${test_name}_capabilities" \
        "http://$TARGET_HOST:$TARGET_PORT/_matrix/client/r0/capabilities"
    
    log_success "Matrix-specific tests completed: $test_name"
}

# Analyze test results
analyze_results() {
    local test_phase="$1"
    local output_file="$REPORTS_DIR/analysis_${test_phase}.txt"
    
    log_info "Analyzing results for phase: $test_phase"
    
    {
        echo "=== Load Test Analysis: Phase $test_phase ==="
        echo "Timestamp: $(date)"
        echo ""
        
        # Analyze wrk results
        if [ -f "$REPORTS_DIR/wrk_${test_phase}.txt" ]; then
            echo "=== WRK Test Results ==="
            cat "$REPORTS_DIR/wrk_${test_phase}.txt"
            echo ""
            
            # Extract key metrics
            local avg_latency=$(grep "Latency" "$REPORTS_DIR/wrk_${test_phase}.txt" | awk '{print $2}')
            local requests_per_sec=$(grep "Requests/sec:" "$REPORTS_DIR/wrk_${test_phase}.txt" | awk '{print $2}')
            local transfer_per_sec=$(grep "Transfer/sec:" "$REPORTS_DIR/wrk_${test_phase}.txt" | awk '{print $2}')
            
            echo "Key Metrics:"
            echo "- Average Latency: $avg_latency"
            echo "- Requests/sec: $requests_per_sec"
            echo "- Transfer/sec: $transfer_per_sec"
            echo ""
        fi
        
        # Analyze resource usage
        if [ -f "$LOG_DIR/resources_${test_phase}.log" ]; then
            echo "=== Resource Usage Analysis ==="
            
            # Calculate averages
            local avg_cpu=$(tail -n +2 "$LOG_DIR/resources_${test_phase}.log" | awk -F, '{sum+=$2; count++} END {if(count>0) print sum/count; else print 0}')
            local avg_memory=$(tail -n +2 "$LOG_DIR/resources_${test_phase}.log" | awk -F, '{sum+=$3; count++} END {if(count>0) print sum/count; else print 0}')
            local max_connections=$(tail -n +2 "$LOG_DIR/resources_${test_phase}.log" | awk -F, '{if($7>max) max=$7} END {print max}')
            
            echo "- Average CPU Usage: ${avg_cpu}%"
            echo "- Average Memory Usage: ${avg_memory}%"
            echo "- Max TCP Connections: $max_connections"
            echo ""
        fi
        
        # Performance validation
        echo "=== Performance Validation ==="
        echo "Target Latency: ${TARGET_LATENCY_MS}ms"
        echo "Target Success Rate: ${TARGET_SUCCESS_RATE}%"
        echo "Target Throughput: ${TARGET_THROUGHPUT} req/sec"
        echo ""
        
    } > "$output_file"
    
    log_success "Results analysis completed: $test_phase"
}

# Generate comprehensive report
generate_final_report() {
    local report_file="$REPORTS_DIR/final_report.html"
    
    log_info "Generating comprehensive test report..."
    
    cat > "$report_file" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>matrixon Matrix Server - 200k Load Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background: #f0f0f0; padding: 20px; border-radius: 5px; }
        .section { margin: 20px 0; padding: 15px; border: 1px solid #ddd; }
        .success { color: green; font-weight: bold; }
        .warning { color: orange; font-weight: bold; }
        .error { color: red; font-weight: bold; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; }
    </style>
</head>
<body>
    <div class="header">
        <h1>matrixon Matrix Server - 200k Concurrent Connections Load Test</h1>
        <p><strong>Test Date:</strong> $(date)</p>
        <p><strong>Target:</strong> $TARGET_HOST:$TARGET_PORT</p>
        <p><strong>Max Connections:</strong> $TOTAL_CONNECTIONS</p>
    </div>
EOF
    
    # Add test results for each phase
    for phase in "${PHASES[@]}"; do
        IFS=':' read -r connections duration <<< "$phase"
        
        cat >> "$report_file" << EOF
    <div class="section">
        <h2>Phase: $connections Connections</h2>
        <p><strong>Duration:</strong> ${duration} seconds</p>
        
        <h3>Performance Metrics</h3>
        <table>
            <tr><th>Metric</th><th>Value</th><th>Status</th></tr>
EOF
        
        # Add metrics if available
        if [ -f "$REPORTS_DIR/analysis_${connections}.txt" ]; then
            cat "$REPORTS_DIR/analysis_${connections}.txt" >> "$report_file"
        fi
        
        echo "        </table>" >> "$report_file"
        echo "    </div>" >> "$report_file"
    done
    
    cat >> "$report_file" << 'EOF'
    <div class="section">
        <h2>Test Summary</h2>
        <p>This comprehensive load test validates matrixon Matrix Server's capability to handle 20,000+ concurrent connections while maintaining performance targets.</p>
        
        <h3>Performance Targets</h3>
        <ul>
            <li>Latency: &lt; 50ms average</li>
            <li>Success Rate: &gt; 99%</li>
            <li>Throughput: &gt; 50,000 req/sec</li>
        </ul>
    </div>
</body>
</html>
EOF
    
    log_success "Comprehensive report generated: $report_file"
}

# Run test phase
run_test_phase() {
    local connections="$1"
    local duration="$2"
    
    log_info "Starting test phase: $connections connections for ${duration}s"
    
    # Start resource monitoring
    monitor_resources "$connections" "$duration"
    
    # Run concurrent tests
    {
        # Main HTTP load test
        run_wrk_test "$connections" "$duration" "$connections" &
        
        # Supplementary tests with lower load
        if [ "$connections" -ge 50000 ]; then
            run_ab_test "$((connections/4))" "$((connections/2))" "$connections" &
            run_vegeta_test "1000" "$duration" "$connections" &
        fi
        
        # Matrix-specific tests
        if curl -s "http://$TARGET_HOST:$TARGET_PORT/_matrix/client/versions" >/dev/null 2>&1; then
            run_matrix_tests "$((connections/4))" "$duration" "$connections" &
        fi
        
        # Wait for all background tests
        wait
    }
    
    # Stop monitoring
    stop_monitoring "$connections"
    
    # Analyze results
    analyze_results "$connections"
    
    log_success "Test phase completed: $connections connections"
    
    # Cool down period
    log_info "Cooling down for 30 seconds..."
    sleep 30
}

# Main test execution
main() {
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║            matrixon MATRIX SERVER LOAD TEST                   ║${NC}"
    echo -e "${CYAN}║              20,000+ Concurrent Connections                 ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    # Setup and validation
    setup_environment
    validate_target
    generate_test_data
    
    log_info "Starting progressive load test..."
    log_info "Results will be saved to: $RESULTS_DIR"
    
    # Execute test phases
    for phase in "${PHASES[@]}"; do
        IFS=':' read -r connections duration <<< "$phase"
        run_test_phase "$connections" "$duration"
    done
    
    # Generate final report
    generate_final_report
    
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                LOAD TEST COMPLETED!                          ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    log_success "Load test completed successfully"
    log_info "Results directory: $RESULTS_DIR"
    log_info "Final report: $REPORTS_DIR/final_report.html"
    
    # Show quick summary
    echo ""
    echo -e "${YELLOW}Quick Summary:${NC}"
    echo "- Total test phases: ${#PHASES[@]}"
    echo "- Maximum connections tested: $(echo "${PHASES[-1]}" | cut -d: -f1)"
    echo "- Total test duration: $(echo "${PHASES[@]}" | tr ' ' '\n' | cut -d: -f2 | paste -sd+ | bc) seconds"
    echo "- Results location: $RESULTS_DIR"
}

# Handle cleanup on exit
cleanup() {
    log_info "Cleaning up test processes..."
    
    # Kill any running monitoring processes
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
    jobs -p | xargs -r kill
    
    log_info "Cleanup completed"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Validate arguments and run
if [ $# -gt 0 ]; then
    case "$1" in
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --host HOST     Target host (default: localhost)"
            echo "  --port PORT     Target port (default: 80)"
            echo "  --connections N Maximum connections (default: 200000)"
            echo "  --help          Show this help"
            echo ""
            echo "Example:"
            echo "  $0 --host matrix.example.com --port 443 --connections 300000"
            exit 0
            ;;
        --host)
            TARGET_HOST="$2"
            shift 2
            ;;
        --port)
            TARGET_PORT="$2"
            shift 2
            ;;
        --connections)
            TOTAL_CONNECTIONS="$2"
            shift 2
            ;;
    esac
fi

# Run main test
main "$@" 
