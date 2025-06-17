#!/bin/bash

##
# matrixon Matrix Server - High Performance Startup Script
# 
# Automatically configures and starts matrixon for 10,000+ concurrent connections
# 
# @author: Matrix Server Performance Team
# @date: 2024-01-01
# @version: 1.0.0
##

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="${SCRIPT_DIR}/configs/high_performance.toml"
PID_FILE="${SCRIPT_DIR}/matrixon.pid"
LOG_FILE="${SCRIPT_DIR}/logs/matrixon-performance.log"

echo -e "${BLUE}🚀 matrixon Matrix Server - High Performance Edition${NC}"
echo -e "${BLUE}===================================================${NC}"

# Create necessary directories
mkdir -p "${SCRIPT_DIR}/logs"
mkdir -p "${SCRIPT_DIR}/data"

# System configuration check
echo -e "${YELLOW}🔧 Checking system configuration...${NC}"

# Check available memory
TOTAL_MEMORY=$(free -m | awk 'NR==2{printf "%.0f", $2}')
if [ "$TOTAL_MEMORY" -lt 8192 ]; then
    echo -e "${YELLOW}⚠️ Warning: System has ${TOTAL_MEMORY}MB RAM. Recommended: 16GB+ for 10k connections${NC}"
else
    echo -e "${GREEN}✅ Memory: ${TOTAL_MEMORY}MB (Sufficient)${NC}"
fi

# Check CPU cores
CPU_CORES=$(nproc)
if [ "$CPU_CORES" -lt 16 ]; then
    echo -e "${YELLOW}⚠️ Warning: System has ${CPU_CORES} CPU cores. Recommended: 16+ cores for optimal performance${NC}"
else
    echo -e "${GREEN}✅ CPU Cores: ${CPU_CORES} (Excellent)${NC}"
fi

# Check file descriptor limits
ULIMIT_N=$(ulimit -n)
if [ "$ULIMIT_N" -lt 65536 ]; then
    echo -e "${YELLOW}⚠️ Warning: File descriptor limit is ${ULIMIT_N}. Setting to 65536...${NC}"
    ulimit -n 65536 || echo -e "${RED}❌ Failed to increase file descriptor limit${NC}"
else
    echo -e "${GREEN}✅ File descriptors: ${ULIMIT_N} (Sufficient)${NC}"
fi

# Environment variables for high performance
export matrixon_CONFIG="${CONFIG_FILE}"
export RUST_LOG="warn"  # Reduce logging for performance
export RUST_BACKTRACE="0"  # Disable backtrace for performance

# JeMalloc configuration for high concurrency
export MALLOC_CONF="background_thread:true,metadata_thp:auto,dirty_decay_ms:30000,muzzy_decay_ms:30000"

# Set PostgreSQL-related environment variables
export DATABASE_URL="${DATABASE_URL:-postgresql://matrixon:matrixon@localhost:5432/matrixon}"
export PGCONNECT_TIMEOUT="${PGCONNECT_TIMEOUT:-10}"
export PGAPPLICATION_NAME="matrixon-high-performance"

echo -e "${YELLOW}🔧 Configuration:${NC}"
echo -e "   📊 Target: 10,000+ concurrent connections"
echo -e "   🧵 Worker threads: 64"
echo -e "   🗄️ Database cache: 8GB"
echo -e "   📝 PDU cache: 5M entries"
echo -e "   🔧 Config file: ${CONFIG_FILE}"

# Check if configuration file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${RED}❌ Configuration file not found: ${CONFIG_FILE}${NC}"
    echo -e "${YELLOW}💡 Creating default high-performance configuration...${NC}"
    
    # Create configuration file if it doesn't exist
    cat > "$CONFIG_FILE" << 'EOF'
[server]
address = "0.0.0.0"
port = 8008
server_name = "matrix.example.com"
max_concurrent_requests = 10000
worker_threads = 64
blocking_threads = 32

[database]
backend = "postgresql"
path = "postgresql://matrixon:matrixon@localhost:5432/matrixon"
db_cache_capacity_mb = 8192.0
pdu_cache_capacity = 5000000

[performance]
cleanup_second_interval = 180
allow_check_for_updates = false

[monitoring]
enable_metrics = true
log_level = "warn"
EOF
    
    echo -e "${GREEN}✅ Created default configuration${NC}"
fi

# Database setup check
echo -e "${YELLOW}🗄️ Checking database connection...${NC}"
if command -v psql >/dev/null 2>&1; then
    if psql "$DATABASE_URL" -c "SELECT 1;" >/dev/null 2>&1; then
        echo -e "${GREEN}✅ PostgreSQL connection successful${NC}"
    else
        echo -e "${RED}❌ PostgreSQL connection failed${NC}"
        echo -e "${YELLOW}💡 Make sure PostgreSQL is running and accessible${NC}"
        echo -e "${YELLOW}💡 Connection string: ${DATABASE_URL}${NC}"
    fi
