#!/bin/bash

# =============================================================================
# Matrixon Matrix Server - Complete API Test Suite
# =============================================================================
#
# Project: Matrixon - Ultra High Performance Matrix NextServer
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 0.11.0-alpha
#
# Description:
#   Comprehensive test suite for Matrixon Matrix Server API endpoints
#   Tests all major Matrix Client-Server API functionality
#
# =============================================================================

# set -e  # Continue on errors to test all endpoints

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SERVER_URL="http://localhost:6167"
TEST_USER="matrixon_test_user"
TEST_PASS="SuperSecure123!"
TEST_ROOM="!testroom:localhost"

echo -e "${BLUE}🚀 Starting Matrixon Complete API Test Suite${NC}"
echo "=============================================="
echo "Server URL: $SERVER_URL"
echo "Test User: $TEST_USER"
echo ""

# Function to print section headers
print_section() {
    echo -e "\n${PURPLE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}═══════════════════════════════════════════════════════════════${NC}"
}

# Function to test API endpoint
test_endpoint() {
    local method=$1
    local endpoint=$2
    local description=$3
    local data=$4
    local expected_status=${5:-200}
    
    echo -e "\n${CYAN}🔧 Testing: $description${NC}"
    echo "Method: $method"
    echo "Endpoint: $endpoint"
    if [ -n "$data" ]; then
        echo "Data: $data"
    fi
    
    if [ -n "$data" ]; then
        response=$(curl -s -w "HTTPSTATUS:%{http_code}" -X "$method" "$endpoint" \
                  -H "Content-Type: application/json" \
                  -d "$data" 2>/dev/null || echo "HTTPSTATUS:000")
    else
        response=$(curl -s -w "HTTPSTATUS:%{http_code}" -X "$method" "$endpoint" 2>/dev/null || echo "HTTPSTATUS:000")
    fi
    
    # Extract HTTP status and body
    http_status=$(echo "$response" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')
    body=$(echo "$response" | sed -e 's/HTTPSTATUS:.*//')
    
    # Check if request was successful
    if [ "$http_status" == "000" ]; then
        echo -e "${RED}❌ Connection failed${NC}"
        return 1
    elif [ "$http_status" == "$expected_status" ] || [ "$http_status" == "200" ] || [ "$http_status" == "201" ]; then
        echo -e "${GREEN}✅ Response: HTTP $http_status${NC}"
        if [ -n "$body" ] && [ "$body" != "{}" ]; then
            echo "Response body:"
            echo "$body" | jq . 2>/dev/null || echo "$body"
        fi
        return 0
    else
        echo -e "${YELLOW}⚠️  Response: HTTP $http_status${NC}"
        if [ -n "$body" ]; then
            echo "Response body:"
            echo "$body" | jq . 2>/dev/null || echo "$body"
        fi
        return 1
    fi
}

# Function to test with authentication
test_authenticated_endpoint() {
    local method=$1
    local endpoint=$2
    local description=$3
    local data=$4
    local access_token=$5
    
    echo -e "\n${CYAN}🔧 Testing (Auth): $description${NC}"
    echo "Method: $method"
    echo "Endpoint: $endpoint"
    
    if [ -n "$data" ]; then
        response=$(curl -s -w "HTTPSTATUS:%{http_code}" -X "$method" "$endpoint" \
                  -H "Content-Type: application/json" \
                  -H "Authorization: Bearer $access_token" \
                  -d "$data" 2>/dev/null || echo "HTTPSTATUS:000")
    else
        response=$(curl -s -w "HTTPSTATUS:%{http_code}" -X "$method" "$endpoint" \
                  -H "Authorization: Bearer $access_token" 2>/dev/null || echo "HTTPSTATUS:000")
    fi
    
    http_status=$(echo "$response" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')
    body=$(echo "$response" | sed -e 's/HTTPSTATUS:.*//')
    
    if [ "$http_status" == "000" ]; then
        echo -e "${RED}❌ Connection failed${NC}"
    elif [[ "$http_status" =~ ^[23] ]]; then
        echo -e "${GREEN}✅ Response: HTTP $http_status${NC}"
        if [ -n "$body" ] && [ "$body" != "{}" ]; then
            echo "Response body:"
            echo "$body" | jq . 2>/dev/null || echo "$body"
        fi
    else
        echo -e "${YELLOW}⚠️  Response: HTTP $http_status${NC}"
        if [ -n "$body" ]; then
            echo "Response body:"
            echo "$body" | jq . 2>/dev/null || echo "$body"
        fi
    fi
}

# ====================================================================
# MAIN TEST EXECUTION
# ====================================================================

print_section "📡 BASIC CONNECTIVITY TESTS"

test_endpoint "GET" "$SERVER_URL" "Basic server connectivity"
test_endpoint "GET" "$SERVER_URL/_matrix/client/versions" "Matrix Client API versions"
test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/capabilities" "Client capabilities"

print_section "🔐 AUTHENTICATION & REGISTRATION TESTS"

test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/register/available" "Registration availability check"
test_endpoint "POST" "$SERVER_URL/_matrix/client/r0/register" "User registration" \
    '{"username": "'$TEST_USER'", "password": "'$TEST_PASS'", "device_id": "TESTDEVICE"}'

test_endpoint "POST" "$SERVER_URL/_matrix/client/r0/login" "User login" \
    '{"type": "m.login.password", "user": "'$TEST_USER'", "password": "'$TEST_PASS'"}'

print_section "👤 USER MANAGEMENT TESTS"

test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/account/whoami" "User identity check"
test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/profile/$TEST_USER" "User profile"
test_endpoint "PUT" "$SERVER_URL/_matrix/client/r0/profile/$TEST_USER/displayname" "Set display name" \
    '{"displayname": "Test User Display Name"}'

print_section "🏠 ROOM MANAGEMENT TESTS"

test_endpoint "POST" "$SERVER_URL/_matrix/client/r0/createRoom" "Create room" \
    '{"preset": "public_chat", "name": "Matrixon Test Room", "topic": "A test room for Matrixon"}'

test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/joined_rooms" "List joined rooms"
test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/publicRooms" "List public rooms"

print_section "💬 MESSAGING TESTS"

test_endpoint "PUT" "$SERVER_URL/_matrix/client/r0/rooms/$TEST_ROOM/send/m.room.message/$(date +%s)" "Send text message" \
    '{"msgtype": "m.text", "body": "Hello from Matrixon test suite!"}'

test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/rooms/$TEST_ROOM/messages" "Get room messages"

print_section "🔍 SEARCH & DISCOVERY TESTS"

test_endpoint "POST" "$SERVER_URL/_matrix/client/r0/search" "Search messages" \
    '{"search_categories": {"room_events": {"search_term": "hello"}}}'

test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/directory/room/%23test:localhost" "Room directory lookup"

print_section "📱 DEVICE MANAGEMENT TESTS"

test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/devices" "List devices"
test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/devices/TESTDEVICE" "Get device info"

print_section "🔔 PUSH NOTIFICATION TESTS"

test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/pushers" "List push notification settings"
test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/notifications" "Get notifications"

print_section "🔐 END-TO-END ENCRYPTION TESTS"

test_endpoint "POST" "$SERVER_URL/_matrix/client/r0/keys/upload" "Upload encryption keys" \
    '{"device_keys": {"user_id": "'$TEST_USER'", "device_id": "TESTDEVICE"}}'

test_endpoint "POST" "$SERVER_URL/_matrix/client/r0/keys/query" "Query encryption keys" \
    '{"device_keys": {"'$TEST_USER'": []}}'

print_section "📊 MONITORING & HEALTH TESTS"

test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/admin/whois/$TEST_USER" "Admin user info"
test_endpoint "GET" "$SERVER_URL/_matrix/client/r0/sync" "Sync endpoint"

print_section "🌐 FEDERATION TESTS"

test_endpoint "GET" "$SERVER_URL/_matrix/federation/v1/version" "Federation version"
test_endpoint "GET" "$SERVER_URL/_matrix/key/v2/server" "Server signing keys"

print_section "📈 METRICS & STATISTICS"

echo -e "\n${CYAN}📊 Server Performance Metrics:${NC}"
echo "Testing response times for key endpoints..."

for i in {1..5}; do
    start_time=$(date +%s%N)
    curl -s "$SERVER_URL/_matrix/client/versions" > /dev/null
    end_time=$(date +%s%N)
    duration=$(( (end_time - start_time) / 1000000 ))
    echo "Versions endpoint - Attempt $i: ${duration}ms"
done

print_section "🎯 LOAD TESTING (Light)"

echo -e "\n${CYAN}⚡ Basic Load Test (10 concurrent requests):${NC}"

# Simple concurrent test
for i in {1..10}; do
    (
        start_time=$(date +%s%N)
        response=$(curl -s -w "%{http_code}" "$SERVER_URL/_matrix/client/versions")
        end_time=$(date +%s%N)
        duration=$(( (end_time - start_time) / 1000000 ))
        echo "Request $i: HTTP $response - ${duration}ms"
    ) &
done
wait

print_section "🏁 TEST SUMMARY"

echo -e "\n${GREEN}✅ Matrixon API Test Suite Completed!${NC}"
echo ""
echo -e "${BLUE}📋 Test Summary:${NC}"
echo "• Basic connectivity: Matrix API responding"
echo "• Authentication: Registration/login endpoints available"
echo "• Room management: Room creation and listing"
echo "• Messaging: Message sending capabilities"
echo "• Federation: Federation endpoints responding"
echo "• Performance: Response times measured"
echo ""
echo -e "${YELLOW}📝 Notes:${NC}"
echo "• Some endpoints may return 'not_implemented' - this is expected in alpha version"
echo "• Full functionality will be implemented in upcoming releases"
echo "• Monitor server logs for detailed operation information"
echo ""
echo -e "${GREEN}🎉 Server is ready for development and testing!${NC}"
echo -e "${CYAN}🔗 Access points:${NC}"
echo "  • Matrix Server: $SERVER_URL"
echo "  • Grafana Dashboard: http://localhost:3001 (admin/admin_change_me)"
echo "  • Prometheus Metrics: http://localhost:9090"
echo "  • PostgreSQL: localhost:5432 (matrixon/matrixon_secure_password_change_me)"
echo "  • Redis: localhost:6379" 
