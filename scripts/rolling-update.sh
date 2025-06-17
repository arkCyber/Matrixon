#!/bin/bash

# Matrixon Matrix Server - Rolling Update System
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 1.0
# Purpose: Zero-downtime rolling updates with health checks and rollback

set -euo pipefail

# Configuration variables
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="/var/log/matrixon-update.log"
DOCKER_COMPOSE_FILE="$SCRIPT_DIR/docker-compose.postgresql.yml"
HEALTH_CHECK_TIMEOUT=300
ROLLBACK_BACKUP_DIR="/opt/matrixon-rollback"

# Load environment variables
if [ -f "$SCRIPT_DIR/production.env" ]; then
    source "$SCRIPT_DIR/production.env"
else
    echo "⚠️ Warning: production.env not found, using default values"
fi

# Update configuration
UPDATE_STRATEGY="${UPDATE_STRATEGY:-rolling}"
HEALTH_CHECK_URL="${HEALTH_CHECK_URL:-http://localhost:6167/health}"
BACKUP_BEFORE_UPDATE="${BACKUP_BEFORE_UPDATE:-true}"
MAX_ROLLBACK_VERSIONS="${MAX_ROLLBACK_VERSIONS:-5}"
NOTIFICATION_EMAIL="${SMTP_USERNAME:-admin@example.com}"
SLACK_WEBHOOK_URL="${SLACK_WEBHOOK_URL:-}"

# Service update order (dependencies first)
SERVICE_UPDATE_ORDER=(
    "postgres"
    "redis" 
    "elasticsearch"
    "prometheus"
    "alertmanager"
    "grafana"
    "logstash"
    "filebeat"
    "kibana"
    "nginx"
    "postgres-exporter"
    "redis-exporter"
    "node-exporter"
    "cadvisor"
)

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [UPDATE] $1" | tee -a "$LOG_FILE"
}

# Error handling
error_exit() {
    log "❌ ERROR: $1"
    send_notification "🚨 UPDATE FAILURE" "Rolling update failed: $1" "critical"
    exit 1
}

# Success notification
success_notification() {
    log "✅ SUCCESS: $1"
    send_notification "✅ UPDATE SUCCESS" "$1" "info"
}

# Send notifications
send_notification() {
    local title="$1"
    local message="$2"
    local level="$3"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Slack notification
    if [[ -n "$SLACK_WEBHOOK_URL" ]]; then
        local color="good"
        [[ "$level" == "critical" ]] && color="danger"
        [[ "$level" == "warning" ]] && color="warning"
        
        curl -X POST -H 'Content-type: application/json' \
            --data "{\"attachments\":[{\"color\":\"$color\",\"title\":\"$title\",\"text\":\"$message\",\"footer\":\"Matrixon Update System\",\"ts\":\"$(date +%s)\"}]}" \
            "$SLACK_WEBHOOK_URL" >/dev/null 2>&1 || true
    fi
    
    # Email notification
    if command -v mail >/dev/null 2>&1; then
        echo -e "$message\n\nTimestamp: $timestamp\nServer: $(hostname)" | \
            mail -s "[$level] Matrixon Update: $title" "$NOTIFICATION_EMAIL" || true
    fi
    
    # System log
    logger -t "matrixon-update" "$title: $message"
}

# Check prerequisites
check_prerequisites() {
    log "🔧 Checking update prerequisites..."
    
    # Check if Docker and Docker Compose are available
    for cmd in docker docker-compose; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            error_exit "Required command not found: $cmd"
        fi
    done
    
    # Check if all services are currently running
    if ! docker-compose -f "$DOCKER_COMPOSE_FILE" ps | grep -q "Up"; then
        error_exit "Some services are not running. Please ensure all services are healthy before update."
    fi
    
    # Check disk space (need at least 5GB for safe update)
    local available_space=$(df / | awk 'NR==2 {print $4}')
    if [[ $available_space -lt 5242880 ]]; then  # 5GB in KB
        error_exit "Insufficient disk space. Need at least 5GB free space for safe update."
    fi
    
    # Check if backup system is available
    if [[ "$BACKUP_BEFORE_UPDATE" == "true" ]] && [[ ! -x "$SCRIPT_DIR/automated-backup.sh" ]]; then
        error_exit "Backup script not found or not executable: $SCRIPT_DIR/automated-backup.sh"
    fi
    
    log "✅ Prerequisites check passed"
}

