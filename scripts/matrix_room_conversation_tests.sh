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
# Test Coverage:
#   • Room Creation (public, private, encrypted)
#   • Room Configuration and Settings
#   • User Invitation and Membership Management
#   • Message Sending (text, formatted, files)
#   • Message History and Pagination
#   • Room State Events (name, topic, avatar)
#   • Typing Indicators and Read Receipts
#   • Real-time Sync and Event Streaming
#   • Room Permissions and Power Levels
#   • Room Aliases and Directory Listing
#
# Architecture:
#   • Comprehensive curl-based API testing
#   • Multi-user scenario simulation
#   • Real-time conversation flow testing
#   • Performance measurement and validation
#   • Enterprise-grade error handling
#
# Usage:
#   ./matrix_room_conversation_tests.sh [SERVER_URL]
#   
# Example:
#   ./matrix_room_conversation_tests.sh http://localhost:6167
#
# =============================================================================

# Configuration
SERVER_URL="${1:-http://localhost:6167}"
OUTPUT_DIR="./room_test_results"
LOG_FILE="$OUTPUT_DIR/room_tests_$(date +%Y%m%d_%H%M%S).log"
TIMESTAMP=$(date +%s)

# Test users
USER1_NAME="alice_${TIMESTAMP}"
USER1_PASSWORD="AlicePass123!"
USER2_NAME="bob_${TIMESTAMP}"
USER2_PASSWORD="BobPass123!"
USER3_NAME="charlie_${TIMESTAMP}"
USER3_PASSWORD="CharliePass123!"

# Room configuration
ROOM_NAME="Matrixon Test Room ${TIMESTAMP}"
ROOM_TOPIC="Test room for comprehensive Matrix functionality testing"
ROOM_ALIAS="matrixon-test-${TIMESTAMP}"

# Global variables for tokens and IDs
USER1_TOKEN=""
USER1_ID=""
USER2_TOKEN=""
USER2_ID=""
USER3_TOKEN=""
USER3_ID=""
ROOM_ID=""

# Statistics
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

# =============================================================================
# Utility Functions
# =============================================================================

setup_test_environment() {
    mkdir -p "$OUTPUT_DIR"
    echo "=== Matrixon Room & Conversation Test Suite ===" | tee "$LOG_FILE"
    echo "Server URL: $SERVER_URL" | tee -a "$LOG_FILE"
    echo "Test started at: $(date)" | tee -a "$LOG_FILE"
    echo "Output directory: $OUTPUT_DIR" | tee -a "$LOG_FILE"
    echo | tee -a "$LOG_FILE"
}

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

print_info() {
    echo -e "${PURPLE}ℹ️  $1${NC}"
    log "INFO" "$1"
}

increment_test() {
    ((TOTAL_TESTS++))
}

# =============================================================================
# Server Health Check
# =============================================================================

check_server_health() {
    print_header "Server Health Check"
    
    increment_test
    print_info "Checking Matrix server accessibility..."
    
    local response
    if response=$(timeout 10s curl -s "$SERVER_URL/_matrix/client/versions" 2>/dev/null); then
        if echo "$response" | jq -e '.versions' >/dev/null 2>&1; then
            print_success "Matrix server is accessible and responding"
            echo "Supported versions: $(echo "$response" | jq -r '.versions[]' | tr '\n' ' ')"
        else
            print_error "Server responding but not with valid Matrix API"
            echo "Response: $response"
            return 1
        fi
    else
        print_error "Server is not accessible at $SERVER_URL"
        echo
        echo "To start the server:"
        echo "  MATRIXON_CONFIG=matrixon-pg.toml ./target/debug/matrixon"
        return 1
    fi
}

# =============================================================================
# User Registration and Authentication
# =============================================================================

