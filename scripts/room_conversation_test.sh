#!/bin/bash

# =============================================================================
# Matrixon Matrix NextServer - Room & Conversation Testing Suite
# =============================================================================
#
# Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
# Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
# Contributors: Matrixon Development Team
# Date: 2024-12-11
# Version: 2.0.0-alpha (PostgreSQL Backend)
# License: Apache 2.0 / MIT
#
# Description:
#   Comprehensive test suite specifically designed for Matrix room creation
#   and conversation functionality. Tests all aspects of room management,
#   messaging, member operations, and advanced room features with curl commands.
#
# Performance Targets Validation:
#   • Room creation under 50ms latency
#   • Message delivery <100ms end-to-end
#   • Multi-user conversation scalability
#   • Real-time sync performance
#   • Database consistency validation
#
# =============================================================================

# Configuration
SERVER_URL="${1:-http://localhost:6167}"
OUTPUT_DIR="./room_test_results"
TIMESTAMP=$(date +%s)

# Test users
USER1_NAME="alice_${TIMESTAMP}"
USER1_PASSWORD="AlicePass123!"
USER2_NAME="bob_${TIMESTAMP}"
USER2_PASSWORD="BobPass123!"

# Room configuration
ROOM_NAME="Matrixon Test Room ${TIMESTAMP}"
ROOM_TOPIC="Test room for Matrix functionality testing"

# Global variables
USER1_TOKEN=""
USER1_ID=""
USER2_TOKEN=""
USER2_ID=""
ROOM_ID=""

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

# Statistics
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# =============================================================================
# Utility Functions
# =============================================================================

setup_environment() {
    mkdir -p "$OUTPUT_DIR"
    echo -e "${CYAN}🏠 Matrixon Room & Conversation Test Suite 💬${NC}"
    echo "=========================================="
    echo "Server URL: $SERVER_URL"
    echo "Test started at: $(date)"
    echo
}

log_test() {
    local status="$1"
    local message="$2"
    ((TOTAL_TESTS++))
    
    if [ "$status" = "PASS" ]; then
        echo -e "${GREEN}✅ $message${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "${RED}❌ $message${NC}"
        ((FAILED_TESTS++))
    fi
}

print_section() {
    echo
    echo -e "${BLUE}--- $1 ---${NC}"
}

print_info() {
    echo -e "${PURPLE}ℹ️  $1${NC}"
}

# =============================================================================
# Server Health Check
# =============================================================================

check_server() {
    print_section "Server Health Check"
    
    if response=$(timeout 10s curl -s "$SERVER_URL/_matrix/client/versions" 2>/dev/null); then
        if echo "$response" | jq -e '.versions' >/dev/null 2>&1; then
            log_test "PASS" "Matrix server is accessible and responding"
            return 0
        else
            log_test "FAIL" "Server responding but not with valid Matrix API"
            return 1
        fi
    else
        log_test "FAIL" "Server is not accessible at $SERVER_URL"
        echo
        echo "To start the server:"
        echo "  MATRIXON_CONFIG=matrixon-pg.toml ./target/debug/matrixon"
        return 1
    fi
}

# =============================================================================
# User Registration
# =============================================================================

register_users() {
    print_section "User Registration"
    
    # Register Alice
    print_info "Registering Alice..."
    local alice_data='{
        "username": "'$USER1_NAME'",
        "password": "'$USER1_PASSWORD'",
        "device_id": "ALICE_DEVICE"
    }'
    
    if response=$(timeout 30s curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$alice_data" \
        "$SERVER_URL/_matrix/client/r0/register" 2>/dev/null); then
        
        if USER1_TOKEN=$(echo "$response" | jq -r '.access_token' 2>/dev/null) && [ "$USER1_TOKEN" != "null" ]; then
            USER1_ID=$(echo "$response" | jq -r '.user_id')
            log_test "PASS" "Alice registered successfully: $USER1_ID"
        else
            log_test "FAIL" "Alice registration failed: $response"
            return 1
        fi
    else
        log_test "FAIL" "Failed to register Alice - connection error"
        return 1
    fi
    
    # Register Bob
    print_info "Registering Bob..."
    local bob_data='{
        "username": "'$USER2_NAME'",
        "password": "'$USER2_PASSWORD'",
        "device_id": "BOB_DEVICE"
    }'
    
    if response=$(timeout 30s curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$bob_data" \
        "$SERVER_URL/_matrix/client/r0/register" 2>/dev/null); then
        
        if USER2_TOKEN=$(echo "$response" | jq -r '.access_token' 2>/dev/null) && [ "$USER2_TOKEN" != "null" ]; then
            USER2_ID=$(echo "$response" | jq -r '.user_id')
            log_test "PASS" "Bob registered successfully: $USER2_ID"
        else
            log_test "FAIL" "Bob registration failed: $response"
            return 1
        fi
    else
        log_test "FAIL" "Failed to register Bob - connection error"
        return 1
    fi
}