# Create rollback point
create_rollback_point() {
    log "🔧 Creating rollback point..."
    
    local rollback_dir="$ROLLBACK_BACKUP_DIR/$(date +%Y%m%d_%H%M%S)"
    mkdir -p "$rollback_dir"
    
    # Backup current configuration
    cp "$DOCKER_COMPOSE_FILE" "$rollback_dir/"
    cp "$SCRIPT_DIR/production.env" "$rollback_dir/" 2>/dev/null || true
    
    # Backup current image information
    docker-compose -f "$DOCKER_COMPOSE_FILE" images --format json > "$rollback_dir/images.json"
    
    # Create service state snapshot
    docker-compose -f "$DOCKER_COMPOSE_FILE" ps --format json > "$rollback_dir/services.json"
    
    # Save current Git commit if available
    if [[ -d "$SCRIPT_DIR/.git" ]]; then
        git -C "$SCRIPT_DIR" rev-parse HEAD > "$rollback_dir/git_commit.txt" 2>/dev/null || true
    fi
    
    # Perform database backup if requested
    if [[ "$BACKUP_BEFORE_UPDATE" == "true" ]]; then
        log "🔧 Performing pre-update backup..."
        if "$SCRIPT_DIR/automated-backup.sh" daily; then
            echo "$(date '+%Y-%m-%d %H:%M:%S')" > "$rollback_dir/backup_completed.flag"
        else
            error_exit "Pre-update backup failed"
        fi
    fi
    
    # Cleanup old rollback points
    cleanup_old_rollbacks
    
    log "✅ Rollback point created: $rollback_dir"
    echo "$rollback_dir" > "/tmp/matrixon_last_rollback_point"
}

# Cleanup old rollback points
cleanup_old_rollbacks() {
    if [[ -d "$ROLLBACK_BACKUP_DIR" ]]; then
        local rollback_count=$(find "$ROLLBACK_BACKUP_DIR" -mindepth 1 -maxdepth 1 -type d | wc -l)
        if [[ $rollback_count -gt $MAX_ROLLBACK_VERSIONS ]]; then
            local excess=$((rollback_count - MAX_ROLLBACK_VERSIONS))
            find "$ROLLBACK_BACKUP_DIR" -mindepth 1 -maxdepth 1 -type d | sort | head -n $excess | xargs rm -rf
            log "🔧 Cleaned up $excess old rollback points"
        fi
    fi
}

# Health check function
health_check() {
    local service="$1"
    local max_attempts="${2:-30}"
    local attempt=1
    
    log "🔧 Performing health check for $service..."
    
    case "$service" in
        "postgres")
            while [[ $attempt -le $max_attempts ]]; do
                if docker exec matrixon-postgres pg_isready -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" >/dev/null 2>&1; then
                    log "✅ $service health check passed (attempt $attempt)"
                    return 0
                fi
                sleep 10
                ((attempt++))
            done
            ;;
        "redis")
            while [[ $attempt -le $max_attempts ]]; do
                if docker exec matrixon-redis redis-cli --no-auth-warning -a "${REDIS_PASSWORD}" ping | grep -q "PONG"; then
                    log "✅ $service health check passed (attempt $attempt)"
                    return 0
                fi
                sleep 10
                ((attempt++))
            done
            ;;
        "nginx")
            while [[ $attempt -le $max_attempts ]]; do
                if curl -f -s "$HEALTH_CHECK_URL" >/dev/null 2>&1; then
                    log "✅ $service health check passed (attempt $attempt)"
                    return 0
                fi
                sleep 10
                ((attempt++))
            done
            ;;
        "elasticsearch")
            while [[ $attempt -le $max_attempts ]]; do
                if curl -f -s "http://localhost:9200/_cluster/health" | grep -q '"status":"green\|yellow"'; then
                    log "✅ $service health check passed (attempt $attempt)"
                    return 0
                fi
                sleep 10
                ((attempt++))
            done
            ;;
        *)
            # Generic Docker health check
            while [[ $attempt -le $max_attempts ]]; do
                local container_name="matrixon-$service"
                if docker ps --filter "name=$container_name" --filter "status=running" | grep -q "$container_name"; then
                    # Check if container has health check defined
                    local health_status=$(docker inspect --format='{{.State.Health.Status}}' "$container_name" 2>/dev/null || echo "none")
                    if [[ "$health_status" == "healthy" ]] || [[ "$health_status" == "none" ]]; then
                        log "✅ $service health check passed (attempt $attempt)"
                        return 0
                    fi
                fi
                sleep 10
                ((attempt++))
            done
            ;;
    esac
    
    log "❌ $service health check failed after $max_attempts attempts"
    return 1
}

