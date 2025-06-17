#!/bin/bash

# =============================================================================
# Comprehensive Matrix Server API Test Suite with Curl
# Created for Matrixon - High-Performance Matrix NextServer
# Version: 2.0.0
# Date: 2024-12-11
# =============================================================================

set -euo pipefail

# Color definitions for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SERVER_URL="http://localhost:6167"
TEST_USERNAME="testuser_$(date +%s)"
TEST_PASSWORD="TestPass123!"
TEST_ROOM_NAME="TestRoom_$(date +%s)"
OUTPUT_DIR="./test_results"
LOG_FILE="$OUTPUT_DIR/matrix_test_$(date +%Y%m%d_%H%M%S).log"

# Statistics
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SERVER_RUNNING=false

# Create output directory
mkdir -p "$OUTPUT_DIR"

# =============================================================================
# Utility Functions
# =============================================================================

log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

print_header() {
    local title="$1"
    echo
    echo -e "${CYAN}=================================================================${NC}"
    echo -e "${CYAN} $title${NC}"
    echo -e "${CYAN}=================================================================${NC}"
    log "INFO" "Starting: $title"
}

print_section() {
    local title="$1"
    echo
    echo -e "${BLUE}--- $title ---${NC}"
    log "INFO" "Section: $title"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
    log "SUCCESS" "$1"
    ((PASSED_TESTS++))
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
    log "ERROR" "$1"
    ((FAILED_TESTS++))
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
    log "WARNING" "$1"
}

print_info() {
    echo -e "${PURPLE}ℹ️  $1${NC}"
    log "INFO" "$1"
}

increment_test() {
    ((TOTAL_TESTS++))
}

# =============================================================================
# Server Detection and Health Check
# =============================================================================

check_server_status() {
    print_header "Matrix Server Health Check"
    
    local response
    local http_code
    
    print_info "Checking server status at $SERVER_URL..."
    
    if response=$(curl -s -w "%{http_code}" -o /dev/null --connect-timeout 5 --max-time 10 "$SERVER_URL/_matrix/client/versions" 2>/dev/null); then
        http_code="$response"
        if [[ "$http_code" =~ ^[23] ]]; then
            SERVER_RUNNING=true
            print_success "Server is running (HTTP $http_code)"
            
            # Test actual Matrix endpoint
            if matrix_response=$(curl -s "$SERVER_URL/_matrix/client/versions" 2>/dev/null); then
                print_success "Matrix API is responding"
                echo "Response: $matrix_response" | jq . 2>/dev/null || echo "Raw response: $matrix_response"
            else
                print_warning "Server running but Matrix API not responding properly"
            fi
        else
            print_error "Server returned HTTP $http_code"
        fi
    else
        print_error "Server is not accessible"
    fi
    
    echo
    if [ "$SERVER_RUNNING" = true ]; then
        print_info "🚀 Server detected! Running full test suite..."
    else
        print_info "📚 Server not running. Providing test examples and documentation..."
    fi
}

# =============================================================================
# Matrix API Test Functions (Active Server Tests)
# =============================================================================

test_server_info() {
    print_section "Server Information Tests"
    
    increment_test
    print_info "Testing /_matrix/client/versions"
    if response=$(curl -s "$SERVER_URL/_matrix/client/versions" 2>/dev/null); then
        print_success "Client versions endpoint accessible"
        echo "$response" | jq . 2>/dev/null || echo "Response: $response"
    else
        print_error "Failed to access client versions"
    fi
    
    increment_test
    print_info "Testing /.well-known/matrix/client"
    if response=$(curl -s "$SERVER_URL/.well-known/matrix/client" 2>/dev/null); then
        print_success "Well-known client endpoint accessible"
        echo "$response" | jq . 2>/dev/null || echo "Response: $response"
    else
        print_error "Failed to access well-known client endpoint"
    fi
    
    increment_test
    print_info "Testing /.well-known/matrix/server"
    if response=$(curl -s "$SERVER_URL/.well-known/matrix/server" 2>/dev/null); then
        print_success "Well-known server endpoint accessible"
        echo "$response" | jq . 2>/dev/null || echo "Response: $response"
    else
        print_error "Failed to access well-known server endpoint"
    fi
}