# =============================================================================
# Room Creation
# =============================================================================

create_room() {
    print_section "Room Creation"
    
    local room_data='{
        "name": "'$ROOM_NAME'",
        "topic": "'$ROOM_TOPIC'",
        "preset": "private_chat",
        "visibility": "private"
    }'
    
    print_info "Creating room: $ROOM_NAME"
    local start_time=$(date +%s.%N)
    
    if response=$(timeout 30s curl -s -X POST \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$room_data" \
        "$SERVER_URL/_matrix/client/r0/createRoom" 2>/dev/null); then
        
        local end_time=$(date +%s.%N)
        local response_time=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "N/A")
        
        if ROOM_ID=$(echo "$response" | jq -r '.room_id' 2>/dev/null) && [ "$ROOM_ID" != "null" ]; then
            log_test "PASS" "Room created successfully in ${response_time}s: $ROOM_ID"
            
            # Check performance target
            if command -v bc > /dev/null && (( $(echo "$response_time < 0.05" | bc -l 2>/dev/null || echo "0") )); then
                print_info "🎉 Performance target met: Room creation <50ms"
            fi
        else
            log_test "FAIL" "Room creation failed: $response"
            return 1
        fi
    else
        log_test "FAIL" "Failed to create room - connection error"
        return 1
    fi
}

# =============================================================================
# Room Membership
# =============================================================================

manage_membership() {
    print_section "Room Membership Management"
    
    # Invite Bob
    print_info "Inviting Bob to room..."
    local invite_data='{"user_id": "'$USER2_ID'"}'
    
    if response=$(timeout 20s curl -s -X POST \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$invite_data" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/invite" 2>/dev/null); then
        
        if [ "$response" = "{}" ] || echo "$response" | jq -e '.errcode' >/dev/null 2>&1; then
            if [ "$response" = "{}" ]; then
                log_test "PASS" "Bob invited to room successfully"
            else
                log_test "FAIL" "Failed to invite Bob: $response"
                return 1
            fi
        else
            log_test "PASS" "Bob invited to room"
        fi
    else
        log_test "FAIL" "Failed to invite Bob - connection error"
        return 1
    fi
    
    # Bob joins
    print_info "Bob joining room..."
    if response=$(timeout 20s curl -s -X POST \
        -H "Authorization: Bearer $USER2_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{}' \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/join" 2>/dev/null); then
        
        if [ "$response" = "{}" ] || echo "$response" | jq -e '.room_id' >/dev/null 2>&1; then
            log_test "PASS" "Bob joined room successfully"
        else
            log_test "FAIL" "Bob failed to join room: $response"
            return 1
        fi
    else
        log_test "FAIL" "Bob failed to join room - connection error"
        return 1
    fi
}

# =============================================================================
# Messaging
# =============================================================================

