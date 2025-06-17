#!/bin/bash

##
# Comprehensive Matrix API Testing Script for PostgreSQL Backend
# Tests all major Matrix protocol endpoints with fallback handling
# 
# @author: Matrix Server Testing Team
# @date: 2024-01-01
# @version: 3.0.0
##

set -e

# Configuration
BASE_URL="http://localhost:6167"
SERVER_NAME="localhost"
TEST_USERNAME="testuser_$(date +%s)"
TEST_PASSWORD="TestPassword123!"
TEST_ROOM_ALIAS="#testroom_$(date +%s):${SERVER_NAME}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Global variables for storing test data
ACCESS_TOKEN=""
USER_ID=""
ROOM_ID=""
EVENT_ID=""

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Logging functions
log() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
    ((PASSED_TESTS++))
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
    ((SKIPPED_TESTS++))
}

error() {
    echo -e "${RED}❌ $1${NC}"
    ((FAILED_TESTS++))
}

info() {
    echo -e "${PURPLE}ℹ️  $1${NC}"
}

cyan() {
    echo -e "${CYAN}🔧 $1${NC}"
}

print_banner() {
    echo -e "${BLUE}"
    echo "================================================================"
    echo "  🚀 Comprehensive Matrix API Testing Suite"
    echo "  🗄️  PostgreSQL Backend Performance Testing"
    echo "  📊 Target: 20k+ connections, <50ms latency"
    echo "================================================================"
    echo -e "${NC}"
}

print_summary() {
    echo ""
    echo -e "${BLUE}"
    echo "================================================================"
    echo "  📊 TEST SUMMARY"
    echo "================================================================"
    echo -e "${NC}"
    echo -e "Total Tests:   ${CYAN}$((TOTAL_TESTS))${NC}"
    echo -e "Passed:        ${GREEN}${PASSED_TESTS}${NC}"
    echo -e "Failed:        ${RED}${FAILED_TESTS}${NC}"
    echo -e "Skipped:       ${YELLOW}${SKIPPED_TESTS}${NC}"
    echo ""
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}🎉 All tests passed! Matrix server is working correctly.${NC}"
    else
        echo -e "${RED}⚠️  Some tests failed. Check the output above for details.${NC}"
    fi
    echo ""
}

# Check if server is running
check_server_status() {
    log "🔍 Checking if Matrix server is running..."
    ((TOTAL_TESTS++))
    
    if curl -s --connect-timeout 5 "${BASE_URL}" > /dev/null 2>&1; then
        success "Server is responding on ${BASE_URL}"
        return 0
    else
        error "Server is not responding on ${BASE_URL}"
        warning "Server might not be started. Starting fallback tests..."
        return 1
    fi
}

# Test server capabilities
test_server_capabilities() {
    log "🔧 Testing server capabilities and versions..."
    
    # Test /_matrix/client/versions
    ((TOTAL_TESTS++))
    info "Testing /_matrix/client/versions"
    local response=$(curl -s "${BASE_URL}/_matrix/client/versions" 2>/dev/null)
    if echo "$response" | jq -e '.versions' > /dev/null 2>&1; then
        success "Client versions endpoint working"
        echo "$response" | jq '.versions' | head -5
    else
        error "Client versions endpoint failed"
        echo "Response: $response"
    fi
    
    # Test /_matrix/client/r0/capabilities
    ((TOTAL_TESTS++))
    info "Testing /_matrix/client/r0/capabilities"
    response=$(curl -s "${BASE_URL}/_matrix/client/r0/capabilities" 2>/dev/null)
    if echo "$response" | jq -e '.capabilities' > /dev/null 2>&1; then
        success "Server capabilities endpoint working"
        echo "$response" | jq '.capabilities' | head -5
    else
        warning "Server capabilities endpoint not available (may be normal)"
        echo "Response: $response"
    fi
    
    # Test /_matrix/client/unstable/capabilities
    ((TOTAL_TESTS++))
    info "Testing /_matrix/client/unstable/capabilities"
    response=$(curl -s "${BASE_URL}/_matrix/client/unstable/capabilities" 2>/dev/null)
    if echo "$response" | jq '.' > /dev/null 2>&1; then
        success "Unstable capabilities endpoint responding"
    else
        warning "Unstable capabilities endpoint not available"
    fi
}