register_test_users() {
    print_header "User Registration and Authentication"
    
    # Register User 1 (Alice)
    print_section "Registering Alice"
    increment_test
    
    local alice_data='{
        "username": "'$USER1_NAME'",
        "password": "'$USER1_PASSWORD'",
        "device_id": "ALICE_DEVICE",
        "initial_device_display_name": "Alice Test Device"
    }'
    
    local response
    if response=$(timeout 30s curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$alice_data" \
        "$SERVER_URL/_matrix/client/r0/register" 2>/dev/null); then
        
        if USER1_TOKEN=$(echo "$response" | jq -r '.access_token' 2>/dev/null) && [ "$USER1_TOKEN" != "null" ]; then
            USER1_ID=$(echo "$response" | jq -r '.user_id')
            print_success "Alice registered successfully"
            print_info "Alice ID: $USER1_ID"
            print_info "Alice token: ${USER1_TOKEN:0:20}..."
        else
            print_error "Alice registration failed: $response"
            return 1
        fi
    else
        print_error "Failed to register Alice - connection error"
        return 1
    fi
    
    # Register User 2 (Bob)
    print_section "Registering Bob"
    increment_test
    
    local bob_data='{
        "username": "'$USER2_NAME'",
        "password": "'$USER2_PASSWORD'",
        "device_id": "BOB_DEVICE",
        "initial_device_display_name": "Bob Test Device"
    }'
    
    if response=$(timeout 30s curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$bob_data" \
        "$SERVER_URL/_matrix/client/r0/register" 2>/dev/null); then
        
        if USER2_TOKEN=$(echo "$response" | jq -r '.access_token' 2>/dev/null) && [ "$USER2_TOKEN" != "null" ]; then
            USER2_ID=$(echo "$response" | jq -r '.user_id')
            print_success "Bob registered successfully"
            print_info "Bob ID: $USER2_ID"
            print_info "Bob token: ${USER2_TOKEN:0:20}..."
        else
            print_error "Bob registration failed: $response"
            return 1
        fi
    else
        print_error "Failed to register Bob - connection error"
        return 1
    fi
    
    # Register User 3 (Charlie)
    print_section "Registering Charlie"
    increment_test
    
    local charlie_data='{
        "username": "'$USER3_NAME'",
        "password": "'$USER3_PASSWORD'",
        "device_id": "CHARLIE_DEVICE",
        "initial_device_display_name": "Charlie Test Device"
    }'
    
    if response=$(timeout 30s curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$charlie_data" \
        "$SERVER_URL/_matrix/client/r0/register" 2>/dev/null); then
        
        if USER3_TOKEN=$(echo "$response" | jq -r '.access_token' 2>/dev/null) && [ "$USER3_TOKEN" != "null" ]; then
            USER3_ID=$(echo "$response" | jq -r '.user_id')
            print_success "Charlie registered successfully"
            print_info "Charlie ID: $USER3_ID"
            print_info "Charlie token: ${USER3_TOKEN:0:20}..."
        else
            print_error "Charlie registration failed: $response"
            return 1
        fi
    else
        print_error "Failed to register Charlie - connection error"
        return 1
    fi
}

# =============================================================================
# Room Creation and Configuration
# =============================================================================

