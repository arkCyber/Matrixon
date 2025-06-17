# Matrixon Matrix Server - Operations Quick Start Guide

**Author:** arkSong (arksong2018@gmail.com)  
**Date:** 2024-12-19  
**Version:** 1.0  
**Purpose:** Complete operations automation quick start guide

## 🚀 Quick Setup Commands

### 1. Initial Security Setup (5 minutes)
```bash
# 1. Configure production environment
cp production.env.example production.env
nano production.env  # Update all passwords and settings

# 2. Setup firewall
sudo ./firewall-setup.sh

# 3. Setup SSL certificates
sudo ./ssl-setup.sh

# 4. Start services
docker-compose -f docker-compose.postgresql.yml up -d
```

### 2. Enable All Automation (10 minutes)
```bash
# 1. Setup automated backups
chmod +x automated-backup.sh
./automated-backup.sh setup

# 2. Setup log monitoring and rotation
chmod +x log-monitoring.sh
sudo cp log-rotation.conf /etc/logrotate.d/matrixon
./log-monitoring.sh setup

# 3. Setup rolling updates
chmod +x rolling-update.sh

# 4. Test everything works
./automated-backup.sh daily
./log-monitoring.sh full
./rolling-update.sh status
```

## 📋 Daily Operations Checklist

### Morning Health Check (5 minutes)
```bash
# System status overview
./rolling-update.sh status

# Check recent logs
./log-monitoring.sh summary 24

# Verify backups
ls -la /opt/matrixon-backups/daily/

# Check disk space
df -h
```

### Service Monitoring (Ongoing)
- **Grafana Dashboard:** http://localhost:3001
- **Prometheus Metrics:** http://localhost:9090
- **AlertManager:** http://localhost:9093
- **Kibana Logs:** http://localhost:5601

## 🔄 Automated Processes

### Backup Schedule
| Frequency | Time | Command | Purpose |
|-----------|------|---------|---------|
| **Daily** | 2:00 AM | `automated-backup.sh daily` | Full daily backup |
| **Weekly** | 3:00 AM Sunday | `automated-backup.sh weekly` | Comprehensive weekly backup |
| **Monthly** | 4:00 AM 1st | `automated-backup.sh test` | Backup restore test |

### Log Management Schedule
| Frequency | Time | Command | Purpose |
|-----------|------|---------|---------|
| **5 minutes** | Continuous | `log-monitoring.sh monitor` | Error pattern monitoring |
| **15 minutes** | Continuous | `log-monitoring.sh analyze` | Error rate analysis |
| **Hourly** | :00 | `log-monitoring.sh disk` | Disk usage check |
| **Daily** | 2:00 AM | `log-monitoring.sh cleanup` | Old log cleanup |
| **Weekly** | 4:00 AM Sunday | `log-monitoring.sh archive` | Log archiving |

### Security Monitoring
- **Fail2Ban:** Automatic IP blocking for brute force
- **Firewall:** UFW with custom Matrix rules
- **SSL:** Automatic certificate renewal
- **Audit:** Security event logging to ELK stack

## ⚡ Emergency Procedures

### 🚨 Critical System Down
```bash
# 1. Quick status check
./rolling-update.sh status
docker ps

# 2. Check recent errors
./log-monitoring.sh realtime 30

# 3. Restart services if needed
docker-compose restart

# 4. Emergency rollback if needed
./rolling-update.sh rollback
```

### 🗄️ Database Issues
```bash
# Check PostgreSQL status
docker exec matrixon-postgres pg_isready

# Check connections
docker exec matrixon-postgres psql -U matrixon -c "SELECT count(*) FROM pg_stat_activity;"

# Emergency restore
./automated-backup.sh restore latest
```

### 🔐 Security Incident
```bash
# Check security logs
./log-monitoring.sh summary 1
grep -i "breach\|attack" /var/log/matrixon/security.log

# Check fail2ban
fail2ban-client status

# Block suspicious IPs
iptables -A INPUT -s SUSPICIOUS_IP -j DROP
```

## 📊 Performance Monitoring

### Key Metrics to Watch
1. **Response Time:** < 50ms average
2. **Error Rate:** < 1% for 4xx/5xx errors
3. **Connection Count:** < 180 active connections
4. **Memory Usage:** < 80% of available RAM
5. **Disk Usage:** < 80% for all partitions
6. **CPU Usage:** < 70% average

### Performance Commands
```bash
# System resources
htop
free -h
df -h
iostat -x 1

# Application metrics
curl -s http://localhost:9090/api/v1/query?query=up
curl -s http://localhost:3001/api/health

# Database performance
docker exec matrixon-postgres psql -U matrixon -c "SELECT * FROM pg_stat_statements ORDER BY total_time DESC LIMIT 5;"
```

## 🔧 Maintenance Procedures

### Weekly Maintenance (30 minutes)
```bash
# 1. System updates
sudo apt update && sudo apt upgrade -y

# 2. Container updates
./rolling-update.sh update

# 3. Database maintenance
docker exec matrixon-postgres psql -U matrixon -c "VACUUM ANALYZE;"

# 4. Log cleanup
./log-monitoring.sh cleanup 7

# 5. Security scan
./firewall-setup.sh status
```

