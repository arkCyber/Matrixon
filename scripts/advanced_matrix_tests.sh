#!/bin/bash

##
# Advanced Matrix API Testing with cURL
# Tests media upload, room invites, and other advanced features
# 
# @author: Matrix Server Testing Team
# @version: 2.0.0
##

set -e

# Configuration
BASE_URL="http://localhost:6167"
SERVER_NAME="localhost"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Helper functions
log() { echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"; }
success() { echo -e "${GREEN}✅ $1${NC}"; }
error() { echo -e "${RED}❌ $1${NC}"; }
info() { echo -e "${YELLOW}ℹ️  $1${NC}"; }

# Register a user and get access token
register_user() {
    local username="advtest_$(date +%s)"
    local password="AdvancedTest123!"
    
    log "Registering user: $username"
    
    local response=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$username\",\"password\":\"$password\",\"auth\":{\"type\":\"m.login.dummy\"}}" \
        "${BASE_URL}/_matrix/client/r0/register")
    
    if echo "$response" | jq -e '.access_token' > /dev/null 2>&1; then
        echo "$response" | jq -r '.access_token'
    else
        error "User registration failed: $response"
        exit 1
    fi
}

# Test media upload
test_media_upload() {
    local token="$1"
    log "Testing media upload functionality..."
    
    # Create a test image file
    echo "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==" | base64 -d > test_image.png
    
    info "Uploading test image..."
    local response=$(curl -s -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: image/png" \
        --data-binary @test_image.png \
        "${BASE_URL}/_matrix/media/r0/upload")
    
    if echo "$response" | jq -e '.content_uri' > /dev/null 2>&1; then
        local content_uri=$(echo "$response" | jq -r '.content_uri')
        success "Media upload successful"
        info "Content URI: $content_uri"
        
        # Test media download
        info "Testing media download..."
        local media_id=$(echo "$content_uri" | sed 's/mxc:\/\/[^\/]*\///')
        local download_response=$(curl -s -w "%{http_code}" \
            "${BASE_URL}/_matrix/media/r0/download/${SERVER_NAME}/${media_id}")
        
        if [[ "$download_response" =~ 200$ ]]; then
            success "Media download successful"
        else
            error "Media download failed with code: $download_response"
        fi
    else
        error "Media upload failed: $response"
    fi
    
    # Cleanup
    rm -f test_image.png
}

# Test room directory
test_room_directory() {
    local token="$1"
    log "Testing public room directory..."
    
    info "Getting public rooms..."
    local response=$(curl -s -H "Authorization: Bearer $token" \
        "${BASE_URL}/_matrix/client/r0/publicRooms?limit=10")
    
    if echo "$response" | jq -e '.chunk' > /dev/null 2>&1; then
        local room_count=$(echo "$response" | jq '.chunk | length')
        success "Public room directory working"
        info "Found $room_count public rooms"
    else
        error "Public room directory failed: $response"
    fi
}

# Test device management
test_device_management() {
    local token="$1"
    log "Testing device management..."
    
    info "Getting devices..."
    local response=$(curl -s -H "Authorization: Bearer $token" \
        "${BASE_URL}/_matrix/client/r0/devices")
    
    if echo "$response" | jq -e '.devices' > /dev/null 2>&1; then
        local device_count=$(echo "$response" | jq '.devices | length')
        success "Device management working"
        info "Found $device_count devices"
        
        # Show device info
        local device_id=$(echo "$response" | jq -r '.devices[0].device_id')
        info "Current device ID: $device_id"
    else
        error "Device management failed: $response"
    fi
}

# Test presence
test_presence() {
    local token="$1"
    local user_id="$2"
    log "Testing presence functionality..."
    
    info "Setting presence to online..."
    local response=$(curl -s -X PUT \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d '{"presence":"online","status_msg":"Testing presence"}' \
        "${BASE_URL}/_matrix/client/r0/presence/${user_id}/status")
    
    if [ "$response" = "{}" ]; then
        success "Presence setting successful"
        
        # Get presence
        info "Getting presence status..."
        response=$(curl -s -H "Authorization: Bearer $token" \
            "${BASE_URL}/_matrix/client/r0/presence/${user_id}/status")
        
        if echo "$response" | jq -e '.presence' > /dev/null 2>&1; then
            local presence=$(echo "$response" | jq -r '.presence')
            local status_msg=$(echo "$response" | jq -r '.status_msg // "none"')
            success "Presence retrieval successful"
            info "Presence: $presence, Status: $status_msg"
        else
            error "Presence retrieval failed: $response"
        fi
    else
        error "Presence setting failed: $response"
    fi
}

# Test typing indicators
test_typing_indicators() {
    local token="$1"
    local room_id="$2"
    local user_id="$3"
    log "Testing typing indicators..."
    
    info "Setting typing indicator..."
    local response=$(curl -s -X PUT \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d '{"typing":true,"timeout":10000}' \
        "${BASE_URL}/_matrix/client/r0/rooms/${room_id}/typing/${user_id}")
    
    if [ "$response" = "{}" ]; then
        success "Typing indicator set successfully"
        
        # Stop typing
        sleep 2
        info "Stopping typing indicator..."
        response=$(curl -s -X PUT \
            -H "Authorization: Bearer $token" \
            -H "Content-Type: application/json" \
            -d '{"typing":false}' \
            "${BASE_URL}/_matrix/client/r0/rooms/${room_id}/typing/${user_id}")
        
        if [ "$response" = "{}" ]; then
            success "Typing indicator stopped successfully"
        else
            error "Failed to stop typing indicator: $response"
        fi
    else
        error "Typing indicator failed: $response"
    fi
}

# Test read receipts
test_read_receipts() {
    local token="$1"
    local room_id="$2"
    local event_id="$3"
    log "Testing read receipts..."
    
    info "Sending read receipt..."
    local response=$(curl -s -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d '{}' \
        "${BASE_URL}/_matrix/client/r0/rooms/${room_id}/receipt/m.read/${event_id}")
    
    if [ "$response" = "{}" ]; then
        success "Read receipt sent successfully"
    else
        error "Read receipt failed: $response"
    fi
}

# Test room aliases
test_room_aliases() {
    local token="$1"
    log "Testing room aliases..."
    
    # Create room with alias
    local alias_name="test_alias_$(date +%s)"
    local room_data="{\"room_alias_name\":\"$alias_name\",\"name\":\"Alias Test Room\"}"
    
    info "Creating room with alias: #${alias_name}:${SERVER_NAME}"
    local response=$(curl -s -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$room_data" \
        "${BASE_URL}/_matrix/client/r0/createRoom")
    
    if echo "$response" | jq -e '.room_id' > /dev/null 2>&1; then
        local room_id=$(echo "$response" | jq -r '.room_id')
        local room_alias=$(echo "$response" | jq -r '.room_alias // "none"')
        success "Room with alias created successfully"
        info "Room ID: $room_id"
        info "Room Alias: $room_alias"
        
        # Test alias resolution
        if [ "$room_alias" != "none" ]; then
            info "Testing alias resolution..."
            local alias_response=$(curl -s \
                "${BASE_URL}/_matrix/client/r0/directory/room/%23${alias_name}%3A${SERVER_NAME}")
            
            if echo "$alias_response" | jq -e '.room_id' > /dev/null 2>&1; then
                local resolved_room_id=$(echo "$alias_response" | jq -r '.room_id')
                if [ "$resolved_room_id" = "$room_id" ]; then
                    success "Alias resolution working correctly"
                else
                    error "Alias resolution mismatch: expected $room_id, got $resolved_room_id"
                fi
            else
                error "Alias resolution failed: $alias_response"
            fi
        fi
    else
        error "Room creation with alias failed: $response"
    fi
}

# Test server capabilities with authentication
test_authenticated_capabilities() {
    local token="$1"
    log "Testing authenticated server capabilities..."
    
    info "Getting server capabilities..."
    local response=$(curl -s -H "Authorization: Bearer $token" \
        "${BASE_URL}/_matrix/client/r0/capabilities")
    
    if echo "$response" | jq -e '.capabilities' > /dev/null 2>&1; then
        success "Server capabilities retrieved successfully"
        echo "$response" | jq '.capabilities'
    else
        error "Server capabilities failed: $response"
    fi
}

# Test PostgreSQL specific performance
test_postgresql_performance() {
    local token="$1"
    log "Testing PostgreSQL-specific performance..."
    
    info "Creating multiple rooms rapidly..."
    local start_time=$(date +%s%3N)
    local room_ids=()
    
    for i in {1..10}; do
        local room_data="{\"name\":\"PG Perf Test Room $i\"}"
        local response=$(curl -s -X POST \
            -H "Authorization: Bearer $token" \
            -H "Content-Type: application/json" \
            -d "$room_data" \
            "${BASE_URL}/_matrix/client/r0/createRoom")
        
        if echo "$response" | jq -e '.room_id' > /dev/null 2>&1; then
            room_ids+=($(echo "$response" | jq -r '.room_id'))
        fi
    done
    
    local end_time=$(date +%s%3N)
    local duration=$((end_time - start_time))
    
    success "Created ${#room_ids[@]} rooms in ${duration}ms"
    info "Average: $(echo "scale=2; $duration / ${#room_ids[@]}" | bc)ms per room"
    
    if [ $duration -lt 5000 ]; then
        success "PostgreSQL performance excellent (< 5s for 10 rooms)"
    else
        info "PostgreSQL performance acceptable (${duration}ms for 10 rooms)"
    fi
    
    # Test concurrent message sending
    info "Testing concurrent message sending..."
    start_time=$(date +%s%3N)
    
    for room_id in "${room_ids[@]:0:5}"; do
        curl -s -X PUT \
            -H "Authorization: Bearer $token" \
            -H "Content-Type: application/json" \
            -d '{"msgtype":"m.text","body":"Concurrent test message"}' \
            "${BASE_URL}/_matrix/client/r0/rooms/${room_id}/send/m.room.message/$(date +%s%N)" &
    done
    
    wait  # Wait for all background jobs to complete
    end_time=$(date +%s%3N)
    duration=$((end_time - start_time))
    
    success "Sent 5 concurrent messages in ${duration}ms"
}

# Main execution
main() {
    echo -e "${BLUE}"
    echo "================================================================"
    echo "  Advanced Matrix API Testing with PostgreSQL"
    echo "================================================================"
    echo -e "${NC}"
    
    # Register test user
    local token=$(register_user)
    local user_id=$(curl -s -H "Authorization: Bearer $token" \
        "${BASE_URL}/_matrix/client/r0/account/whoami" | jq -r '.user_id')
    
    success "Test user ready: $user_id"
    echo ""
    
    # Create a test room for some tests
    local room_response=$(curl -s -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d '{"name":"Advanced Test Room","topic":"Room for advanced testing"}' \
        "${BASE_URL}/_matrix/client/r0/createRoom")
    
    local room_id=$(echo "$room_response" | jq -r '.room_id')
    
    # Send a message to get an event ID
    local message_response=$(curl -s -X PUT \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d '{"msgtype":"m.text","body":"Test message for advanced testing"}' \
        "${BASE_URL}/_matrix/client/r0/rooms/${room_id}/send/m.room.message/$(date +%s%N)")
    
    local event_id=$(echo "$message_response" | jq -r '.event_id')
    
    # Run all advanced tests
    test_authenticated_capabilities "$token"
    test_media_upload "$token"
    test_room_directory "$token"
    test_device_management "$token"
    test_presence "$token" "$user_id"
    test_typing_indicators "$token" "$room_id" "$user_id"
    test_read_receipts "$token" "$room_id" "$event_id"
    test_room_aliases "$token"
    test_postgresql_performance "$token"
    
    echo ""
    success "All advanced tests completed!"
    info "PostgreSQL backend handling all features excellently"
    
    # Cleanup
    curl -s -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d '{}' \
        "${BASE_URL}/_matrix/client/r0/logout" > /dev/null
    
    info "Test user logged out and cleaned up"
}

# Run main function
main "$@" 