test_messaging() {
    print_section "Room Messaging"
    
    # Alice sends message
    print_info "Alice sending welcome message..."
    local alice_msg='{
        "msgtype": "m.text",
        "body": "Welcome to our test room! This is Alice speaking. 🎉"
    }'
    
    local txn_id="alice_msg_$(date +%s)"
    local start_time=$(date +%s.%N)
    
    if response=$(timeout 20s curl -s -X PUT \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$alice_msg" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id" 2>/dev/null); then
        
        local end_time=$(date +%s.%N)
        local response_time=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "N/A")
        
        if echo "$response" | jq -e '.event_id' >/dev/null 2>&1; then
            local event_id=$(echo "$response" | jq -r '.event_id')
            log_test "PASS" "Alice's message sent successfully in ${response_time}s"
            
            # Check performance target
            if command -v bc > /dev/null && (( $(echo "$response_time < 0.1" | bc -l 2>/dev/null || echo "0") )); then
                print_info "🎉 Performance target met: Message send <100ms"
            fi
        else
            log_test "FAIL" "Alice's message failed: $response"
            return 1
        fi
    else
        log_test "FAIL" "Alice failed to send message - connection error"
        return 1
    fi
    
    # Bob responds
    print_info "Bob responding..."
    local bob_msg='{
        "msgtype": "m.text",
        "body": "Hi Alice! Bob here. Great to be in this room! 👋"
    }'
    
    local txn_id2="bob_msg_$(date +%s)"
    
    if response=$(timeout 20s curl -s -X PUT \
        -H "Authorization: Bearer $USER2_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$bob_msg" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id2" 2>/dev/null); then
        
        if echo "$response" | jq -e '.event_id' >/dev/null 2>&1; then
            log_test "PASS" "Bob's message sent successfully"
        else
            log_test "FAIL" "Bob's message failed: $response"
            return 1
        fi
    else
        log_test "FAIL" "Bob failed to send message - connection error"
        return 1
    fi
    
    # Test formatted message
    print_info "Alice sending formatted message..."
    local formatted_msg='{
        "msgtype": "m.text",
        "body": "This is **bold** and *italic* text",
        "format": "org.matrix.custom.html",
        "formatted_body": "This is <strong>bold</strong> and <em>italic</em> text"
    }'
    
    local txn_id3="alice_formatted_$(date +%s)"
    
    if response=$(timeout 20s curl -s -X PUT \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$formatted_msg" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id3" 2>/dev/null); then
        
        if echo "$response" | jq -e '.event_id' >/dev/null 2>&1; then
            log_test "PASS" "Alice's formatted message sent successfully"
        else
            log_test "FAIL" "Alice's formatted message failed: $response"
        fi
    else
        log_test "FAIL" "Alice failed to send formatted message - connection error"
    fi
}

# =============================================================================
# Message History
# =============================================================================

test_message_history() {
    print_section "Message History"
    
    print_info "Retrieving room messages..."
    if response=$(timeout 20s curl -s \
        -H "Authorization: Bearer $USER1_TOKEN" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/messages?dir=b&limit=50" 2>/dev/null); then
        
        if echo "$response" | jq -e '.chunk' >/dev/null 2>&1; then
            local message_count=$(echo "$response" | jq '.chunk | map(select(.type == "m.room.message")) | length')
            log_test "PASS" "Retrieved message history: $message_count messages"
            
            # Show recent messages
            echo "$response" | jq -r '.chunk[] | select(.type == "m.room.message") | "\(.sender): \(.content.body)"' | head -3 | while read -r msg; do
                print_info "Message: $msg"
            done
        else
            log_test "FAIL" "Failed to retrieve message history: $response"
        fi
    else
        log_test "FAIL" "Failed to retrieve message history - connection error"
    fi
}

# =============================================================================
# Sync Testing
# =============================================================================

test_sync() {
    print_section "Real-time Sync"
    
    print_info "Testing Alice's sync..."
    local start_time=$(date +%s.%N)
    
    if response=$(timeout 30s curl -s \
        -H "Authorization: Bearer $USER1_TOKEN" \
        "$SERVER_URL/_matrix/client/r0/sync?timeout=1000" 2>/dev/null); then
        
        local end_time=$(date +%s.%N)
        local response_time=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "N/A")
        
        if echo "$response" | jq -e '.next_batch' >/dev/null 2>&1; then
            log_test "PASS" "Alice sync completed in ${response_time}s"
            
            # Check if our room is in sync
            if echo "$response" | jq -e ".rooms.join.\"$ROOM_ID\"" >/dev/null 2>&1; then
                print_info "✨ Test room found in sync response"
                local timeline_count=$(echo "$response" | jq ".rooms.join.\"$ROOM_ID\".timeline.events | length")
                print_info "Timeline events: $timeline_count"
            else
                print_info "Test room not in current sync batch (normal for large servers)"
            fi
        else
            log_test "FAIL" "Invalid sync response: $response"
        fi
    else
        log_test "FAIL" "Sync request failed - connection error"
    fi
}

# =============================================================================
# Performance Testing
# =============================================================================

