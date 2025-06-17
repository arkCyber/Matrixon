#!/bin/bash

# Matrixon Matrix Server - Log Monitoring and Management
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 1.0
# Purpose: Comprehensive log monitoring, analysis, and cleanup

set -euo pipefail

# Configuration variables
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="/var/log/matrixon-log-monitor.log"
ALERT_THRESHOLD_ERROR=10  # Alert if more than 10 errors per minute
ALERT_THRESHOLD_WARN=50   # Alert if more than 50 warnings per minute
DISK_USAGE_THRESHOLD=80   # Alert if log disk usage > 80%

# Load environment variables
if [ -f "$SCRIPT_DIR/production.env" ]; then
    source "$SCRIPT_DIR/production.env"
else
    echo "⚠️ Warning: production.env not found, using default values"
fi

# Notification settings
NOTIFICATION_EMAIL="${SMTP_USERNAME:-admin@example.com}"
SLACK_WEBHOOK_URL="${SLACK_WEBHOOK_URL:-}"

# Log paths to monitor
declare -A LOG_PATHS=(
    ["application"]="/var/log/matrixon/*.log"
    ["nginx"]="/var/log/nginx/*.log"
    ["postgresql"]="/var/log/postgresql/*.log"
    ["system"]="/var/log/matrixon-*.log"
    ["security"]="/var/log/matrixon/security.log"
    ["audit"]="/var/log/matrixon/audit.log"
    ["auth"]="/var/log/auth.log"
    ["syslog"]="/var/log/syslog"
)

# Log patterns to monitor
declare -A ERROR_PATTERNS=(
    ["database_error"]="connection.*failed|database.*error|deadlock|timeout"
    ["authentication_failure"]="authentication.*failed|invalid.*password|login.*failed"
    ["security_threat"]="brute.*force|ddos|injection|breach|intrusion"
    ["service_error"]="service.*unavailable|internal.*server.*error|fatal.*error"
    ["network_error"]="connection.*refused|network.*unreachable|timeout.*error"
    ["ssl_error"]="ssl.*error|certificate.*error|tls.*error|handshake.*failed"
)

# Logging function
log() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [LOG-MONITOR] $1" | tee -a "$LOG_FILE"
}

# Error handling function
handle_error() {
    local error_msg="$1"
    local severity="${2:-error}"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    case "$severity" in
        "critical")
            log "🚨 CRITICAL: $error_msg"
            send_alert "Critical Error" "$error_msg" "critical"
            ;;
        "error")
            log "❌ ERROR: $error_msg"
            send_alert "Error" "$error_msg" "error"
            ;;
        "warning")
            log "⚠️ WARNING: $error_msg"
            send_alert "Warning" "$error_msg" "warning"
            ;;
        *)
            log "ℹ️ INFO: $error_msg"
            ;;
    esac
}

# Send notifications
send_alert() {
    local title="$1"
    local message="$2"
    local severity="${3:-info}"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Log the alert
    log "🔔 Alert: $title - $message (Severity: $severity)"
    
    # Send email if configured
    if [ -n "$NOTIFICATION_EMAIL" ]; then
        echo "Subject: [$severity] Matrixon Alert: $title
From: matrixon-alerts@$(hostname)
To: $NOTIFICATION_EMAIL
Date: $(date -R)

$message

Timestamp: $timestamp
Severity: $severity
Host: $(hostname)
" | sendmail -t 2>/dev/null || handle_error "Failed to send email alert" "warning"
    fi
    
    # Send Slack notification if configured
    if [ -n "$SLACK_WEBHOOK_URL" ]; then
        curl -s -X POST -H 'Content-type: application/json' \
            --data "{
                \"text\": \"*[$severity] Matrixon Alert: $title*\n$message\n\nTimestamp: $timestamp\nSeverity: $severity\nHost: $(hostname)\"
            }" \
            "$SLACK_WEBHOOK_URL" || handle_error "Failed to send Slack alert" "warning"
    fi
}

# Check disk usage for log directories
check_log_disk_usage() {
    log "🔧 Checking log disk usage..."
    
    for log_type in "${!LOG_PATHS[@]}"; do
        local log_path="${LOG_PATHS[$log_type]}"
        
        # Get directory from path pattern
        local log_dir=$(dirname $(echo $log_path | cut -d'*' -f1))
        
        if [[ -d "$log_dir" ]]; then
            local usage=$(df "$log_dir" | awk 'NR==2 {print $5}' | sed 's/%//')
            
            if [[ $usage -gt $DISK_USAGE_THRESHOLD ]]; then
                send_alert "High Disk Usage" "Log directory $log_dir is ${usage}% full" "warning"
            fi
            
            log "📊 $log_type logs disk usage: ${usage}%"
        fi
    done
}