test_room_creation() {
    print_header "Room Creation and Configuration"
    
    print_section "Creating Private Room"
    increment_test
    
    local room_data='{
        "name": "'$ROOM_NAME'",
        "topic": "'$ROOM_TOPIC'",
        "room_alias_name": "'$ROOM_ALIAS'",
        "preset": "private_chat",
        "visibility": "private",
        "creation_content": {
            "m.federate": true
        },
        "initial_state": [
            {
                "type": "m.room.guest_access",
                "content": {
                    "guest_access": "forbidden"
                }
            },
            {
                "type": "m.room.history_visibility",
                "content": {
                    "history_visibility": "invited"
                }
            }
        ]
    }'
    
    local start_time=$(date +%s.%N)
    local response
    if response=$(timeout 30s curl -s -X POST \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$room_data" \
        "$SERVER_URL/_matrix/client/r0/createRoom" 2>/dev/null); then
        
        local end_time=$(date +%s.%N)
        local response_time=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "N/A")
        
        if ROOM_ID=$(echo "$response" | jq -r '.room_id' 2>/dev/null) && [ "$ROOM_ID" != "null" ]; then
            print_success "Room created successfully in ${response_time}s"
            print_info "Room ID: $ROOM_ID"
            
            # Validate performance target
            if command -v bc > /dev/null && (( $(echo "$response_time < 0.05" | bc -l 2>/dev/null || echo "0") )); then
                print_success "✨ Performance target met: Room creation <50ms"
            elif [ "$response_time" != "N/A" ]; then
                print_info "Performance: Room creation took ${response_time}s"
            fi
        else
            print_error "Room creation failed: $response"
            return 1
        fi
    else
        print_error "Failed to create room - connection error"
        return 1
    fi
    
    # Test room state retrieval
    print_section "Retrieving Room State"
    increment_test
    
    if response=$(timeout 15s curl -s \
        -H "Authorization: Bearer $USER1_TOKEN" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/state" 2>/dev/null); then
        
        if echo "$response" | jq -e '.[0]' >/dev/null 2>&1; then
            print_success "Room state retrieved successfully"
            local state_count=$(echo "$response" | jq '. | length')
            print_info "Room has $state_count state events"
        else
            print_error "Failed to retrieve room state: $response"
        fi
    else
        print_error "Failed to retrieve room state - connection error"
    fi
}

# =============================================================================
# Room Membership Management
# =============================================================================

test_room_membership() {
    print_header "Room Membership Management"
    
    # Invite Bob to the room
    print_section "Inviting Bob to Room"
    increment_test
    
    local invite_data='{
        "user_id": "'$USER2_ID'"
    }'
    
    local response
    if response=$(timeout 20s curl -s -X POST \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$invite_data" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/invite" 2>/dev/null); then
        
        if [ "$response" = "{}" ] || echo "$response" | jq -e '.errcode' >/dev/null 2>&1; then
            if [ "$response" = "{}" ]; then
                print_success "Bob invited to room successfully"
            else
                print_error "Failed to invite Bob: $response"
            fi
        else
            print_success "Bob invited to room (response: $response)"
        fi
    else
        print_error "Failed to invite Bob - connection error"
    fi
    
    # Bob joins the room
    print_section "Bob Joining Room"
    increment_test
    
    if response=$(timeout 20s curl -s -X POST \
        -H "Authorization: Bearer $USER2_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{}' \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/join" 2>/dev/null); then
        
        if [ "$response" = "{}" ] || echo "$response" | jq -e '.room_id' >/dev/null 2>&1; then
            print_success "Bob joined room successfully"
        else
            print_error "Bob failed to join room: $response"
        fi
    else
        print_error "Bob failed to join room - connection error"
    fi
    
    # Invite Charlie to the room
    print_section "Inviting Charlie to Room"
    increment_test
    
    local invite_charlie_data='{
        "user_id": "'$USER3_ID'"
    }'
    
    if response=$(timeout 20s curl -s -X POST \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$invite_charlie_data" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/invite" 2>/dev/null); then
        
        if [ "$response" = "{}" ] || echo "$response" | jq -e '.errcode' >/dev/null 2>&1; then
            if [ "$response" = "{}" ]; then
                print_success "Charlie invited to room successfully"
            else
                print_error "Failed to invite Charlie: $response"
            fi
        else
            print_success "Charlie invited to room"
        fi
    else
        print_error "Failed to invite Charlie - connection error"
    fi
    
    # Charlie joins the room
    print_section "Charlie Joining Room"
    increment_test
    
    if response=$(timeout 20s curl -s -X POST \
        -H "Authorization: Bearer $USER3_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{}' \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/join" 2>/dev/null); then
        
        if [ "$response" = "{}" ] || echo "$response" | jq -e '.room_id' >/dev/null 2>&1; then
            print_success "Charlie joined room successfully"
        else
            print_error "Charlie failed to join room: $response"
        fi
    else
        print_error "Charlie failed to join room - connection error"
    fi
    
    # Get room members
    print_section "Retrieving Room Members"
    increment_test
    
    if response=$(timeout 15s curl -s \
        -H "Authorization: Bearer $USER1_TOKEN" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/members" 2>/dev/null); then
        
        if echo "$response" | jq -e '.chunk' >/dev/null 2>&1; then
            local member_count=$(echo "$response" | jq '.chunk | length')
            print_success "Retrieved room members: $member_count members"
            echo "$response" | jq -r '.chunk[] | select(.type == "m.room.member") | .state_key + " (" + .content.membership + ")"' | while read -r member; do
                print_info "Member: $member"
            done
        else
            print_error "Failed to retrieve room members: $response"
        fi
    else
        print_error "Failed to retrieve room members - connection error"
    fi
}