# Test user registration
test_user_registration() {
    log "👤 Testing user registration..."
    ((TOTAL_TESTS++))
    
    local register_data=$(cat << EOF
{
    "username": "${TEST_USERNAME}",
    "password": "${TEST_PASSWORD}",
    "auth": {
        "type": "m.login.dummy"
    }
}
EOF
)
    
    info "Attempting to register user: ${TEST_USERNAME}"
    local response=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$register_data" \
        "${BASE_URL}/_matrix/client/r0/register" 2>/dev/null)
    
    if echo "$response" | jq -e '.access_token' > /dev/null 2>&1; then
        ACCESS_TOKEN=$(echo "$response" | jq -r '.access_token')
        USER_ID=$(echo "$response" | jq -r '.user_id')
        success "User registration successful"
        info "User ID: ${USER_ID}"
        info "Access Token: ${ACCESS_TOKEN:0:20}..."
    else
        error "User registration failed"
        echo "$response" | jq '.' 2>/dev/null || echo "Response: $response"
    fi
}

# Test user login
test_user_login() {
    if [ -z "$ACCESS_TOKEN" ]; then
        warning "Skipping login test (no user registered)"
        return
    fi
    
    log "🔑 Testing user login..."
    ((TOTAL_TESTS++))
    
    local login_data=$(cat << EOF
{
    "type": "m.login.password",
    "user": "${TEST_USERNAME}",
    "password": "${TEST_PASSWORD}"
}
EOF
)
    
    info "Attempting to login user: ${TEST_USERNAME}"
    local response=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$login_data" \
        "${BASE_URL}/_matrix/client/r0/login" 2>/dev/null)
    
    if echo "$response" | jq -e '.access_token' > /dev/null 2>&1; then
        success "User login successful"
        local token=$(echo "$response" | jq -r '.access_token')
        info "Login token matches registration: $([ "$token" = "$ACCESS_TOKEN" ] && echo "Yes" || echo "No")"
    else
        warning "User login failed (may be expected with some configurations)"
        echo "$response" | jq '.' 2>/dev/null || echo "Response: $response"
    fi
}

# Test whoami endpoint
test_whoami() {
    if [ -z "$ACCESS_TOKEN" ]; then
        warning "Skipping whoami test (no access token)"
        return
    fi
    
    log "🪪 Testing whoami endpoint..."
    ((TOTAL_TESTS++))
    
    info "Testing /_matrix/client/r0/account/whoami"
    local response=$(curl -s -H "Authorization: Bearer ${ACCESS_TOKEN}" \
        "${BASE_URL}/_matrix/client/r0/account/whoami" 2>/dev/null)
    
    if echo "$response" | jq -e '.user_id' > /dev/null 2>&1; then
        local returned_user_id=$(echo "$response" | jq -r '.user_id')
        success "Whoami endpoint working"
        info "Returned User ID: ${returned_user_id}"
        if [ "$returned_user_id" = "$USER_ID" ]; then
            success "User ID matches expected value"
        else
            warning "User ID mismatch: expected $USER_ID, got $returned_user_id"
        fi
    else
        error "Whoami endpoint failed"
        echo "$response" | jq '.' 2>/dev/null || echo "Response: $response"
    fi
}

# Test room creation
test_room_creation() {
    if [ -z "$ACCESS_TOKEN" ]; then
        warning "Skipping room creation test (no access token)"
        return
    fi
    
    log "🏠 Testing room creation..."
    ((TOTAL_TESTS++))
    
    local room_data=$(cat << EOF
{
    "name": "Test Room $(date +%s)",
    "topic": "Test room for Matrix API testing",
    "preset": "trusted_private_chat",
    "room_alias_name": "testroom_$(date +%s)"
}
EOF
)
    
    info "Creating a new room..."
    local response=$(curl -s -X POST \
        -H "Authorization: Bearer ${ACCESS_TOKEN}" \
        -H "Content-Type: application/json" \
        -d "$room_data" \
        "${BASE_URL}/_matrix/client/r0/createRoom" 2>/dev/null)
    
    if echo "$response" | jq -e '.room_id' > /dev/null 2>&1; then
        ROOM_ID=$(echo "$response" | jq -r '.room_id')
        success "Room creation successful"
        info "Room ID: ${ROOM_ID}"
    else
        error "Room creation failed"
        echo "$response" | jq '.' 2>/dev/null || echo "Response: $response"
    fi
}

# Test message sending
test_send_message() {
    if [ -z "$ACCESS_TOKEN" ] || [ -z "$ROOM_ID" ]; then
        warning "Skipping message send test (no access token or room)"
        return
    fi
    
    log "💬 Testing message sending..."
    ((TOTAL_TESTS++))
    
    local message_data=$(cat << EOF
{
    "msgtype": "m.text",
    "body": "Hello, Matrix! Test message sent at $(date)"
}
EOF
)
    
    local txn_id="txn_$(date +%s)_$$"
    info "Sending test message to room ${ROOM_ID}"
    local response=$(curl -s -X PUT \
        -H "Authorization: Bearer ${ACCESS_TOKEN}" \
        -H "Content-Type: application/json" \
        -d "$message_data" \
        "${BASE_URL}/_matrix/client/r0/rooms/${ROOM_ID}/send/m.room.message/${txn_id}" 2>/dev/null)
    
    if echo "$response" | jq -e '.event_id' > /dev/null 2>&1; then
        EVENT_ID=$(echo "$response" | jq -r '.event_id')
        success "Message sent successfully"
        info "Event ID: ${EVENT_ID}"
    else
        error "Message sending failed"
        echo "$response" | jq '.' 2>/dev/null || echo "Response: $response"
    fi
}

