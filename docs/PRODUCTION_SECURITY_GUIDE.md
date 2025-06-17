# Matrixon Matrix Server - Production Security Deployment Guide

**Author:** arkSong (arksong2018@gmail.com)  
**Date:** 2024-12-19  
**Version:** 1.0  
**Purpose:** Complete security configuration for production deployment

## 🚀 Quick Start - Security Setup

### 1. Change Default Passwords

**CRITICAL: Change ALL default passwords before production deployment!**

```bash
# 1. Copy and customize environment file
cp production.env production.env.local
nano production.env.local

# 2. Generate strong passwords (example)
openssl rand -base64 32  # For database passwords
openssl rand -hex 16     # For shorter keys
```

### 2. SSL/TLS Certificate Setup

```bash
# Run SSL setup script (requires root)
sudo chmod +x ssl-setup.sh
sudo ./ssl-setup.sh

# For Let's Encrypt (production domains)
sudo certbot certonly --nginx -d your-domain.com
```

### 3. Firewall Configuration

```bash
# Run firewall setup script (requires root)
sudo chmod +x firewall-setup.sh
sudo ./firewall-setup.sh

# Verify firewall status
sudo iptables -L -n
sudo fail2ban-client status
```

### 4. Deploy with Security

```bash
# Use production environment file
docker-compose --env-file production.env.local -f docker-compose.postgresql.yml up -d

# Verify all services are running
docker-compose ps
```

## 🔐 Security Configuration Details

### Password Requirements

| Service | Requirement | Example Generation |
|---------|-------------|-------------------|
| PostgreSQL | 32+ chars, mixed case, numbers, symbols | `openssl rand -base64 32` |
| Redis | 32+ chars, no special chars | `openssl rand -hex 32` |
| Admin | 16+ chars, mixed case, numbers | `pwgen -s 16 1` |
| Grafana | 16+ chars, mixed case | `pwgen -s 16 1` |
| SMTP | As per provider requirements | Provider-specific |

### SSL/TLS Configuration

#### Self-Signed Certificates (Development)
```bash
# Generated automatically by ssl-setup.sh
# Files: /etc/ssl/matrixon/certs/matrixon.crt
#        /etc/ssl/matrixon/private/matrixon.key
```

#### Let's Encrypt (Production)
```bash
# Prerequisites: Domain must resolve to your server
sudo certbot certonly --nginx -d matrixon.yourdomain.com
sudo ln -sf /etc/letsencrypt/live/matrixon.yourdomain.com/fullchain.pem /etc/ssl/matrixon/certs/letsencrypt.crt
sudo ln -sf /etc/letsencrypt/live/matrixon.yourdomain.com/privkey.pem /etc/ssl/matrixon/private/letsencrypt.key
```

#### Custom CA Certificates
```bash
# For enterprise environments with internal CA
# Copy your CA certificate to: /etc/ssl/matrixon/ca/ca-cert.pem
# Copy your server certificate to: /etc/ssl/matrixon/certs/matrixon.crt
# Copy your private key to: /etc/ssl/matrixon/private/matrixon.key
```

### Firewall Rules Summary

| Port | Service | Access | Protection |
|------|---------|--------|------------|
| 22 | SSH | Internal networks only | Rate limiting, Fail2Ban |
| 80 | HTTP | Public | Rate limiting, redirect to HTTPS |
| 443 | HTTPS | Public | Rate limiting, SSL/TLS |
| 8443 | Admin | Internal networks only | Client certificates, basic auth |
| 8448 | Federation | Public | Rate limiting |
| 5432 | PostgreSQL | Container network only | Network isolation |
| 6379 | Redis | Container network only | Network isolation |
| 9090 | Prometheus | Internal networks only | Basic auth |
| 3001 | Grafana | Internal networks only | Application auth |
| 5601 | Kibana | Internal networks only | Network restriction |

## 🛡️ Security Features Enabled

### 1. Network Security
- **Default deny** firewall policy
- **Rate limiting** on all public ports
- **DDoS protection** (SYN flood, ping of death)
- **Geographic blocking** (configurable countries)
- **Port scan detection** and blocking
- **Network isolation** for databases

### 2. Authentication & Authorization
- **Strong passwords** (32+ characters)
- **Multi-factor authentication** support
- **Client certificates** for admin access
- **Basic authentication** for monitoring interfaces
- **Session management** with configurable timeouts

### 3. SSL/TLS Security
- **TLS 1.2/1.3 only** (older versions disabled)
- **Strong cipher suites** (ECDHE preferred)
- **Perfect Forward Secrecy** (PFS)
- **HSTS** (HTTP Strict Transport Security)
- **OCSP stapling** for certificate validation
- **Certificate monitoring** and expiry alerts

### 4. Application Security
- **Security headers** (CSP, X-Frame-Options, etc.)
- **CORS configuration** for Matrix clients
- **Input validation** and sanitization
- **SQL injection protection** (prepared statements)
- **XSS protection** headers
- **File upload restrictions** (type and size)

### 5. Monitoring & Alerting
- **Security event logging** (all authentication attempts)
- **Fail2Ban** integration for automated blocking
- **Real-time alerting** for security events
- **Audit logging** with structured format
- **Performance monitoring** with thresholds
- **Certificate expiry monitoring**

## 🔧 Production Environment Variables

### Critical Security Variables

