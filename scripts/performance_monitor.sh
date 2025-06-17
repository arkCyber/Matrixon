#!/bin/bash

##
# Performance Monitoring Script for matrixon Matrix Server
# 
# Real-time monitoring of server performance, database metrics,
# and system resources during testing and production
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
MONITOR_INTERVAL=5
MAX_SAMPLES=1000
LOG_FILE="performance_monitor.log"
POSTGRES_DB="matrixon_test"
POSTGRES_USER="matrixon"
POSTGRES_HOST="localhost"
POSTGRES_PORT="5432"

# Initialize monitoring data
declare -a CPU_SAMPLES
declare -a MEMORY_SAMPLES
declare -a DB_CONNECTIONS
declare -a DISK_USAGE

log() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo -e "${BLUE}[$timestamp]${NC} $1"
    echo "[$timestamp] $1" >> "$LOG_FILE"
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

# Get system metrics
get_system_metrics() {
    # CPU usage (overall percentage)
    local cpu_usage=$(top -l 1 -n 0 | grep "CPU usage" | awk '{print $3}' | sed 's/%//')
    if [[ -z "$cpu_usage" ]]; then
        # Fallback for Linux systems
        cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//')
    fi
    
    # Memory usage
    local memory_total=$(sysctl hw.memsize 2>/dev/null | awk '{print $2}' || echo "0")
    local memory_used
    if [[ "$memory_total" -gt 0 ]]; then
        # macOS
        local memory_pressure=$(memory_pressure | grep "System-wide memory free percentage" | awk '{print $5}' | sed 's/%//')
        memory_used=$((100 - memory_pressure))
    else
        # Linux fallback
        memory_used=$(free | grep Mem | awk '{printf "%.1f", ($3/$2) * 100.0}')
    fi
    
    # Disk usage for current directory
    local disk_usage=$(df -h . | tail -1 | awk '{print $5}' | sed 's/%//')
    
    echo "${cpu_usage:-0},${memory_used:-0},${disk_usage:-0}"
}

# Get PostgreSQL metrics
get_postgres_metrics() {
    local db_connections=0
    local db_size=0
    local active_queries=0
    local cache_hit_ratio=0
    
    if command -v psql > /dev/null 2>&1; then
        # Number of connections
        db_connections=$(psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -t -c "SELECT count(*) FROM pg_stat_activity;" 2>/dev/null | xargs || echo "0")
        
        # Database size in MB
        db_size=$(psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -t -c "SELECT pg_size_pretty(pg_database_size('$POSTGRES_DB'));" 2>/dev/null | xargs || echo "0 bytes")
        
        # Active queries
        active_queries=$(psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -t -c "SELECT count(*) FROM pg_stat_activity WHERE state = 'active';" 2>/dev/null | xargs || echo "0")
        
        # Cache hit ratio
        cache_hit_ratio=$(psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -t -c "SELECT round((sum(blks_hit) * 100.0 / (sum(blks_hit) + sum(blks_read)))::numeric, 2) FROM pg_stat_database;" 2>/dev/null | xargs || echo "0")
    fi
    
    echo "${db_connections},${db_size},${active_queries},${cache_hit_ratio}"
}

# Get matrixon server metrics (if available)
get_matrixon_metrics() {
    local server_pid=""
    local cpu_usage=0
    local memory_mb=0
    local open_files=0
    
    # Try to find matrixon process
    server_pid=$(pgrep -f "matrixon" 2>/dev/null | head -1 || echo "")
    
    if [[ -n "$server_pid" ]]; then
        # CPU usage for the process
        cpu_usage=$(ps -p $server_pid -o %cpu --no-headers 2>/dev/null | xargs || echo "0")
        
        # Memory usage in MB
        memory_mb=$(ps -p $server_pid -o rss --no-headers 2>/dev/null | awk '{print $1/1024}' || echo "0")
        
        # Open file descriptors
        if [[ -d "/proc/$server_pid/fd" ]]; then
            open_files=$(ls -1 /proc/$server_pid/fd 2>/dev/null | wc -l || echo "0")
        else
            open_files=$(lsof -p $server_pid 2>/dev/null | wc -l || echo "0")
        fi
    fi
    
    echo "${server_pid:-N/A},${cpu_usage},${memory_mb},${open_files}"
}