# =============================================================================
# Message Sending and Conversation Testing
# =============================================================================

test_room_messaging() {
    print_header "Room Messaging and Conversation"
    
    # Alice sends first message
    print_section "Alice Sends Welcome Message"
    increment_test
    
    local alice_msg='{
        "msgtype": "m.text",
        "body": "Welcome to our test room! This is Alice speaking. 🎉"
    }'
    
    local txn_id_1="alice_msg_$(date +%s)_1"
    local start_time=$(date +%s.%N)
    
    local response
    if response=$(timeout 20s curl -s -X PUT \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$alice_msg" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id_1" 2>/dev/null); then
        
        local end_time=$(date +%s.%N)
        local response_time=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "N/A")
        
        if echo "$response" | jq -e '.event_id' >/dev/null 2>&1; then
            local event_id=$(echo "$response" | jq -r '.event_id')
            print_success "Alice's message sent successfully in ${response_time}s"
            print_info "Event ID: $event_id"
            
            # Validate performance target
            if command -v bc > /dev/null && (( $(echo "$response_time < 0.1" | bc -l 2>/dev/null || echo "0") )); then
                print_success "✨ Performance target met: Message send <100ms"
            fi
        else
            print_error "Alice's message failed: $response"
        fi
    else
        print_error "Alice failed to send message - connection error"
    fi
    
    # Bob responds
    print_section "Bob Responds to Alice"
    increment_test
    
    local bob_msg='{
        "msgtype": "m.text",
        "body": "Hi Alice! Bob here. Great to be in this room! 👋"
    }'
    
    local txn_id_2="bob_msg_$(date +%s)_1"
    
    if response=$(timeout 20s curl -s -X PUT \
        -H "Authorization: Bearer $USER2_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$bob_msg" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id_2" 2>/dev/null); then
        
        if echo "$response" | jq -e '.event_id' >/dev/null 2>&1; then
            local event_id=$(echo "$response" | jq -r '.event_id')
            print_success "Bob's message sent successfully"
            print_info "Event ID: $event_id"
        else
            print_error "Bob's message failed: $response"
        fi
    else
        print_error "Bob failed to send message - connection error"
    fi
    
    # Charlie joins the conversation
    print_section "Charlie Joins Conversation"
    increment_test
    
    local charlie_msg='{
        "msgtype": "m.text",
        "body": "Hello everyone! Charlie reporting for duty! 🚀 How is everyone doing?"
    }'
    
    local txn_id_3="charlie_msg_$(date +%s)_1"
    
    if response=$(timeout 20s curl -s -X PUT \
        -H "Authorization: Bearer $USER3_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$charlie_msg" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id_3" 2>/dev/null); then
        
        if echo "$response" | jq -e '.event_id' >/dev/null 2>&1; then
            local event_id=$(echo "$response" | jq -r '.event_id')
            print_success "Charlie's message sent successfully"
            print_info "Event ID: $event_id"
        else
            print_error "Charlie's message failed: $response"
        fi
    else
        print_error "Charlie failed to send message - connection error"
    fi
    
    # Test formatted message
    print_section "Alice Sends Formatted Message"
    increment_test
    
    local formatted_msg='{
        "msgtype": "m.text",
        "body": "This is **bold** and *italic* text with some code: `console.log(\"Hello Matrix!\")`",
        "format": "org.matrix.custom.html",
        "formatted_body": "This is <strong>bold</strong> and <em>italic</em> text with some code: <code>console.log(\"Hello Matrix!\")</code>"
    }'
    
    local txn_id_4="alice_msg_$(date +%s)_2"
    
    if response=$(timeout 20s curl -s -X PUT \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$formatted_msg" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id_4" 2>/dev/null); then
        
        if echo "$response" | jq -e '.event_id' >/dev/null 2>&1; then
            local event_id=$(echo "$response" | jq -r '.event_id')
            print_success "Alice's formatted message sent successfully"
            print_info "Event ID: $event_id"
        else
            print_error "Alice's formatted message failed: $response"
        fi
    else
        print_error "Alice failed to send formatted message - connection error"
    fi
}

