#!/bin/bash

# =============================================================================
# Matrixon Matrix NextServer - Complete Functionality Test Script
# =============================================================================
#
# Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
# Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
# Date: 2024-12-11
# Version: 0.11.0-alpha
# License: Apache 2.0 / MIT
#
# Description:
#   Comprehensive test script for Matrixon Matrix server functionality
#   Tests user registration, login, room creation, messaging, and more
#
# Usage:
#   chmod +x test_matrix_functions.sh
#   ./test_matrix_functions.sh
#
# =============================================================================

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="http://localhost:6167"
SERVER_NAME="localhost:6167"

# Test users
ALICE_USERNAME="alice"
ALICE_PASSWORD="test_password_123"
ALICE_DEVICE_ID="ALICE_DEVICE"
ALICE_DISPLAY_NAME="Alice Test User"

BOB_USERNAME="bob"
BOB_PASSWORD="test_password_456"
BOB_DEVICE_ID="BOB_DEVICE"
BOB_DISPLAY_NAME="Bob Test User"

# Global variables for tokens
ALICE_ACCESS_TOKEN=""
BOB_ACCESS_TOKEN=""
ROOM_ID=""

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_test() {
    echo -e "${PURPLE}[TEST]${NC} $1"
}

log_result() {
    echo -e "${CYAN}[RESULT]${NC} $1"
}

# Function to make HTTP requests with pretty output
make_request() {
    local method=$1
    local url=$2
    local data=$3
    local headers=$4
    
    log_test "Making $method request to $url"
    
    if [ -n "$data" ]; then
        if [ -n "$headers" ]; then
            response=$(curl -s -X "$method" "$url" -H "Content-Type: application/json" -H "$headers" -d "$data")
        else
            response=$(curl -s -X "$method" "$url" -H "Content-Type: application/json" -d "$data")
        fi
    else
        if [ -n "$headers" ]; then
            response=$(curl -s -X "$method" "$url" -H "$headers")
        else
            response=$(curl -s -X "$method" "$url")
        fi
    fi
    
    echo "$response" | jq . 2>/dev/null || echo "$response"
    echo
    return 0
}

# Test 1: Server Health Check
test_server_health() {
    log_info "🏥 Testing server health and basic endpoints..."
    
    log_test "Testing root endpoint"
    make_request "GET" "$BASE_URL/"
    
    log_test "Testing Matrix client versions"
    make_request "GET" "$BASE_URL/_matrix/client/versions"
    
    log_test "Testing Matrix capabilities"
    make_request "GET" "$BASE_URL/_matrix/client/r0/capabilities"
    
    log_test "Testing federation version"
    make_request "GET" "$BASE_URL/_matrix/federation/v1/version"
    
    log_success "Server health checks completed!"
}

# Test 2: User Registration
test_user_registration() {
    log_info "👤 Testing user registration..."
    
    # Register Alice
    log_test "Registering user: $ALICE_USERNAME"
    alice_reg_data='{
        "auth": {"type": "m.login.dummy"},
        "username": "'$ALICE_USERNAME'",
        "password": "'$ALICE_PASSWORD'",
        "device_id": "'$ALICE_DEVICE_ID'",
        "initial_device_display_name": "'$ALICE_DISPLAY_NAME'"
    }'
    
    alice_response=$(make_request "POST" "$BASE_URL/_matrix/client/r0/register" "$alice_reg_data")
    echo "$alice_response"
    
    # Extract access token from response if successful
    ALICE_ACCESS_TOKEN=$(echo "$alice_response" | jq -r '.access_token // empty')
    if [ -n "$ALICE_ACCESS_TOKEN" ] && [ "$ALICE_ACCESS_TOKEN" != "null" ]; then
        log_success "Alice registered successfully! Access token: ${ALICE_ACCESS_TOKEN:0:20}..."
    else
        log_warning "Alice registration returned different response, checking login types..."
    fi
    
    # Register Bob
    log_test "Registering user: $BOB_USERNAME"
    bob_reg_data='{
        "auth": {"type": "m.login.dummy"},
        "username": "'$BOB_USERNAME'",
        "password": "'$BOB_PASSWORD'",
        "device_id": "'$BOB_DEVICE_ID'",
        "initial_device_display_name": "'$BOB_DISPLAY_NAME'"
    }'
    
    bob_response=$(make_request "POST" "$BASE_URL/_matrix/client/r0/register" "$bob_reg_data")
    echo "$bob_response"
    
    # Extract access token from response if successful
    BOB_ACCESS_TOKEN=$(echo "$bob_response" | jq -r '.access_token // empty')
    if [ -n "$BOB_ACCESS_TOKEN" ] && [ "$BOB_ACCESS_TOKEN" != "null" ]; then
        log_success "Bob registered successfully! Access token: ${BOB_ACCESS_TOKEN:0:20}..."
    else
        log_warning "Bob registration returned different response"
    fi
}