# Monitor log patterns
monitor_log_patterns() {
    local time_window="${1:-1}"  # minutes
    log "🔧 Monitoring log patterns (${time_window}m window)..."
    
    local since_time=$(date -d "-${time_window} minutes" '+%Y-%m-%d %H:%M:%S')
    local total_errors=0
    
    for pattern_name in "${!ERROR_PATTERNS[@]}"; do
        local pattern="${ERROR_PATTERNS[$pattern_name]}"
        local count=0
        
        # Search across all relevant log files
        for log_type in "${!LOG_PATHS[@]}"; do
            local log_path="${LOG_PATHS[$log_type]}"
            
            if ls $log_path >/dev/null 2>&1; then
                # Count occurrences since the time window
                local file_count=$(find $(dirname $log_path) -name "$(basename $log_path)" -newer <(date -d "$since_time" '+%Y%m%d%H%M') 2>/dev/null | \
                    xargs grep -iE "$pattern" 2>/dev/null | wc -l || echo 0)
                
                count=$((count + file_count))
            fi
        done
        
        if [[ $count -gt 0 ]]; then
            log "⚠️ Found $count occurrences of $pattern_name in last ${time_window}m"
            total_errors=$((total_errors + count))
            
            # Alert on high frequency
            if [[ $count -gt $ALERT_THRESHOLD_ERROR ]]; then
                handle_error "Found $count occurrences of $pattern_name in last ${time_window} minutes" "critical"
            elif [[ $count -gt $((ALERT_THRESHOLD_ERROR / 2)) ]]; then
                handle_error "Found $count occurrences of $pattern_name in last ${time_window} minutes" "warning"
            fi
        fi
    done
    
    # Alert on total error count
    if [[ $total_errors -gt $ALERT_THRESHOLD_ERROR ]]; then
        handle_error "Total error count ($total_errors) exceeds threshold in last ${time_window} minutes" "critical"
    fi
}

# Analyze error rates
analyze_error_rates() {
    log "🔧 Analyzing error rates..."
    
    # NGINX error rate
    if [[ -f "/var/log/nginx/access.log" ]]; then
        local total_requests=$(tail -n 1000 /var/log/nginx/access.log | wc -l)
        local error_requests=$(tail -n 1000 /var/log/nginx/access.log | grep -E ' (4|5)[0-9][0-9] ' | wc -l)
        
        if [[ $total_requests -gt 0 ]]; then
            local error_rate=$((error_requests * 100 / total_requests))
            log "📊 NGINX error rate: ${error_rate}% (${error_requests}/${total_requests})"
            
            if [[ $error_rate -gt 5 ]]; then
                send_alert "High NGINX Error Rate" "Error rate: ${error_rate}% (${error_requests}/${total_requests})" "warning"
            fi
        fi
    fi
    
    # Application error rate from logs
    for log_type in "${!LOG_PATHS[@]}"; do
        local log_path="${LOG_PATHS[$log_type]}"
        
        if ls $log_path >/dev/null 2>&1; then
            local total_lines=$(find $(dirname $log_path) -name "$(basename $log_path)" -mmin -5 2>/dev/null | \
                xargs wc -l 2>/dev/null | tail -n 1 | awk '{print $1}' || echo 0)
            
            local error_lines=$(find $(dirname $log_path) -name "$(basename $log_path)" -mmin -5 2>/dev/null | \
                xargs grep -iE "error|critical|fatal" 2>/dev/null | wc -l || echo 0)
            
            if [[ $total_lines -gt 100 ]] && [[ $error_lines -gt 0 ]]; then
                local error_rate=$((error_lines * 100 / total_lines))
                log "📊 $log_type error rate: ${error_rate}% (${error_lines}/${total_lines})"
                
                if [[ $error_rate -gt 10 ]]; then
                    send_alert "High $log_type Error Rate" "Error rate: ${error_rate}% in last 5 minutes" "warning"
                fi
            fi
        fi
    done
}

