#!/bin/bash

# =============================================================================
# Matrixon Local Testing Environment
# =============================================================================
#
# Project: Matrixon - Ultra High Performance Matrix NextServer
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 0.11.0-alpha
#
# Description:
#   Local testing environment for Matrixon without Docker
#   Tests the Matrixon server and APIs using curl commands
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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
LOG_FILE="$SCRIPT_DIR/logs/local_test_$TIMESTAMP.log"

# Configuration
MATRIXON_URL="http://localhost:6167"
TEST_TIMEOUT=30

# Create logs directory
mkdir -p "$SCRIPT_DIR/logs"

echo -e "${BOLD}${GREEN}"
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║              MATRIXON LOCAL TEST ENVIRONMENT                  ║"
echo "║                    COMPREHENSIVE TEST SUITE                   ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Function to log messages
log() {
    local level=$1
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

# Function to print status
print_status() {
    echo -e "${CYAN}🔧 $1${NC}"
    log "INFO" "$1"
}

# Function to print success
print_success() {
    echo -e "${GREEN}✅ $1${NC}"
    log "SUCCESS" "$1"
}

# Function to print error
print_error() {
    echo -e "${RED}❌ $1${NC}"
    log "ERROR" "$1"
}

# Function to test HTTP endpoint
test_endpoint() {
    local method=$1
    local url=$2
    local description=$3
    local expected_status=${4:-200}
    local data=$5
    local headers=$6
    
    print_status "Testing: $description"
    
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
        print_error "Connection failed for: $description"
        return 1
    elif [ "$http_status" == "$expected_status" ]; then
        print_success "$description - HTTP $http_status"
        if [ -n "$body" ] && [ "$body" != "{}" ]; then
            echo -e "   ${CYAN}Response:${NC} $(echo "$body" | jq . 2>/dev/null || echo "$body")"
        fi
        return 0
    else
        print_error "$description - Expected HTTP $expected_status, got $http_status"
        if [ -n "$body" ]; then
            echo -e "   ${RED}Response:${NC} $(echo "$body" | jq . 2>/dev/null || echo "$body")"
        fi
        return 1
    fi
}

# Check if server is running
check_server() {
    print_status "Checking if Matrixon server is running..."
    
    if curl -s -f "$MATRIXON_URL/health" >/dev/null 2>&1; then
        print_success "Matrixon server is already running"
        return 0
    else
        print_status "Matrixon server not running, starting it..."
        return 1
    fi
}

# Start server
start_server() {
    print_status "Building and starting Matrixon server..."
    
    # Build the project
    if ! cargo build --bin matrixon --release; then
        print_error "Failed to build Matrixon server"
        return 1
    fi
    
    print_success "Matrixon server built successfully"
    
    # Start the server in background
    print_status "Starting Matrixon server on port 6167..."
    RUST_LOG=info ./target/release/matrixon &
    SERVER_PID=$!
    
    # Wait for server to start
    sleep 5
    
    # Check if server is responding
    local attempts=0
    local max_attempts=10
    
    while [ $attempts -lt $max_attempts ]; do
        if curl -s -f "$MATRIXON_URL/health" >/dev/null 2>&1; then
            print_success "Matrixon server started successfully (PID: $SERVER_PID)"
            return 0
        fi
        
        ((attempts++))
        print_status "Waiting for server to start... (attempt $attempts/$max_attempts)"
        sleep 2
    done
    
    print_error "Failed to start Matrixon server"
    return 1
}

# Stop server
stop_server() {
    if [ -n "$SERVER_PID" ]; then
        print_status "Stopping Matrixon server (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
        print_success "Matrixon server stopped"
    fi
}

# Comprehensive API tests
run_comprehensive_tests() {
    echo -e "\n${PURPLE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${PURPLE}COMPREHENSIVE API TESTING${NC}"
    echo -e "${PURPLE}═══════════════════════════════════════════════════════════════${NC}\n"
    
    local tests_passed=0
    local tests_failed=0
    
    # Health and metrics tests
    test_endpoint "GET" "$MATRIXON_URL/health" "Health Check" && ((tests_passed++)) || ((tests_failed++))
    test_endpoint "GET" "$MATRIXON_URL/metrics" "Metrics Endpoint" && ((tests_passed++)) || ((tests_failed++))
    
    # Matrix API tests
    test_endpoint "GET" "$MATRIXON_URL/_matrix/client/versions" "Matrix Client API Versions" && ((tests_passed++)) || ((tests_failed++))
    test_endpoint "GET" "$MATRIXON_URL/_matrix/client/r0/capabilities" "Client Capabilities" && ((tests_passed++)) || ((tests_failed++))
    test_endpoint "GET" "$MATRIXON_URL/_matrix/federation/v1/version" "Federation Version" && ((tests_passed++)) || ((tests_failed++))
    
    # Registration test
    local test_user="test_user_$(date +%s)"
    local test_pass="TestPassword123!"
    
    test_endpoint "GET" "$MATRIXON_URL/_matrix/client/r0/register/available?username=$test_user" "Registration Availability" && ((tests_passed++)) || ((tests_failed++))
    
    test_endpoint "POST" "$MATRIXON_URL/_matrix/client/r0/register" "User Registration" "200" \
        "{\"username\": \"$test_user\", \"password\": \"$test_pass\", \"device_id\": \"TEST_DEVICE\"}" && ((tests_passed++)) || ((tests_failed++))
    
    # Login test
    local login_response=$(curl -s -X POST "$MATRIXON_URL/_matrix/client/r0/login" \
        -H "Content-Type: application/json" \
        -d "{\"type\": \"m.login.password\", \"user\": \"$test_user\", \"password\": \"$test_pass\"}" \
        2>/dev/null || echo "{}")
    
    local access_token=$(echo "$login_response" | jq -r '.access_token // empty' 2>/dev/null)
    
    if [ -n "$access_token" ] && [ "$access_token" != "null" ]; then
        print_success "User Login - Access token obtained"
        ((tests_passed++))
        
        # Authenticated endpoints
        test_endpoint "GET" "$MATRIXON_URL/_matrix/client/r0/account/whoami" "User Identity Check" "200" "" \
            "-H 'Authorization: Bearer $access_token'" && ((tests_passed++)) || ((tests_failed++))
        test_endpoint "GET" "$MATRIXON_URL/_matrix/client/r0/joined_rooms" "List Joined Rooms" "200" "" \
            "-H 'Authorization: Bearer $access_token'" && ((tests_passed++)) || ((tests_failed++))
        test_endpoint "GET" "$MATRIXON_URL/_matrix/client/r0/devices" "List Devices" "200" "" \
            "-H 'Authorization: Bearer $access_token'" && ((tests_passed++)) || ((tests_failed++))
    else
        print_error "User Login - Failed to obtain access token"
        ((tests_failed++))
    fi
    
    # Performance test
    echo -e "\n${PURPLE}Performance Testing${NC}"
    print_status "Running performance test with 10 concurrent requests..."
    
    local start_time=$(date +%s.%N)
    for i in {1..10}; do
        curl -s "$MATRIXON_URL/_matrix/client/versions" >/dev/null &
    done
    wait
    local end_time=$(date +%s.%N)
    local duration=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "0")
    
    if [ -n "$duration" ] && [ "$duration" != "0" ]; then
        print_success "Performance test completed in ${duration}s"
        ((tests_passed++))
    else
        print_error "Performance test failed"
        ((tests_failed++))
    fi
    
    # Summary
    echo -e "\n${BOLD}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BOLD}TEST SUMMARY${NC}"
    echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}✅ Tests Passed: $tests_passed${NC}"
    echo -e "${RED}❌ Tests Failed: $tests_failed${NC}"
    
    local total_tests=$((tests_passed + tests_failed))
    local success_rate=$((tests_passed * 100 / total_tests))
    echo -e "${BOLD}Success Rate: $success_rate%${NC}"
    
    if [ $tests_failed -eq 0 ]; then
        echo -e "\n${GREEN}🎉 All tests passed successfully!${NC}"
        return 0
    else
        echo -e "\n${RED}⚠️  Some tests failed. Check the logs for details.${NC}"
        return 1
    fi
}

# Cleanup function
cleanup() {
    print_status "Cleaning up..."
    stop_server
    print_success "Cleanup completed"
}

# Set up trap for cleanup
trap cleanup EXIT

# Main execution
main() {
    log "INFO" "Starting Matrixon Local Test Environment"
    
    # Check if server is already running
    if ! check_server; then
        # Start the server
        if ! start_server; then
            print_error "Failed to start server"
            exit 1
        fi
    fi
    
    # Wait a moment for server to be fully ready
    sleep 2
    
    # Run comprehensive tests
    if run_comprehensive_tests; then
        print_success "All tests completed successfully!"
        echo -e "\n${CYAN}📄 Detailed logs: $LOG_FILE${NC}"
        exit 0
    else
        print_error "Some tests failed"
        echo -e "\n${CYAN}📄 Detailed logs: $LOG_FILE${NC}"
        exit 1
    fi
}

# Run main function
main "$@" 