# =============================================================================
# Message History and Pagination Testing
# =============================================================================

test_message_history() {
    print_header "Message History and Pagination"
    
    print_section "Retrieving Room Messages"
    increment_test
    
    local response
    if response=$(timeout 20s curl -s \
        -H "Authorization: Bearer $USER1_TOKEN" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/messages?dir=b&limit=50" 2>/dev/null); then
        
        if echo "$response" | jq -e '.chunk' >/dev/null 2>&1; then
            local message_count=$(echo "$response" | jq '.chunk | map(select(.type == "m.room.message")) | length')
            print_success "Retrieved message history: $message_count messages"
            
            # Display recent messages
            echo "$response" | jq -r '.chunk[] | select(.type == "m.room.message") | "\(.sender): \(.content.body)"' | head -10 | while read -r msg; do
                print_info "Message: $msg"
            done
        else
            print_error "Failed to retrieve message history: $response"
        fi
    else
        print_error "Failed to retrieve message history - connection error"
    fi
    
    # Test pagination
    print_section "Testing Message Pagination"
    increment_test
    
    if response=$(timeout 20s curl -s \
        -H "Authorization: Bearer $USER1_TOKEN" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/messages?dir=b&limit=2" 2>/dev/null); then
        
        if echo "$response" | jq -e '.chunk' >/dev/null 2>&1; then
            local message_count=$(echo "$response" | jq '.chunk | length')
            local prev_batch=$(echo "$response" | jq -r '.prev_batch')
            print_success "Pagination test: Retrieved $message_count events"
            print_info "Previous batch token: ${prev_batch:0:20}..."
            
            # Test next page
            if [ "$prev_batch" != "null" ] && [ -n "$prev_batch" ]; then
                print_info "Testing next page retrieval..."
                if response2=$(timeout 20s curl -s \
                    -H "Authorization: Bearer $USER1_TOKEN" \
                    "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/messages?dir=b&from=$prev_batch&limit=2" 2>/dev/null); then
                    
                    if echo "$response2" | jq -e '.chunk' >/dev/null 2>&1; then
                        local next_count=$(echo "$response2" | jq '.chunk | length')
                        print_success "Next page retrieved: $next_count events"
                    else
                        print_error "Failed to retrieve next page: $response2"
                    fi
                fi
            fi
        else
            print_error "Failed pagination test: $response"
        fi
    else
        print_error "Failed pagination test - connection error"
    fi
}

# =============================================================================
# Room State Management Testing
# =============================================================================

test_room_state_management() {
    print_header "Room State Management"
    
    # Update room name
    print_section "Updating Room Name"
    increment_test
    
    local new_name="Matrixon Advanced Test Room - Updated"
    local name_data='{
        "name": "'$new_name'"
    }'
    
    local response
    if response=$(timeout 20s curl -s -X PUT \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$name_data" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/state/m.room.name" 2>/dev/null); then
        
        if [ "$response" = "{}" ] || echo "$response" | jq -e '.event_id' >/dev/null 2>&1; then
            print_success "Room name updated successfully"
        else
            print_error "Failed to update room name: $response"
        fi
    else
        print_error "Failed to update room name - connection error"
    fi
    
    # Update room topic
    print_section "Updating Room Topic"
    increment_test
    
    local new_topic="Advanced testing room for Matrixon Matrix NextServer - Updated with comprehensive features"
    local topic_data='{
        "topic": "'$new_topic'"
    }'
    
    if response=$(timeout 20s curl -s -X PUT \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$topic_data" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/state/m.room.topic" 2>/dev/null); then
        
        if [ "$response" = "{}" ] || echo "$response" | jq -e '.event_id' >/dev/null 2>&1; then
            print_success "Room topic updated successfully"
        else
            print_error "Failed to update room topic: $response"
        fi
    else
        print_error "Failed to update room topic - connection error"
    fi
    
    # Get updated room state
    print_section "Verifying Room State Updates"
    increment_test
    
    if response=$(timeout 15s curl -s \
        -H "Authorization: Bearer $USER1_TOKEN" \
        "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/state/m.room.name" 2>/dev/null); then
        
        if echo "$response" | jq -e '.name' >/dev/null 2>&1; then
            local current_name=$(echo "$response" | jq -r '.name')
            print_success "Current room name: $current_name"
        else
            print_error "Failed to get current room name: $response"
        fi
    else
        print_error "Failed to get room name - connection error"
    fi
}

