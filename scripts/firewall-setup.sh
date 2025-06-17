#!/bin/bash

# Matrixon Matrix Server - Production Firewall Configuration
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 1.0
# Purpose: Comprehensive firewall setup for production security

set -euo pipefail

# Configuration variables
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="/var/log/matrixon-firewall.log"
BACKUP_DIR="/etc/iptables/backup"

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [FIREWALL] $1" | tee -a "$LOG_FILE"
}

# Error handling
error_exit() {
    log "❌ ERROR: $1"
    exit 1
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        error_exit "This script must be run as root"
    fi
}

# Backup existing iptables rules
backup_rules() {
    log "🔧 Backing up existing iptables rules..."
    mkdir -p "$BACKUP_DIR"
    iptables-save > "$BACKUP_DIR/iptables-backup-$(date +%Y%m%d-%H%M%S).rules"
    ip6tables-save > "$BACKUP_DIR/ip6tables-backup-$(date +%Y%m%d-%H%M%S).rules"
    log "✅ Backup completed"
}

# Install required packages
install_packages() {
    log "🔧 Installing required firewall packages..."
    
    # Detect OS and install appropriate packages
    if command -v apt-get >/dev/null 2>&1; then
        apt-get update
        apt-get install -y iptables iptables-persistent ufw fail2ban geoipupdate
    elif command -v yum >/dev/null 2>&1; then
        yum install -y iptables iptables-services fail2ban geoipupdate
    elif command -v dnf >/dev/null 2>&1; then
        dnf install -y iptables iptables-services fail2ban geoipupdate
    else
        error_exit "Unsupported operating system"
    fi
    
    log "✅ Packages installed successfully"
}

# Clear existing rules
clear_rules() {
    log "🔧 Clearing existing iptables rules..."
    
    # IPv4 rules
    iptables -F
    iptables -X
    iptables -t nat -F
    iptables -t nat -X
    iptables -t mangle -F
    iptables -t mangle -X
    
    # IPv6 rules
    ip6tables -F
    ip6tables -X
    ip6tables -t mangle -F
    ip6tables -t mangle -X
    
    log "✅ Rules cleared"
}

# Set default policies
set_default_policies() {
    log "🔧 Setting default policies..."
    
    # Default policies - DROP for security
    iptables -P INPUT DROP
    iptables -P FORWARD DROP
    iptables -P OUTPUT ACCEPT
    
    # IPv6 - block by default (can be enabled later)
    ip6tables -P INPUT DROP
    ip6tables -P FORWARD DROP
    ip6tables -P OUTPUT DROP
    
    log "✅ Default policies set"
}

# Allow loopback traffic
allow_loopback() {
    log "🔧 Allowing loopback traffic..."
    
    iptables -A INPUT -i lo -j ACCEPT
    iptables -A OUTPUT -o lo -j ACCEPT
    
    log "✅ Loopback traffic allowed"
}

# Allow established and related connections
allow_established() {
    log "🔧 Allowing established and related connections..."
    
    iptables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT
    iptables -A OUTPUT -m conntrack --ctstate ESTABLISHED -j ACCEPT
    
    log "✅ Established connections allowed"
}

# SSH protection with rate limiting
setup_ssh_protection() {
    log "🔧 Setting up SSH protection..."
    
    # Allow SSH with rate limiting
    iptables -N SSH_PROTECTION
    iptables -A SSH_PROTECTION -m recent --name ssh_attack --set
    iptables -A SSH_PROTECTION -m recent --name ssh_attack --rcheck --seconds 60 --hitcount 4 -j LOG --log-prefix "SSH_ATTACK: "
    iptables -A SSH_PROTECTION -m recent --name ssh_attack --rcheck --seconds 60 --hitcount 4 -j DROP
    iptables -A SSH_PROTECTION -j ACCEPT
    
    # Allow SSH from specific networks
    iptables -A INPUT -p tcp --dport 22 -s 10.0.0.0/8 -j SSH_PROTECTION
    iptables -A INPUT -p tcp --dport 22 -s 172.16.0.0/12 -j SSH_PROTECTION
    iptables -A INPUT -p tcp --dport 22 -s 192.168.0.0/16 -j SSH_PROTECTION
    
    log "✅ SSH protection configured"
}