### Monthly Maintenance (1 hour)
```bash
# 1. Backup verification
./automated-backup.sh test

# 2. SSL certificate check
./ssl-setup.sh check

# 3. Performance review
./log-monitoring.sh summary 720  # 30 days

# 4. Security audit
grep -i "failed\|error" /var/log/auth.log | tail -20

# 5. Capacity planning
df -h
free -h
docker system df
```

## 🎯 Automation Scripts Reference

### automated-backup.sh
```bash
# Manual backup
./automated-backup.sh daily
./automated-backup.sh weekly

# Setup automation
./automated-backup.sh setup

# Test restore
./automated-backup.sh test

# List backups
ls -la /opt/matrixon-backups/
```

### rolling-update.sh
```bash
# Update all services
./rolling-update.sh update

# Update single service
./rolling-update.sh single nginx

# Check status
./rolling-update.sh status

# Rollback
./rolling-update.sh rollback

# List rollback points
./rolling-update.sh list
```

### log-monitoring.sh
```bash
# Monitor errors
./log-monitoring.sh monitor

# Generate report
./log-monitoring.sh summary 24

# Clean old logs
./log-monitoring.sh cleanup 30

# Real-time monitoring
./log-monitoring.sh realtime 60

# Setup automation
./log-monitoring.sh setup
```

### Security Scripts
```bash
# Firewall management
./firewall-setup.sh          # Setup
./firewall-setup.sh status   # Check status
./firewall-setup.sh restart  # Restart

# SSL management  
./ssl-setup.sh               # Setup
./ssl-setup.sh renew         # Renew certificates
./ssl-setup.sh check         # Check expiry
```

## 📈 Scaling and Optimization

### Horizontal Scaling
```bash
# Scale specific services
docker-compose scale redis=2
docker-compose scale nginx=3

# Monitor resource usage
docker stats

# Load balancer configuration
# Edit nginx-ssl.conf for upstream changes
```

### Vertical Scaling
```bash
# Update resource limits in docker-compose.postgresql.yml
# Restart affected services
docker-compose restart postgres
docker-compose restart elasticsearch
```

### Performance Tuning
1. **PostgreSQL:** Adjust shared_buffers, effective_cache_size
2. **Redis:** Configure maxmemory policies
3. **NGINX:** Tune worker processes and connections
4. **System:** Configure kernel parameters for high connections

## 🔔 Alerting and Notifications

### Slack Integration
```bash
# Set in production.env
SLACK_WEBHOOK_URL="https://hooks.slack.com/services/..."

# Test notifications
./automated-backup.sh daily  # Will send status
./log-monitoring.sh monitor  # Will alert on issues
```

### Email Alerts
```bash
# Configure SMTP in production.env
SMTP_HOST="smtp.gmail.com"
SMTP_PORT="587"
SMTP_USERNAME="alerts@matrixon.com"
SMTP_PASSWORD="app_password"

# Test email
echo "Test alert" | mail -s "Test" admin@matrixon.com
```

### Monitoring Integration
- **PagerDuty:** Configure in alertmanager-enhanced.yml
- **Grafana Alerts:** Set up in Grafana dashboards
- **Custom Webhooks:** Add to notification scripts

## 🛡️ Security Best Practices

### Access Control
```bash
# Regular user audit
last -n 20
who
w

# SSH key management
cat ~/.ssh/authorized_keys
# Remove old/unused keys

# Service account review
docker exec matrixon-postgres psql -U matrixon -c "SELECT * FROM pg_user;"
```

### Network Security
```bash
# Port scanning
nmap -sS localhost

# Connection monitoring
netstat -tulpn | grep :443
ss -tuln

# Fail2ban status
fail2ban-client status
fail2ban-client status sshd
```

### Data Protection
```bash
# Encryption verification
ls -la /opt/matrixon-backups/daily/*/database/*.enc

# SSL certificate status
openssl x509 -in /etc/ssl/matrixon/matrixon.crt -text -noout

# Permission audit
find /var/log/matrixon -type f -exec ls -la {} \;
```

## 📞 Emergency Contacts

### Internal Team
- **On-Call Engineer:** +1-XXX-XXX-XXXX
- **DevOps Lead:** +1-XXX-XXX-XXXX  
- **Security Team:** +1-XXX-XXX-XXXX

### External Services
- **Cloud Provider:** AWS Support
- **DNS Provider:** CloudFlare Support
- **Monitoring:** PagerDuty

## 📚 Additional Resources

### Documentation
- [Incident Response Procedures](./INCIDENT_RESPONSE_PROCEDURES.md)
- [Production Security Guide](./PRODUCTION_SECURITY_GUIDE.md)
- [Matrix Protocol Docs](https://matrix.org/docs/)

### Monitoring URLs
- **Grafana:** http://localhost:3001
- **Prometheus:** http://localhost:9090
- **AlertManager:** http://localhost:9093
- **Kibana:** http://localhost:5601

### Log Locations
- **Application:** `/var/log/matrixon/`
- **NGINX:** `/var/log/nginx/`
- **PostgreSQL:** `/var/log/postgresql/`
- **System:** `/var/log/syslog`

---

**🎯 Remember:** Automate everything, monitor continuously, and respond quickly. When in doubt, check the logs and escalate early. 