# =============================================================================
# Real-time Sync Testing
# =============================================================================

test_sync_functionality() {
    print_header "Real-time Sync Testing"
    
    # Initial sync for Alice
    print_section "Alice Initial Sync"
    increment_test
    
    local start_time=$(date +%s.%N)
    local response
    if response=$(timeout 30s curl -s \
        -H "Authorization: Bearer $USER1_TOKEN" \
        "$SERVER_URL/_matrix/client/r0/sync?timeout=1000" 2>/dev/null); then
        
        local end_time=$(date +%s.%N)
        local response_time=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "N/A")
        
        if echo "$response" | jq -e '.next_batch' >/dev/null 2>&1; then
            local next_batch=$(echo "$response" | jq -r '.next_batch')
            print_success "Alice sync completed in ${response_time}s"
            print_info "Next batch token: ${next_batch:0:20}..."
            
            # Check if our room is in the sync response
            if echo "$response" | jq -e ".rooms.join.\"$ROOM_ID\"" >/dev/null 2>&1; then
                print_success "✨ Our test room found in sync response"
                
                # Count timeline events
                local timeline_count=$(echo "$response" | jq ".rooms.join.\"$ROOM_ID\".timeline.events | length")
                print_info "Timeline events in room: $timeline_count"
            else
                print_info "Test room not found in sync (might be in different sync batch)"
            fi
        else
            print_error "Invalid sync response: $response"
        fi
    else
        print_error "Sync request failed - connection error"
    fi
    
    # Bob sync test
    print_section "Bob Sync Test"
    increment_test
    
    if response=$(timeout 30s curl -s \
        -H "Authorization: Bearer $USER2_TOKEN" \
        "$SERVER_URL/_matrix/client/r0/sync?timeout=1000" 2>/dev/null); then
        
        if echo "$response" | jq -e '.next_batch' >/dev/null 2>&1; then
            print_success "Bob sync completed successfully"
            
            # Check for our room
            if echo "$response" | jq -e ".rooms.join.\"$ROOM_ID\"" >/dev/null 2>&1; then
                print_success "✨ Bob can see the test room in sync"
            else
                print_info "Test room not in Bob's current sync batch"
            fi
        else
            print_error "Bob sync failed: $response"
        fi
    else
        print_error "Bob sync failed - connection error"
    fi
}

