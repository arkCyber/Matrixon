#!/bin/bash

##
# matrixon Matrix Server - Stop Script
##

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}🛑 Stopping matrixon Matrix Server${NC}"
echo -e "${YELLOW}==================================${NC}"

PID_FILE="./matrixon.pid"

# Check if PID file exists
if [ ! -f "$PID_FILE" ]; then
    echo -e "${YELLOW}⚠️ No PID file found. Server may not be running.${NC}"
    # Try to find and kill any running matrixon processes
    if pgrep -f "./target/release/matrixon" > /dev/null; then
        echo -e "${YELLOW}💡 Found running matrixon processes, stopping them...${NC}"
        pkill -f "./target/release/matrixon"
        sleep 2
        echo -e "${GREEN}✅ Stopped all matrixon processes${NC}"
    else
        echo -e "${GREEN}✅ No matrixon processes found${NC}"
    fi
    exit 0
fi

# Read PID from file
PID=$(cat "$PID_FILE")

# Check if process is running
if ! kill -0 "$PID" 2>/dev/null; then
    echo -e "${YELLOW}⚠️ Process ${PID} is not running${NC}"
    rm -f "$PID_FILE"
    exit 0
fi

# Send SIGTERM for graceful shutdown
echo -e "${YELLOW}📤 Sending SIGTERM to matrixon (PID: $PID)...${NC}"
kill "$PID"

# Wait for graceful shutdown
for i in {1..10}; do
    if ! kill -0 "$PID" 2>/dev/null; then
        echo -e "${GREEN}✅ matrixon shut down gracefully${NC}"
        rm -f "$PID_FILE"
        exit 0
    fi
    echo -e "${YELLOW}⏳ Waiting for graceful shutdown... ($i/10)${NC}"
    sleep 1
done

# Force kill if needed
echo -e "${YELLOW}⚡ Force killing matrixon process...${NC}"
kill -9 "$PID" 2>/dev/null || true
sleep 1

if kill -0 "$PID" 2>/dev/null; then
    echo -e "${RED}❌ Failed to stop matrixon${NC}"
    exit 1
else
    echo -e "${GREEN}✅ matrixon stopped${NC}"
    rm -f "$PID_FILE"
fi 