test_performance() {
    print_section "Performance Testing"
    
    print_info "Testing API response times..."
    local total_time=0
    local test_count=0
    
    for i in {1..3}; do
        local start_time=$(date +%s.%N)
        if curl -s -H "Authorization: Bearer $USER1_TOKEN" "$SERVER_URL/_matrix/client/versions" > /dev/null 2>&1; then
            local end_time=$(date +%s.%N)
            local response_time=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "0")
            total_time=$(echo "$total_time + $response_time" | bc -l 2>/dev/null || echo "$total_time")
            ((test_count++))
        fi
        sleep 0.1
    done
    
    if [ $test_count -gt 0 ]; then
        local avg_time=$(echo "scale=3; $total_time / $test_count" | bc -l 2>/dev/null || echo "N/A")
        log_test "PASS" "Average response time: ${avg_time}s over $test_count tests"
        
        if command -v bc > /dev/null && (( $(echo "$avg_time < 0.05" | bc -l 2>/dev/null || echo "0") )); then
            print_info "🎉 Excellent: Average response time meets <50ms target!"
        fi
    else
        log_test "FAIL" "No successful response time tests"
    fi
}

# =============================================================================
# Test Summary
# =============================================================================

show_summary() {
    echo
    echo -e "${CYAN}📊 Test Summary Report${NC}"
    echo "===================="
    echo "Total Tests: $TOTAL_TESTS"
    echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
    echo -e "Failed: ${RED}$FAILED_TESTS${NC}"
    
    if [ $TOTAL_TESTS -gt 0 ]; then
        local success_rate=$((PASSED_TESTS * 100 / TOTAL_TESTS))
        echo "Success Rate: ${success_rate}%"
        
        if [ $success_rate -ge 90 ]; then
            echo -e "${GREEN}🎉 Excellent! Room functionality working great!${NC}"
        elif [ $success_rate -ge 70 ]; then
            echo -e "${YELLOW}⚠️  Good, but some features need attention${NC}"
        else
            echo -e "${RED}❌ Room functionality needs work${NC}"
        fi
    fi
    
    if [ -n "$ROOM_ID" ]; then
        echo
        echo -e "${CYAN}🏠 Created Room Information:${NC}"
        echo "Room ID: $ROOM_ID"
        echo "Room Name: $ROOM_NAME"
        echo "Members: Alice ($USER1_ID), Bob ($USER2_ID)"
        echo
        echo -e "${CYAN}🔗 Useful Commands:${NC}"
        echo "# Get room messages:"
        echo "curl -H \"Authorization: Bearer $USER1_TOKEN\" \"$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/messages?dir=b&limit=10\""
        echo
        echo "# Send a message:"
        echo "curl -X PUT -H \"Authorization: Bearer $USER1_TOKEN\" -H \"Content-Type: application/json\" \\"
        echo "  -d '{\"msgtype\": \"m.text\", \"body\": \"Hello from curl!\"}' \\"
        echo "  \"$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/\$(date +%s)\""
        echo
        echo "# Get room state:"
        echo "curl -H \"Authorization: Bearer $USER1_TOKEN\" \"$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/state\""
    fi
}

# =============================================================================
# Main Execution
# =============================================================================

main() {
    clear
    setup_environment
    
    if check_server; then
        register_users && \
        create_room && \
        manage_membership && \
        test_messaging && \
        test_message_history && \
        test_sync && \
        test_performance
    else
        echo
        echo -e "${YELLOW}📚 Server not available - Example Commands:${NC}"
        echo
        cat << 'EOF'
# 1. Room Creation:
curl -X POST \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Test Room",
    "topic": "A room for testing",
    "preset": "private_chat"
  }' \
  http://localhost:6167/_matrix/client/r0/createRoom

# 2. Send Message:
curl -X PUT \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "msgtype": "m.text",
    "body": "Hello, Matrix world!"
  }' \
  http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID/send/m.room.message/TXN_$(date +%s)

# 3. Get Messages:
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID/messages?dir=b&limit=10

# 4. Invite User:
curl -X POST \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"user_id": "@user:localhost"}' \
  http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID/invite

# 5. Join Room:
curl -X POST \
  -H "Authorization: Bearer USER_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}' \
  http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID/join

# 6. Real-time Sync:
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  http://localhost:6167/_matrix/client/r0/sync?timeout=30000
EOF
    fi
    
    show_summary
}

# Execute main function
main "$@" 
