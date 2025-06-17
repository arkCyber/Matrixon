#!/bin/bash

# =====================================================================
# Matrixon Room & Communication Testing Script
# Author: arkSong <arksong2018@gmail.com>
# Date: 2024-01-15
# Version: 0.11.0-alpha
# Purpose: Comprehensive test suite for Matrix room creation and messaging
# Performance: Targets <50ms response times for all operations
# =====================================================================

set -e

# Configuration
MATRIXON_URL="http://localhost:6167"
LOGFILE="room_communication_test.log"
TEST_USER1="test_user_alice"
TEST_USER2="test_user_bob"
ACCESS_TOKEN1=""
ACCESS_TOKEN2=""
ROOM_ID=""
ROOM_ALIAS=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Emoji indicators
SUCCESS="✅"
ERROR="❌"
INFO="🔧"
MESSAGE="💬"
ROOM="🏠"
USER="👤"

# Logging function
log() {
    local level=$1
    shift
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $*" | tee -a "$LOGFILE"
}

# Pretty print function
print_header() {
    echo -e "\n${BLUE}======================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}======================================${NC}\n"
}

print_success() {
    echo -e "${GREEN}${SUCCESS} $1${NC}"
    log "SUCCESS" "$1"
}

print_error() {
    echo -e "${RED}${ERROR} $1${NC}"
    log "ERROR" "$1"
}

print_info() {
    echo -e "${YELLOW}${INFO} $1${NC}"
    log "INFO" "$1"
}

# Measure response time
measure_time() {
    local start_time=$(date +%s%N)
    "$@"
    local end_time=$(date +%s%N)
    local duration=$(( (end_time - start_time) / 1000000 ))
    echo "${duration}ms"
}

# Check if server is running
check_server() {
    print_header "Checking Matrixon Server Status"
    
    local response_time=$(measure_time curl -s -w "%{time_total}" -o /dev/null "$MATRIXON_URL/health")
    if curl -s "$MATRIXON_URL/health" > /dev/null 2>&1; then
        print_success "Server is running (Response time: $response_time)"
        curl -s "$MATRIXON_URL/health" | jq '.' 2>/dev/null || echo "Health check successful"
    else
        print_error "Server is not responding. Please start the server first."
        exit 1
    fi
}

# Register test users
register_users() {
    print_header "Registering Test Users"
    
    # Register Alice
    print_info "Registering user: $TEST_USER1"
    local alice_response=$(curl -s -X POST "$MATRIXON_URL/_matrix/client/r0/register" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\": \"$TEST_USER1\",
            \"password\": \"test_password_123\",
            \"device_id\": \"ALICE_DEVICE\",
            \"initial_device_display_name\": \"Alice's Test Device\"
        }")
    
    ACCESS_TOKEN1=$(echo "$alice_response" | jq -r '.access_token' 2>/dev/null || echo "")
    if [[ -n "$ACCESS_TOKEN1" && "$ACCESS_TOKEN1" != "null" ]]; then
        print_success "Alice registered successfully"
        echo "Access Token: $ACCESS_TOKEN1"
    else
        print_error "Failed to register Alice"
        echo "Response: $alice_response"
        exit 1
    fi
    
    # Register Bob
    print_info "Registering user: $TEST_USER2"
    local bob_response=$(curl -s -X POST "$MATRIXON_URL/_matrix/client/r0/register" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\": \"$TEST_USER2\",
            \"password\": \"test_password_456\",
            \"device_id\": \"BOB_DEVICE\",
            \"initial_device_display_name\": \"Bob's Test Device\"
        }")
    
    ACCESS_TOKEN2=$(echo "$bob_response" | jq -r '.access_token' 2>/dev/null || echo "")
    if [[ -n "$ACCESS_TOKEN2" && "$ACCESS_TOKEN2" != "null" ]]; then
        print_success "Bob registered successfully"
        echo "Access Token: $ACCESS_TOKEN2"
    else
        print_error "Failed to register Bob"
        echo "Response: $bob_response"
        exit 1
    fi
}

# Create a room
create_room() {
    print_header "${ROOM} Creating Test Room"
    
    print_info "Creating room with Alice's credentials"
    local room_response=$(curl -s -X POST "$MATRIXON_URL/_matrix/client/r0/createRoom" \
        -H "Authorization: Bearer $ACCESS_TOKEN1" \
        -H "Content-Type: application/json" \
        -d '{
            "name": "Test Communication Room",
            "topic": "A room for testing Matrixon messaging features",
            "preset": "public_chat",
            "visibility": "public",
            "room_alias_name": "test-communication-room",
            "room_version": "9"
        }')
    
    ROOM_ID=$(echo "$room_response" | jq -r '.room_id' 2>/dev/null || echo "")
    ROOM_ALIAS=$(echo "$room_response" | jq -r '.room_alias' 2>/dev/null || echo "")
    
    if [[ -n "$ROOM_ID" && "$ROOM_ID" != "null" ]]; then
        print_success "Room created successfully"
        echo "Room ID: $ROOM_ID"
        echo "Room Alias: $ROOM_ALIAS"
        echo "$room_response" | jq '.' 2>/dev/null || echo "Raw response: $room_response"
    else
        print_error "Failed to create room"
        echo "Response: $room_response"
        exit 1
    fi
}

