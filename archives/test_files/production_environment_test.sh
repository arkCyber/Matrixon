#!/bin/bash

# =============================================================================
# Matrixon Production Environment - Comprehensive Docker Services Test Suite
# =============================================================================
#
# Project: Matrixon - Ultra High Performance Matrix NextServer
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 0.11.0-alpha
#
# Description:
#   Complete production environment testing suite for all Docker services
#   Tests infrastructure, APIs, monitoring, and performance endpoints
#
# Usage:
#   ./production_environment_test.sh [options]
#   
# Options:
#   --quick     Run only basic health checks
#   --full      Run comprehensive tests including load testing
#   --services  Test only infrastructure services
#   --api       Test only Matrix API endpoints
#   --monitor   Test only monitoring services
#
# =============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
LOG_FILE="$SCRIPT_DIR/logs/production_test_$TIMESTAMP.log"
RESULTS_FILE="$SCRIPT_DIR/logs/test_results_$TIMESTAMP.json"

# Service endpoints
MATRIXON_URL="http://localhost:6167"
NGINX_URL="http://localhost:80"
NGINX_HTTPS_URL="https://localhost:443"
POSTGRES_HOST="localhost"
POSTGRES_PORT="5432"
REDIS_URL="redis://localhost:6379"
IPFS_API_URL="http://localhost:5001"
IPFS_GATEWAY_URL="http://localhost:8080"
PROMETHEUS_URL="http://localhost:9090"
GRAFANA_URL="http://localhost:3001"

# Test configuration
TEST_USER="production_test_user_$(date +%s)"
TEST_PASS="ProductionTest123!"
TEST_TIMEOUT=30
MAX_RETRIES=3

# Statistics
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Initialize logging
mkdir -p "$SCRIPT_DIR/logs"

# Function to log messages
log() {
    local level=$1
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

# Function to print styled headers
print_header() {
    echo -e "\n${BOLD}${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BOLD}${BLUE}$1${NC}"
    echo -e "${BOLD}${BLUE}═══════════════════════════════════════════════════════════════${NC}\n"
    log "INFO" "Starting test section: $1"
}

# Function to print test results
print_result() {
    local status=$1
    local description=$2
    local details=$3
    
    case $status in
        "PASS")
            echo -e "${GREEN}✅ PASS${NC}: $description"
            [ -n "$details" ] && echo -e "   ${CYAN}$details${NC}"
            ((PASSED_TESTS++))
            ;;
        "FAIL")
            echo -e "${RED}❌ FAIL${NC}: $description"
            [ -n "$details" ] && echo -e "   ${RED}$details${NC}"
            ((FAILED_TESTS++))
            ;;
        "SKIP")
            echo -e "${YELLOW}⏭️  SKIP${NC}: $description"
            [ -n "$details" ] && echo -e "   ${YELLOW}$details${NC}"
            ((SKIPPED_TESTS++))
            ;;
        "INFO")
            echo -e "${CYAN}ℹ️  INFO${NC}: $description"
            [ -n "$details" ] && echo -e "   $details"
            ;;
    esac
    ((TOTAL_TESTS++))
    log "$status" "$description - $details"
}