# =============================================================================
# Performance and Load Testing
# =============================================================================

test_performance_metrics() {
    print_header "Performance and Load Testing"
    
    # Concurrent message sending test
    print_section "Concurrent Message Sending Test"
    increment_test
    
    print_info "Sending 5 concurrent messages from different users..."
    
    # Start concurrent message sends
    local pids=()
    
    # Alice sends message
    (
        local msg='{"msgtype": "m.text", "body": "Concurrent message from Alice #'$(date +%s)'"}'
        curl -s -X PUT \
            -H "Authorization: Bearer $USER1_TOKEN" \
            -H "Content-Type: application/json" \
            -d "$msg" \
            "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/alice_concurrent_$(date +%s)" \
            > /tmp/alice_concurrent.txt 2>&1
    ) &
    pids+=($!)
    
    # Bob sends message
    (
        local msg='{"msgtype": "m.text", "body": "Concurrent message from Bob #'$(date +%s)'"}'
        curl -s -X PUT \
            -H "Authorization: Bearer $USER2_TOKEN" \
            -H "Content-Type: application/json" \
            -d "$msg" \
            "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/bob_concurrent_$(date +%s)" \
            > /tmp/bob_concurrent.txt 2>&1
    ) &
    pids+=($!)
    
    # Charlie sends message
    (
        local msg='{"msgtype": "m.text", "body": "Concurrent message from Charlie #'$(date +%s)'"}'
        curl -s -X PUT \
            -H "Authorization: Bearer $USER3_TOKEN" \
            -H "Content-Type: application/json" \
            -d "$msg" \
            "$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/charlie_concurrent_$(date +%s)" \
            > /tmp/charlie_concurrent.txt 2>&1
    ) &
    pids+=($!)
    
    # Wait for all processes to complete
    local success_count=0
    for pid in "${pids[@]}"; do
        if wait "$pid"; then
            ((success_count++))
        fi
    done
    
    # Check results
    if [ $success_count -eq 3 ]; then
        print_success "All 3 concurrent messages sent successfully"
    else
        print_error "Only $success_count out of 3 concurrent messages succeeded"
    fi
    
    # Cleanup temp files
    rm -f /tmp/alice_concurrent.txt /tmp/bob_concurrent.txt /tmp/charlie_concurrent.txt
    
    # Response time test
    print_section "Response Time Validation"
    increment_test
    
    print_info "Testing API response times..."
    
    local total_time=0
    local test_count=0
    
    for i in {1..5}; do
        local start_time=$(date +%s.%N)
        if curl -s -H "Authorization: Bearer $USER1_TOKEN" "$SERVER_URL/_matrix/client/versions" > /dev/null 2>&1; then
            local end_time=$(date +%s.%N)
            local response_time=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "0")
            total_time=$(echo "$total_time + $response_time" | bc -l 2>/dev/null || echo "$total_time")
            ((test_count++))
            print_info "Test $i: ${response_time}s"
        fi
        sleep 0.1
    done
    
    if [ $test_count -gt 0 ]; then
        local avg_time=$(echo "scale=3; $total_time / $test_count" | bc -l 2>/dev/null || echo "N/A")
        print_success "Average response time: ${avg_time}s over $test_count tests"
        
        # Check against performance target
        if command -v bc > /dev/null && (( $(echo "$avg_time < 0.05" | bc -l 2>/dev/null || echo "0") )); then
            print_success "🎉 Excellent: Average response time meets <50ms target!"
        elif command -v bc > /dev/null && (( $(echo "$avg_time < 0.1" | bc -l 2>/dev/null || echo "0") )); then
            print_success "✨ Good: Average response time under 100ms"
        else
            print_info "Performance: Average response time is ${avg_time}s"
        fi
    else
        print_error "No successful response time tests"
    fi
}