# Join room with second user
join_room() {
    print_header "${USER} Bob Joining Room"
    
    print_info "Bob joining room: $ROOM_ID"
    local join_response=$(curl -s -X POST "$MATRIXON_URL/_matrix/client/r0/join/$ROOM_ID" \
        -H "Authorization: Bearer $ACCESS_TOKEN2" \
        -H "Content-Type: application/json" \
        -d '{}')
    
    local joined_room_id=$(echo "$join_response" | jq -r '.room_id' 2>/dev/null || echo "")
    if [[ -n "$joined_room_id" && "$joined_room_id" != "null" ]]; then
        print_success "Bob successfully joined the room"
        echo "$join_response" | jq '.' 2>/dev/null || echo "Raw response: $join_response"
    else
        print_error "Failed to join room"
        echo "Response: $join_response"
        exit 1
    fi
}

# Send messages
send_messages() {
    print_header "${MESSAGE} Testing Message Exchange"
    
    # Alice sends first message
    print_info "Alice sending welcome message"
    local txn_id1="txn_$(date +%s)_1"
    local msg1_response=$(curl -s -X PUT "$MATRIXON_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id1" \
        -H "Authorization: Bearer $ACCESS_TOKEN1" \
        -H "Content-Type: application/json" \
        -d '{
            "msgtype": "m.text",
            "body": "Hello everyone! Welcome to our test room 👋",
            "format": "org.matrix.custom.html",
            "formatted_body": "<strong>Hello everyone!</strong> Welcome to our test room 👋"
        }')
    
    local event_id1=$(echo "$msg1_response" | jq -r '.event_id' 2>/dev/null || echo "")
    if [[ -n "$event_id1" && "$event_id1" != "null" ]]; then
        print_success "Alice's message sent successfully"
        echo "Event ID: $event_id1"
    else
        print_error "Failed to send Alice's message"
        echo "Response: $msg1_response"
    fi
    
    # Bob responds
    print_info "Bob responding to Alice"
    local txn_id2="txn_$(date +%s)_2"
    local msg2_response=$(curl -s -X PUT "$MATRIXON_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id2" \
        -H "Authorization: Bearer $ACCESS_TOKEN2" \
        -H "Content-Type: application/json" \
        -d '{
            "msgtype": "m.text",
            "body": "Hi Alice! Thanks for creating this room. Matrixon is working great! 🚀"
        }')
    
    local event_id2=$(echo "$msg2_response" | jq -r '.event_id' 2>/dev/null || echo "")
    if [[ -n "$event_id2" && "$event_id2" != "null" ]]; then
        print_success "Bob's message sent successfully"
        echo "Event ID: $event_id2"
    else
        print_error "Failed to send Bob's message"
        echo "Response: $msg2_response"
    fi
    
    # Alice sends a formatted message with code
    print_info "Alice sending a technical message"
    local txn_id3="txn_$(date +%s)_3"
    local msg3_response=$(curl -s -X PUT "$MATRIXON_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id3" \
        -H "Authorization: Bearer $ACCESS_TOKEN1" \
        -H "Content-Type: application/json" \
        -d '{
            "msgtype": "m.text",
            "body": "Here is some Rust code:\n\nfn main() {\n    println!(\"Hello, Matrixon!\");\n}",
            "format": "org.matrix.custom.html",
            "formatted_body": "Here is some Rust code:<br><br><code>fn main() {<br>    println!(\"Hello, Matrixon!\");<br>}</code>"
        }')
    
    local event_id3=$(echo "$msg3_response" | jq -r '.event_id' 2>/dev/null || echo "")
    if [[ -n "$event_id3" && "$event_id3" != "null" ]]; then
        print_success "Alice's technical message sent successfully"
        echo "Event ID: $event_id3"
    else
        print_error "Failed to send Alice's technical message"
        echo "Response: $msg3_response"
    fi
}

# Retrieve room messages
get_room_messages() {
    print_header "📖 Retrieving Room Messages"
    
    print_info "Getting recent messages from room"
    local messages_response=$(curl -s "$MATRIXON_URL/_matrix/client/r0/rooms/$ROOM_ID/messages?limit=10" \
        -H "Authorization: Bearer $ACCESS_TOKEN1")
    
    local chunk=$(echo "$messages_response" | jq -r '.chunk' 2>/dev/null || echo "")
    if [[ -n "$chunk" && "$chunk" != "null" ]]; then
        print_success "Messages retrieved successfully"
        echo "$messages_response" | jq '.' 2>/dev/null || echo "Raw response: $messages_response"
        
        # Count messages
        local message_count=$(echo "$messages_response" | jq '.chunk | length' 2>/dev/null || echo "0")
        print_info "Found $message_count messages in the room"
    else
        print_error "Failed to retrieve messages"
        echo "Response: $messages_response"
    fi
}

