#!/bin/bash
# Matrixon Matrix Server - Connection Pool and Resource Monitor
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 1.0
# Purpose: Monitor connection pools and resource usage with threshold alerting

set -e

# Configuration
POSTGRES_HOST="postgres"
POSTGRES_PORT="5432"
POSTGRES_USER="matrixon"
POSTGRES_DB="matrixon"
REDIS_HOST="redis"
REDIS_PORT="6379"
REDIS_PASSWORD="matrixon_redis_password_change_me"
LOGSTASH_HOST="logstash"
LOGSTASH_PORT="5000"
MONITOR_LOG="/var/log/monitoring/connection-pool.log"

# Thresholds
POSTGRES_CONN_WARNING=150
POSTGRES_CONN_CRITICAL=180
POSTGRES_CONN_MAX=200
REDIS_CONN_WARNING=800
REDIS_CONN_CRITICAL=900
REDIS_CONN_MAX=1000
CPU_WARNING=80
CPU_CRITICAL=95
MEMORY_WARNING=85
MEMORY_CRITICAL=95
DISK_WARNING=80
DISK_CRITICAL=90

# Create log directory
mkdir -p "$(dirname "$MONITOR_LOG")"

# Logging function
log_message() {
    local level="$1"
    local message="$2"
    local timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    echo "[$timestamp] $level: $message" | tee -a "$MONITOR_LOG"
}

# Send metric to Logstash
send_metric() {
    local metric_data="$1"
    echo "$metric_data" | nc -w 5 "$LOGSTASH_HOST" "$LOGSTASH_PORT" 2>/dev/null || log_message "WARNING" "Failed to send metric to Logstash"
}

# Check PostgreSQL connection pool
check_postgres_connections() {
    log_message "INFO" "🔧 Checking PostgreSQL connection pool..."
    
    local conn_count=$(PGPASSWORD="$PGPASSWORD" psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -c "SELECT count(*) FROM pg_stat_activity WHERE state != 'idle';" 2>/dev/null | xargs)
    
    if [[ -z "$conn_count" || ! "$conn_count" =~ ^[0-9]+$ ]]; then
        log_message "ERROR" "❌ Failed to get PostgreSQL connection count"
        return 1
    fi
    
    local usage_percent=$((conn_count * 100 / POSTGRES_CONN_MAX))
    
    log_message "INFO" "PostgreSQL connections: $conn_count/$POSTGRES_CONN_MAX ($usage_percent%)"
    
    # Send metric
    send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"metric_type\":\"connection_pool\",\"service\":\"postgres\",\"active_connections\":$conn_count,\"max_connections\":$POSTGRES_CONN_MAX,\"usage_percent\":$usage_percent,\"status\":\"ok\"}"
    
    # Check thresholds
    if [ "$conn_count" -ge "$POSTGRES_CONN_CRITICAL" ]; then
        log_message "CRITICAL" "❌ PostgreSQL connection pool critical: $conn_count/$POSTGRES_CONN_MAX"
        send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"alert\",\"alert_type\":\"connection_pool_critical\",\"service\":\"postgres\",\"severity\":\"critical\",\"active_connections\":$conn_count,\"threshold\":$POSTGRES_CONN_CRITICAL,\"message\":\"PostgreSQL connection pool critical\"}"
    elif [ "$conn_count" -ge "$POSTGRES_CONN_WARNING" ]; then
        log_message "WARNING" "⚠️ PostgreSQL connection pool high: $conn_count/$POSTGRES_CONN_MAX"
        send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"alert\",\"alert_type\":\"connection_pool_warning\",\"service\":\"postgres\",\"severity\":\"warning\",\"active_connections\":$conn_count,\"threshold\":$POSTGRES_CONN_WARNING,\"message\":\"PostgreSQL connection pool usage high\"}"
    else
        log_message "INFO" "✅ PostgreSQL connection pool normal: $conn_count/$POSTGRES_CONN_MAX"
    fi
}