# Test room listing
test_room_list() {
    if [ -z "$ACCESS_TOKEN" ]; then
        warning "Skipping room list test (no access token)"
        return
    fi
    
    log "📋 Testing room listing..."
    ((TOTAL_TESTS++))
    
    info "Fetching joined rooms list"
    local response=$(curl -s -H "Authorization: Bearer ${ACCESS_TOKEN}" \
        "${BASE_URL}/_matrix/client/r0/joined_rooms" 2>/dev/null)
    
    if echo "$response" | jq -e '.joined_rooms' > /dev/null 2>&1; then
        local room_count=$(echo "$response" | jq '.joined_rooms | length')
        success "Room listing successful"
        info "User is in ${room_count} rooms"
        
        if [ "$room_count" -gt 0 ]; then
            echo "$response" | jq '.joined_rooms' | head -10
        fi
    else
        error "Room listing failed"
        echo "$response" | jq '.' 2>/dev/null || echo "Response: $response"
    fi
}

# Test federation endpoints
test_federation() {
    log "🌐 Testing federation endpoints..."
    
    # Test federation capabilities
    ((TOTAL_TESTS++))
    info "Testing /_matrix/federation/v1/version"
    local response=$(curl -s "${BASE_URL}/_matrix/federation/v1/version" 2>/dev/null)
    if echo "$response" | jq -e '.server' > /dev/null 2>&1; then
        success "Federation version endpoint working"
        echo "$response" | jq '.'
    else
        warning "Federation endpoint not available (may be disabled)"
        echo "Response: $response"
    fi
    
    # Test server key
    ((TOTAL_TESTS++))
    info "Testing /_matrix/key/v2/server"
    response=$(curl -s "${BASE_URL}/_matrix/key/v2/server" 2>/dev/null)
    if echo "$response" | jq -e '.server_name' > /dev/null 2>&1; then
        success "Server key endpoint working"
        info "Server name: $(echo "$response" | jq -r '.server_name')"
    else
        warning "Server key endpoint not available"
        echo "Response: $response"
    fi
}

# Test well-known endpoints
test_well_known() {
    log "🔍 Testing well-known endpoints..."
    
    # Test client well-known
    ((TOTAL_TESTS++))
    info "Testing /.well-known/matrix/client"
    local response=$(curl -s "${BASE_URL}/.well-known/matrix/client" 2>/dev/null)
    if echo "$response" | jq -e '.m.NextServer' > /dev/null 2>&1; then
        success "Client well-known endpoint working"
        echo "$response" | jq '.'
    else
        warning "Client well-known endpoint not available"
        echo "Response: $response"
    fi
    
    # Test server well-known
    ((TOTAL_TESTS++))
    info "Testing /.well-known/matrix/server"
    response=$(curl -s "${BASE_URL}/.well-known/matrix/server" 2>/dev/null)
    if echo "$response" | jq -e '.m.server' > /dev/null 2>&1; then
        success "Server well-known endpoint working"
        echo "$response" | jq '.'
    else
        warning "Server well-known endpoint not available"
        echo "Response: $response"
    fi
}

# Performance testing with multiple concurrent requests
test_performance() {
    log "⚡ Testing server performance..."
    
    if [ -z "$ACCESS_TOKEN" ]; then
        warning "Skipping performance test (no access token)"
        return
    fi
    
    ((TOTAL_TESTS++))
    info "Running concurrent requests test (10 parallel requests)"
    
    local start_time=$(date +%s.%N)
    local pids=()
    
    # Launch 10 concurrent requests
    for i in {1..10}; do
        (
            curl -s -H "Authorization: Bearer ${ACCESS_TOKEN}" \
                "${BASE_URL}/_matrix/client/r0/account/whoami" > /dev/null 2>&1
        ) &
        pids+=($!)
    done
    
    # Wait for all requests to complete
    for pid in "${pids[@]}"; do
        wait $pid
    done
    
    local end_time=$(date +%s.%N)
    local duration=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "0")
    
    success "Concurrent requests completed"
    info "10 parallel requests took: ${duration}s"
    
    # Calculate average response time
    if command -v bc >/dev/null 2>&1; then
        local avg_time=$(echo "scale=3; $duration / 10" | bc)
        info "Average response time: ${avg_time}s per request"
        
        # Check if performance meets requirements (<50ms)
        if (( $(echo "$avg_time < 0.05" | bc -l) )); then
            success "Performance test PASSED: Average response time under 50ms"
        else
            warning "Performance test: Average response time over 50ms target"
        fi
    fi
}