# Function to test HTTP endpoint with retry
test_http_endpoint() {
    local method=$1
    local url=$2
    local description=$3
    local expected_status=${4:-200}
    local data=$5
    local headers=$6
    local retry_count=0
    
    while [ $retry_count -lt $MAX_RETRIES ]; do
        local curl_cmd="curl -s -m $TEST_TIMEOUT -w 'HTTPSTATUS:%{http_code}'"
        
        if [ -n "$headers" ]; then
            curl_cmd="$curl_cmd $headers"
        fi
        
        if [ -n "$data" ]; then
            curl_cmd="$curl_cmd -H 'Content-Type: application/json' -d '$data'"
        fi
        
        curl_cmd="$curl_cmd -X $method '$url'"
        
        local response=$(eval $curl_cmd 2>/dev/null || echo "HTTPSTATUS:000")
        local http_status=$(echo "$response" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')
        local body=$(echo "$response" | sed -e 's/HTTPSTATUS:.*//')
        
        if [ "$http_status" == "000" ]; then
            ((retry_count++))
            if [ $retry_count -lt $MAX_RETRIES ]; then
                sleep 2
                continue
            else
                print_result "FAIL" "$description" "Connection failed after $MAX_RETRIES retries"
                return 1
            fi
        elif [ "$http_status" == "$expected_status" ]; then
            print_result "PASS" "$description" "HTTP $http_status"
            return 0
        else
            print_result "FAIL" "$description" "Expected HTTP $expected_status, got $http_status"
            return 1
        fi
    done
}

# Function to test TCP port connectivity
test_tcp_port() {
    local host=$1
    local port=$2
    local description=$3
    
    if nc -z -w5 "$host" "$port" 2>/dev/null; then
        print_result "PASS" "$description" "Port $port is open"
        return 0
    else
        print_result "FAIL" "$description" "Port $port is not accessible"
        return 1
    fi
}

# Function to test Redis connectivity
test_redis() {
    local description=$1
    
    if command -v redis-cli >/dev/null 2>&1; then
        local result=$(redis-cli -h localhost -p 6379 ping 2>/dev/null || echo "ERROR")
        if [ "$result" == "PONG" ]; then
            print_result "PASS" "$description" "Redis responding to PING"
            return 0
        else
            print_result "FAIL" "$description" "Redis not responding"
            return 1
        fi
    else
        print_result "SKIP" "$description" "redis-cli not available"
        return 1
    fi
}

# Function to test PostgreSQL connectivity
test_postgres() {
    local description=$1
    
    if command -v psql >/dev/null 2>&1; then
        local result=$(PGPASSWORD=matrixon_secure_password_change_me psql -h localhost -p 5432 -U matrixon -d matrixon -c "SELECT 1;" 2>/dev/null | grep -c "1 row" || echo "0")
        if [ "$result" -gt "0" ]; then
            print_result "PASS" "$description" "PostgreSQL connection successful"
            return 0
        else
            print_result "FAIL" "$description" "PostgreSQL connection failed"
            return 1
        fi
    else
        print_result "SKIP" "$description" "psql not available"
        return 1
    fi
}

# Function to test Docker container health
test_docker_health() {
    local container_name=$1
    local description=$2
    
    if command -v docker >/dev/null 2>&1; then
        local health_status=$(docker inspect --format='{{.State.Health.Status}}' "$container_name" 2>/dev/null || echo "unknown")
        case $health_status in
            "healthy")
                print_result "PASS" "$description" "Container is healthy"
                return 0
                ;;
            "unhealthy")
                print_result "FAIL" "$description" "Container is unhealthy"
                return 1
                ;;
            "starting")
                print_result "INFO" "$description" "Container is starting"
                return 0
                ;;
            *)
                print_result "FAIL" "$description" "Container status unknown or not running"
                return 1
                ;;
        esac
    else
        print_result "SKIP" "$description" "Docker not available"
        return 1
    fi
}

# Function to perform load testing
perform_load_test() {
    local endpoint=$1
    local concurrent_requests=${2:-10}
    local total_requests=${3:-100}
    local description=$4
    
    if command -v ab >/dev/null 2>&1; then
        local results=$(ab -n "$total_requests" -c "$concurrent_requests" -q "$endpoint" 2>/dev/null)
        local success_rate=$(echo "$results" | grep "Non-2xx responses" | awk '{print $3}' || echo "0")
        local avg_time=$(echo "$results" | grep "Time per request.*mean" | head -n1 | awk '{print $4}')
        
        if [ "$success_rate" == "0" ] && [ -n "$avg_time" ]; then
            print_result "PASS" "$description" "Load test successful - Avg: ${avg_time}ms"
            return 0
        else
            print_result "FAIL" "$description" "Load test failed - Success rate issues"
            return 1
        fi
    else
        print_result "SKIP" "$description" "Apache bench (ab) not available"
        return 1
    fi
}

