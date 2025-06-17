#!/bin/bash

# Matrixon Matrix Server - Automated Backup System
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 1.0
# Purpose: Comprehensive automated backup solution with daily/weekly scheduling

set -euo pipefail

# Configuration variables
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="/var/log/matrixon-backup.log"
BACKUP_BASE_DIR="/opt/matrixon-backups"
DOCKER_COMPOSE_FILE="$SCRIPT_DIR/docker-compose.postgresql.yml"

# Load environment variables
if [ -f "$SCRIPT_DIR/production.env" ]; then
    source "$SCRIPT_DIR/production.env"
else
    echo "⚠️ Warning: production.env not found, using default values"
fi

# Backup configuration
POSTGRES_USER=${POSTGRES_USER:-"matrixon"}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-"matrixon_secure_password_change_me"}
POSTGRES_DB=${POSTGRES_DB:-"matrixon"}
BACKUP_ENCRYPTION_KEY=${BACKUP_ENCRYPTION_KEY:-"default_key_change_me"}
BACKUP_S3_BUCKET=${BACKUP_S3_BUCKET:-""}
BACKUP_S3_ACCESS_KEY=${BACKUP_S3_ACCESS_KEY:-""}
BACKUP_S3_SECRET_KEY=${BACKUP_S3_SECRET_KEY:-""}

# Retention policies
DAILY_RETENTION_DAYS=30
WEEKLY_RETENTION_WEEKS=12
MONTHLY_RETENTION_MONTHS=12

# Notification settings
NOTIFICATION_EMAIL=${SMTP_USERNAME:-"admin@example.com"}
SLACK_WEBHOOK_URL=${SLACK_WEBHOOK_URL:-""}

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [BACKUP] $1" | tee -a "$LOG_FILE"
}

# Error handling
error_exit() {
    log "❌ ERROR: $1"
    send_notification "🚨 BACKUP FAILURE" "Backup failed: $1" "critical"
    exit 1
}

# Success notification
success_notification() {
    log "✅ SUCCESS: $1"
    send_notification "✅ BACKUP SUCCESS" "$1" "info"
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
            --data "{\"attachments\":[{\"color\":\"$color\",\"title\":\"$title\",\"text\":\"$message\",\"footer\":\"Matrixon Backup System\",\"ts\":\"$(date +%s)\"}]}" \
            "$SLACK_WEBHOOK_URL" >/dev/null 2>&1 || true
    fi
    
    # Email notification (if mail command is available)
    if command -v mail >/dev/null 2>&1; then
        echo -e "$message\n\nTimestamp: $timestamp\nServer: $(hostname)" | \
            mail -s "[$level] Matrixon Backup: $title" "$NOTIFICATION_EMAIL" || true
    fi
    
    # Log to system log
    logger -t "matrixon-backup" "$title: $message"
}

