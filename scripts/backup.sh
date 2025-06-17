#!/bin/sh
# Matrixon Matrix Server - PostgreSQL Backup Script
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 1.0
# Purpose: Automated PostgreSQL backup with audit logging

set -e
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backups"
DB_HOST="postgres"
DB_NAME="matrixon"
DB_USER="matrixon"

mkdir -p "$BACKUP_DIR"

echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] 🔧 Starting PostgreSQL backup..." | tee -a "$BACKUP_DIR/backup.log"

# Create database backup
pg_dump -h $DB_HOST -U $DB_USER -d $DB_NAME | gzip > "$BACKUP_DIR/matrixon_backup_$DATE.sql.gz"

# Verify backup was created
if [ -f "$BACKUP_DIR/matrixon_backup_$DATE.sql.gz" ]; then
  BACKUP_SIZE=$(du -h "$BACKUP_DIR/matrixon_backup_$DATE.sql.gz" | cut -f1)
  echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] ✅ Backup completed: matrixon_backup_$DATE.sql.gz (Size: $BACKUP_SIZE)" | tee -a "$BACKUP_DIR/backup.log"
  
  # Log backup event to audit system
  echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"backup\",\"event_type\":\"database_backup\",\"severity\":\"info\",\"database\":\"$DB_NAME\",\"backup_file\":\"matrixon_backup_$DATE.sql.gz\",\"backup_size\":\"$BACKUP_SIZE\",\"message\":\"Database backup completed successfully\"}" | nc logstash 5000 2>/dev/null || echo "Warning: Could not send audit log to Logstash"
else
  echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] ❌ ERROR: Backup failed - file not created" | tee -a "$BACKUP_DIR/backup.log"
  exit 1
fi

echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] 🔧 Cleaning up old backups (>30 days)..." | tee -a "$BACKUP_DIR/backup.log"
find "$BACKUP_DIR" -name "matrixon_backup_*.sql.gz" -mtime +30 -delete

# Count remaining backups
BACKUP_COUNT=$(find "$BACKUP_DIR" -name "matrixon_backup_*.sql.gz" | wc -l)
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] ✅ Backup process finished. Total backups retained: $BACKUP_COUNT" | tee -a "$BACKUP_DIR/backup.log"

# Log cleanup event to audit system
echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"backup\",\"event_type\":\"backup_cleanup\",\"severity\":\"info\",\"database\":\"$DB_NAME\",\"backups_retained\":$BACKUP_COUNT,\"message\":\"Backup cleanup completed\"}" | nc logstash 5000 2>/dev/null || echo "Warning: Could not send audit log to Logstash" 