# Display current metrics
display_metrics() {
    clear
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║                  matrixon PERFORMANCE MONITOR                   ║${NC}"
    echo -e "${CYAN}╠═══════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${CYAN}║${NC} $(date '+%Y-%m-%d %H:%M:%S')                                       ${CYAN}║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    # System metrics
    local system_metrics=$(get_system_metrics)
    IFS=',' read -r cpu memory disk <<< "$system_metrics"
    
    echo -e "${YELLOW}📊 SYSTEM METRICS${NC}"
    echo "┌─────────────────────────────────────────────────────────────┐"
    printf "│ CPU Usage:      %s%-6s%s                                  │\n" "$GREEN" "${cpu}%" "$NC"
    printf "│ Memory Usage:   %s%-6s%s                                  │\n" "$GREEN" "${memory}%" "$NC"
    printf "│ Disk Usage:     %s%-6s%s                                  │\n" "$GREEN" "${disk}%" "$NC"
    echo "└─────────────────────────────────────────────────────────────┘"
    echo ""
    
    # PostgreSQL metrics
    local postgres_metrics=$(get_postgres_metrics)
    IFS=',' read -r db_conn db_size active_q cache_hit <<< "$postgres_metrics"
    
    echo -e "${YELLOW}🗄️  POSTGRESQL METRICS${NC}"
    echo "┌─────────────────────────────────────────────────────────────┐"
    printf "│ Active Connections: %s%-8s%s                           │\n" "$GREEN" "$db_conn" "$NC"
    printf "│ Database Size:      %s%-12s%s                       │\n" "$GREEN" "$db_size" "$NC"
    printf "│ Active Queries:     %s%-8s%s                           │\n" "$GREEN" "$active_q" "$NC"
    printf "│ Cache Hit Ratio:    %s%-6s%%%s                          │\n" "$GREEN" "$cache_hit" "$NC"
    echo "└─────────────────────────────────────────────────────────────┘"
    echo ""
    
    # matrixon server metrics
    local matrixon_metrics=$(get_matrixon_metrics)
    IFS=',' read -r pid cpu_proc mem_proc files <<< "$matrixon_metrics"
    
    echo -e "${YELLOW}🚀 matrixon SERVER METRICS${NC}"
    echo "┌─────────────────────────────────────────────────────────────┐"
    printf "│ Process ID:         %s%-8s%s                           │\n" "$GREEN" "$pid" "$NC"
    printf "│ CPU Usage:          %s%-6s%%%s                          │\n" "$GREEN" "$cpu_proc" "$NC"
    printf "│ Memory Usage:       %s%-8.1f MB%s                      │\n" "$GREEN" "$mem_proc" "$NC"
    printf "│ Open Files:         %s%-8s%s                           │\n" "$GREEN" "$files" "$NC"
    echo "└─────────────────────────────────────────────────────────────┘"
    echo ""
    
    # Performance alerts
    show_alerts "$cpu" "$memory" "$db_conn" "$cache_hit"
    
    echo -e "${CYAN}Press Ctrl+C to stop monitoring${NC}"
}