# Check for log rotation issues
check_log_rotation() {
    log "🔧 Checking log rotation status..."
    
    # Check if logrotate is working
    if [[ -f "/var/log/logrotate.log" ]]; then
        local last_rotation=$(grep "matrixon" /var/log/logrotate.log | tail -n 1 | awk '{print $1}')
        if [[ -n "$last_rotation" ]]; then
            local days_since_rotation=$(( ($(date +%s) - $(date -d "$last_rotation" +%s)) / 86400 ))
            
            if [[ $days_since_rotation -gt 7 ]]; then
                send_alert "Log Rotation Issue" "Last matrixon log rotation was $days_since_rotation days ago" "warning"
            fi
        fi
    fi
    
    # Check for oversized log files
    for log_type in "${!LOG_PATHS[@]}"; do
        local log_path="${LOG_PATHS[$log_type]}"
        
        if ls $log_path >/dev/null 2>&1; then
            find $(dirname $log_path) -name "$(basename $log_path)" -size +100M 2>/dev/null | while read -r large_file; do
                local size=$(du -h "$large_file" | cut -f1)
                send_alert "Large Log File" "Log file $large_file is $size" "warning"
            done
        fi
    done
}

# Generate log summary report
generate_log_summary() {
    local period="${1:-24}"  # hours
    log "📊 Generating log summary for last ${period} hours..."
    
    local report_file="/tmp/matrixon_log_summary_$(date +%Y%m%d_%H%M%S).txt"
    
    cat > "$report_file" << EOF
Matrixon Matrix Server - Log Summary Report
==========================================

Report Period: Last ${period} hours
Generated: $(date '+%Y-%m-%d %H:%M:%S')
Server: $(hostname)

SYSTEM OVERVIEW
===============
$(uptime)
$(df -h | grep -E "/$|/var")

LOG FILE SIZES
==============
EOF

    # Add log file sizes
    for log_type in "${!LOG_PATHS[@]}"; do
        local log_path="${LOG_PATHS[$log_type]}"
        
        if ls $log_path >/dev/null 2>&1; then
            echo "[$log_type]" >> "$report_file"
            find $(dirname $log_path) -name "$(basename $log_path)" -mtime -1 2>/dev/null | \
                xargs ls -lh 2>/dev/null | awk '{print "  " $9 " - " $5}' >> "$report_file" || true
            echo "" >> "$report_file"
        fi
    done
    
    cat >> "$report_file" << EOF

ERROR SUMMARY
=============
EOF

    # Add error summaries
    for pattern_name in "${!ERROR_PATTERNS[@]}"; do
        local pattern="${ERROR_PATTERNS[$pattern_name]}"
        local count=0
        
        for log_type in "${!LOG_PATHS[@]}"; do
            local log_path="${LOG_PATHS[$log_type]}"
            
            if ls $log_path >/dev/null 2>&1; then
                local file_count=$(find $(dirname $log_path) -name "$(basename $log_path)" -mtime -1 2>/dev/null | \
                    xargs grep -iE "$pattern" 2>/dev/null | wc -l || echo 0)
                count=$((count + file_count))
            fi
        done
        
        echo "$pattern_name: $count occurrences" >> "$report_file"
    done
    
    cat >> "$report_file" << EOF

TOP ERROR SOURCES
=================
EOF

    # Find top error sources
    for log_type in "${!LOG_PATHS[@]}"; do
        local log_path="${LOG_PATHS[$log_type]}"
        
        if ls $log_path >/dev/null 2>&1; then
            echo "[$log_type]" >> "$report_file"
            find $(dirname $log_path) -name "$(basename $log_path)" -mtime -1 2>/dev/null | \
                xargs grep -iE "error|critical|fatal" 2>/dev/null | \
                awk '{print $NF}' | sort | uniq -c | sort -nr | head -5 | \
                awk '{print "  " $1 "x " $2}' >> "$report_file" || true
            echo "" >> "$report_file"
        fi
    done
    
    log "📄 Log summary report generated: $report_file"
    echo "$report_file"
}

# Clean old logs
cleanup_old_logs() {
    local retention_days="${1:-30}"
    log "🔧 Cleaning logs older than $retention_days days..."
    
    local cleaned_count=0
    local freed_space=0
    
    for log_type in "${!LOG_PATHS[@]}"; do
        local log_path="${LOG_PATHS[$log_type]}"
        local log_dir=$(dirname $log_path)
        
        if [[ -d "$log_dir" ]]; then
            # Find and remove old log files
            while IFS= read -r -d '' old_file; do
                local file_size=$(stat -f%z "$old_file" 2>/dev/null || stat -c%s "$old_file")
                freed_space=$((freed_space + file_size))
                rm -f "$old_file"
                cleaned_count=$((cleaned_count + 1))
            done < <(find "$log_dir" -name "*.log*" -mtime +$retention_days -type f -print0 2>/dev/null)
            
            # Clean compressed log files
            while IFS= read -r -d '' old_file; do
                local file_size=$(stat -f%z "$old_file" 2>/dev/null || stat -c%s "$old_file")
                freed_space=$((freed_space + file_size))
                rm -f "$old_file"
                cleaned_count=$((cleaned_count + 1))
            done < <(find "$log_dir" -name "*.gz" -mtime +$retention_days -type f -print0 2>/dev/null)
        fi
    done
    
    local freed_mb=$((freed_space / 1024 / 1024))
    log "✅ Cleaned $cleaned_count old log files, freed ${freed_mb}MB"
    
    if [[ $cleaned_count -gt 0 ]]; then
        logger -t "matrixon-log-cleanup" "Cleaned $cleaned_count log files, freed ${freed_mb}MB"
    fi
}