# Matrix server ports
setup_matrix_ports() {
    log "🔧 Setting up Matrix server ports..."
    
    # HTTP/HTTPS with rate limiting
    iptables -N HTTP_RATE_LIMIT
    iptables -A HTTP_RATE_LIMIT -m limit --limit 100/minute --limit-burst 20 -j ACCEPT
    iptables -A HTTP_RATE_LIMIT -j LOG --log-prefix "HTTP_RATE_LIMIT: "
    iptables -A HTTP_RATE_LIMIT -j DROP
    
    # HTTP (redirect to HTTPS)
    iptables -A INPUT -p tcp --dport 80 -j HTTP_RATE_LIMIT
    
    # HTTPS
    iptables -A INPUT -p tcp --dport 443 -j HTTP_RATE_LIMIT
    
    # Matrix Federation (8448)
    iptables -N FEDERATION_RATE_LIMIT
    iptables -A FEDERATION_RATE_LIMIT -m limit --limit 200/minute --limit-burst 50 -j ACCEPT
    iptables -A FEDERATION_RATE_LIMIT -j LOG --log-prefix "FEDERATION_RATE_LIMIT: "
    iptables -A FEDERATION_RATE_LIMIT -j DROP
    iptables -A INPUT -p tcp --dport 8448 -j FEDERATION_RATE_LIMIT
    
    # Admin interface (8443) - restricted networks only
    iptables -A INPUT -p tcp --dport 8443 -s 10.0.0.0/8 -j ACCEPT
    iptables -A INPUT -p tcp --dport 8443 -s 172.16.0.0/12 -j ACCEPT
    iptables -A INPUT -p tcp --dport 8443 -s 192.168.0.0/16 -j ACCEPT
    iptables -A INPUT -p tcp --dport 8443 -j LOG --log-prefix "ADMIN_ACCESS_DENIED: "
    iptables -A INPUT -p tcp --dport 8443 -j DROP
    
    log "✅ Matrix ports configured"
}

# Monitoring and metrics ports (internal only)
setup_monitoring_ports() {
    log "🔧 Setting up monitoring ports..."
    
    local INTERNAL_NETWORKS="10.0.0.0/8,172.16.0.0/12,192.168.0.0/16"
    
    # Prometheus (9090)
    iptables -A INPUT -p tcp --dport 9090 -s 10.0.0.0/8 -j ACCEPT
    iptables -A INPUT -p tcp --dport 9090 -s 172.16.0.0/12 -j ACCEPT
    iptables -A INPUT -p tcp --dport 9090 -s 192.168.0.0/16 -j ACCEPT
    
    # AlertManager (9093)
    iptables -A INPUT -p tcp --dport 9093 -s 10.0.0.0/8 -j ACCEPT
    iptables -A INPUT -p tcp --dport 9093 -s 172.16.0.0/12 -j ACCEPT
    iptables -A INPUT -p tcp --dport 9093 -s 192.168.0.0/16 -j ACCEPT
    
    # Grafana (3001)
    iptables -A INPUT -p tcp --dport 3001 -s 10.0.0.0/8 -j ACCEPT
    iptables -A INPUT -p tcp --dport 3001 -s 172.16.0.0/12 -j ACCEPT
    iptables -A INPUT -p tcp --dport 3001 -s 192.168.0.0/16 -j ACCEPT
    
    # Kibana (5601) 
    iptables -A INPUT -p tcp --dport 5601 -s 10.0.0.0/8 -j ACCEPT
    iptables -A INPUT -p tcp --dport 5601 -s 172.16.0.0/12 -j ACCEPT
    iptables -A INPUT -p tcp --dport 5601 -s 192.168.0.0/16 -j ACCEPT
    
    # Various exporters (internal only)
    for port in 9100 9187 9121 8080; do
        iptables -A INPUT -p tcp --dport $port -s 10.0.0.0/8 -j ACCEPT
        iptables -A INPUT -p tcp --dport $port -s 172.16.0.0/12 -j ACCEPT
        iptables -A INPUT -p tcp --dport $port -s 192.168.0.0/16 -j ACCEPT
    done
    
    log "✅ Monitoring ports configured"
}