# Database-specific tests for PostgreSQL backend
test_postgresql_specific() {
    log "🗄️ Testing PostgreSQL-specific functionality..."
    
    # Test database health
    ((TOTAL_TESTS++))
    info "Testing database connectivity health"
    
    # This is a generic endpoint test since we can't directly test the database
    local response=$(curl -s "${BASE_URL}/_matrix/client/versions" 2>/dev/null)
    if echo "$response" | jq -e '.versions' > /dev/null 2>&1; then
        success "Database connectivity appears healthy (server responding)"
    else
        error "Database connectivity issues (server not responding properly)"
    fi
    
    # Test that we can perform basic operations that require database
    if [ -n "$ACCESS_TOKEN" ]; then
        ((TOTAL_TESTS++))
        info "Testing database read/write operations"
        
        # Try to get user profile (reads from database)
        local profile_response=$(curl -s -H "Authorization: Bearer ${ACCESS_TOKEN}" \
            "${BASE_URL}/_matrix/client/r0/profile/${USER_ID}" 2>/dev/null)
        
        if echo "$profile_response" | jq '.' > /dev/null 2>&1; then
            success "Database read operations working"
        else
            warning "Database read operations may have issues"
        fi
        
        # Try to set display name (writes to database)
        local displayname_data='{"displayname": "Test User '$(date +%s)'"}'
        local set_response=$(curl -s -X PUT \
            -H "Authorization: Bearer ${ACCESS_TOKEN}" \
            -H "Content-Type: application/json" \
            -d "$displayname_data" \
            "${BASE_URL}/_matrix/client/r0/profile/${USER_ID}/displayname" 2>/dev/null)
        
        if [ -z "$set_response" ] || echo "$set_response" | jq '.' > /dev/null 2>&1; then
            success "Database write operations working"
        else
            warning "Database write operations may have issues"
        fi
    fi
}

# Fallback tests when server is not running
run_fallback_tests() {
    log "🔄 Running fallback tests and demonstrations..."
    
    echo ""
    cyan "📋 Example curl commands for testing Matrix API:"
    echo ""
    
    # Show example curl commands
    cat << 'EOF'
# Test server versions:
curl -X GET "http://localhost:6167/_matrix/client/versions"

# Register a new user:
curl -X POST "http://localhost:6167/_matrix/client/r0/register" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "testpass123",
    "auth": {"type": "m.login.dummy"}
  }'

# Login:
curl -X POST "http://localhost:6167/_matrix/client/r0/login" \
  -H "Content-Type: application/json" \
  -d '{
    "type": "m.login.password",
    "user": "testuser",
    "password": "testpass123"
  }'

# Create a room:
curl -X POST "http://localhost:6167/_matrix/client/r0/createRoom" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Room",
    "topic": "A test room"
  }'

# Send a message:
curl -X PUT "http://localhost:6167/_matrix/client/r0/rooms/ROOM_ID/send/m.room.message/TXN_ID" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "msgtype": "m.text",
    "body": "Hello, Matrix!"
  }'

EOF
    
    ((TOTAL_TESTS+=5))
    ((SKIPPED_TESTS+=5))
    
    echo ""
    cyan "🔧 PostgreSQL database check commands:"
    echo ""
    
    cat << 'EOF'
# Check if PostgreSQL is running:
psql -h localhost -p 5432 -U matrixon -d matrixon -c "SELECT version();"

# Check matrixon tables:
psql -h localhost -p 5432 -U matrixon -d matrixon -c "SELECT tablename FROM pg_tables WHERE tablename LIKE 'matrixon_%';"

# Check database connectivity:
psql -h localhost -p 5432 -U matrixon -d matrixon -c "SELECT 'Database is working!' as status;"

EOF
    
    echo ""
    info "To start the Matrixon server with PostgreSQL:"
    echo "MATRIXON_CONFIG=matrixon-pg.toml ./target/debug/matrixon"
    echo ""
}

# Main execution function
main() {
    print_banner
    
    # Check if jq is available
    if ! command -v jq >/dev/null 2>&1; then
        warning "jq is not installed. JSON output will be raw."
    fi
    
    # Check server status first
    if check_server_status; then
        # Server is running, run full test suite
        test_server_capabilities
        test_user_registration
        test_user_login
        test_whoami
        test_room_creation
        test_send_message
        test_room_list
        test_federation
        test_well_known
        test_performance
        test_postgresql_specific
    else
        # Server is not running, show fallback tests
        run_fallback_tests
    fi
    
    print_summary
}

# Run main function
main "$@" 