# Main testing functions
test_infrastructure_services() {
    print_header "🏗️  INFRASTRUCTURE SERVICES HEALTH CHECK"
    
    # Docker containers health
    test_docker_health "matrixon-postgres" "PostgreSQL Container Health"
    test_docker_health "matrixon-redis" "Redis Container Health"
    test_docker_health "matrixon-ipfs" "IPFS Container Health"
    test_docker_health "matrixon-prometheus" "Prometheus Container Health"
    test_docker_health "matrixon-grafana" "Grafana Container Health"
    test_docker_health "matrixon-nginx" "Nginx Container Health"
    test_docker_health "matrixon-server" "Matrixon Server Container Health"
    
    # Network connectivity
    test_tcp_port "localhost" "5432" "PostgreSQL Port Connectivity"
    test_tcp_port "localhost" "6379" "Redis Port Connectivity"
    test_tcp_port "localhost" "5001" "IPFS API Port Connectivity"
    test_tcp_port "localhost" "8080" "IPFS Gateway Port Connectivity"
    test_tcp_port "localhost" "9090" "Prometheus Port Connectivity"
    test_tcp_port "localhost" "3001" "Grafana Port Connectivity"
    test_tcp_port "localhost" "80" "Nginx HTTP Port Connectivity"
    test_tcp_port "localhost" "6167" "Matrixon API Port Connectivity"
    
    # Service-specific tests
    test_postgres "PostgreSQL Database Connection"
    test_redis "Redis Cache Connection"
    test_http_endpoint "GET" "$IPFS_API_URL/api/v0/version" "IPFS API Health"
    test_http_endpoint "GET" "$PROMETHEUS_URL/-/healthy" "Prometheus Health"
    test_http_endpoint "GET" "$GRAFANA_URL/api/health" "Grafana Health"
}

test_matrix_api_endpoints() {
    print_header "📡 MATRIX API ENDPOINTS TESTING"
    
    # Basic API availability
    test_http_endpoint "GET" "$MATRIXON_URL/_matrix/client/versions" "Matrix Client API Versions"
    test_http_endpoint "GET" "$MATRIXON_URL/_matrix/client/r0/capabilities" "Client Capabilities"
    test_http_endpoint "GET" "$MATRIXON_URL/_matrix/federation/v1/version" "Federation Version"
    
    # Registration and authentication
    test_http_endpoint "GET" "$MATRIXON_URL/_matrix/client/r0/register/available" "Registration Availability"
    test_http_endpoint "POST" "$MATRIXON_URL/_matrix/client/r0/register" "User Registration" "200" \
        "{\"username\": \"$TEST_USER\", \"password\": \"$TEST_PASS\", \"device_id\": \"PRODTEST\"}"
    
    # Login test
    local login_response=$(curl -s -X POST "$MATRIXON_URL/_matrix/client/r0/login" \
        -H "Content-Type: application/json" \
        -d "{\"type\": \"m.login.password\", \"user\": \"$TEST_USER\", \"password\": \"$TEST_PASS\"}" \
        2>/dev/null || echo "{}")
    
    local access_token=$(echo "$login_response" | jq -r '.access_token // empty' 2>/dev/null)
    
    if [ -n "$access_token" ] && [ "$access_token" != "null" ]; then
        print_result "PASS" "User Login" "Access token obtained"
        
        # Authenticated endpoints
        test_http_endpoint "GET" "$MATRIXON_URL/_matrix/client/r0/account/whoami" "User Identity Check" "200" "" \
            "-H 'Authorization: Bearer $access_token'"
        test_http_endpoint "GET" "$MATRIXON_URL/_matrix/client/r0/joined_rooms" "List Joined Rooms" "200" "" \
            "-H 'Authorization: Bearer $access_token'"
        test_http_endpoint "GET" "$MATRIXON_URL/_matrix/client/r0/devices" "List Devices" "200" "" \
            "-H 'Authorization: Bearer $access_token'"
    else
        print_result "FAIL" "User Login" "Failed to obtain access token"
    fi
}