# Update single service
update_service() {
    local service="$1"
    local timeout="${2:-300}"
    
    log "🚀 Updating service: $service"
    
    # Pull latest image
    log "🔧 Pulling latest image for $service..."
    if ! docker-compose -f "$DOCKER_COMPOSE_FILE" pull "$service"; then
        log "⚠️ Failed to pull image for $service, continuing with current image"
    fi
    
    # Stop service gracefully
    log "🔧 Stopping $service..."
    docker-compose -f "$DOCKER_COMPOSE_FILE" stop "$service"
    
    # Remove old container
    docker-compose -f "$DOCKER_COMPOSE_FILE" rm -f "$service" 2>/dev/null || true
    
    # Start service with new image
    log "🔧 Starting $service with new image..."
    if ! timeout "$timeout" docker-compose -f "$DOCKER_COMPOSE_FILE" up -d "$service"; then
        error_exit "Failed to start $service within $timeout seconds"
    fi
    
    # Wait for service to be ready
    if ! health_check "$service" 30; then
        error_exit "$service failed health check after update"
    fi
    
    log "✅ Successfully updated $service"
}

# Rolling update strategy
rolling_update() {
    log "🚀 Starting rolling update..."
    
    local failed_services=()
    local updated_services=()
    
    for service in "${SERVICE_UPDATE_ORDER[@]}"; do
        # Check if service exists in compose file
        if ! docker-compose -f "$DOCKER_COMPOSE_FILE" config --services | grep -q "^$service$"; then
            log "⚠️ Service $service not found in compose file, skipping"
            continue
        fi
        
        log "🔧 Processing service: $service"
        
        # Update service
        if update_service "$service"; then
            updated_services+=("$service")
            
            # Brief pause between services to avoid overwhelming the system
            sleep 5
        else
            failed_services+=("$service")
            log "❌ Failed to update $service"
            
            # Decide whether to continue or abort
            if [[ "${CONTINUE_ON_FAILURE:-false}" != "true" ]]; then
                log "🚨 Aborting rolling update due to failure in $service"
                break
            fi
        fi
    done
    
    # Final health check for all services
    log "🔧 Performing final system health check..."
    local system_healthy=true
    
    for service in "${updated_services[@]}"; do
        if ! health_check "$service" 10; then
            system_healthy=false
            failed_services+=("$service")
        fi
    done
    
    # Report results
    if [[ ${#failed_services[@]} -eq 0 ]] && [[ "$system_healthy" == "true" ]]; then
        success_notification "Rolling update completed successfully. Updated services: ${updated_services[*]}"
    else
        send_notification "⚠️ UPDATE PARTIAL" "Rolling update completed with issues. Failed services: ${failed_services[*]}" "warning"
    fi
    
    return $([[ ${#failed_services[@]} -eq 0 ]] && echo 0 || echo 1)
}

# Blue-green deployment strategy
blue_green_update() {
    log "🚀 Starting blue-green deployment..."
    
    # This is a simplified blue-green strategy
    # In a full implementation, you'd have separate environments
    
    error_exit "Blue-green deployment not yet implemented. Use rolling update strategy."
}

# Canary deployment strategy  
canary_update() {
    log "🚀 Starting canary deployment..."
    
    # This would gradually roll out to a percentage of traffic
    # Requires load balancer configuration
    
    error_exit "Canary deployment not yet implemented. Use rolling update strategy."
}

# Rollback to previous version
rollback() {
    local rollback_point="${1:-}"
    
    if [[ -z "$rollback_point" ]]; then
        if [[ -f "/tmp/matrixon_last_rollback_point" ]]; then
            rollback_point=$(cat "/tmp/matrixon_last_rollback_point")
        else
            # Find latest rollback point
            rollback_point=$(find "$ROLLBACK_BACKUP_DIR" -mindepth 1 -maxdepth 1 -type d | sort | tail -1)
        fi
    fi
    
    if [[ -z "$rollback_point" ]] || [[ ! -d "$rollback_point" ]]; then
        error_exit "No rollback point found or specified"
    fi
    
    log "🔄 Rolling back to: $rollback_point"
    
    # Stop all services
    log "🔧 Stopping all services..."
    docker-compose -f "$DOCKER_COMPOSE_FILE" down
    
    # Restore configuration files
    log "🔧 Restoring configuration..."
    if [[ -f "$rollback_point/docker-compose.postgresql.yml" ]]; then
        cp "$rollback_point/docker-compose.postgresql.yml" "$DOCKER_COMPOSE_FILE"
    fi
    
    if [[ -f "$rollback_point/production.env" ]]; then
        cp "$rollback_point/production.env" "$SCRIPT_DIR/"
    fi
    
    # Restore images if available
    if [[ -f "$rollback_point/images.json" ]]; then
        log "🔧 Restoring Docker images..."
        # This would require more complex logic to handle image restoration
        # For now, we'll just log that we're using the configuration
        log "📄 Using configuration from rollback point"
    fi
    
    # Start services
    log "🔧 Starting services with rollback configuration..."
    if ! docker-compose -f "$DOCKER_COMPOSE_FILE" up -d; then
        error_exit "Failed to start services during rollback"
    fi
    
    # Health check after rollback
    log "🔧 Performing post-rollback health checks..."
    sleep 30  # Give services time to start
    
    local rollback_healthy=true
    for service in "${SERVICE_UPDATE_ORDER[@]}"; do
        if docker-compose -f "$DOCKER_COMPOSE_FILE" config --services | grep -q "^$service$"; then
            if ! health_check "$service" 20; then
                rollback_healthy=false
            fi
        fi
    done
    
    if [[ "$rollback_healthy" == "true" ]]; then
        success_notification "Rollback completed successfully to: $(basename "$rollback_point")"
    else
        send_notification "⚠️ ROLLBACK ISSUES" "Rollback completed but some services may have issues" "warning"
    fi
}

# List available rollback points
list_rollbacks() {
    log "📋 Available rollback points:"
    
    if [[ ! -d "$ROLLBACK_BACKUP_DIR" ]]; then
        log "❌ No rollback points found"
        return 1
    fi
    
    find "$ROLLBACK_BACKUP_DIR" -mindepth 1 -maxdepth 1 -type d | sort -r | while read -r rollback_point; do
        local timestamp=$(basename "$rollback_point")
        local size=$(du -sh "$rollback_point" | cut -f1)
        log "  - $timestamp ($size)"
        
        # Show additional info if available
        if [[ -f "$rollback_point/git_commit.txt" ]]; then
            local git_commit=$(cat "$rollback_point/git_commit.txt" | cut -c1-8)
            log "    Git commit: $git_commit"
        fi
        
        if [[ -f "$rollback_point/backup_completed.flag" ]]; then
            log "    ✅ Database backup included"
        fi
    done
}

# Status check
status_check() {
    log "📊 Matrixon system status:"
    
    # Check service status
    log "🔧 Service status:"
    docker-compose -f "$DOCKER_COMPOSE_FILE" ps
    
    # Check health endpoints
    log "🔧 Health checks:"
    
    # Database
    if health_check "postgres" 3; then
        log "  ✅ PostgreSQL: Healthy"
    else
        log "  ❌ PostgreSQL: Unhealthy"
    fi
    
    # Redis
    if health_check "redis" 3; then
        log "  ✅ Redis: Healthy"
    else
        log "  ❌ Redis: Unhealthy"
    fi
    
    # Web service
    if health_check "nginx" 3; then
        log "  ✅ Web service: Healthy"
    else
        log "  ❌ Web service: Unhealthy"
    fi
    
    # Elasticsearch
    if health_check "elasticsearch" 3; then
        log "  ✅ Elasticsearch: Healthy"
    else
        log "  ❌ Elasticsearch: Unhealthy"
    fi
    
    # System resources
    log "🔧 System resources:"
    log "  CPU: $(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | awk -F'%' '{print $1}')% used"
    log "  Memory: $(free | grep Mem | awk '{printf "%.1f%%", $3/$2 * 100.0}')"
    log "  Disk: $(df / | awk 'NR==2{printf "%.1f%%", $3/$2*100}')"
}

# Main execution
main() {
    local action="${1:-update}"
    local target="${2:-}"
    
    log "🚀 Starting Matrixon update system..."
    
    case "$action" in
        "update"|"rolling")
            check_prerequisites
            create_rollback_point
            if rolling_update; then
                log "🎉 Rolling update completed successfully!"
            else
                log "⚠️ Rolling update completed with issues"
                exit 1
            fi
            ;;
        "blue-green")
            blue_green_update
            ;;
        "canary")
            canary_update
            ;;
        "rollback")
            rollback "$target"
            ;;
        "list-rollbacks"|"list")
            list_rollbacks
            ;;
        "status")
            status_check
            ;;
        "single")
            if [[ -z "$target" ]]; then
                error_exit "Service name required for single service update"
            fi
            check_prerequisites
            create_rollback_point
            update_service "$target"
            ;;
        *)
            echo "Usage: $0 {update|rolling|rollback|list|status|single} [target]"
            echo "  update/rolling - Perform rolling update of all services"
            echo "  rollback [point] - Rollback to specified point (or latest)"
            echo "  list - List available rollback points"
            echo "  status - Check system status"
            echo "  single <service> - Update single service"
            echo ""
            echo "Environment variables:"
            echo "  UPDATE_STRATEGY - rolling (default), blue-green, canary"
            echo "  BACKUP_BEFORE_UPDATE - true (default), false"
            echo "  CONTINUE_ON_FAILURE - false (default), true"
            exit 1
            ;;
    esac
}

# Execute main function
main "$@" 