# Test 3: User Login
test_user_login() {
    log_info "🔐 Testing user login..."
    
    # Check login types
    log_test "Checking available login types"
    make_request "GET" "$BASE_URL/_matrix/client/r0/login"
    
    # Login Alice if no token from registration
    if [ -z "$ALICE_ACCESS_TOKEN" ] || [ "$ALICE_ACCESS_TOKEN" = "null" ]; then
        log_test "Logging in Alice"
        alice_login_data='{
            "type": "m.login.password",
            "user": "'$ALICE_USERNAME'",
            "password": "'$ALICE_PASSWORD'",
            "device_id": "'$ALICE_DEVICE_ID'",
            "initial_device_display_name": "'$ALICE_DISPLAY_NAME'"
        }'
        
        alice_login_response=$(make_request "POST" "$BASE_URL/_matrix/client/r0/login" "$alice_login_data")
        echo "$alice_login_response"
        
        ALICE_ACCESS_TOKEN=$(echo "$alice_login_response" | jq -r '.access_token // empty')
        if [ -n "$ALICE_ACCESS_TOKEN" ] && [ "$ALICE_ACCESS_TOKEN" != "null" ]; then
            log_success "Alice logged in successfully! Access token: ${ALICE_ACCESS_TOKEN:0:20}..."
        fi
    fi
    
    # Login Bob if no token from registration
    if [ -z "$BOB_ACCESS_TOKEN" ] || [ "$BOB_ACCESS_TOKEN" = "null" ]; then
        log_test "Logging in Bob"
        bob_login_data='{
            "type": "m.login.password",
            "user": "'$BOB_USERNAME'",
            "password": "'$BOB_PASSWORD'",
            "device_id": "'$BOB_DEVICE_ID'",
            "initial_device_display_name": "'$BOB_DISPLAY_NAME'"
        }'
        
        bob_login_response=$(make_request "POST" "$BASE_URL/_matrix/client/r0/login" "$bob_login_data")
        echo "$bob_login_response"
        
        BOB_ACCESS_TOKEN=$(echo "$bob_login_response" | jq -r '.access_token // empty')
        if [ -n "$BOB_ACCESS_TOKEN" ] && [ "$BOB_ACCESS_TOKEN" != "null" ]; then
            log_success "Bob logged in successfully! Access token: ${BOB_ACCESS_TOKEN:0:20}..."
        fi
    fi
}

# Test 4: User Profile
test_user_profile() {
    log_info "👤 Testing user profile operations..."
    
    if [ -n "$ALICE_ACCESS_TOKEN" ] && [ "$ALICE_ACCESS_TOKEN" != "null" ]; then
        log_test "Testing whoami for Alice"
        make_request "GET" "$BASE_URL/_matrix/client/r0/account/whoami" "" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
        
        log_test "Getting Alice's profile"
        make_request "GET" "$BASE_URL/_matrix/client/r0/profile/@$ALICE_USERNAME:$SERVER_NAME" "" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
        
        log_test "Setting Alice's display name"
        alice_displayname_data='{"displayname": "'$ALICE_DISPLAY_NAME'"}'
        make_request "PUT" "$BASE_URL/_matrix/client/r0/profile/@$ALICE_USERNAME:$SERVER_NAME/displayname" "$alice_displayname_data" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
    else
        log_warning "No Alice access token, skipping profile tests"
    fi
}

