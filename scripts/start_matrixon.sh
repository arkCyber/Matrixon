#!/bin/bash

##
# matrixon Matrix Server - Simple Startup Script
##

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting matrixon Matrix Server${NC}"
echo -e "${BLUE}==================================${NC}"

# Configuration
CONFIG_FILE="./matrixon.toml"
PID_FILE="./matrixon.pid"

# Check if configuration exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${YELLOW}⚠️ Configuration file not found: ${CONFIG_FILE}${NC}"
    exit 1
fi

# Check if already running
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat "$PID_FILE")
    if kill -0 "$OLD_PID" 2>/dev/null; then
        echo -e "${YELLOW}⚠️ matrixon is already running (PID: $OLD_PID)${NC}"
        echo -e "${YELLOW}💡 Use ./stop_matrixon.sh to stop it first${NC}"
        exit 1
    else
        rm -f "$PID_FILE"
    fi
fi

# Start the server
echo -e "${YELLOW}🔧 Starting matrixon Matrix Server...${NC}"
echo -e "${YELLOW}📝 Config: ${CONFIG_FILE}${NC}"
echo -e "${YELLOW}🌐 URL: http://localhost:6167${NC}"

matrixon_CONFIG="$CONFIG_FILE" ./target/release/matrixon &
matrixon_PID=$!
echo "$matrixon_PID" > "$PID_FILE"

echo -e "${GREEN}✅ matrixon started successfully!${NC}"
echo -e "${YELLOW}📊 Process ID: ${matrixon_PID}${NC}"
echo -e "${YELLOW}🛑 To stop: ./stop_matrixon.sh${NC}"
echo -e "${YELLOW}📊 To test: curl http://localhost:6167/_matrix/client/versions${NC}"

# Wait a moment and check if it's still running
sleep 2
if ! kill -0 "$matrixon_PID" 2>/dev/null; then
    echo -e "${RED}❌ Server failed to start${NC}"
    rm -f "$PID_FILE"
    exit 1
fi

echo -e "${GREEN}🎉 matrixon Matrix Server is running!${NC}" 