test_monitoring_services() {
    print_header "📊 MONITORING SERVICES TESTING"
    
    # Prometheus metrics
    test_http_endpoint "GET" "$PROMETHEUS_URL/metrics" "Prometheus Metrics Endpoint"
    test_http_endpoint "GET" "$PROMETHEUS_URL/api/v1/status/config" "Prometheus Configuration"
    test_http_endpoint "GET" "$PROMETHEUS_URL/api/v1/targets" "Prometheus Targets"
    
    # Grafana API
    test_http_endpoint "GET" "$GRAFANA_URL/api/health" "Grafana Health Check"
    test_http_endpoint "GET" "$GRAFANA_URL/api/datasources" "Grafana Datasources" "401"  # Expected unauthorized
    test_http_endpoint "GET" "$GRAFANA_URL/login" "Grafana Login Page"
    
    # Custom metrics endpoints (if available)
    test_http_endpoint "GET" "$MATRIXON_URL/metrics" "Matrixon Custom Metrics" "200"
    test_http_endpoint "GET" "$MATRIXON_URL/health" "Matrixon Health Check" "200"
}

test_nginx_proxy() {
    print_header "🌐 NGINX REVERSE PROXY TESTING"
    
    # Basic nginx functionality
    test_http_endpoint "GET" "$NGINX_URL/health" "Nginx Health Check"
    test_http_endpoint "GET" "$NGINX_URL" "Nginx Root" "301"  # Should redirect to HTTPS
    
    # Matrix API through nginx proxy
    test_http_endpoint "GET" "$NGINX_URL/_matrix/client/versions" "Matrix API via Nginx" "301"
    
    # SSL certificate test (if certificates are available)
    if curl -k -s "$NGINX_HTTPS_URL" >/dev/null 2>&1; then
        test_http_endpoint "GET" "$NGINX_HTTPS_URL/_matrix/client/versions" "Matrix API via HTTPS"
    else
        print_result "SKIP" "HTTPS Testing" "SSL certificates not configured"
    fi
}

test_ipfs_storage() {
    print_header "🗄️  IPFS DISTRIBUTED STORAGE TESTING"
    
    # IPFS API tests
    test_http_endpoint "GET" "$IPFS_API_URL/api/v0/version" "IPFS Version"
    test_http_endpoint "POST" "$IPFS_API_URL/api/v0/id" "IPFS Node ID"
    test_http_endpoint "GET" "$IPFS_API_URL/api/v0/swarm/peers" "IPFS Swarm Peers"
    
    # IPFS Gateway tests
    test_http_endpoint "GET" "$IPFS_GATEWAY_URL/ipfs/QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn" "IPFS Gateway"
    
    # Test file upload and retrieval (if IPFS is writable)
    local test_content="Matrixon IPFS Test $(date)"
    local upload_result=$(echo "$test_content" | curl -s -X POST -F "file=@-" "$IPFS_API_URL/api/v0/add" 2>/dev/null || echo "{}")
    local hash=$(echo "$upload_result" | jq -r '.Hash // empty' 2>/dev/null)
    
    if [ -n "$hash" ] && [ "$hash" != "null" ]; then
        print_result "PASS" "IPFS File Upload" "Hash: $hash"
        test_http_endpoint "GET" "$IPFS_GATEWAY_URL/ipfs/$hash" "IPFS File Retrieval"
    else
        print_result "SKIP" "IPFS File Upload/Retrieval" "Upload failed or IPFS not writable"
    fi
}

