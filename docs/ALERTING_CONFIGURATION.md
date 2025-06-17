# Matrixon Matrix Server - Enhanced Alerting and Resource Monitoring

## 🚨 Overview

This document describes the comprehensive alerting and monitoring system implemented for the Matrixon Matrix server. The system provides critical threshold monitoring, connection pool tracking, and resource usage alerting.

## 📊 Monitoring Components

### 1. **Prometheus Enhanced Configuration**
- **File**: `prometheus-enhanced.yml`
- **Purpose**: Comprehensive metrics collection and alerting
- **Features**:
  - Connection pool monitoring for PostgreSQL and Redis
  - System resource tracking (CPU, Memory, Disk, Network)
  - Application performance metrics
  - Container resource monitoring
  - Recording rules for performance optimization

### 2. **AlertManager Enhanced Configuration**
- **File**: `alertmanager-enhanced.yml`
- **Purpose**: Intelligent alert routing and notification management
- **Features**:
  - Multi-channel notifications (Email, Slack, PagerDuty, Webhooks)
  - Team-based alert routing
  - Alert inhibition rules to prevent spam
  - Severity-based grouping and timing

### 3. **Alert Rules Configuration**
- **File**: `prometheus_rules/matrixon-alerts.yml`
- **Purpose**: Define critical thresholds and alerting conditions
- **Coverage**: 50+ rules across 8 categories

### 4. **Connection Pool Monitor**
- **File**: `connection-pool-monitor.sh`
- **Purpose**: Real-time monitoring of connection pools and system resources
- **Features**:
  - PostgreSQL connection pool tracking
  - Redis connection pool monitoring
  - System resource utilization
  - Threshold-based alerting
  - Integration with centralized logging

## 🎯 Connection Pool Monitoring

### PostgreSQL Connection Pool Thresholds
```bash
POSTGRES_CONN_WARNING=150    # 75% of max connections
POSTGRES_CONN_CRITICAL=180   # 90% of max connections
POSTGRES_CONN_MAX=200        # Maximum allowed connections
```

### Redis Connection Pool Thresholds
```bash
REDIS_CONN_WARNING=800       # 80% of max connections
REDIS_CONN_CRITICAL=900      # 90% of max connections
REDIS_CONN_MAX=1000          # Maximum allowed connections
```

### System Resource Thresholds
```bash
CPU_WARNING=80%              # High CPU usage threshold
CPU_CRITICAL=95%             # Critical CPU usage threshold
MEMORY_WARNING=85%           # High memory usage threshold
MEMORY_CRITICAL=95%          # Critical memory usage threshold
DISK_WARNING=80%             # High disk usage threshold
DISK_CRITICAL=90%            # Critical disk usage threshold
```

## 🚨 Alert Categories

### 1. **PostgreSQL Database Alerts**
- `PostgreSQLDown`: Database instance down
- `PostgreSQLConnectionPoolHigh`: >150 active connections
- `PostgreSQLConnectionPoolCritical`: >180 active connections
- `PostgreSQLSlowQueries`: Long-running queries detected
- `PostgreSQLDeadlocks`: Database deadlocks detected
- `PostgreSQLDiskSpaceHigh`: Database size >50GB

### 2. **Redis Cache Alerts**
- `RedisDown`: Redis instance down
- `RedisMemoryHigh`: >80% memory usage
- `RedisConnectionsHigh`: >800 connected clients
- `RedisKeyspaceHitRateLow`: <80% cache hit rate

### 3. **System Resource Alerts**
- `HighCPUUsage`: >80% CPU utilization
- `CriticalCPUUsage`: >95% CPU utilization
- `HighMemoryUsage`: >85% memory usage
- `CriticalMemoryUsage`: >95% memory usage
- `DiskSpaceHigh`: >80% disk usage
- `DiskSpaceCritical`: >90% disk usage
- `HighNetworkTraffic`: >100MB/s network traffic

### 4. **Container Resource Alerts**
- `ContainerCPUUsageHigh`: >80% container CPU usage
- `ContainerMemoryUsageHigh`: >80% container memory usage
- `ContainerRestartFrequent`: Container restart detected

### 5. **Application-Specific Alerts**
- `MatrixonServerDown`: Server unreachable
- `MatrixonHighResponseTime`: >2s response time
- `MatrixonHighErrorRate`: >5% error rate
- `MatrixonConnectionPoolExhausted`: >90% connection pool usage

### 6. **Security Alerts**
- `SuspiciousAuthenticationActivity`: >50 failed auth in 5 min
- `UnauthorizedAccessAttempt`: >100 401 responses in 5 min
- `DDoSAttackSuspected`: >1000 req/s for 2 minutes

### 7. **Elasticsearch Alerts**
- `ElasticsearchDown`: Elasticsearch cluster down
- `ElasticsearchClusterHealthYellow`: Cluster health degraded
- `ElasticsearchClusterHealthRed`: Cluster health critical

