#!/bin/bash

# Matrixon Matrix Server - Resource Monitoring Script
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 1.0
# Purpose: Monitor connection pools and resource usage with alerting

set -euo pipefail

# Configuration
POSTGRES_HOST="${POSTGRES_HOST:-localhost}"
POSTGRES_PORT="${POSTGRES_PORT:-5432}"
POSTGRES_USER="${POSTGRES_USER:-matrixon}"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-matrixon_secure_password_change_me}"
POSTGRES_DB="${POSTGRES_DB:-matrixon}"

REDIS_HOST="${REDIS_HOST:-localhost}"
REDIS_PORT="${REDIS_PORT:-6379}"
REDIS_PASSWORD="${REDIS_PASSWORD:-matrixon_redis_password_change_me}"

LOG_FILE="${LOG_FILE:-/var/log/matrixon/resource-monitor.log}"
METRICS_FILE="${METRICS_FILE:-/var/log/matrixon/metrics.json}"
ALERT_WEBHOOK="${ALERT_WEBHOOK:-}"

# Thresholds
CPU_WARNING_THRESHOLD=80
CPU_CRITICAL_THRESHOLD=90
MEMORY_WARNING_THRESHOLD=85
MEMORY_CRITICAL_THRESHOLD=95
DISK_WARNING_THRESHOLD=85
DISK_CRITICAL_THRESHOLD=95
DB_CONN_WARNING_THRESHOLD=150
DB_CONN_CRITICAL_THRESHOLD=180
REDIS_CONN_WARNING_THRESHOLD=800
REDIS_MEMORY_WARNING_THRESHOLD=80

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    local level="$1"
    local message="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

# Alert function
send_alert() {
    local severity="$1"
    local service="$2"
    local message="$3"
    local value="$4"
    
    local emoji=""
    case "$severity" in
        "critical") emoji="🚨" ;;
        "warning") emoji="⚠️" ;;
        "info") emoji="ℹ️" ;;
    esac
    
    log "$severity" "$emoji $service: $message (value: $value)"
    
    if [[ -n "$ALERT_WEBHOOK" ]]; then
        local payload=$(cat <<EOF
{
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "severity": "$severity",
    "service": "$service",
    "message": "$message",
    "value": "$value",
    "host": "$(hostname)"
}
EOF
        )
        
        curl -s -X POST \
            -H "Content-Type: application/json" \
            -d "$payload" \
            "$ALERT_WEBHOOK" || true
    fi
}

# Get system CPU usage
get_cpu_usage() {
    local cpu_idle=$(top -l 1 | grep "CPU usage" | awk '{print $7}' | sed 's/%//' | sed 's/idle//')
    if [[ -n "$cpu_idle" ]]; then
        echo "scale=2; 100 - $cpu_idle" | bc
    else
        # Fallback for Linux
        grep 'cpu ' /proc/stat | awk '{usage=($2+$4)*100/($2+$4+$5)} END {print usage}'
    fi
}

# Get system memory usage
get_memory_usage() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        vm_stat | awk '
        /Pages free/ { free = $3 }
        /Pages active/ { active = $3 }
        /Pages inactive/ { inactive = $3 }
        /Pages speculative/ { speculative = $3 }
        /Pages wired/ { wired = $3 }
        END {
            total = free + active + inactive + speculative + wired
            used = active + inactive + wired
            print (used / total) * 100
        }'
    else
        # Linux
        free | awk '/^Mem:/ {print ($3/$2) * 100.0}'
    fi
}

# Get disk usage
get_disk_usage() {
    local path="${1:-/}"
    df "$path" | awk 'NR==2 {print $5}' | sed 's/%//'
}

# Get PostgreSQL connection count
get_postgres_connections() {
    export PGPASSWORD="$POSTGRES_PASSWORD"
    psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
        -t -c "SELECT count(*) FROM pg_stat_activity WHERE state = 'active';" 2>/dev/null | xargs || echo "0"
}

# Get PostgreSQL database size
get_postgres_db_size() {
    export PGPASSWORD="$POSTGRES_PASSWORD"
    psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
        -t -c "SELECT pg_size_pretty(pg_database_size('$POSTGRES_DB'));" 2>/dev/null | xargs || echo "0 MB"
}

# Get PostgreSQL slow queries
get_postgres_slow_queries() {
    export PGPASSWORD="$POSTGRES_PASSWORD"
    psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
        -t -c "SELECT count(*) FROM pg_stat_activity WHERE state = 'active' AND now() - query_start > interval '30 seconds';" 2>/dev/null | xargs || echo "0"
}

# Get Redis connection count
get_redis_connections() {
    if [[ -n "$REDIS_PASSWORD" ]]; then
        redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" -a "$REDIS_PASSWORD" info clients 2>/dev/null | \
            grep "connected_clients:" | cut -d: -f2 | tr -d '\r' || echo "0"
    else
        redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" info clients 2>/dev/null | \
            grep "connected_clients:" | cut -d: -f2 | tr -d '\r' || echo "0"
    fi
}