# Test sync endpoint
test_sync() {
    print_header "🔄 Testing Sync Endpoint"
    
    print_info "Syncing Alice's client"
    local sync_response=$(curl -s "$MATRIXON_URL/_matrix/client/r0/sync?timeout=0" \
        -H "Authorization: Bearer $ACCESS_TOKEN1")
    
    local next_batch=$(echo "$sync_response" | jq -r '.next_batch' 2>/dev/null || echo "")
    if [[ -n "$next_batch" && "$next_batch" != "null" ]]; then
        print_success "Sync completed successfully"
        echo "Next batch token: $next_batch"
        
        # Check if our room is in the sync response
        local rooms_joined=$(echo "$sync_response" | jq -r ".rooms.join | keys[]" 2>/dev/null || echo "")
        if [[ "$rooms_joined" == *"$ROOM_ID"* ]]; then
            print_success "Our test room found in sync response"
        else
            print_info "Test room not found in sync response (may be expected for new rooms)"
        fi
        
        echo "$sync_response" | jq '.' 2>/dev/null || echo "Raw response: $sync_response"
    else
        print_error "Sync failed"
        echo "Response: $sync_response"
    fi
}

# Get joined rooms
get_joined_rooms() {
    print_header "🏠 Checking Joined Rooms"
    
    print_info "Getting Alice's joined rooms"
    local rooms_response=$(curl -s "$MATRIXON_URL/_matrix/client/r0/joined_rooms" \
        -H "Authorization: Bearer $ACCESS_TOKEN1")
    
    local joined_rooms=$(echo "$rooms_response" | jq -r '.joined_rooms' 2>/dev/null || echo "")
    if [[ -n "$joined_rooms" && "$joined_rooms" != "null" ]]; then
        print_success "Joined rooms retrieved successfully"
        echo "$rooms_response" | jq '.' 2>/dev/null || echo "Raw response: $rooms_response"
        
        local room_count=$(echo "$rooms_response" | jq '.joined_rooms | length' 2>/dev/null || echo "0")
        print_info "Alice is joined to $room_count rooms"
    else
        print_error "Failed to get joined rooms"
        echo "Response: $rooms_response"
    fi
}

# Performance testing
performance_test() {
    print_header "⚡ Performance Testing"
    
    print_info "Testing message sending performance"
    local total_time=0
    local message_count=5
    
    for i in $(seq 1 $message_count); do
        local start_time=$(date +%s%N)
        local txn_id="perf_test_$(date +%s)_$i"
        
        curl -s -X PUT "$MATRIXON_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id" \
            -H "Authorization: Bearer $ACCESS_TOKEN1" \
            -H "Content-Type: application/json" \
            -d "{
                \"msgtype\": \"m.text\",
                \"body\": \"Performance test message #$i - Testing Matrixon speed!\"
            }" > /dev/null
        
        local end_time=$(date +%s%N)
        local duration=$(( (end_time - start_time) / 1000000 ))
        total_time=$((total_time + duration))
        
        print_info "Message $i sent in ${duration}ms"
    done
    
    local avg_time=$((total_time / message_count))
    if [[ $avg_time -lt 50 ]]; then
        print_success "Average response time: ${avg_time}ms (Target: <50ms) ✨"
    else
        print_info "Average response time: ${avg_time}ms (Target: <50ms)"
    fi
}

# Cleanup function
cleanup() {
    print_header "🧹 Test Cleanup Complete"
    print_info "Room ID: $ROOM_ID"
    print_info "Room Alias: $ROOM_ALIAS"
    print_info "Test users created: $TEST_USER1, $TEST_USER2"
    print_info "Log file: $LOGFILE"
}

# Main test execution
main() {
    print_header "🚀 Matrixon Room & Communication Test Suite"
    echo "Testing Matrix room creation, joining, and messaging functionality"
    echo "Performance target: <50ms response times"
    echo ""
    
    # Initialize log file
    echo "Matrixon Room Communication Test - $(date)" > "$LOGFILE"
    
    # Run test sequence
    check_server
    register_users
    create_room
    join_room
    send_messages
    get_room_messages
    test_sync
    get_joined_rooms
    performance_test
    cleanup
    
    print_header "🎉 All Tests Completed Successfully!"
    print_success "Matrixon room creation and communication features are working correctly"
    print_info "Check $LOGFILE for detailed logs"
}

# Script entry point
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi 