test_user_registration() {
    print_section "User Registration Test"
    
    increment_test
    print_info "Attempting user registration for: $TEST_USERNAME"
    
    local registration_data='{
        "username": "'$TEST_USERNAME'",
        "password": "'$TEST_PASSWORD'",
        "device_id": "TESTDEVICE",
        "initial_device_display_name": "Test Device"
    }'
    
    if response=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$registration_data" \
        "$SERVER_URL/_matrix/client/r0/register" 2>/dev/null); then
        
        if echo "$response" | jq -e '.access_token' >/dev/null 2>&1; then
            ACCESS_TOKEN=$(echo "$response" | jq -r '.access_token')
            USER_ID=$(echo "$response" | jq -r '.user_id')
            print_success "User registration successful"
            print_info "User ID: $USER_ID"
            print_info "Access token: ${ACCESS_TOKEN:0:20}..."
        else
            print_error "Registration failed: $response"
        fi
    else
        print_error "Failed to connect for registration"
    fi
}

test_user_login() {
    print_section "User Login Test"
    
    if [ -z "${ACCESS_TOKEN:-}" ]; then
        increment_test
        print_info "Attempting login for: $TEST_USERNAME"
        
        local login_data='{
            "type": "m.login.password",
            "user": "'$TEST_USERNAME'",
            "password": "'$TEST_PASSWORD'"
        }'
        
        if response=$(curl -s -X POST \
            -H "Content-Type: application/json" \
            -d "$login_data" \
            "$SERVER_URL/_matrix/client/r0/login" 2>/dev/null); then
            
            if echo "$response" | jq -e '.access_token' >/dev/null 2>&1; then
                ACCESS_TOKEN=$(echo "$response" | jq -r '.access_token')
                USER_ID=$(echo "$response" | jq -r '.user_id')
                print_success "User login successful"
                print_info "User ID: $USER_ID"
            else
                print_error "Login failed: $response"
            fi
        else
            print_error "Failed to connect for login"
        fi
    else
        print_info "Already authenticated from registration"
    fi
}

test_user_profile() {
    print_section "User Profile Tests"
    
    if [ -n "${ACCESS_TOKEN:-}" ]; then
        increment_test
        print_info "Testing profile retrieval"
        if response=$(curl -s -H "Authorization: Bearer $ACCESS_TOKEN" \
            "$SERVER_URL/_matrix/client/r0/profile/$USER_ID" 2>/dev/null); then
            print_success "Profile retrieved successfully"
            echo "$response" | jq . 2>/dev/null || echo "Response: $response"
        else
            print_error "Failed to retrieve profile"
        fi
        
        increment_test
        print_info "Testing display name update"
        local display_name_data='{"displayname": "Test User Display Name"}'
        if response=$(curl -s -X PUT \
            -H "Authorization: Bearer $ACCESS_TOKEN" \
            -H "Content-Type: application/json" \
            -d "$display_name_data" \
            "$SERVER_URL/_matrix/client/r0/profile/$USER_ID/displayname" 2>/dev/null); then
            print_success "Display name updated successfully"
        else
            print_error "Failed to update display name: $response"
        fi
    else
        print_warning "Skipping profile tests - no access token"
    fi
}

test_room_operations() {
    print_section "Room Operations Tests"
    
    if [ -n "${ACCESS_TOKEN:-}" ]; then
        increment_test
        print_info "Creating a new room: $TEST_ROOM_NAME"
        
        local room_data='{
            "name": "'$TEST_ROOM_NAME'",
            "topic": "Test room for Matrix API testing",
            "preset": "private_chat"
        }'
        
        if response=$(curl -s -X POST \
            -H "Authorization: Bearer $ACCESS_TOKEN" \
            -H "Content-Type: application/json" \
            -d "$room_data" \
            "$SERVER_URL/_matrix/client/r0/createRoom" 2>/dev/null); then
            
            if echo "$response" | jq -e '.room_id' >/dev/null 2>&1; then
                ROOM_ID=$(echo "$response" | jq -r '.room_id')
                print_success "Room created successfully: $ROOM_ID"
                test_room_messaging
            else
                print_error "Room creation failed: $response"
            fi
        else
            print_error "Failed to connect for room creation"
        fi
    else
        print_warning "Skipping room tests - no access token"
    fi
}

