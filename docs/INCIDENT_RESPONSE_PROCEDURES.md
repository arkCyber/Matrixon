# Matrixon Matrix Server - Incident Response Procedures

**Author:** arkSong (arksong2018@gmail.com)  
**Date:** 2024-12-19  
**Version:** 1.0  
**Purpose:** Comprehensive incident response procedures for production incidents

## 🚨 Emergency Contact Information

### Primary Contacts
| Role | Contact | Phone | Email | Escalation Level |
|------|---------|-------|-------|------------------|
| **On-Call Engineer** | Primary Response | +1-XXX-XXX-XXXX | oncall@matrixon.com | Level 1 |
| **DevOps Lead** | Infrastructure | +1-XXX-XXX-XXXX | devops@matrixon.com | Level 2 |
| **Security Team** | Security Incidents | +1-XXX-XXX-XXXX | security@matrixon.com | Level 1 |
| **Database Admin** | Database Issues | +1-XXX-XXX-XXXX | dba@matrixon.com | Level 2 |
| **Engineering Manager** | Escalation | +1-XXX-XXX-XXXX | engineering@matrixon.com | Level 3 |
| **CTO** | Major Incidents | +1-XXX-XXX-XXXX | cto@matrixon.com | Level 4 |

### External Contacts
| Service | Contact | Account ID | Notes |
|---------|---------|------------|-------|
| **Cloud Provider** | AWS Support | Account: XXXXXXXXXXXX | Business Support |
| **CDN Provider** | CloudFlare | Account: XXXXXXXXXXXX | Enterprise Plan |
| **DNS Provider** | Route53/CloudFlare | Account: XXXXXXXXXXXX | Critical DNS |
| **Monitoring** | PagerDuty | Service: matrixon-prod | 24/7 Alerting |

## 📊 Incident Severity Levels

### Severity 1 (Critical) - Response Time: ≤ 15 minutes
- **Complete service outage** affecting all users
- **Data breach** or security compromise
- **Data loss** or corruption
- **Revenue impact** > $10,000/hour
- **Legal/compliance** violations

**Examples:**
- Matrix server completely down
- Database corruption/failure
- Security breach detected
- Major data loss
- SSL certificate expired causing complete outage

### Severity 2 (High) - Response Time: ≤ 1 hour
- **Partial service degradation** affecting >50% users
- **Performance issues** causing significant user impact
- **Security vulnerabilities** requiring immediate patching
- **Revenue impact** $1,000-$10,000/hour

**Examples:**
- Federation not working
- Significant performance degradation
- Authentication issues
- Monitoring/alerting system down
- High error rates (>5%)

### Severity 3 (Medium) - Response Time: ≤ 4 hours
- **Minor functionality** affected
- **Performance degradation** for <50% users
- **Non-critical security** issues
- **Revenue impact** <$1,000/hour

**Examples:**
- Single service container restart needed
- Non-critical feature not working
- Logging issues
- Minor configuration problems

### Severity 4 (Low) - Response Time: ≤ 24 hours
- **Cosmetic issues** or minor bugs
- **Documentation** problems
- **Enhancement requests**
- **No revenue impact**

## 🔥 Immediate Response Protocol

### Step 1: Detection and Assessment (0-5 minutes)
1. **Acknowledge Alert**
   ```bash
   # Log into monitoring systems
   # Check Grafana: http://localhost:3001
   # Check Prometheus: http://localhost:9090
   # Check AlertManager: http://localhost:9093
   ```

2. **Initial Assessment**
   ```bash
   # Quick system status check
   ./rolling-update.sh status
   
   # Check all services
   docker-compose -f docker-compose.postgresql.yml ps
   
   # Check system resources
   htop
   df -h
   free -h
   ```

3. **Determine Severity**
   - Use severity matrix above
   - Document initial findings
   - Escalate if needed