# Show performance alerts
show_alerts() {
    local cpu=$1
    local memory=$2
    local db_conn=$3
    local cache_hit=$4
    
    echo -e "${YELLOW}⚠️  PERFORMANCE ALERTS${NC}"
    echo "┌─────────────────────────────────────────────────────────────┐"
    
    local has_alerts=false
    
    # CPU alert
    if (( $(echo "$cpu > 80" | bc -l 2>/dev/null || echo "0") )); then
        printf "│ %s⚠️  HIGH CPU USAGE: ${cpu}%%%s                              │\n" "$RED" "$NC"
        has_alerts=true
    fi
    
    # Memory alert
    if (( $(echo "$memory > 85" | bc -l 2>/dev/null || echo "0") )); then
        printf "│ %s⚠️  HIGH MEMORY USAGE: ${memory}%%%s                        │\n" "$RED" "$NC"
        has_alerts=true
    fi
    
    # Database connections alert
    if (( db_conn > 400 )); then
        printf "│ %s⚠️  HIGH DB CONNECTIONS: ${db_conn}%s                       │\n" "$RED" "$NC"
        has_alerts=true
    fi
    
    # Cache hit ratio alert
    if (( $(echo "$cache_hit < 90" | bc -l 2>/dev/null || echo "1") )); then
        printf "│ %s⚠️  LOW CACHE HIT RATIO: ${cache_hit}%%%s                   │\n" "$RED" "$NC"
        has_alerts=true
    fi
    
    if [[ "$has_alerts" == false ]]; then
        printf "│ %s✅ All metrics within normal ranges%s                   │\n" "$GREEN" "$NC"
    fi
    
    echo "└─────────────────────────────────────────────────────────────┘"
    echo ""
}

# Run performance test while monitoring
run_performance_test() {
    local test_type=${1:-"basic"}
    
    log "Starting performance test: $test_type"
    
    # Start monitoring in background
    monitor_loop &
    local monitor_pid=$!
    
    # Run the appropriate test
    case "$test_type" in
        "database")
            log "Running database functionality tests..."
            cargo test --features backend_postgresql database_tests -- --test-threads=1
            ;;
        "concurrent")
            log "Running concurrent performance tests..."
            cargo test --features backend_postgresql test_concurrent_connections -- --test-threads=1
            ;;
        "sustained")
            log "Running sustained load tests..."
            cargo test --features backend_postgresql test_sustained_load -- --test-threads=1
            ;;
        "full")
            log "Running full system performance tests..."
            cargo test --features backend_postgresql test_full_system_performance -- --test-threads=1
            ;;
        "all")
            log "Running all performance tests..."
            cargo test --features backend_postgresql performance_tests -- --test-threads=1
            ;;
        *)
            log "Running basic database tests..."
            cargo test --features backend_postgresql test_database_connection -- --test-threads=1
            ;;
    esac
    
    # Stop monitoring
    kill $monitor_pid 2>/dev/null || true
    
    success "Performance test completed. Check $LOG_FILE for detailed logs."
}

# Continuous monitoring loop
monitor_loop() {
    local sample_count=0
    
    while [[ $sample_count -lt $MAX_SAMPLES ]]; do
        display_metrics
        sleep $MONITOR_INTERVAL
        ((sample_count++))
    done
}