test_room_messaging() {
    print_section "Room Messaging Tests"
    
    if [ -n "${ROOM_ID:-}" ]; then
        increment_test
        print_info "Sending a message to room: $ROOM_ID"
        
        local message_data='{
            "msgtype": "m.text",
            "body": "Hello, this is a test message from the Matrix API test suite!"
        }'
        
        local txn_id="txn_$(date +%s)_$$"
        
        if response=$(curl -s -X PUT \
            -H "Authorization: Bearer $ACCESS_TOKEN" \
            -H "Content-Type: application/json" \
            -d "$message_data" \
            "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id" 2>/dev/null); then
            
            if echo "$response" | jq -e '.event_id' >/dev/null 2>&1; then
                EVENT_ID=$(echo "$response" | jq -r '.event_id')
                print_success "Message sent successfully: $EVENT_ID"
            else
                print_error "Message sending failed: $response"
            fi
        else
            print_error "Failed to connect for message sending"
        fi
        
        increment_test
        print_info "Retrieving room messages"
        if response=$(curl -s -H "Authorization: Bearer $ACCESS_TOKEN" \
            "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/messages?dir=b&limit=10" 2>/dev/null); then
            print_success "Room messages retrieved successfully"
            echo "$response" | jq . 2>/dev/null | head -20 || echo "Response: $response"
        else
            print_error "Failed to retrieve room messages"
        fi
    fi
}

test_sync_endpoint() {
    print_section "Sync Endpoint Test"
    
    if [ -n "${ACCESS_TOKEN:-}" ]; then
        increment_test
        print_info "Testing initial sync"
        if response=$(curl -s -H "Authorization: Bearer $ACCESS_TOKEN" \
            "$SERVER_URL/_matrix/client/r0/sync?timeout=1000" 2>/dev/null); then
            print_success "Sync endpoint accessible"
            echo "Sync response summary:" 
            echo "$response" | jq '{next_batch: .next_batch, rooms: (.rooms | keys)}' 2>/dev/null || echo "Raw response length: ${#response}"
        else
            print_error "Failed to access sync endpoint"
        fi
    else
        print_warning "Skipping sync test - no access token"
    fi
}

test_federation_endpoints() {
    print_section "Federation Endpoint Tests"
    
    increment_test
    print_info "Testing server key endpoint"
    if response=$(curl -s "$SERVER_URL/_matrix/key/v2/server" 2>/dev/null); then
        print_success "Server key endpoint accessible"
        echo "$response" | jq . 2>/dev/null || echo "Response: $response"
    else
        print_error "Failed to access server key endpoint"
    fi
    
    increment_test
    print_info "Testing version endpoint"
    if response=$(curl -s "$SERVER_URL/_matrix/federation/v1/version" 2>/dev/null); then
        print_success "Federation version endpoint accessible"
        echo "$response" | jq . 2>/dev/null || echo "Response: $response"
    else
        print_error "Failed to access federation version endpoint"
    fi
}

# =============================================================================
# Performance Tests
# =============================================================================

test_performance() {
    print_section "Performance Tests"
    
    if [ "$SERVER_RUNNING" = true ]; then
        increment_test
        print_info "Running concurrent request test (10 requests)"
        
        local start_time=$(date +%s.%N)
        local pids=()
        
        for i in {1..10}; do
            (curl -s -o /dev/null -w "%{time_total}\n" "$SERVER_URL/_matrix/client/versions" >> "$OUTPUT_DIR/perf_times.txt") &
            pids+=($!)
        done
        
        # Wait for all requests to complete
        for pid in "${pids[@]}"; do
            wait "$pid"
        done
        
        local end_time=$(date +%s.%N)
        local total_time=$(echo "$end_time - $start_time" | bc -l)
        
        if [ -f "$OUTPUT_DIR/perf_times.txt" ]; then
            local avg_time=$(awk '{sum+=$1} END {print sum/NR}' "$OUTPUT_DIR/perf_times.txt")
            local max_time=$(sort -nr "$OUTPUT_DIR/perf_times.txt" | head -1)
            
            print_success "Performance test completed"
            print_info "Total time: ${total_time}s"
            print_info "Average response time: ${avg_time}s"
            print_info "Max response time: ${max_time}s"
            
            # Check if under 50ms target
            if (( $(echo "$avg_time < 0.05" | bc -l) )); then
                print_success "✨ Performance target met: <50ms average latency"
            else
                print_warning "Performance target not met: >50ms average latency"
            fi
            
            rm -f "$OUTPUT_DIR/perf_times.txt"
        else
            print_error "Performance test failed - no timing data"
        fi
    fi
}

# =============================================================================
# PostgreSQL Database Tests
# =============================================================================