# Archive logs to backup storage
archive_logs() {
    local archive_age_days="${1:-7}"
    log "🔧 Archiving logs older than $archive_age_days days..."
    
    local archive_dir="/opt/matrixon-log-archives/$(date +%Y%m)"
    mkdir -p "$archive_dir"
    
    for log_type in "${!LOG_PATHS[@]}"; do
        local log_path="${LOG_PATHS[$log_type]}"
        local log_dir=$(dirname $log_path)
        
        if [[ -d "$log_dir" ]]; then
            # Find logs to archive
            find "$log_dir" -name "*.log*" -mtime +$archive_age_days -type f 2>/dev/null | while read -r log_file; do
                # Create archive if it doesn't exist
                local archive_file="$archive_dir/${log_type}_$(date +%Y%m%d).tar.gz"
                
                # Add to archive
                tar -czf "$archive_file.tmp" -C "$(dirname "$log_file")" "$(basename "$log_file")" 2>/dev/null || true
                
                if [[ -f "$archive_file" ]]; then
                    # Merge with existing archive
                    tar -czf "$archive_file.new" "$archive_file" "$archive_file.tmp" 2>/dev/null || true
                    [[ -f "$archive_file.new" ]] && mv "$archive_file.new" "$archive_file"
                else
                    mv "$archive_file.tmp" "$archive_file" 2>/dev/null || true
                fi
                
                rm -f "$archive_file.tmp" 2>/dev/null || true
                
                # Remove original file after archiving
                rm -f "$log_file"
            done
        fi
    done
    
    log "✅ Log archiving completed"
}

# Monitor log growth rate
monitor_log_growth() {
    log "📈 Monitoring log growth rates..."
    
    local growth_file="/tmp/matrixon_log_growth.dat"
    local current_time=$(date +%s)
    
    # Create or read previous measurements
    declare -A previous_sizes
    if [[ -f "$growth_file" ]]; then
        source "$growth_file"
    fi
    
    # Measure current sizes and calculate growth
    declare -A current_sizes
    for log_type in "${!LOG_PATHS[@]}"; do
        local log_path="${LOG_PATHS[$log_type]}"
        
        if ls $log_path >/dev/null 2>&1; then
            local current_size=$(find $(dirname $log_path) -name "$(basename $log_path)" 2>/dev/null | \
                xargs du -sb 2>/dev/null | awk '{sum+=$1} END {print sum+0}')
            
            current_sizes[$log_type]=$current_size
            
            # Calculate growth rate
            if [[ -n "${previous_sizes[$log_type]:-}" ]] && [[ -n "${previous_time:-}" ]]; then
                local size_diff=$((current_size - ${previous_sizes[$log_type]}))
                local time_diff=$((current_time - previous_time))
                
                if [[ $time_diff -gt 0 ]]; then
                    local growth_rate=$((size_diff / time_diff))  # bytes per second
                    local growth_mb_hour=$((growth_rate * 3600 / 1024 / 1024))
                    
                    log "📈 $log_type growth rate: ${growth_mb_hour}MB/hour"
                    
                    # Alert on excessive growth
                    if [[ $growth_mb_hour -gt 100 ]]; then
                        send_alert "High Log Growth" "$log_type growing at ${growth_mb_hour}MB/hour" "warning"
                    fi
                fi
            fi
        fi
    done
    
    # Save current measurements for next run
    cat > "$growth_file" << EOF
# Matrixon log growth tracking
previous_time=$current_time
EOF
    
    for log_type in "${!current_sizes[@]}"; do
        echo "previous_sizes[$log_type]=${current_sizes[$log_type]}" >> "$growth_file"
    done
}

