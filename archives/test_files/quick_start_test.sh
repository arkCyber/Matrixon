#!/bin/bash

# =============================================================================
# Matrixon Matrix Server - Quick Start Test Script
# =============================================================================
#
# Project: Matrixon - Ultra High Performance Matrix NextServer
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 0.11.0-alpha
#
# Description:
#   Quick test script to run Matrixon server and test basic functionality
#   with curl commands.
#
# =============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting Matrixon Matrix Server Test${NC}"
echo "======================================"

# Configuration
SERVER_URL="http://localhost:6167"
CONFIG_FILE="test-config.toml"

# Function to test if server is running
test_server() {
    local endpoint=$1
    local description=$2
    echo -e "\n${YELLOW}🔧 Testing: $description${NC}"
    echo "Endpoint: $endpoint"
    echo "Command: curl -s -o /dev/null -w \"%{http_code}\" \"$endpoint\""
    
    response=$(curl -s -o /dev/null -w "%{http_code}" "$endpoint" 2>/dev/null || echo "000")
    
    if [ "$response" != "000" ]; then
        echo -e "${GREEN}✅ Response: HTTP $response${NC}"
        # Show actual response
        echo "Response body:"
        curl -s "$endpoint" 2>/dev/null | head -20
    else
        echo -e "${RED}❌ Connection failed${NC}"
    fi
}

# Function to start server in background
start_server() {
    echo -e "\n${BLUE}🔧 Starting Matrixon server...${NC}"
    echo "Using config: $CONFIG_FILE"
    echo "Server will run on: $SERVER_URL"
    
    # Kill any existing process on port 6167
    lsof -ti:6167 | xargs kill -9 2>/dev/null || true
    
    # Start server
    echo "Running: cargo run --bin matrixon -- --config $CONFIG_FILE"
    cargo run --bin matrixon -- --config "$CONFIG_FILE" &
    SERVER_PID=$!
    echo "Server PID: $SERVER_PID"
    
    # Wait for server to start
    echo "Waiting for server to start..."
    for i in {1..30}; do
        if curl -s "$SERVER_URL" >/dev/null 2>&1; then
            echo -e "${GREEN}✅ Server started successfully!${NC}"
            return 0
        fi
        sleep 1
        echo -n "."
    done
    
    echo -e "${RED}❌ Server failed to start within 30 seconds${NC}"
    return 1
}

# Main testing function
run_tests() {
    echo -e "\n${BLUE}🧪 Running Matrix API Tests${NC}"
    echo "============================"
    
    # Basic connectivity
    test_server "$SERVER_URL" "Basic connectivity"
    test_server "$SERVER_URL/_matrix/client/versions" "Matrix Client versions"
    test_server "$SERVER_URL/_matrix/client/r0/capabilities" "Client capabilities"
    test_server "$SERVER_URL/_matrix/client/r0/register/available" "Registration check"
    
    # Health and monitoring
    test_server "$SERVER_URL/health" "Health check"
    test_server "$SERVER_URL/metrics" "Metrics endpoint"
    
    echo -e "\n${BLUE}📊 Manual Testing Commands${NC}"
    echo "==========================="
    echo "You can now manually test the server with these curl commands:"
    echo ""
    echo "# Check server info:"
    echo "curl -X GET '$SERVER_URL/_matrix/client/versions'"
    echo ""
    echo "# Register a new user:"
    echo "curl -X POST '$SERVER_URL/_matrix/client/r0/register' \\"
    echo "  -H 'Content-Type: application/json' \\"
    echo "  -d '{\"username\": \"testuser\", \"password\": \"testpass\"}'"
    echo ""
    echo "# Login:"
    echo "curl -X POST '$SERVER_URL/_matrix/client/r0/login' \\"
    echo "  -H 'Content-Type: application/json' \\"
    echo "  -d '{\"type\": \"m.login.password\", \"user\": \"testuser\", \"password\": \"testpass\"}'"
    echo ""
    echo "# Create a room:"
    echo "curl -X POST '$SERVER_URL/_matrix/client/r0/createRoom' \\"
    echo "  -H 'Content-Type: application/json' \\"
    echo "  -H 'Authorization: Bearer YOUR_ACCESS_TOKEN' \\"
    echo "  -d '{\"preset\": \"public_chat\", \"name\": \"Test Room\"}'"
    echo ""
    echo -e "${GREEN}🎉 Server is running at: $SERVER_URL${NC}"
    echo -e "${YELLOW}📝 Press Ctrl+C to stop the server${NC}"
}

# Main execution
main() {
    # Check if config file exists
    if [ ! -f "$CONFIG_FILE" ]; then
        echo -e "${RED}❌ Config file $CONFIG_FILE not found${NC}"
        exit 1
    fi
    
    # Start server
    if start_server; then
        run_tests
        
        # Keep script running until user stops it
        echo -e "\n${BLUE}🔄 Server is running. Press Ctrl+C to stop...${NC}"
        trap "echo -e '\n${YELLOW}🛑 Stopping server...$' ; kill $SERVER_PID 2>/dev/null ; echo -e '${GREEN}✅ Server stopped${NC}' ; exit 0" INT
        
        # Keep alive
        while kill -0 $SERVER_PID 2>/dev/null; do
            sleep 1
        done
    else
        echo -e "${RED}❌ Failed to start server${NC}"
        exit 1
    fi
}

# Run the script
main "$@" 