### 8. **E2EE & Federation Alerts**
- `E2EEVerificationFailed`: Failed E2EE verification attempts
- `FederationSyncDelayed`: Federation sync delayed >30s
- `FederationEventBacklog`: Federation event backlog >1000
- `E2EEKeyUploadFailed`: Failed E2EE key uploads

## 📧 Alert Routing and Notifications

### Team-Based Alert Routing
1. **Critical Alerts**: `critical@matrixon.com` (5s response time)
2. **Security Team**: `security@matrixon.com` + Slack #security-alerts
3. **Database Team**: `dba@matrixon.com` + PagerDuty
4. **Infrastructure Team**: `infra@matrixon.com`
5. **Development Team**: `dev@matrixon.com` + Slack #dev-alerts
6. **DevOps Team**: `devops@matrixon.com`

### Notification Channels
- **Email**: All teams with custom templates
- **Slack**: Security and development teams
- **PagerDuty**: Database team for critical issues
- **Webhooks**: Integration with audit logging system

## 🔧 Configuration Files Summary

| File | Purpose | Key Features |
|------|---------|-------------|
| `prometheus_rules/matrixon-alerts.yml` | Define alerting conditions | 50+ rules across 8 categories |
| `prometheus-enhanced.yml` | Metrics collection | Enhanced scraping & recording rules |
| `alertmanager-enhanced.yml` | Alert routing | Multi-channel notifications |
| `connection-pool-monitor.sh` | Real-time monitoring | Connection pools & resources |
| `docker-compose.postgresql.yml` | Service orchestration | Enhanced monitoring stack |

## 🚀 Usage Instructions

### Starting the Enhanced Monitoring Stack
```bash
# Start all services with enhanced monitoring
docker-compose -f docker-compose.postgresql.yml up -d

# Check monitoring services status
docker-compose -f docker-compose.postgresql.yml ps
```

### Accessing Monitoring Dashboards
- **Prometheus**: http://localhost:9090
- **AlertManager**: http://localhost:9093
- **Grafana**: http://localhost:3001
- **Kibana**: http://localhost:5601

### Viewing Real-time Logs
```bash
# Connection pool monitor logs
docker logs -f matrixon-resource-monitor

# Audit processor logs
docker logs -f matrixon-audit-processor

# All monitoring logs
docker-compose -f docker-compose.postgresql.yml logs -f prometheus alertmanager grafana
```

### Manual Health Checks
```bash
# Check PostgreSQL connections
docker exec matrixon-postgres psql -U matrixon -d matrixon -c "SELECT count(*) FROM pg_stat_activity;"

# Check Redis connections
docker exec matrixon-redis redis-cli -a matrixon_redis_password_change_me info clients

# Run manual backup
docker exec matrixon-backup /tmp/backup.sh
```

## 📈 Performance Optimizations

### Recording Rules
- Connection pool utilization percentages
- CPU, memory, disk usage percentages
- Network throughput calculations
- Application performance metrics (request rate, error rate, response times)

### Metric Retention
- **Prometheus**: 30 days / 50GB storage
- **Elasticsearch**: Daily indices with automatic cleanup
- **Log Files**: 7-day rotation with compression

## 🛡️ Security Considerations

### Authentication & Authorization
- AlertManager basic auth for webhooks
- Secure email configuration with TLS
- Network isolation within Docker bridge

### Data Protection
- Sensitive credentials in environment variables
- Read-only volume mounts for configuration files
- Secure inter-service communication

## 🔍 Troubleshooting

### Common Issues
1. **Alerts not firing**: Check Prometheus targets and alert rules syntax
2. **Notifications not received**: Verify AlertManager configuration and SMTP settings
3. **High resource usage**: Review monitoring frequency and retention settings
4. **Connection pool alerts**: Check database configuration and application connection management

### Debug Commands
```bash
# Check Prometheus configuration
docker exec matrixon-prometheus promtool check config /etc/prometheus/prometheus.yml

# Validate alert rules
docker exec matrixon-prometheus promtool check rules /etc/prometheus/alert-rules.yml

# Test AlertManager configuration
docker exec matrixon-alertmanager amtool check-config /etc/alertmanager/alertmanager.yml
```

## 📊 Monitoring Best Practices

1. **Threshold Setting**: Based on historical data and capacity planning
2. **Alert Fatigue Prevention**: Proper grouping and inhibition rules
3. **Escalation Procedures**: Clear team assignments and response times
4. **Regular Review**: Monthly threshold and rule evaluation
5. **Documentation**: Keep alert runbooks updated

This comprehensive monitoring and alerting system provides enterprise-grade observability for the Matrixon Matrix server, ensuring high availability and performance through proactive monitoring and intelligent alerting.