test_postgresql_functionality() {
    print_section "PostgreSQL Database Tests"
    
    increment_test
    print_info "Testing PostgreSQL connection"
    if command -v psql >/dev/null 2>&1; then
        if psql -h localhost -U matrixon -d matrixon -c "SELECT version();" >/dev/null 2>&1; then
            print_success "PostgreSQL connection successful"
            
            # Test database schema
            local table_count=$(psql -h localhost -U matrixon -d matrixon -t -c "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public';" 2>/dev/null | xargs)
            print_info "Database tables found: $table_count"
            
            if [ "$table_count" -gt 0 ]; then
                print_success "Database schema exists"
            else
                print_warning "No tables found in database"
            fi
        else
            print_error "Failed to connect to PostgreSQL"
        fi
    else
        print_warning "psql not available for database testing"
    fi
}

# =============================================================================
# Documentation and Examples (Fallback Mode)
# =============================================================================

show_test_examples() {
    print_header "Matrix API Test Examples and Documentation"
    
    print_section "Basic Server Information"
    cat << 'EOF'
# Check server versions
curl -s http://localhost:6167/_matrix/client/versions | jq .

# Check well-known configuration
curl -s http://localhost:6167/.well-known/matrix/client | jq .
curl -s http://localhost:6167/.well-known/matrix/server | jq .

# Get server public key
curl -s http://localhost:6167/_matrix/key/v2/server | jq .
EOF

    print_section "User Registration and Authentication"
    cat << 'EOF'
# Register a new user
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "testpass123",
    "device_id": "TESTDEVICE"
  }' \
  http://localhost:6167/_matrix/client/r0/register

# Login (get access token)
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "type": "m.login.password",
    "user": "testuser", 
    "password": "testpass123"
  }' \
  http://localhost:6167/_matrix/client/r0/login
EOF

    print_section "Room Operations"
    cat << 'EOF'
# Create a room
curl -X POST \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Room",
    "topic": "A test room",
    "preset": "private_chat"
  }' \
  http://localhost:6167/_matrix/client/r0/createRoom

# Send a message
curl -X PUT \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "msgtype": "m.text",
    "body": "Hello, Matrix!"
  }' \
  http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID/send/m.room.message/TXN_ID

# Get room messages
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID/messages?dir=b&limit=10
EOF

    print_section "User Profile Management"
    cat << 'EOF'
# Get user profile
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  http://localhost:6167/_matrix/client/r0/profile/@user:localhost

# Set display name
curl -X PUT \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"displayname": "My Display Name"}' \
  http://localhost:6167/_matrix/client/r0/profile/@user:localhost/displayname

# Set avatar URL
curl -X PUT \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "mxc://localhost/media_id"}' \
  http://localhost:6167/_matrix/client/r0/profile/@user:localhost/avatar_url
EOF

    print_section "Sync and Real-time Updates"
    cat << 'EOF'
# Initial sync
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  http://localhost:6167/_matrix/client/r0/sync?timeout=30000

# Sync with since token
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  http://localhost:6167/_matrix/client/r0/sync?since=SINCE_TOKEN&timeout=30000
EOF

    print_section "Federation Testing"
    cat << 'EOF'
# Test federation with another server
curl -s http://localhost:6167/_matrix/federation/v1/version

# Server discovery
curl -s https://matrix.org/.well-known/matrix/server

# Key exchange test
curl -s http://localhost:6167/_matrix/key/v2/server/localhost
EOF
}

show_postgresql_examples() {
    print_section "PostgreSQL Database Commands"
    cat << 'EOF'
# Connect to the database
psql -h localhost -U matrixon -d matrixon

# Check database tables
\dt

# View Matrix-specific tables
SELECT table_name FROM information_schema.tables 
WHERE table_schema = 'public' AND table_name LIKE 'matrixon_%';

# Check user data
SELECT * FROM matrixon_userid_password LIMIT 5;

# Monitor database performance
SELECT 
  query,
  calls,
  total_time,
  mean_time,
  rows
FROM pg_stat_statements 
ORDER BY total_time DESC 
LIMIT 10;

# Database size information
SELECT 
  pg_size_pretty(pg_database_size('matrixon')) as database_size,
  pg_size_pretty(pg_total_relation_size('matrixon_global')) as global_table_size;
EOF
}