# Database ports (internal only)
setup_database_ports() {
    log "🔧 Setting up database ports..."
    
    # PostgreSQL (5432) - internal only
    iptables -A INPUT -p tcp --dport 5432 -s 172.22.0.0/16 -j ACCEPT
    iptables -A INPUT -p tcp --dport 5432 -j LOG --log-prefix "DB_ACCESS_DENIED: "
    iptables -A INPUT -p tcp --dport 5432 -j DROP
    
    # Redis (6379) - internal only
    iptables -A INPUT -p tcp --dport 6379 -s 172.22.0.0/16 -j ACCEPT
    iptables -A INPUT -p tcp --dport 6379 -j LOG --log-prefix "REDIS_ACCESS_DENIED: "
    iptables -A INPUT -p tcp --dport 6379 -j DROP
    
    # Elasticsearch (9200, 9300) - internal only
    iptables -A INPUT -p tcp --dport 9200 -s 172.22.0.0/16 -j ACCEPT
    iptables -A INPUT -p tcp --dport 9300 -s 172.22.0.0/16 -j ACCEPT
    
    log "✅ Database ports configured"
}

# DDoS protection
setup_ddos_protection() {
    log "🔧 Setting up DDoS protection..."
    
    # SYN flood protection
    iptables -N SYN_FLOOD
    iptables -A SYN_FLOOD -m limit --limit 1/s --limit-burst 3 -j RETURN
    iptables -A SYN_FLOOD -j LOG --log-prefix "SYN_FLOOD_ATTACK: "
    iptables -A SYN_FLOOD -j DROP
    iptables -A INPUT -p tcp --syn -j SYN_FLOOD
    
    # Ping of death protection
    iptables -A INPUT -p icmp --icmp-type echo-request -m limit --limit 1/s --limit-burst 2 -j ACCEPT
    iptables -A INPUT -p icmp --icmp-type echo-request -j DROP
    
    # Port scan protection
    iptables -N PORT_SCAN
    iptables -A PORT_SCAN -m recent --name portscan --set -j LOG --log-prefix "PORT_SCAN_ATTACK: "
    iptables -A PORT_SCAN -m recent --name portscan --set -j DROP
    iptables -A INPUT -p tcp --tcp-flags SYN,ACK,FIN,RST RST -m limit --limit 1/s --limit-burst 2 -j ACCEPT
    iptables -A INPUT -p tcp --tcp-flags SYN,ACK,FIN,RST RST -j PORT_SCAN
    
    log "✅ DDoS protection configured"
}

# Geographic blocking (requires GeoIP)
setup_geo_blocking() {
    log "🔧 Setting up geographic blocking..."
    
    # Block specific countries (example: CN, RU, KP, IR)
    if [ -f /usr/share/xt_geoip/GeoLite2-Country.mmdb ]; then
        iptables -A INPUT -m geoip --src-cc CN,RU,KP,IR -j LOG --log-prefix "GEO_BLOCKED: "
        iptables -A INPUT -m geoip --src-cc CN,RU,KP,IR -j DROP
        log "✅ Geographic blocking enabled"
    else
        log "⚠️ GeoIP database not found, skipping geographic blocking"
    fi
}

# Security logging
setup_security_logging() {
    log "🔧 Setting up security logging..."
    
    # Log denied packets
    iptables -N LOGGING
    iptables -A LOGGING -m limit --limit 2/min -j LOG --log-prefix "IPTABLES_DENIED: " --log-level 4
    iptables -A LOGGING -j DROP
    
    # Invalid packets
    iptables -A INPUT -m conntrack --ctstate INVALID -j LOG --log-prefix "INVALID_PACKET: "
    iptables -A INPUT -m conntrack --ctstate INVALID -j DROP
    
    log "✅ Security logging configured"
}

# Save rules
save_rules() {
    log "🔧 Saving iptables rules..."
    
    if command -v iptables-save >/dev/null 2>&1; then
        iptables-save > /etc/iptables/rules.v4
        ip6tables-save > /etc/iptables/rules.v6
        
        # Enable iptables persistence
        if command -v systemctl >/dev/null 2>&1; then
            systemctl enable iptables
            systemctl enable ip6tables
        fi
        
        log "✅ Rules saved and persistence enabled"
    else
        log "⚠️ iptables-save not found, rules not persisted"
    fi
}