else
    echo -e "${YELLOW}⚠️ psql not found, skipping database check${NC}"
fi

# Build the project if needed
echo -e "${YELLOW}🔨 Building matrixon...${NC}"
if ! cargo build --release --features backend_postgresql; then
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Build completed${NC}"

# Stop existing instance if running
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat "$PID_FILE")
    if kill -0 "$OLD_PID" 2>/dev/null; then
        echo -e "${YELLOW}🛑 Stopping existing matrixon instance (PID: $OLD_PID)...${NC}"
        kill "$OLD_PID"
        sleep 2
        if kill -0 "$OLD_PID" 2>/dev/null; then
            echo -e "${YELLOW}⚡ Force killing existing instance...${NC}"
            kill -9 "$OLD_PID"
        fi
    fi
    rm -f "$PID_FILE"
fi

# Start performance monitoring if available
if command -v htop >/dev/null 2>&1 && [ "${ENABLE_MONITORING:-false}" = "true" ]; then
    echo -e "${YELLOW}📊 Starting performance monitoring...${NC}"
    htop &
    HTOP_PID=$!
fi

# Start matrixon with high-performance settings
echo -e "${GREEN}🚀 Starting matrixon Matrix Server (High Performance Edition)...${NC}"
echo -e "${BLUE}===================================================${NC}"
echo -e "${YELLOW}📊 Performance Targets:${NC}"
echo -e "   🔗 Concurrent connections: 10,000+"
echo -e "   📈 Throughput: 10,000+ ops/sec"
echo -e "   ⚡ Latency: < 100ms average"
echo -e "   💾 Memory usage: < 16GB"
echo -e "${BLUE}===================================================${NC}"

# Start the server
(
    echo "$(date): Starting matrixon with high-performance configuration"
    
    # Use nice to prioritize the process
    nice -n -10 ./target/release/matrixon 2>&1 | tee -a "$LOG_FILE"
) &

matrixon_PID=$!
echo "$matrixon_PID" > "$PID_FILE"

echo -e "${GREEN}✅ matrixon started successfully!${NC}"
echo -e "${YELLOW}📊 Process ID: ${matrixon_PID}${NC}"
echo -e "${YELLOW}📝 Log file: ${LOG_FILE}${NC}"
echo -e "${YELLOW}⚙️ Config file: ${CONFIG_FILE}${NC}"

# Setup signal handlers for graceful shutdown
cleanup() {
    echo -e "\n${YELLOW}🛑 Shutting down matrixon...${NC}"
    
    if [ -n "${matrixon_PID:-}" ] && kill -0 "$matrixon_PID" 2>/dev/null; then
        echo -e "${YELLOW}📤 Sending SIGTERM to matrixon (PID: $matrixon_PID)...${NC}"
        kill "$matrixon_PID"
        
        # Wait for graceful shutdown
        for i in {1..30}; do
            if ! kill -0 "$matrixon_PID" 2>/dev/null; then
                echo -e "${GREEN}✅ matrixon shut down gracefully${NC}"
                break
            fi
            echo -e "${YELLOW}⏳ Waiting for graceful shutdown... ($i/30)${NC}"
            sleep 1
        done
        
        # Force kill if still running
        if kill -0 "$matrixon_PID" 2>/dev/null; then
            echo -e "${YELLOW}⚡ Force killing matrixon...${NC}"
            kill -9 "$matrixon_PID"
        fi
    fi
    
    # Stop monitoring if started
    if [ -n "${HTOP_PID:-}" ] && kill -0 "$HTOP_PID" 2>/dev/null; then
        kill "$HTOP_PID" 2>/dev/null || true
    fi
    
    # Clean up PID file
    rm -f "$PID_FILE"
    
    echo -e "${GREEN}✅ Shutdown complete${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Monitor the process
echo -e "${YELLOW}📊 Monitoring matrixon performance... (Press Ctrl+C to stop)${NC}"
echo -e "${YELLOW}💡 View logs: tail -f ${LOG_FILE}${NC}"
echo -e "${YELLOW}💡 Check metrics: curl http://localhost:9090/metrics${NC}"

# Simple monitoring loop
while kill -0 "$matrixon_PID" 2>/dev/null; do
    sleep 10
    
    # Get process stats
    if command -v ps >/dev/null 2>&1; then
        MEMORY_MB=$(ps -o rss= -p "$matrixon_PID" 2>/dev/null | awk '{print int($1/1024)}' || echo "0")
        CPU_PERCENT=$(ps -o %cpu= -p "$matrixon_PID" 2>/dev/null | awk '{print $1}' || echo "0")
        
        if [ "$MEMORY_MB" -gt 0 ]; then
            echo -e "${BLUE}📊 $(date +'%H:%M:%S') - Memory: ${MEMORY_MB}MB, CPU: ${CPU_PERCENT}%${NC}"
        fi
    fi
done

echo -e "${RED}❌ matrixon process stopped unexpectedly${NC}"
cleanup 