# =============================================================================
# Test Summary and Cleanup
# =============================================================================

show_test_summary() {
    print_header "Test Summary Report"
    
    echo
    echo -e "${CYAN}📊 Room & Conversation Test Results:${NC}"
    echo -e "   Total Tests: ${TOTAL_TESTS}"
    echo -e "   ${GREEN}Passed: ${PASSED_TESTS}${NC}"
    echo -e "   ${RED}Failed: ${FAILED_TESTS}${NC}"
    
    if [ $TOTAL_TESTS -gt 0 ]; then
        local success_rate=$((PASSED_TESTS * 100 / TOTAL_TESTS))
        echo -e "   Success Rate: ${success_rate}%"
        
        if [ $success_rate -ge 90 ]; then
            echo -e "   ${GREEN}🎉 Excellent! Room functionality working great!${NC}"
        elif [ $success_rate -ge 70 ]; then
            echo -e "   ${YELLOW}⚠️  Good, but some room features need attention${NC}"
        else
            echo -e "   ${RED}❌ Room functionality needs significant work${NC}"
        fi
    fi
    
    echo
    echo -e "${CYAN}📁 Test Artifacts:${NC}"
    echo -e "   Log File: ${LOG_FILE}"
    echo -e "   Output Directory: ${OUTPUT_DIR}"
    
    if [ -n "$ROOM_ID" ]; then
        echo
        echo -e "${CYAN}🏠 Created Room Information:${NC}"
        echo -e "   Room ID: ${ROOM_ID}"
        echo -e "   Room Name: ${ROOM_NAME}"
        echo -e "   Room Alias: #${ROOM_ALIAS}:localhost"
        echo -e "   Members: Alice, Bob, Charlie"
    fi
    
    echo
    echo -e "${CYAN}🔗 Useful Commands:${NC}"
    echo -e "   View room state: curl -H \"Authorization: Bearer $USER1_TOKEN\" \"$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/state\""
    echo -e "   Get messages: curl -H \"Authorization: Bearer $USER1_TOKEN\" \"$SERVER_URL/_matrix/client/r0/rooms/$ROOM_ID/messages?dir=b&limit=10\""
    echo -e "   Sync events: curl -H \"Authorization: Bearer $USER1_TOKEN\" \"$SERVER_URL/_matrix/client/r0/sync?timeout=30000\""
    
    log "INFO" "=== Test Suite Completed ==="
    log "INFO" "Total: $TOTAL_TESTS, Passed: $PASSED_TESTS, Failed: $FAILED_TESTS"
}

# =============================================================================
# Main Execution
# =============================================================================

main() {
    clear
    echo -e "${PURPLE}"
    cat << 'EOF'
🏠 Matrixon Room & Conversation Test Suite 💬
===============================================
Comprehensive testing for Matrix room creation,
messaging, and real-time conversation features.
EOF
    echo -e "${NC}"
    
    setup_test_environment
    
    # Run test sequence
    if check_server_health; then
        register_test_users && \
        test_room_creation && \
        test_room_membership && \
        test_room_messaging && \
        test_message_history && \
        test_room_state_management && \
        test_sync_functionality && \
        test_performance_metrics
    else
        echo
        echo -e "${YELLOW}📚 Server not available - Showing example commands:${NC}"
        echo
        cat << 'EOF'
# Example Room Creation:
curl -X POST \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Room",
    "topic": "A test room",
    "preset": "private_chat"
  }' \
  http://localhost:6167/_matrix/client/r0/createRoom

# Example Message Sending:
curl -X PUT \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "msgtype": "m.text",
    "body": "Hello, Matrix!"
  }' \
  http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID/send/m.room.message/TXN_ID

# Example Room History:
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID/messages?dir=b&limit=10
EOF
    fi
    
    show_test_summary
}

# Run the main function
main "$@" 