# Generate performance report
generate_report() {
    local report_file="performance_report_$(date +%Y%m%d_%H%M%S).html"
    
    log "Generating performance report: $report_file"
    
    cat > "$report_file" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>matrixon Performance Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background-color: #f0f0f0; padding: 20px; border-radius: 5px; }
        .metric { margin: 10px 0; padding: 10px; border-left: 4px solid #007acc; }
        .alert { background-color: #ffe6e6; border-left-color: #ff0000; }
        .success { background-color: #e6ffe6; border-left-color: #00aa00; }
        .log { background-color: #f5f5f5; padding: 10px; font-family: monospace; white-space: pre-wrap; }
    </style>
</head>
<body>
    <div class="header">
        <h1>matrixon Matrix Server Performance Report</h1>
        <p>Generated: $(date)</p>
        <p>Host: $(hostname)</p>
        <p>Monitoring Period: Last $MAX_SAMPLES samples</p>
    </div>
    
    <h2>System Summary</h2>
    <div class="metric">
        <strong>Operating System:</strong> $(uname -s) $(uname -r)
    </div>
    <div class="metric">
        <strong>CPU Cores:</strong> $(sysctl -n hw.ncpu 2>/dev/null || nproc)
    </div>
    <div class="metric">
        <strong>Total Memory:</strong> $(sysctl -n hw.memsize 2>/dev/null | awk '{print $1/1024/1024/1024 " GB"}' || free -h | grep Mem | awk '{print $2}')
    </div>
    
    <h2>Current Metrics</h2>
EOF

    # Add current metrics to report
    local system_metrics=$(get_system_metrics)
    local postgres_metrics=$(get_postgres_metrics)
    local matrixon_metrics=$(get_matrixon_metrics)
    
    IFS=',' read -r cpu memory disk <<< "$system_metrics"
    IFS=',' read -r db_conn db_size active_q cache_hit <<< "$postgres_metrics"
    IFS=',' read -r pid cpu_proc mem_proc files <<< "$matrixon_metrics"
    
    cat >> "$report_file" << EOF
    <div class="metric">
        <strong>CPU Usage:</strong> ${cpu}%
    </div>
    <div class="metric">
        <strong>Memory Usage:</strong> ${memory}%
    </div>
    <div class="metric">
        <strong>Database Connections:</strong> ${db_conn}
    </div>
    <div class="metric">
        <strong>Cache Hit Ratio:</strong> ${cache_hit}%
    </div>
    
    <h2>Recent Log Entries</h2>
    <div class="log">
$(tail -n 50 "$LOG_FILE" 2>/dev/null || echo "No log entries found")
    </div>
    
</body>
</html>
EOF

    success "Report generated: $report_file"
}

# Show usage
usage() {
    echo "Usage: $0 [command] [options]"
    echo ""
    echo "Commands:"
    echo "  monitor              - Start real-time monitoring (default)"
    echo "  test [type]          - Run performance test with monitoring"
    echo "                        Types: basic, database, concurrent, sustained, full, all"
    echo "  report               - Generate HTML performance report"
    echo "  help                 - Show this help"
    echo ""
    echo "Options:"
    echo "  --interval SECONDS   - Monitoring interval (default: $MONITOR_INTERVAL)"
    echo "  --samples COUNT      - Maximum samples to collect (default: $MAX_SAMPLES)"
    echo "  --log FILE           - Log file path (default: $LOG_FILE)"
    echo ""
    echo "Examples:"
    echo "  $0 monitor --interval 2"
    echo "  $0 test concurrent"
    echo "  $0 test all --log test_results.log"
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --interval)
                MONITOR_INTERVAL="$2"
                shift 2
                ;;
            --samples)
                MAX_SAMPLES="$2"
                shift 2
                ;;
            --log)
                LOG_FILE="$2"
                shift 2
                ;;
            *)
                break
                ;;
        esac
    done
}

# Signal handler for clean exit
cleanup() {
    echo ""
    log "Monitoring stopped by user"
    exit 0
}

# Main execution
main() {
    # Parse arguments first
    parse_args "$@"
    
    # Get remaining arguments
    local cmd="${1:-monitor}"
    local test_type="${2:-basic}"
    
    # Set up signal handlers
    trap cleanup SIGINT SIGTERM
    
    # Initialize log file
    echo "# matrixon Performance Monitor Log - $(date)" > "$LOG_FILE"
    
    case "$cmd" in
        "monitor")
            log "Starting performance monitoring..."
            log "Interval: ${MONITOR_INTERVAL}s, Max samples: $MAX_SAMPLES"
            monitor_loop
            ;;
        "test")
            run_performance_test "$test_type"
            ;;
        "report")
            generate_report
            ;;
        "help"|"--help"|"-h")
            usage
            ;;
        *)
            error "Unknown command: $cmd"
            usage
            exit 1
            ;;
    esac
}

# Check dependencies
check_dependencies() {
    for cmd in bc; do
        if ! command -v $cmd > /dev/null 2>&1; then
            warning "Command '$cmd' not found. Some features may not work correctly."
        fi
    done
}

# Initialize
check_dependencies
main "$@" 