# Real-time log monitoring
real_time_monitor() {
    local duration="${1:-60}"  # seconds
    log "🔄 Starting real-time log monitoring for ${duration}s..."
    
    # Create named pipes for log streams
    local pipe_dir="/tmp/matrixon_log_pipes"
    mkdir -p "$pipe_dir"
    
    # Start monitoring background processes
    local pids=()
    
    for log_type in "${!LOG_PATHS[@]}"; do
        local log_path="${LOG_PATHS[$log_type]}"
        
        if ls $log_path >/dev/null 2>&1; then
            # Monitor each log type
            (
                tail -f $log_path 2>/dev/null | while read -r line; do
                    # Check for immediate threats
                    for pattern_name in "${!ERROR_PATTERNS[@]}"; do
                        local pattern="${ERROR_PATTERNS[$pattern_name]}"
                        if echo "$line" | grep -qiE "$pattern"; then
                            echo "$(date '+%H:%M:%S') [$log_type] $pattern_name: $line" | \
                                tee -a "/tmp/matrixon_realtime_alerts.log"
                        fi
                    done
                done
            ) &
            pids+=($!)
        fi
    done
    
    # Wait for specified duration
    sleep "$duration"
    
    # Stop monitoring
    for pid in "${pids[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    
    # Report findings
    if [[ -f "/tmp/matrixon_realtime_alerts.log" ]]; then
        local alert_count=$(wc -l < "/tmp/matrixon_realtime_alerts.log")
        log "🚨 Real-time monitoring found $alert_count potential issues"
        
        if [[ $alert_count -gt 0 ]]; then
            send_alert "Real-time Log Issues" "Found $alert_count potential issues in last ${duration}s" "warning"
        fi
        
        rm -f "/tmp/matrixon_realtime_alerts.log"
    fi
    
    # Cleanup
    rm -rf "$pipe_dir"
    log "✅ Real-time monitoring completed"
}

# Setup log monitoring cron jobs
setup_monitoring_schedule() {
    log "🔧 Setting up log monitoring schedule..."
    
    # Remove existing matrixon log monitoring cron jobs
    (crontab -l 2>/dev/null | grep -v "matrixon-log-monitor" || true) | crontab -
    
    # Add new cron jobs
    (crontab -l 2>/dev/null; cat << EOF
# Matrixon log monitoring
# Monitor patterns every 5 minutes
*/5 * * * * $SCRIPT_DIR/log-monitoring.sh monitor >/dev/null 2>&1
# Analyze error rates every 15 minutes
*/15 * * * * $SCRIPT_DIR/log-monitoring.sh analyze >/dev/null 2>&1
# Check disk usage hourly
0 * * * * $SCRIPT_DIR/log-monitoring.sh disk >/dev/null 2>&1
# Daily cleanup and summary
0 2 * * * $SCRIPT_DIR/log-monitoring.sh cleanup >/dev/null 2>&1
0 3 * * * $SCRIPT_DIR/log-monitoring.sh summary >/dev/null 2>&1
# Weekly archiving
0 4 * * 0 $SCRIPT_DIR/log-monitoring.sh archive >/dev/null 2>&1
EOF
    ) | crontab -
    
    log "✅ Log monitoring schedule configured"
}

# Main execution
main() {
    local action="${1:-monitor}"
    local param="${2:-}"
    
    case "$action" in
        "monitor")
            monitor_log_patterns 5
            ;;
        "analyze")
            analyze_error_rates
            ;;
        "disk")
            check_log_disk_usage
            ;;
        "rotation")
            check_log_rotation
            ;;
        "summary")
            generate_log_summary "${param:-24}"
            ;;
        "cleanup")
            cleanup_old_logs "${param:-30}"
            ;;
        "archive")
            archive_logs "${param:-7}"
            ;;
        "growth")
            monitor_log_growth
            ;;
        "realtime")
            real_time_monitor "${param:-60}"
            ;;
        "setup")
            setup_monitoring_schedule
            ;;
        "full")
            log "🚀 Running full log monitoring suite..."
            check_log_disk_usage
            monitor_log_patterns 10
            analyze_error_rates
            check_log_rotation
            monitor_log_growth
            log "✅ Full monitoring completed"
            ;;
        *)
            echo "Usage: $0 {monitor|analyze|disk|rotation|summary|cleanup|archive|growth|realtime|setup|full} [param]"
            echo "  monitor - Monitor log patterns for errors"
            echo "  analyze - Analyze error rates across services"
            echo "  disk - Check log disk usage"
            echo "  rotation - Check log rotation status"
            echo "  summary [hours] - Generate log summary report"
            echo "  cleanup [days] - Clean old logs (default: 30 days)"
            echo "  archive [days] - Archive old logs (default: 7 days)"
            echo "  growth - Monitor log growth rates"
            echo "  realtime [seconds] - Real-time monitoring (default: 60s)"
            echo "  setup - Setup automated monitoring schedule"
            echo "  full - Run complete monitoring suite"
            exit 1
            ;;
    esac
}

# Execute main function
main "$@" 