# Check Redis connections
check_redis_connections() {
    log_message "INFO" "🔧 Checking Redis connection pool..."
    
    local conn_count=$(redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" -a "$REDIS_PASSWORD" --no-auth-warning info clients 2>/dev/null | grep connected_clients | cut -d: -f2 | tr -d '\r\n')
    
    if [[ -z "$conn_count" || ! "$conn_count" =~ ^[0-9]+$ ]]; then
        log_message "ERROR" "❌ Failed to get Redis connection count"
        return 1
    fi
    
    local usage_percent=$((conn_count * 100 / REDIS_CONN_MAX))
    
    log_message "INFO" "Redis connections: $conn_count/$REDIS_CONN_MAX ($usage_percent%)"
    
    # Send metric
    send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"metric_type\":\"connection_pool\",\"service\":\"redis\",\"connected_clients\":$conn_count,\"max_clients\":$REDIS_CONN_MAX,\"usage_percent\":$usage_percent,\"status\":\"ok\"}"
    
    # Check thresholds
    if [ "$conn_count" -ge "$REDIS_CONN_CRITICAL" ]; then
        log_message "CRITICAL" "❌ Redis connection pool critical: $conn_count/$REDIS_CONN_MAX"
        send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"alert\",\"alert_type\":\"connection_pool_critical\",\"service\":\"redis\",\"severity\":\"critical\",\"connected_clients\":$conn_count,\"threshold\":$REDIS_CONN_CRITICAL,\"message\":\"Redis connection pool critical\"}"
    elif [ "$conn_count" -ge "$REDIS_CONN_WARNING" ]; then
        log_message "WARNING" "⚠️ Redis connection pool high: $conn_count/$REDIS_CONN_MAX"
        send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"alert\",\"alert_type\":\"connection_pool_warning\",\"service\":\"redis\",\"severity\":\"warning\",\"connected_clients\":$conn_count,\"threshold\":$REDIS_CONN_WARNING,\"message\":\"Redis connection pool usage high\"}"
    else
        log_message "INFO" "✅ Redis connection pool normal: $conn_count/$REDIS_CONN_MAX"
    fi
}

# Check system resources
check_system_resources() {
    log_message "INFO" "🔧 Checking system resources..."
    
    # CPU usage
    local cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//' | head -1)
    if [[ -z "$cpu_usage" ]]; then
        cpu_usage=$(grep 'cpu ' /proc/stat | awk '{usage=($2+$4)*100/($2+$4+$5)} END {print int(usage)}')
    fi
    
    # Memory usage
    local mem_total=$(free | grep Mem | awk '{print $2}')
    local mem_used=$(free | grep Mem | awk '{print $3}')
    local mem_usage=$((mem_used * 100 / mem_total))
    
    # Disk usage (root filesystem)
    local disk_usage=$(df / | tail -1 | awk '{print int($5)}' | sed 's/%//')
    
    log_message "INFO" "System resources - CPU: ${cpu_usage}%, Memory: ${mem_usage}%, Disk: ${disk_usage}%"
    
    # Send metrics
    send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"metric_type\":\"system_resources\",\"cpu_usage\":$cpu_usage,\"memory_usage\":$mem_usage,\"disk_usage\":$disk_usage,\"status\":\"ok\"}"
    
    # Check CPU thresholds
    if (( $(echo "$cpu_usage >= $CPU_CRITICAL" | bc -l) )); then
        log_message "CRITICAL" "❌ CPU usage critical: ${cpu_usage}%"
        send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"alert\",\"alert_type\":\"cpu_critical\",\"service\":\"system\",\"severity\":\"critical\",\"cpu_usage\":$cpu_usage,\"threshold\":$CPU_CRITICAL,\"message\":\"Critical CPU usage detected\"}"
    elif (( $(echo "$cpu_usage >= $CPU_WARNING" | bc -l) )); then
        log_message "WARNING" "⚠️ CPU usage high: ${cpu_usage}%"
        send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"alert\",\"alert_type\":\"cpu_warning\",\"service\":\"system\",\"severity\":\"warning\",\"cpu_usage\":$cpu_usage,\"threshold\":$CPU_WARNING,\"message\":\"High CPU usage detected\"}"
    fi
    
    # Check memory thresholds
    if [ "$mem_usage" -ge "$MEMORY_CRITICAL" ]; then
        log_message "CRITICAL" "❌ Memory usage critical: ${mem_usage}%"
        send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"alert\",\"alert_type\":\"memory_critical\",\"service\":\"system\",\"severity\":\"critical\",\"memory_usage\":$mem_usage,\"threshold\":$MEMORY_CRITICAL,\"message\":\"Critical memory usage detected\"}"
    elif [ "$mem_usage" -ge "$MEMORY_WARNING" ]; then
        log_message "WARNING" "⚠️ Memory usage high: ${mem_usage}%"
        send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"alert\",\"alert_type\":\"memory_warning\",\"service\":\"system\",\"severity\":\"warning\",\"memory_usage\":$mem_usage,\"threshold\":$MEMORY_WARNING,\"message\":\"High memory usage detected\"}"
    fi
    
    # Check disk thresholds
    if [ "$disk_usage" -ge "$DISK_CRITICAL" ]; then
        log_message "CRITICAL" "❌ Disk usage critical: ${disk_usage}%"
        send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"alert\",\"alert_type\":\"disk_critical\",\"service\":\"system\",\"severity\":\"critical\",\"disk_usage\":$disk_usage,\"threshold\":$DISK_CRITICAL,\"message\":\"Critical disk usage detected\"}"
    elif [ "$disk_usage" -ge "$DISK_WARNING" ]; then
        log_message "WARNING" "⚠️ Disk usage high: ${disk_usage}%"
        send_metric "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"alert\",\"alert_type\":\"disk_warning\",\"service\":\"system\",\"severity\":\"warning\",\"disk_usage\":$disk_usage,\"threshold\":$DISK_WARNING,\"message\":\"High disk usage detected\"}"
    fi
}

# Main monitoring loop
main() {
    log_message "INFO" "🎉 Starting Matrixon connection pool and resource monitor..."
    
    while true; do
        check_postgres_connections
        sleep 2
        check_redis_connections
        sleep 2
        check_system_resources
        
        log_message "INFO" "✅ Monitoring cycle completed. Sleeping for 60 seconds..."
        sleep 60
    done
}

# Handle signals
trap 'log_message "INFO" "Monitor stopped"; exit 0' SIGTERM SIGINT

# Check if running as main script
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi 