```bash
# Domain and SSL
MATRIXON_DOMAIN=matrixon.yourdomain.com
SSL_CERT_PATH=/etc/ssl/matrixon/certs/matrixon.crt
SSL_KEY_PATH=/etc/ssl/matrixon/private/matrixon.key

# Database Security
POSTGRES_DB=matrixon_prod
POSTGRES_USER=matrixon_admin
POSTGRES_PASSWORD=CHANGE_ME_Strong_DB_Password_2024!@#$

# Redis Security
REDIS_PASSWORD=CHANGE_ME_Strong_Redis_Password_2024!@#$

# Admin Security
ADMIN_PASSWORD=CHANGE_ME_Strong_Admin_Password_2024!@#$
GRAFANA_ADMIN_PASSWORD=CHANGE_ME_Strong_Grafana_Password_2024!@#$

# Email Alerting
SMTP_HOST=smtp.yourdomain.com
SMTP_USERNAME=alerts@yourdomain.com
SMTP_PASSWORD=CHANGE_ME_Strong_SMTP_Password_2024!@#$

# Security Settings
ENVIRONMENT=production
DEBUG=false
ENABLE_REGISTRATION=false
```

## 📋 Pre-Production Checklist

### Security Hardening
- [ ] All default passwords changed
- [ ] Strong passwords (32+ characters) generated
- [ ] SSL/TLS certificates obtained and configured
- [ ] Firewall rules implemented and tested
- [ ] Fail2Ban configured and active
- [ ] Geographic blocking configured (if needed)
- [ ] Admin access restricted to specific networks
- [ ] Database access limited to container network
- [ ] Monitoring access secured with authentication

### SSL/TLS Configuration
- [ ] Valid SSL certificate obtained (Let's Encrypt or CA)
- [ ] Certificate chain verified
- [ ] SSL configuration tested (SSLLabs.com)
- [ ] HSTS enabled and configured
- [ ] Certificate monitoring set up
- [ ] Auto-renewal configured (for Let's Encrypt)

### Network Security
- [ ] Firewall rules tested and verified
- [ ] DDoS protection tested
- [ ] Rate limiting tested on all endpoints
- [ ] SSH access tested from allowed networks only
- [ ] Admin interface access tested and secured
- [ ] Database ports blocked from external access

### Monitoring & Alerting
- [ ] Alert manager configured with proper routing
- [ ] Email/Slack notifications tested
- [ ] Security monitoring rules active
- [ ] Log aggregation working (ELK stack)
- [ ] Certificate expiry alerts configured
- [ ] Connection pool monitoring active

### Application Security
- [ ] Registration disabled (unless needed)
- [ ] Federation enabled and tested
- [ ] Admin interface secured
- [ ] File upload restrictions in place
- [ ] Security headers configured
- [ ] CORS properly configured

## 🚨 Security Incident Response

### Immediate Actions
1. **Isolate** affected systems
2. **Block** malicious IPs via firewall
3. **Analyze** logs for scope of compromise
4. **Notify** security team and stakeholders
5. **Document** incident details

### Investigation Steps
```bash
# Check for failed login attempts
grep "Failed login" /var/log/matrixon/security.log

# Review firewall logs
grep "DENIED" /var/log/syslog

# Check Fail2Ban status
fail2ban-client status

# Review Elasticsearch security logs
curl -X GET "localhost:9200/security-*/_search?q=level:ERROR"
```

### Recovery Actions
1. **Patch** vulnerabilities
2. **Update** compromised passwords
3. **Rotate** SSL certificates if needed
4. **Review** and update security rules
5. **Test** all security measures

## 📊 Security Monitoring

### Key Metrics to Monitor
- Failed authentication attempts (threshold: >10/hour)
- Database connection pool usage (threshold: >80%)
- SSL certificate expiry (warning: 30 days)
- Disk space usage (warning: 80%, critical: 90%)
- Memory usage (warning: 85%, critical: 95%)
- Network traffic anomalies
- Geographic access patterns

### Log Files to Monitor
- `/var/log/matrixon/security.log` - Application security events
- `/var/log/nginx/access.log` - Web server access logs
- `/var/log/postgresql/postgresql.log` - Database audit logs
- `/var/log/fail2ban.log` - Automated blocking events
- `/var/log/matrixon-firewall.log` - Firewall configuration events

## 🔄 Maintenance Schedule

### Daily
- [ ] Review security logs for anomalies
- [ ] Check service health and performance
- [ ] Verify backup completion
- [ ] Monitor alert status

### Weekly
- [ ] Review and analyze security metrics
- [ ] Update security threat intelligence
- [ ] Test backup restoration process
- [ ] Review access logs for patterns

### Monthly
- [ ] Update GeoIP database
- [ ] Review and update firewall rules
- [ ] Security patch management
- [ ] Penetration testing (if applicable)
- [ ] Access review and cleanup

### Quarterly
- [ ] Full security audit
- [ ] Password rotation policy
- [ ] SSL certificate renewal check
- [ ] Disaster recovery testing
- [ ] Security policy review

## 🆘 Emergency Contacts

| Role | Contact | Escalation |
|------|---------|------------|
| Security Team | security@yourdomain.com | Critical incidents |
| DevOps Team | devops@yourdomain.com | Infrastructure issues |
| DBA Team | dba@yourdomain.com | Database emergencies |
| Management | management@yourdomain.com | Major incidents |

## 📚 Additional Resources

- [Matrix.org Security Guide](https://matrix.org/security/)
- [NGINX Security Best Practices](https://nginx.org/en/docs/security/)
- [PostgreSQL Security](https://www.postgresql.org/docs/current/security.html)
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)
- [OWASP Security Guidelines](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/)

---

**⚠️ CRITICAL REMINDER:** This deployment contains enterprise-grade security features. All passwords must be changed before production use. Regular security audits and monitoring are essential for maintaining security posture. 