### Step 2: Communication (5-10 minutes)
1. **Create Incident Ticket**
   - Use incident management system
   - Include: Severity, Impact, Initial Assessment
   - Set up incident channel (#incident-YYYYMMDD-HHMMSS)

2. **Initial Notifications**
   ```bash
   # Send initial notification
   # Slack: Post in #incidents
   # Email: Send to incident-response@matrixon.com
   # PagerDuty: Acknowledge and update
   ```

3. **Assemble Response Team**
   - Assign Incident Commander
   - Call in SMEs based on incident type
   - Brief team on situation

### Step 3: Investigation (10-30 minutes)
1. **Gather Information**
   ```bash
   # Check recent deployments
   ./rolling-update.sh list
   
   # Review logs
   tail -f /var/log/matrixon/*.log
   journalctl -u docker -f
   
   # Check monitoring
   # Look for anomalies in Grafana dashboards
   # Review Prometheus alerts
   ```

2. **Common Investigation Commands**
   ```bash
   # Service health checks
   ./rolling-update.sh status
   
   # Database connectivity
   docker exec matrixon-postgres pg_isready -U matrixon
   
   # Redis connectivity  
   docker exec matrixon-redis redis-cli ping
   
   # Network connectivity
   curl -I http://localhost/_matrix/client/versions
   
   # SSL certificate status
   openssl s_client -connect localhost:443 -servername matrixon.com
   
   # Resource usage
   docker stats
   
   # Recent errors
   grep -i error /var/log/matrixon/*.log | tail -20
   ```

## 🛠️ Incident Response Playbooks

### Database Issues

#### Symptoms
- Connection timeouts
- Slow queries
- High CPU/memory usage
- Disk space full

#### Investigation
```bash
# Check PostgreSQL status
docker exec matrixon-postgres pg_isready

# Check connection count
docker exec matrixon-postgres psql -U matrixon -c "SELECT count(*) FROM pg_stat_activity;"

# Check slow queries
docker exec matrixon-postgres psql -U matrixon -c "SELECT query, state, query_start FROM pg_stat_activity WHERE state != 'idle' ORDER BY query_start;"

# Check disk usage
docker exec matrixon-postgres df -h

# Check locks
docker exec matrixon-postgres psql -U matrixon -c "SELECT * FROM pg_locks WHERE NOT granted;"
```

#### Resolution Steps
1. **Connection Pool Exhaustion**
   ```bash
   # Restart application to reset connections
   docker-compose restart matrixon-app
   
   # Or kill long-running queries
   docker exec matrixon-postgres psql -U matrixon -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state != 'idle' AND query_start < now() - interval '5 minutes';"
   ```

2. **Disk Space Full**
   ```bash
   # Clean old logs
   ./log-cleanup.sh
   
   # Vacuum database
   docker exec matrixon-postgres psql -U matrixon -c "VACUUM;"
   ```

3. **Database Corruption**
   ```bash
   # Stop services
   docker-compose stop
   
   # Restore from backup
   ./automated-backup.sh restore latest
   
   # Start services
   docker-compose up -d
   ```

### Network/Connectivity Issues

#### Symptoms
- Timeouts
- Connection refused
- DNS resolution failures
- SSL/TLS errors

#### Investigation
```bash
# Check network connectivity
ping 8.8.8.8
curl -I https://matrix.org

# Check DNS resolution
nslookup matrixon.com
dig matrixon.com

# Check port accessibility
netstat -tulpn | grep :443
netstat -tulpn | grep :8448

# Check SSL certificate
openssl s_client -connect matrixon.com:443 -servername matrixon.com

# Check firewall rules
iptables -L -n
```

#### Resolution Steps
1. **DNS Issues**
   ```bash
   # Restart DNS service
   systemctl restart systemd-resolved
   
   # Flush DNS cache
   systemctl flush-dns
   ```

2. **SSL Certificate Issues**
   ```bash
   # Check certificate expiry
   /usr/local/bin/check-cert-expiry.sh
   
   # Renew Let's Encrypt certificate
   certbot renew --force-renewal
   
   # Restart NGINX
   docker-compose restart nginx
   ```

3. **Firewall Issues**
   ```bash
   # Check blocked IPs
   fail2ban-client status
   
   # Unblock IP if needed
   fail2ban-client set sshd unbanip 192.168.1.100
   
   # Restart firewall if needed
   ./firewall-setup.sh
   ```

### Performance Degradation

#### Symptoms
- High response times
- High CPU/memory usage
- Slow database queries
- High error rates

#### Investigation
```bash
# Check system resources
top
htop
iotop
free -h
df -h

# Check service performance
docker stats

# Check database performance
docker exec matrixon-postgres psql -U matrixon -c "SELECT * FROM pg_stat_statements ORDER BY total_time DESC LIMIT 10;"

# Check NGINX performance
tail -f /var/log/nginx/access.log | grep -E "\"[45][0-9][0-9]"

# Check application logs
grep -i "slow\|timeout\|error" /var/log/matrixon/*.log
```

#### Resolution Steps
1. **High CPU Usage**
   ```bash
   # Identify CPU-intensive processes
   top -o %CPU
   
   # Scale down non-essential services
   docker-compose scale cadvisor=0
   
   # Restart services if needed
   docker-compose restart
   ```

2. **Memory Issues**
   ```bash
   # Check memory usage
   docker stats --no-stream
   
   # Clear system cache
   echo 3 > /proc/sys/vm/drop_caches
   
   # Restart high-memory services
   docker-compose restart elasticsearch
   ```

3. **Database Performance**
   ```bash
   # Kill slow queries
   docker exec matrixon-postgres psql -U matrixon -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state = 'active' AND query_start < now() - interval '30 seconds';"
   
   # Reindex if needed
   docker exec matrixon-postgres psql -U matrixon -c "REINDEX DATABASE matrixon;"
   ```

### Security Incidents

#### Symptoms
- Suspicious login attempts
- Unusual network traffic
- Security alerts
- Data breach indicators

#### Investigation
```bash
# Check security logs
grep -i "authentication failure\|invalid user\|break-in" /var/log/auth.log
grep -i "security\|breach\|attack" /var/log/matrixon/security.log

# Check fail2ban status
fail2ban-client status
fail2ban-client status sshd

# Check firewall logs
grep "DENIED" /var/log/syslog

# Check user activities
last -n 20
who
w

# Check network connections
netstat -an | grep ESTABLISHED
ss -tuln
```

#### Resolution Steps
1. **Brute Force Attack**
   ```bash
   # Check fail2ban status
   fail2ban-client status sshd
   
   # Block additional IPs if needed
   iptables -A INPUT -s 192.168.1.100 -j DROP
   
   # Update fail2ban configuration
   nano /etc/fail2ban/jail.local
   systemctl restart fail2ban
   ```

2. **Suspected Compromise**
   ```bash
   # Immediately isolate system
   iptables -P INPUT DROP
   iptables -P FORWARD DROP
   
   # Change all passwords
   # Revoke all API keys
   # Check for backdoors
   
   # Restore from clean backup
   ./automated-backup.sh restore clean_backup_date
   ```

## 📝 Post-Incident Procedures

### Step 1: Service Recovery Verification (30-60 minutes)
1. **Full System Health Check**
   ```bash
   # Comprehensive status check
   ./rolling-update.sh status
   
   # Monitor key metrics for 30 minutes
   # Check error rates
   # Verify performance metrics
   # Test critical user flows
   ```

2. **User Communication**
   - Update status page
   - Send user notifications if needed
   - Post in community channels

### Step 2: Documentation (1-2 hours)
1. **Incident Report**
   - Timeline of events
   - Root cause analysis
   - Impact assessment
   - Resolution steps taken

2. **Update Runbooks**
   - Document new procedures
   - Update contact information
   - Revise severity definitions if needed

### Step 3: Post-Mortem (24-48 hours)
1. **Schedule Post-Mortem Meeting**
   - Include all responders
   - Invite stakeholders
   - Prepare timeline and data

2. **Post-Mortem Agenda**
   - What happened?
   - What went well?
   - What could be improved?
   - Action items with owners

## 🔧 Recovery and Rollback Procedures

### Quick Rollback
```bash
# Emergency rollback to last known good state
./rolling-update.sh rollback

# Or to specific point
./rolling-update.sh rollback 20241219_143000
```

### Database Recovery
```bash
# Stop all services
docker-compose down

# Restore database from backup
./automated-backup.sh restore latest

# Start services
docker-compose up -d

# Verify recovery
./rolling-update.sh status
```

### Configuration Recovery
```bash
# Restore configuration from backup
cp /opt/matrixon-rollback/latest/docker-compose.postgresql.yml .
cp /opt/matrixon-rollback/latest/production.env .

# Restart with restored configuration
docker-compose down
docker-compose up -d
```

## 📊 Escalation Matrix

### When to Escalate

| Condition | Escalate To | Timeframe |
|-----------|-------------|-----------|
| Severity 1 not resolved in 30 min | Engineering Manager | Immediate |
| Severity 2 not resolved in 2 hours | Engineering Manager | Within 15 min |
| Data breach suspected | Security Team + Legal | Immediate |
| Revenue impact >$10k | Engineering Manager + CTO | Immediate |
| Multiple services affected | DevOps Lead | Within 15 min |
| Customer complaints rising | Customer Success + PR | Within 30 min |

### Escalation Process
1. **Update incident ticket** with escalation reason
2. **Call/page** the escalation contact
3. **Brief** them on current situation
4. **Transfer** incident command if requested
5. **Continue** working on resolution

## 🛡️ Prevention and Monitoring

### Proactive Monitoring
- **Set up comprehensive alerts** for all critical metrics
- **Regular health checks** and synthetic monitoring
- **Capacity planning** and resource monitoring
- **Security scanning** and vulnerability assessments

### Regular Drills
- **Monthly incident response drills**
- **Quarterly disaster recovery tests**
- **Annual security incident simulations**
- **Regular backup/restore testing**

### Continuous Improvement
- **Review incidents monthly**
- **Update procedures quarterly**
- **Train team on new procedures**
- **Automate common resolution steps**

---

**⚠️ Remember:** The goal is to restore service as quickly as possible while minimizing impact. When in doubt, escalate early and communicate frequently. 