perform_performance_tests() {
    print_header "⚡ PERFORMANCE & LOAD TESTING"
    
    # Load testing for key endpoints
    perform_load_test "$MATRIXON_URL/_matrix/client/versions" 5 50 "Matrix API Load Test (Light)"
    perform_load_test "$PROMETHEUS_URL/-/healthy" 10 100 "Prometheus Load Test"
    perform_load_test "$GRAFANA_URL/api/health" 5 25 "Grafana Load Test"
    perform_load_test "$NGINX_URL/health" 10 100 "Nginx Load Test"
    
    # Memory and resource usage
    if command -v docker >/dev/null 2>&1; then
        print_result "INFO" "Docker Resource Usage" "$(docker stats --no-stream --format 'table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}')"
    fi
}

# Parse command line arguments
TEST_MODE="full"
while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            TEST_MODE="quick"
            shift
            ;;
        --full)
            TEST_MODE="full"
            shift
            ;;
        --services)
            TEST_MODE="services"
            shift
            ;;
        --api)
            TEST_MODE="api"
            shift
            ;;
        --monitor)
            TEST_MODE="monitor"
            shift
            ;;
        --help)
            echo "Usage: $0 [--quick|--full|--services|--api|--monitor]"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Main execution
main() {
    echo -e "${BOLD}${GREEN}"
    echo "╔═══════════════════════════════════════════════════════════════╗"
    echo "║              MATRIXON PRODUCTION ENVIRONMENT TEST             ║"
    echo "║                    COMPREHENSIVE TEST SUITE                   ║"
    echo "╚═══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    
    log "INFO" "Starting Matrixon Production Environment Test Suite"
    log "INFO" "Test mode: $TEST_MODE"
    log "INFO" "Timestamp: $TIMESTAMP"
    
    case $TEST_MODE in
        "quick")
            test_infrastructure_services
            ;;
        "services")
            test_infrastructure_services
            test_ipfs_storage
            ;;
        "api")
            test_matrix_api_endpoints
            ;;
        "monitor")
            test_monitoring_services
            ;;
        "full")
            test_infrastructure_services
            test_matrix_api_endpoints
            test_monitoring_services
            test_nginx_proxy
            test_ipfs_storage
            perform_performance_tests
            ;;
    esac
    
    # Generate test summary
    print_header "📋 TEST SUMMARY"
    echo -e "${BOLD}Total Tests: $TOTAL_TESTS${NC}"
    echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
    echo -e "${RED}Failed: $FAILED_TESTS${NC}"
    echo -e "${YELLOW}Skipped: $SKIPPED_TESTS${NC}"
    
    local success_rate=$((PASSED_TESTS * 100 / TOTAL_TESTS))
    echo -e "${BOLD}Success Rate: $success_rate%${NC}"
    
    # Generate JSON report
    cat > "$RESULTS_FILE" << EOF
{
    "timestamp": "$TIMESTAMP",
    "test_mode": "$TEST_MODE",
    "summary": {
        "total_tests": $TOTAL_TESTS,
        "passed_tests": $PASSED_TESTS,
        "failed_tests": $FAILED_TESTS,
        "skipped_tests": $SKIPPED_TESTS,
        "success_rate": $success_rate
    },
    "log_file": "$LOG_FILE"
}
EOF
    
    log "INFO" "Test completed. Results saved to $RESULTS_FILE"
    echo -e "\n${CYAN}📄 Detailed logs: $LOG_FILE${NC}"
    echo -e "${CYAN}📊 Results summary: $RESULTS_FILE${NC}"
    
    # Exit with appropriate code
    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "\n${GREEN}🎉 All tests passed successfully!${NC}"
        exit 0
    else
        echo -e "\n${RED}⚠️  Some tests failed. Please check the logs.${NC}"
        exit 1
    fi
}

# Run main function
main "$@" 
