#!/bin/sh
# Matrixon Matrix Server - Security Audit Event Processor
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 1.0
# Purpose: Monitor and process security events in real-time

AUDIT_LOG="/var/log/audit/security.log"
ALERT_LOG="/var/log/audit/alerts.log"

mkdir -p /var/log/audit

echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] 🔧 Audit processor started" | tee -a "$AUDIT_LOG"

# Monitor for suspicious activities
while true; do
  # Check for failed authentication attempts
  FAILED_LOGINS=$(grep -c "authentication failed" /var/log/postgresql/*.log 2>/dev/null || echo 0)
  if [ "$FAILED_LOGINS" -gt 10 ]; then
    ALERT="[$(date -u +%Y-%m-%dT%H:%M:%SZ)] ❌ SECURITY_ALERT: Multiple failed login attempts detected ($FAILED_LOGINS)"
    echo "$ALERT" | tee -a "$ALERT_LOG"
    # Send to Logstash for immediate processing
    echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"security\",\"alert_type\":\"authentication_failure\",\"severity\":\"high\",\"count\":$FAILED_LOGINS,\"message\":\"Multiple failed authentication attempts\"}" | nc logstash 5000
  fi
  
  # Check for privilege escalation attempts
  PRIVILEGE_ATTEMPTS=$(grep -c "permission denied" /var/log/postgresql/*.log 2>/dev/null || echo 0)
  if [ "$PRIVILEGE_ATTEMPTS" -gt 5 ]; then
    ALERT="[$(date -u +%Y-%m-%dT%H:%M:%SZ)] ⚠️ SECURITY_ALERT: Potential privilege escalation attempts ($PRIVILEGE_ATTEMPTS)"
    echo "$ALERT" | tee -a "$ALERT_LOG"
    echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"security\",\"alert_type\":\"privilege_escalation\",\"severity\":\"critical\",\"count\":$PRIVILEGE_ATTEMPTS,\"message\":\"Privilege escalation attempts detected\"}" | nc logstash 5000
  fi
  
  # Check for unusual query patterns
  LONG_QUERIES=$(grep -c "duration:" /var/log/postgresql/*.log | awk -F: '{sum+=$2} END {print sum}' 2>/dev/null || echo 0)
  if [ "$LONG_QUERIES" -gt 100 ]; then
    ALERT="[$(date -u +%Y-%m-%dT%H:%M:%SZ)] 🎉 PERFORMANCE_ALERT: Unusual number of long-running queries ($LONG_QUERIES)"
    echo "$ALERT" | tee -a "$ALERT_LOG"
    echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"performance\",\"alert_type\":\"slow_queries\",\"severity\":\"medium\",\"count\":$LONG_QUERIES,\"message\":\"High number of slow queries detected\"}" | nc logstash 5000
  fi
  
  # Check for suspicious SQL injection patterns
  INJECTION_ATTEMPTS=$(grep -iE "(union|select|drop|insert|update|delete).*(from|into|table)" /var/log/postgresql/*.log 2>/dev/null | wc -l)
  if [ "$INJECTION_ATTEMPTS" -gt 0 ]; then
    ALERT="[$(date -u +%Y-%m-%dT%H:%M:%SZ)] ❌ SECURITY_ALERT: Potential SQL injection attempts detected ($INJECTION_ATTEMPTS)"
    echo "$ALERT" | tee -a "$ALERT_LOG"
    echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"audit_type\":\"security\",\"alert_type\":\"sql_injection\",\"severity\":\"critical\",\"count\":$INJECTION_ATTEMPTS,\"message\":\"SQL injection attempts detected\"}" | nc logstash 5000
  fi
  
  # Log audit status
  echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] ✅ Audit check completed - Auth failures: $FAILED_LOGINS, Privilege attempts: $PRIVILEGE_ATTEMPTS, Long queries: $LONG_QUERIES, Injection attempts: $INJECTION_ATTEMPTS" | tee -a "$AUDIT_LOG"
  
  sleep 60
done 