# Setup fail2ban
setup_fail2ban() {
    log "🔧 Setting up Fail2Ban..."
    
    cat > /etc/fail2ban/jail.local << 'EOF'
[DEFAULT]
# Ban time (24 hours)
bantime = 86400
# Find time window (10 minutes)
findtime = 600
# Max retry attempts
maxretry = 3
# Ignore local networks
ignoreip = 127.0.0.1/8 10.0.0.0/8 172.16.0.0/12 192.168.0.0/16

[sshd]
enabled = true
port = ssh
filter = sshd
logpath = /var/log/auth.log
maxretry = 3
bantime = 3600

[nginx-http-auth]
enabled = true
filter = nginx-http-auth
logpath = /var/log/nginx/error.log
maxretry = 3

[nginx-limit-req]
enabled = true
filter = nginx-limit-req
logpath = /var/log/nginx/error.log
maxretry = 3

[matrix-auth]
enabled = true
filter = matrix-auth
logpath = /var/log/matrixon/security.log
maxretry = 5
bantime = 7200
EOF

    # Custom filter for Matrix authentication
    cat > /etc/fail2ban/filter.d/matrix-auth.conf << 'EOF'
[Definition]
failregex = ^.*\[SECURITY\].*Failed login attempt.*from <HOST>.*$
            ^.*\[SECURITY\].*Brute force attack.*from <HOST>.*$
            ^.*\[SECURITY\].*Suspicious activity.*from <HOST>.*$
ignoreregex =
EOF

    systemctl enable fail2ban
    systemctl restart fail2ban
    
    log "✅ Fail2Ban configured and started"
}

# Setup UFW (alternative simple firewall)
setup_ufw_rules() {
    log "🔧 Setting up UFW rules as backup..."
    
    ufw --force reset
    ufw default deny incoming
    ufw default allow outgoing
    
    # SSH from internal networks
    ufw allow from 10.0.0.0/8 to any port 22
    ufw allow from 172.16.0.0/12 to any port 22
    ufw allow from 192.168.0.0/16 to any port 22
    
    # HTTP/HTTPS
    ufw allow 80/tcp
    ufw allow 443/tcp
    ufw allow 8448/tcp
    
    # Admin interface (restricted)
    ufw allow from 10.0.0.0/8 to any port 8443
    ufw allow from 172.16.0.0/12 to any port 8443
    ufw allow from 192.168.0.0/16 to any port 8443
    
    ufw --force enable
    
    log "✅ UFW configured as backup firewall"
}

# Main execution
main() {
    log "🚀 Starting Matrixon firewall configuration..."
    
    check_root
    install_packages
    backup_rules
    clear_rules
    set_default_policies
    allow_loopback
    allow_established
    setup_ssh_protection
    setup_matrix_ports
    setup_monitoring_ports
    setup_database_ports
    setup_ddos_protection
    setup_geo_blocking
    setup_security_logging
    
    # Add final rule to log and drop everything else
    iptables -A INPUT -j LOGGING
    
    save_rules
    setup_fail2ban
    setup_ufw_rules
    
    log "🎉 Matrixon firewall configuration completed successfully!"
    log "📊 Current rule count: $(iptables -L | wc -l) IPv4 rules, $(ip6tables -L | wc -l) IPv6 rules"
    
    # Display summary
    echo
    echo "========================================"
    echo "MATRIXON FIREWALL CONFIGURATION SUMMARY"
    echo "========================================"
    echo "✅ Default policies: DROP (secure by default)"
    echo "✅ SSH: Protected with rate limiting"
    echo "✅ HTTP/HTTPS: Rate limited and secured"
    echo "✅ Matrix Federation: Rate limited"
    echo "✅ Admin Interface: Restricted to internal networks"
    echo "✅ Monitoring: Internal networks only"
    echo "✅ Database: Container network only"
    echo "✅ DDoS Protection: Enabled"
    echo "✅ Geographic Blocking: Configured"
    echo "✅ Fail2Ban: Active"
    echo "✅ Security Logging: Enabled"
    echo "========================================"
    echo
    echo "⚠️  IMPORTANT REMINDERS:"
    echo "1. Test SSH access before logging out"
    echo "2. Monitor logs: tail -f $LOG_FILE"
    echo "3. Check Fail2Ban: fail2ban-client status"
    echo "4. View active rules: iptables -L -n"
    echo "5. Update GeoIP database monthly"
    echo "========================================"
}

# Execute main function
main "$@" 