# Get Redis memory usage
get_redis_memory_usage() {
    if [[ -n "$REDIS_PASSWORD" ]]; then
        local used=$(redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" -a "$REDIS_PASSWORD" info memory 2>/dev/null | \
            grep "used_memory:" | cut -d: -f2 | tr -d '\r' || echo "0")
        local max=$(redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" -a "$REDIS_PASSWORD" config get maxmemory 2>/dev/null | \
            tail -1 || echo "1")
    else
        local used=$(redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" info memory 2>/dev/null | \
            grep "used_memory:" | cut -d: -f2 | tr -d '\r' || echo "0")
        local max=$(redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" config get maxmemory 2>/dev/null | \
            tail -1 || echo "1")
    fi
    
    if [[ "$max" -eq 0 ]]; then
        echo "0"
    else
        echo "scale=2; ($used / $max) * 100" | bc
    fi
}

# Check system resources
check_system_resources() {
    log "info" "🔍 Checking system resources..."
    
    # CPU Usage
    local cpu_usage=$(get_cpu_usage)
    local cpu_int=${cpu_usage%.*}
    
    if [[ $cpu_int -ge $CPU_CRITICAL_THRESHOLD ]]; then
        send_alert "critical" "system" "Critical CPU usage" "${cpu_usage}%"
    elif [[ $cpu_int -ge $CPU_WARNING_THRESHOLD ]]; then
        send_alert "warning" "system" "High CPU usage" "${cpu_usage}%"
    fi
    
    # Memory Usage
    local memory_usage=$(get_memory_usage)
    local memory_int=${memory_usage%.*}
    
    if [[ $memory_int -ge $MEMORY_CRITICAL_THRESHOLD ]]; then
        send_alert "critical" "system" "Critical memory usage" "${memory_usage}%"
    elif [[ $memory_int -ge $MEMORY_WARNING_THRESHOLD ]]; then
        send_alert "warning" "system" "High memory usage" "${memory_usage}%"
    fi
    
    # Disk Usage
    local disk_usage=$(get_disk_usage)
    
    if [[ $disk_usage -ge $DISK_CRITICAL_THRESHOLD ]]; then
        send_alert "critical" "system" "Critical disk usage" "${disk_usage}%"
    elif [[ $disk_usage -ge $DISK_WARNING_THRESHOLD ]]; then
        send_alert "warning" "system" "High disk usage" "${disk_usage}%"
    fi
    
    echo -e "${BLUE}System Resources:${NC}"
    echo -e "  CPU Usage: ${cpu_usage}%"
    echo -e "  Memory Usage: ${memory_usage}%"
    echo -e "  Disk Usage: ${disk_usage}%"
}

# Check database connections
check_database_connections() {
    log "info" "🔍 Checking database connections..."
    
    # PostgreSQL
    local pg_connections=$(get_postgres_connections)
    local pg_db_size=$(get_postgres_db_size)
    local pg_slow_queries=$(get_postgres_slow_queries)
    
    if [[ $pg_connections -ge $DB_CONN_CRITICAL_THRESHOLD ]]; then
        send_alert "critical" "postgresql" "Connection pool nearly exhausted" "$pg_connections/200"
    elif [[ $pg_connections -ge $DB_CONN_WARNING_THRESHOLD ]]; then
        send_alert "warning" "postgresql" "High connection usage" "$pg_connections/200"
    fi
    
    if [[ $pg_slow_queries -gt 0 ]]; then
        send_alert "warning" "postgresql" "Slow queries detected" "$pg_slow_queries queries"
    fi
    
    echo -e "${BLUE}PostgreSQL:${NC}"
    echo -e "  Active Connections: $pg_connections/200"
    echo -e "  Database Size: $pg_db_size"
    echo -e "  Slow Queries: $pg_slow_queries"
}

# Check Redis connections
check_redis_connections() {
    log "info" "🔍 Checking Redis connections..."
    
    local redis_connections=$(get_redis_connections)
    local redis_memory=$(get_redis_memory_usage)
    local redis_memory_int=${redis_memory%.*}
    
    if [[ $redis_connections -ge $REDIS_CONN_WARNING_THRESHOLD ]]; then
        send_alert "warning" "redis" "High connection count" "$redis_connections connections"
    fi
    
    if [[ $redis_memory_int -ge $REDIS_MEMORY_WARNING_THRESHOLD ]]; then
        send_alert "warning" "redis" "High memory usage" "${redis_memory}%"
    fi
    
    echo -e "${BLUE}Redis:${NC}"
    echo -e "  Connected Clients: $redis_connections"
    echo -e "  Memory Usage: ${redis_memory}%"
}

# Generate metrics JSON
generate_metrics() {
    local timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    local cpu_usage=$(get_cpu_usage)
    local memory_usage=$(get_memory_usage)
    local disk_usage=$(get_disk_usage)
    local pg_connections=$(get_postgres_connections)
    local redis_connections=$(get_redis_connections)
    local redis_memory=$(get_redis_memory_usage)
    
    cat > "$METRICS_FILE" <<EOF
{
    "timestamp": "$timestamp",
    "system": {
        "cpu_usage_percent": $cpu_usage,
        "memory_usage_percent": $memory_usage,
        "disk_usage_percent": $disk_usage
    },
    "postgresql": {
        "active_connections": $pg_connections,
        "max_connections": 200
    },
    "redis": {
        "connected_clients": $redis_connections,
        "memory_usage_percent": $redis_memory
    }
}
EOF
    
    log "info" "📊 Metrics saved to $METRICS_FILE"
}

# Main monitoring function
main() {
    echo -e "${GREEN}🔍 Matrixon Resource Monitor - $(date)${NC}"
    echo "=================================================="
    
    # Create log directory if it doesn't exist
    mkdir -p "$(dirname "$LOG_FILE")"
    mkdir -p "$(dirname "$METRICS_FILE")"
    
    # Run checks
    check_system_resources
    echo
    check_database_connections
    echo
    check_redis_connections
    echo
    
    # Generate metrics
    generate_metrics
    
    echo -e "${GREEN}✅ Monitoring complete${NC}"
    log "info" "✅ Monitoring cycle completed"
}

# Handle signals
trap 'log "info" "📝 Resource monitor stopped"; exit 0' SIGTERM SIGINT

# Run main function
main "$@" 