show_troubleshooting_guide() {
    print_section "Troubleshooting Guide"
    cat << 'EOF'
Common Issues and Solutions:

1. Server not starting:
   - Check PostgreSQL is running: sudo brew services status postgresql
   - Verify database exists: psql -l | grep matrixon
   - Check configuration file: cat matrixon-pg.toml
   - Review logs for keypair errors

2. Connection refused:
   - Verify server is running on correct port (6167)
   - Check firewall settings
   - Ensure MATRIXON_CONFIG environment variable is set

3. Database errors:
   - Reset database: dropdb matrixon && createdb matrixon
   - Run setup script: ./setup_postgresql.sh
   - Check PostgreSQL logs

4. Authentication failures:
   - Verify registration is enabled in config
   - Check username/password requirements
   - Ensure proper Content-Type headers

5. Performance issues:
   - Monitor PostgreSQL connections
   - Check database query performance
   - Review server resource usage
   - Tune PostgreSQL configuration

Environment Variables:
export MATRIXON_CONFIG=matrixon-pg.toml
export RUST_LOG=debug
export POSTGRES_PASSWORD=matrixon

Useful Commands:
# Start PostgreSQL (macOS)
sudo brew services start postgresql

# Start Matrixon server
MATRIXON_CONFIG=matrixon-pg.toml ./target/debug/matrixon

# Monitor server logs
tail -f matrixon.log

# Check server status
curl -s http://localhost:6167/_matrix/client/versions
EOF
}

# =============================================================================
# Main Test Execution
# =============================================================================

run_full_test_suite() {
    print_header "Running Full Matrix API Test Suite"
    
    test_server_info
    test_user_registration
    test_user_login
    test_user_profile
    test_room_operations
    test_sync_endpoint
    test_federation_endpoints
    test_performance
    test_postgresql_functionality
}

show_final_summary() {
    print_header "Test Summary Report"
    
    echo
    echo -e "${CYAN}📊 Test Statistics:${NC}"
    echo -e "   Total Tests: ${TOTAL_TESTS}"
    echo -e "   ${GREEN}Passed: ${PASSED_TESTS}${NC}"
    echo -e "   ${RED}Failed: ${FAILED_TESTS}${NC}"
    
    if [ $TOTAL_TESTS -gt 0 ]; then
        local success_rate=$((PASSED_TESTS * 100 / TOTAL_TESTS))
        echo -e "   Success Rate: ${success_rate}%"
        
        if [ $success_rate -ge 90 ]; then
            echo -e "   ${GREEN}🎉 Excellent performance!${NC}"
        elif [ $success_rate -ge 70 ]; then
            echo -e "   ${YELLOW}⚠️  Good, but room for improvement${NC}"
        else
            echo -e "   ${RED}❌ Needs attention${NC}"
        fi
    fi
    
    echo
    echo -e "${CYAN}📁 Output Files:${NC}"
    echo -e "   Log File: ${LOG_FILE}"
    echo -e "   Results Directory: ${OUTPUT_DIR}"
    
    if [ "$SERVER_RUNNING" = false ]; then
        echo
        echo -e "${YELLOW}💡 Server not running. Start the server to run active tests:${NC}"
        echo -e "   MATRIXON_CONFIG=matrixon-pg.toml ./target/debug/matrixon"
    fi
    
    echo
    echo -e "${PURPLE}🚀 Matrix Server Performance Targets:${NC}"
    echo -e "   • 20k+ concurrent connections"
    echo -e "   • <50ms response latency"
    echo -e "   • >99% success rate"
    echo -e "   • PostgreSQL backend optimization"
}

# =============================================================================
# Main Script Execution
# =============================================================================

main() {
    clear
    echo -e "${PURPLE}"
    cat << 'EOF'
 __  __       _        _                    
|  \/  | __ _| |_ _ __(_)_  _  ___  _ __    
| |\/| |/ _` | __| '__| \ \/ |/ _ \| '_ \   
| |  | | (_| | |_| |  | |>  <| (_) | | | |  
|_|  |_|\__,_|\__|_|  |_/_/\_\\___/|_| |_|  
                                           
High-Performance Matrix NextServer Testing Suite
EOF
    echo -e "${NC}"
    
    log "INFO" "=== Matrix Test Suite Started ==="
    log "INFO" "Server URL: $SERVER_URL"
    log "INFO" "Output Directory: $OUTPUT_DIR"
    
    # Check server status
    check_server_status
    
    if [ "$SERVER_RUNNING" = true ]; then
        # Run full test suite
        run_full_test_suite
    else
        # Show examples and documentation
        show_test_examples
        show_postgresql_examples
        show_troubleshooting_guide
    fi
    
    # Show final summary
    show_final_summary
    
    log "INFO" "=== Matrix Test Suite Completed ==="
}

# Run the main function
main "$@" 