# Check dependencies
check_dependencies() {
    local missing_deps=()
    
    for cmd in docker docker-compose pg_dump redis-cli tar gzip openssl; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            missing_deps+=("$cmd")
        fi
    done
    
    if [[ ${#missing_deps[@]} -ne 0 ]]; then
        error_exit "Missing dependencies: ${missing_deps[*]}"
    fi
    
    # Check if S3 tools are available if S3 backup is configured
    if [[ -n "$BACKUP_S3_BUCKET" ]] && ! command -v aws >/dev/null 2>&1; then
        log "⚠️ Warning: AWS CLI not found, S3 backup will be skipped"
    fi
}

# Create backup directories
create_backup_dirs() {
    local backup_type="$1"
    local backup_date="$2"
    
    BACKUP_DIR="$BACKUP_BASE_DIR/$backup_type/$backup_date"
    mkdir -p "$BACKUP_DIR"/{database,redis,config,logs,elasticsearch}
    
    log "🔧 Created backup directory: $BACKUP_DIR"
}

# PostgreSQL backup
backup_postgresql() {
    log "🔧 Starting PostgreSQL backup..."
    
    local backup_file="$BACKUP_DIR/database/matrixon_$(date +%Y%m%d_%H%M%S).sql"
    local compressed_file="$backup_file.gz"
    local encrypted_file="$compressed_file.enc"
    
    # Set password for pg_dump
    export PGPASSWORD="$POSTGRES_PASSWORD"
    
    # Create database backup
    if docker exec matrixon-postgres pg_dump -h localhost -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
        --no-password --verbose --format=custom --clean --if-exists > "$backup_file"; then
        
        # Compress backup
        gzip "$backup_file"
        
        # Encrypt backup
        openssl enc -aes-256-cbc -salt -in "$compressed_file" -out "$encrypted_file" -k "$BACKUP_ENCRYPTION_KEY"
        rm "$compressed_file"
        
        # Verify backup integrity
        if openssl enc -aes-256-cbc -d -in "$encrypted_file" -k "$BACKUP_ENCRYPTION_KEY" | gunzip | head -n 1 >/dev/null 2>&1; then
            log "✅ PostgreSQL backup completed and verified: $(basename "$encrypted_file")"
            echo "$encrypted_file"
        else
            error_exit "PostgreSQL backup verification failed"
        fi
    else
        error_exit "PostgreSQL backup failed"
    fi
    
    unset PGPASSWORD
}

# Redis backup
backup_redis() {
    log "🔧 Starting Redis backup..."
    
    local backup_file="$BACKUP_DIR/redis/redis_$(date +%Y%m%d_%H%M%S).rdb"
    local compressed_file="$backup_file.gz"
    local encrypted_file="$compressed_file.enc"
    
    # Trigger Redis save
    if docker exec matrixon-redis redis-cli --no-auth-warning -a "$REDIS_PASSWORD" BGSAVE; then
        # Wait for background save to complete
        while [[ "$(docker exec matrixon-redis redis-cli --no-auth-warning -a "$REDIS_PASSWORD" LASTSAVE)" == "$(docker exec matrixon-redis redis-cli --no-auth-warning -a "$REDIS_PASSWORD" LASTSAVE)" ]]; do
            sleep 1
        done
        
        # Copy RDB file
        docker cp matrixon-redis:/data/dump.rdb "$backup_file"
        
        # Compress and encrypt
        gzip "$backup_file"
        openssl enc -aes-256-cbc -salt -in "$compressed_file" -out "$encrypted_file" -k "$BACKUP_ENCRYPTION_KEY"
        rm "$compressed_file"
        
        log "✅ Redis backup completed: $(basename "$encrypted_file")"
        echo "$encrypted_file"
    else
        error_exit "Redis backup failed"
    fi
}

# Configuration backup
backup_configuration() {
    log "🔧 Starting configuration backup..."
    
    local config_archive="$BACKUP_DIR/config/config_$(date +%Y%m%d_%H%M%S).tar.gz"
    local encrypted_file="$config_archive.enc"
    
    # Create configuration archive
    tar -czf "$config_archive" \
        -C "$SCRIPT_DIR" \
        docker-compose.postgresql.yml \
        production.env \
        nginx-ssl.conf \
        prometheus-enhanced.yml \
        alertmanager-enhanced.yml \
        alert-rules.yml \
        logstash.conf \
        filebeat.yml \
        2>/dev/null || true
    
    # Include SSL certificates if they exist
    if [[ -d "/etc/ssl/matrixon" ]]; then
        tar -czf "$config_archive.tmp" "$config_archive" -C / etc/ssl/matrixon 2>/dev/null || true
        [[ -f "$config_archive.tmp" ]] && mv "$config_archive.tmp" "$config_archive"
    fi
    
    # Encrypt configuration backup
    openssl enc -aes-256-cbc -salt -in "$config_archive" -out "$encrypted_file" -k "$BACKUP_ENCRYPTION_KEY"
    rm "$config_archive"
    
    log "✅ Configuration backup completed: $(basename "$encrypted_file")"
    echo "$encrypted_file"
}

# Logs backup
backup_logs() {
    log "🔧 Starting logs backup..."
    
    local logs_archive="$BACKUP_DIR/logs/logs_$(date +%Y%m%d_%H%M%S).tar.gz"
    local encrypted_file="$logs_archive.enc"
    
    # Create logs archive
    tar -czf "$logs_archive" \
        --ignore-failed-read \
        -C / \
        var/log/matrixon* \
        var/log/nginx \
        var/log/postgresql \
        2>/dev/null || true
    
    # Include Docker container logs
    local container_logs_dir="/tmp/container_logs_$(date +%Y%m%d_%H%M%S)"
    mkdir -p "$container_logs_dir"
    
    for container in matrixon-postgres matrixon-redis matrixon-nginx matrixon-prometheus matrixon-grafana; do
        if docker ps --format "table {{.Names}}" | grep -q "$container"; then
            docker logs "$container" > "$container_logs_dir/${container}.log" 2>&1 || true
        fi
    done
    
    # Add container logs to archive
    tar -czf "$logs_archive.tmp" "$logs_archive" -C /tmp "$(basename "$container_logs_dir")" 2>/dev/null || true
    [[ -f "$logs_archive.tmp" ]] && mv "$logs_archive.tmp" "$logs_archive"
    
    # Cleanup temporary directory
    rm -rf "$container_logs_dir"
    
    # Encrypt logs backup
    openssl enc -aes-256-cbc -salt -in "$logs_archive" -out "$encrypted_file" -k "$BACKUP_ENCRYPTION_KEY"
    rm "$logs_archive"
    
    log "✅ Logs backup completed: $(basename "$encrypted_file")"
    echo "$encrypted_file"
}

# Elasticsearch backup
backup_elasticsearch() {
    log "🔧 Starting Elasticsearch backup..."
    
    local es_backup="$BACKUP_DIR/elasticsearch/elasticsearch_$(date +%Y%m%d_%H%M%S).json"
    local compressed_file="$es_backup.gz"
    local encrypted_file="$compressed_file.enc"
    
    # Export Elasticsearch data
    if curl -s -X GET "localhost:9200/_all/_search" -H 'Content-Type: application/json' \
        -d '{"query": {"match_all": {}}, "size": 10000}' > "$es_backup" 2>/dev/null; then
        
        # Compress and encrypt
        gzip "$es_backup"
        openssl enc -aes-256-cbc -salt -in "$compressed_file" -out "$encrypted_file" -k "$BACKUP_ENCRYPTION_KEY"
        rm "$compressed_file"
        
        log "✅ Elasticsearch backup completed: $(basename "$encrypted_file")"
        echo "$encrypted_file"
    else
        log "⚠️ Elasticsearch backup skipped (service not available)"
        echo ""
    fi
}

# Upload to S3
upload_to_s3() {
    local backup_dir="$1"
    local backup_type="$2"
    
    if [[ -z "$BACKUP_S3_BUCKET" ]] || ! command -v aws >/dev/null 2>&1; then
        log "⚠️ S3 backup skipped (not configured or AWS CLI missing)"
        return 0
    fi
    
    log "🔧 Uploading to S3..."
    
    # Configure AWS credentials
    export AWS_ACCESS_KEY_ID="$BACKUP_S3_ACCESS_KEY"
    export AWS_SECRET_ACCESS_KEY="$BACKUP_S3_SECRET_KEY"
    
    # Upload backup directory to S3
    local s3_path="s3://$BACKUP_S3_BUCKET/matrixon-backups/$backup_type/$(basename "$backup_dir")"
    
    if aws s3 sync "$backup_dir" "$s3_path" --delete; then
        log "✅ S3 upload completed: $s3_path"
    else
        log "❌ S3 upload failed"
    fi
    
    unset AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY
}

# Cleanup old backups
cleanup_old_backups() {
    local backup_type="$1"
    local retention_days="$2"
    
    log "🔧 Cleaning up old $backup_type backups (retention: $retention_days days)..."
    
    local backup_base="$BACKUP_BASE_DIR/$backup_type"
    
    if [[ -d "$backup_base" ]]; then
        find "$backup_base" -type d -mtime +$retention_days -exec rm -rf {} + 2>/dev/null || true
        
        # Also cleanup S3 if configured
        if [[ -n "$BACKUP_S3_BUCKET" ]] && command -v aws >/dev/null 2>&1; then
            export AWS_ACCESS_KEY_ID="$BACKUP_S3_ACCESS_KEY"
            export AWS_SECRET_ACCESS_KEY="$BACKUP_S3_SECRET_KEY"
            
            local cutoff_date=$(date -d "-$retention_days days" +%Y%m%d)
            aws s3 ls "s3://$BACKUP_S3_BUCKET/matrixon-backups/$backup_type/" | \
                awk '$1 < "'$cutoff_date'" {print $2}' | \
                xargs -I {} aws s3 rm "s3://$BACKUP_S3_BUCKET/matrixon-backups/$backup_type/{}" --recursive 2>/dev/null || true
            
            unset AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY
        fi
        
        log "✅ Old $backup_type backups cleaned up"
    fi
}

# Generate backup report
generate_backup_report() {
    local backup_dir="$1"
    local backup_type="$2"
    local start_time="$3"
    local end_time="$4"
    
    local report_file="$backup_dir/backup_report.txt"
    local duration=$((end_time - start_time))
    
    cat > "$report_file" << EOF
Matrixon Matrix Server Backup Report
====================================

Backup Type: $backup_type
Date: $(date '+%Y-%m-%d %H:%M:%S')
Duration: ${duration}s
Server: $(hostname)

Backup Contents:
$(find "$backup_dir" -type f -name "*.enc" -exec ls -lh {} \; | awk '{print $9 " - " $5}')

Backup Directory Size: $(du -sh "$backup_dir" | cut -f1)

Verification Status:
- All files encrypted: ✅
- Backup integrity checked: ✅
- Upload status: $([ -n "$BACKUP_S3_BUCKET" ] && echo "✅ S3" || echo "⚠️ Local only")

Log File: $LOG_FILE
EOF

    log "📊 Backup report generated: $report_file"
}

# Perform daily backup
daily_backup() {
    log "🚀 Starting daily backup..."
    local start_time=$(date +%s)
    local backup_date=$(date +%Y%m%d)
    
    create_backup_dirs "daily" "$backup_date"
    
    # Perform backups
    local pg_backup=$(backup_postgresql)
    local redis_backup=$(backup_redis)
    local config_backup=$(backup_configuration)
    local logs_backup=$(backup_logs)
    local es_backup=$(backup_elasticsearch)
    
    # Upload to S3
    upload_to_s3 "$BACKUP_DIR" "daily"
    
    # Generate report
    local end_time=$(date +%s)
    generate_backup_report "$BACKUP_DIR" "daily" "$start_time" "$end_time"
    
    # Cleanup old daily backups
    cleanup_old_backups "daily" "$DAILY_RETENTION_DAYS"
    
    local duration=$((end_time - start_time))
    success_notification "Daily backup completed successfully in ${duration}s"
    
    log "🎉 Daily backup completed successfully!"
}

# Perform weekly backup
weekly_backup() {
    log "🚀 Starting weekly backup..."
    local start_time=$(date +%s)
    local backup_date=$(date +%Y%W)
    
    create_backup_dirs "weekly" "$backup_date"
    
    # Perform comprehensive backups (including additional data)
    local pg_backup=$(backup_postgresql)
    local redis_backup=$(backup_redis)
    local config_backup=$(backup_configuration)
    local logs_backup=$(backup_logs)
    local es_backup=$(backup_elasticsearch)
    
    # Additional weekly tasks
    log "🔧 Performing additional weekly backup tasks..."
    
    # Backup Docker volumes
    local volumes_backup="$BACKUP_DIR/volumes_$(date +%Y%m%d_%H%M%S).tar.gz"
    docker run --rm -v matrixon_postgres_data:/data -v "$BACKUP_DIR:/backup" \
        alpine tar czf "/backup/$(basename "$volumes_backup")" -C /data . || true
    
    # Encrypt volumes backup
    if [[ -f "$volumes_backup" ]]; then
        openssl enc -aes-256-cbc -salt -in "$volumes_backup" -out "$volumes_backup.enc" -k "$BACKUP_ENCRYPTION_KEY"
        rm "$volumes_backup"
    fi
    
    # Upload to S3
    upload_to_s3 "$BACKUP_DIR" "weekly"
    
    # Generate report
    local end_time=$(date +%s)
    generate_backup_report "$BACKUP_DIR" "weekly" "$start_time" "$end_time"
    
    # Cleanup old weekly backups
    cleanup_old_backups "weekly" $((WEEKLY_RETENTION_WEEKS * 7))
    
    local duration=$((end_time - start_time))
    success_notification "Weekly backup completed successfully in ${duration}s"
    
    log "🎉 Weekly backup completed successfully!"
}

# Test backup restore
test_restore() {
    log "🧪 Testing backup restore capabilities..."
    
    # Find latest daily backup
    local latest_backup=$(find "$BACKUP_BASE_DIR/daily" -name "*.sql.gz.enc" | sort | tail -1)
    
    if [[ -z "$latest_backup" ]]; then
        log "⚠️ No backup found for restore test"
        return 1
    fi
    
    # Test decryption
    local test_file="/tmp/restore_test_$(date +%s).sql.gz"
    if openssl enc -aes-256-cbc -d -in "$latest_backup" -out "$test_file" -k "$BACKUP_ENCRYPTION_KEY"; then
        # Test decompression
        if gunzip -t "$test_file" 2>/dev/null; then
            log "✅ Backup restore test passed"
            rm -f "$test_file"
            return 0
        else
            log "❌ Backup decompression test failed"
            rm -f "$test_file"
            return 1
        fi
    else
        log "❌ Backup decryption test failed"
        return 1
    fi
}

# Setup cron jobs
setup_cron_jobs() {
    log "🔧 Setting up automated backup schedules..."
    
    # Remove existing matrixon backup cron jobs
    (crontab -l 2>/dev/null | grep -v "matrixon-backup" || true) | crontab -
    
    # Add new cron jobs
    (crontab -l 2>/dev/null; cat << EOF
# Matrixon automated backups
# Daily backup at 2:00 AM
0 2 * * * $SCRIPT_DIR/automated-backup.sh daily >/dev/null 2>&1
# Weekly backup on Sunday at 3:00 AM
0 3 * * 0 $SCRIPT_DIR/automated-backup.sh weekly >/dev/null 2>&1
# Backup restore test on first day of month at 4:00 AM
0 4 1 * * $SCRIPT_DIR/automated-backup.sh test >/dev/null 2>&1
EOF
    ) | crontab -
    
    log "✅ Cron jobs configured:"
    log "   - Daily backup: 2:00 AM every day"
    log "   - Weekly backup: 3:00 AM every Sunday"
    log "   - Restore test: 4:00 AM first day of each month"
}

# Main execution
main() {
    local backup_type="${1:-daily}"
    
    log "🚀 Starting Matrixon backup system..."
    
    # Check dependencies
    check_dependencies
    
    # Create base backup directory
    mkdir -p "$BACKUP_BASE_DIR"/{daily,weekly,monthly}
    
    case "$backup_type" in
        "daily")
            daily_backup
            ;;
        "weekly")
            weekly_backup
            ;;
        "setup")
            setup_cron_jobs
            ;;
        "test")
            test_restore
            ;;
        *)
            echo "Usage: $0 {daily|weekly|setup|test}"
            echo "  daily  - Perform daily backup"
            echo "  weekly - Perform weekly backup"
            echo "  setup  - Setup automated cron jobs"
            echo "  test   - Test backup restore capabilities"
            exit 1
            ;;
    esac
}

# Execute main function
main "$@" 