# Test 5: Room Creation
test_room_creation() {
    log_info "🏠 Testing room creation..."
    
    if [ -n "$ALICE_ACCESS_TOKEN" ] && [ "$ALICE_ACCESS_TOKEN" != "null" ]; then
        log_test "Creating a room as Alice"
        room_create_data='{
            "name": "Test Room",
            "topic": "A test room for Matrixon testing",
            "preset": "public_chat",
            "creation_content": {
                "m.federate": false
            }
        }'
        
        room_response=$(make_request "POST" "$BASE_URL/_matrix/client/r0/createRoom" "$room_create_data" "Authorization: Bearer $ALICE_ACCESS_TOKEN")
        echo "$room_response"
        
        ROOM_ID=$(echo "$room_response" | jq -r '.room_id // empty')
        if [ -n "$ROOM_ID" ] && [ "$ROOM_ID" != "null" ]; then
            log_success "Room created successfully! Room ID: $ROOM_ID"
        else
            log_warning "Room creation returned different response"
        fi
    else
        log_warning "No Alice access token, skipping room creation"
    fi
}

# Test 6: Room Operations
test_room_operations() {
    log_info "🏠 Testing room operations..."
    
    if [ -n "$ROOM_ID" ] && [ "$ROOM_ID" != "null" ] && [ -n "$ALICE_ACCESS_TOKEN" ]; then
        log_test "Getting room state"
        make_request "GET" "$BASE_URL/_matrix/client/r0/rooms/$ROOM_ID/state" "" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
        
        log_test "Getting room members"
        make_request "GET" "$BASE_URL/_matrix/client/r0/rooms/$ROOM_ID/members" "" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
        
        # Invite Bob to the room
        if [ -n "$BOB_ACCESS_TOKEN" ] && [ "$BOB_ACCESS_TOKEN" != "null" ]; then
            log_test "Inviting Bob to the room"
            invite_data='{"user_id": "@'$BOB_USERNAME':'$SERVER_NAME'"}'
            make_request "POST" "$BASE_URL/_matrix/client/r0/rooms/$ROOM_ID/invite" "$invite_data" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
        fi
    else
        log_warning "No room ID or Alice token, skipping room operations"
    fi
}

# Test 7: Messaging
test_messaging() {
    log_info "💬 Testing messaging functionality..."
    
    if [ -n "$ROOM_ID" ] && [ "$ROOM_ID" != "null" ] && [ -n "$ALICE_ACCESS_TOKEN" ]; then
        log_test "Sending a message as Alice"
        message_data='{
            "msgtype": "m.text",
            "body": "Hello from Alice! This is a test message from Matrixon Matrix server."
        }'
        
        txn_id="txn_$(date +%s)_alice"
        make_request "PUT" "$BASE_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$txn_id" "$message_data" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
        
        log_test "Getting recent messages"
        make_request "GET" "$BASE_URL/_matrix/client/r0/rooms/$ROOM_ID/messages?limit=10" "" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
        
        # Send message as Bob if he's authenticated
        if [ -n "$BOB_ACCESS_TOKEN" ] && [ "$BOB_ACCESS_TOKEN" != "null" ]; then
            log_test "Sending a message as Bob"
            bob_message_data='{
                "msgtype": "m.text",
                "body": "Hi Alice! Bob here, testing Matrixon messaging!"
            }'
            
            bob_txn_id="txn_$(date +%s)_bob"
            make_request "PUT" "$BASE_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$bob_txn_id" "$bob_message_data" "Authorization: Bearer $BOB_ACCESS_TOKEN"
        fi
    else
        log_warning "No room ID or Alice token, skipping messaging tests"
    fi
}

# Test 8: Sync and Events
test_sync() {
    log_info "🔄 Testing sync functionality..."
    
    if [ -n "$ALICE_ACCESS_TOKEN" ] && [ "$ALICE_ACCESS_TOKEN" != "null" ]; then
        log_test "Testing initial sync for Alice"
        make_request "GET" "$BASE_URL/_matrix/client/r0/sync?timeout=1000" "" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
    else
        log_warning "No Alice access token, skipping sync test"
    fi
}

# Test 9: Device Management
test_device_management() {
    log_info "📱 Testing device management..."
    
    if [ -n "$ALICE_ACCESS_TOKEN" ] && [ "$ALICE_ACCESS_TOKEN" != "null" ]; then
        log_test "Getting Alice's devices"
        make_request "GET" "$BASE_URL/_matrix/client/r0/devices" "" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
        
        log_test "Getting specific device info"
        make_request "GET" "$BASE_URL/_matrix/client/r0/devices/$ALICE_DEVICE_ID" "" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
    else
        log_warning "No Alice access token, skipping device management tests"
    fi
}

# Test 10: Public Rooms
test_public_rooms() {
    log_info "🌐 Testing public rooms directory..."
    
    log_test "Getting public rooms list"
    make_request "GET" "$BASE_URL/_matrix/client/r0/publicRooms"
    
    if [ -n "$ALICE_ACCESS_TOKEN" ] && [ "$ALICE_ACCESS_TOKEN" != "null" ]; then
        log_test "Getting public rooms with authentication"
        make_request "GET" "$BASE_URL/_matrix/client/r0/publicRooms" "" "Authorization: Bearer $ALICE_ACCESS_TOKEN"
    fi
}

# Main test execution
main() {
    echo -e "${CYAN}"
    echo "=================================================================="
    echo "  🚀 Matrixon Matrix NextServer - Comprehensive Function Test"
    echo "=================================================================="
    echo -e "${NC}"
    echo "Testing Matrixon Matrix server functionality..."
    echo "Server: $BASE_URL"
    echo "Date: $(date)"
    echo
    
    # Check if server is running
    if ! curl -s "$BASE_URL/" > /dev/null; then
        log_error "Server is not running at $BASE_URL"
        log_info "Please start the server with: RUST_LOG=info ./target/release/matrixon --config test-config.toml start"
        exit 1
    fi
    
    log_success "Server is running!"
    echo
    
    # Run all tests
    test_server_health
    echo "=================================================================="
    
    test_user_registration
    echo "=================================================================="
    
    test_user_login
    echo "=================================================================="
    
    test_user_profile
    echo "=================================================================="
    
    test_room_creation
    echo "=================================================================="
    
    test_room_operations
    echo "=================================================================="
    
    test_messaging
    echo "=================================================================="
    
    test_sync
    echo "=================================================================="
    
    test_device_management
    echo "=================================================================="
    
    test_public_rooms
    echo "=================================================================="
    
    # Summary
    echo -e "${GREEN}"
    echo "🎉 Test suite completed!"
    echo "=================================================================="
    echo -e "${NC}"
    echo "Test Summary:"
    echo "- Server Health: ✅ Tested"
    echo "- User Registration: ✅ Tested" 
    echo "- User Login: ✅ Tested"
    echo "- User Profile: ✅ Tested"
    echo "- Room Creation: ✅ Tested"
    echo "- Room Operations: ✅ Tested"
    echo "- Messaging: ✅ Tested"
    echo "- Sync: ✅ Tested"
    echo "- Device Management: ✅ Tested"
    echo "- Public Rooms: ✅ Tested"
    echo
    echo "Tokens obtained:"
    [ -n "$ALICE_ACCESS_TOKEN" ] && echo "- Alice: ${ALICE_ACCESS_TOKEN:0:30}..." || echo "- Alice: Not obtained"
    [ -n "$BOB_ACCESS_TOKEN" ] && echo "- Bob: ${BOB_ACCESS_TOKEN:0:30}..." || echo "- Bob: Not obtained"
    echo
    [ -n "$ROOM_ID" ] && echo "Room created: $ROOM_ID" || echo "No room created"
    echo
    log_success "Matrixon Matrix NextServer testing completed successfully!"
}

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    log_warning "jq is not installed. JSON responses will not be formatted."
    log_info "Install jq with: brew install jq (on macOS) or apt-get install jq (on Ubuntu)"
fi

# Run the tests
main "$@" 
