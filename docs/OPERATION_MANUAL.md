# Matrixon - AI & Web3 Communication Platform Operation Manual

![Status](https://img.shields.io/badge/status-production--ready-green?style=flat-square)
![Version](https://img.shields.io/badge/version-0.11.0--alpha-orange?style=flat-square)
![AI Ready](https://img.shields.io/badge/AI-Ready-purple?style=flat-square)
![Web3](https://img.shields.io/badge/Web3-Blockchain-orange?style=flat-square)

**Matrixon Team** - Next-Generation Communication Platform for AI & Web3  
**Contact**: arksong2018@gmail.com

## 📋 Quick Start

### System Requirements
- **OS**: Linux (Ubuntu 22.04+ recommended), macOS 12+, Windows 10+
- **CPU**: 4+ cores (16+ for production)
- **RAM**: 8GB minimum (64GB+ for production)
- **Storage**: 100GB SSD minimum (1TB+ NVMe for production)
- **Database**: PostgreSQL 13+ or SQLite 3.35+

### Installation Methods

#### Method 1: Docker (Recommended for Quick Start)
```bash
# Quick deployment
docker run -d --name matrixon \
  -p 8008:8008 \
  -v matrixon-data:/data \
  matrixon/matrixon:latest

# Production deployment with compose
curl -O https://raw.githubusercontent.com/arkSong/Matrixon/main/docker-compose.yml
docker-compose up -d
```

#### Method 2: Binary Installation
```bash
# Download and install
curl -L https://github.com/arkSong/Matrixon/releases/latest/download/matrixon-linux-x86_64.tar.gz | tar xz
sudo mv matrixon /usr/local/bin/
sudo chmod +x /usr/local/bin/matrixon

# Verify installation
matrixon --version
```

#### Method 3: Build from Source
```bash
# Prerequisites: Rust 1.83+
git clone https://github.com/arkSong/Matrixon.git
cd Matrixon
cargo build --release --features=production
sudo cp target/release/matrixon /usr/local/bin/
```

---

## ⚙️ Configuration

### Initial Setup
```bash
# Create system user and directories
sudo useradd -r -s /bin/false matrixon
sudo mkdir -p /etc/matrixon /var/lib/matrixon /var/log/matrixon
sudo chown matrixon:matrixon /var/lib/matrixon /var/log/matrixon

# Generate configuration
matrixon generate-config --output /etc/matrixon/matrixon.toml
```

### Essential Configuration (`/etc/matrixon/matrixon.toml`)
```toml
[global]
server_name = "matrix.example.com"
database_url = "postgresql://matrixon:password@localhost/matrixon"

[http]
bind_address = "0.0.0.0"
port = 8008
max_connections = 10000

[tls]
private_key = "/etc/matrixon/tls/private.key"
certificate_chain = "/etc/matrixon/tls/certificate.crt"

[logging]
level = "info"
file = "/var/log/matrixon/matrixon.log"
format = "json"

[federation]
enabled = true
verify_keys = true
```

### Database Setup (PostgreSQL)
```bash
# Install and configure PostgreSQL
sudo apt install postgresql postgresql-contrib
sudo -u postgres createuser -P matrixon
sudo -u postgres createdb -O matrixon matrixon

# Initialize database
matrixon db migrate --config /etc/matrixon/matrixon.toml
```

---

## 🚀 Service Management

### Systemd Service Setup
```bash
# Create service file
sudo tee /etc/systemd/system/matrixon.service << EOF
[Unit]
Description=Matrixon Matrix NextServer
After=network.target postgresql.service

[Service]
Type=exec
User=matrixon
Group=matrixon
ExecStart=/usr/local/bin/matrixon --config /etc/matrixon/matrixon.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable matrixon
sudo systemctl start matrixon
```

### Service Operations
```bash
# Basic operations
sudo systemctl start matrixon
sudo systemctl stop matrixon
sudo systemctl restart matrixon
sudo systemctl status matrixon

# View logs
sudo journalctl -u matrixon -f

# Health check
curl http://localhost:8008/_matrix/client/versions
```

---

## 👥 User Management

### Admin Operations
```bash
# Create admin user
matrixon admin create-user \
  --username admin \
  --password secure_password \
  --admin \
  --config /etc/matrixon/matrixon.toml

# User management
matrixon admin list-users --config /etc/matrixon/matrixon.toml
matrixon admin user-info --username alice --config /etc/matrixon/matrixon.toml
matrixon admin deactivate-user --username alice --config /etc/matrixon/matrixon.toml
```

### Room Management
```bash
# Room operations
matrixon admin list-rooms --config /etc/matrixon/matrixon.toml
matrixon admin create-room --name "General" --creator admin --config /etc/matrixon/matrixon.toml
matrixon admin delete-room --room-id '!example:matrix.org' --config /etc/matrixon/matrixon.toml
```

---

## 🌐 Federation & SSL

### SSL Certificate Setup (Let's Encrypt)
```bash
# Install certbot
sudo apt install certbot

# Get certificate
sudo certbot certonly --standalone -d matrix.example.com

# Link certificates
sudo ln -s /etc/letsencrypt/live/matrix.example.com/privkey.pem /etc/matrixon/tls/private.key
sudo ln -s /etc/letsencrypt/live/matrix.example.com/fullchain.pem /etc/matrixon/tls/certificate.crt
```

### NGINX Reverse Proxy
```nginx
server {
    listen 443 ssl http2;
    server_name matrix.example.com;

    ssl_certificate /etc/letsencrypt/live/matrix.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/matrix.example.com/privkey.pem;

    location /_matrix {
        proxy_pass http://127.0.0.1:8008;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Host $host;
    }
}
```

### Federation Setup
```bash
# Generate signing key
matrixon admin generate-key --output /etc/matrixon/signing.key

# Test federation
matrixon admin federation-test --server matrix.org --config /etc/matrixon/matrixon.toml

# Well-known delegation (/.well-known/matrix/server)
{"m.server": "matrix.example.com:443"}
```

---

## 📊 Monitoring & Backup

### Monitoring Setup
```toml
# Add to matrixon.toml
[metrics]
enabled = true
bind_address = "127.0.0.1"
port = 9090
```

```bash
# View metrics
curl http://localhost:9090/metrics

# Health check script
#!/bin/bash
curl -f http://localhost:8008/_matrix/client/versions || exit 1
matrixon db-check --config /etc/matrixon/matrixon.toml || exit 1
echo "Health check passed"
```

### Backup Strategy
```bash
# Automated backup script
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backup/matrixon"

# Database backup
pg_dump -U matrixon matrixon | gzip > $BACKUP_DIR/db_$DATE.sql.gz

# Configuration backup
tar -czf $BACKUP_DIR/config_$DATE.tar.gz /etc/matrixon/

# Media backup
tar -czf $BACKUP_DIR/media_$DATE.tar.gz /var/lib/matrixon/media/

# Cleanup old backups (keep 30 days)
find $BACKUP_DIR -name "*.gz" -mtime +30 -delete
```

---

## 🔧 Troubleshooting

### Common Issues

#### Service Won't Start
```bash
# Check configuration
matrixon config validate --config /etc/matrixon/matrixon.toml

# Check permissions
sudo chown -R matrixon:matrixon /etc/matrixon/ /var/lib/matrixon/

# View logs
sudo journalctl -u matrixon -n 50
```

#### Database Issues
```bash
# Test database connection
psql -U matrixon -d matrixon -c "SELECT version();"

# Check database status
matrixon db status --config /etc/matrixon/matrixon.toml

# Run migrations
matrixon db migrate --config /etc/matrixon/matrixon.toml
```

#### Federation Problems
```bash
# Test federation connectivity
dig _matrix._tcp.matrix.org SRV
matrixon admin federation-test --server matrix.org

# Check DNS and certificates
openssl s_client -connect matrix.org:443 -servername matrix.org
```

### Performance Optimization

#### System Tuning
```bash
# Increase file descriptor limits
echo "matrixon soft nofile 65536" >> /etc/security/limits.conf
echo "matrixon hard nofile 65536" >> /etc/security/limits.conf

# TCP optimization
echo "net.core.somaxconn = 65536" >> /etc/sysctl.conf
sysctl -p
```

#### Application Tuning
```toml
# Performance settings in matrixon.toml
[performance]
worker_threads = 16
max_blocking_threads = 512
connection_pool_size = 100
cache_size = "1GB"

[database]
max_connections = 200
connection_timeout = 5
```

---

## 🚨 Emergency Procedures

### Emergency Operations
```bash
# Emergency stop
sudo systemctl stop matrixon

# Emergency restart with debug
RUST_LOG=debug matrixon --config /etc/matrixon/matrixon.toml

# Quick recovery from backup
sudo systemctl stop matrixon
gunzip -c /backup/matrixon/db_latest.sql.gz | psql -U matrixon matrixon
sudo systemctl start matrixon

# Scale with Docker
docker-compose up --scale matrixon=3
```

### Maintenance Tasks
```bash
# Weekly maintenance
sudo apt update && sudo apt upgrade -y
matrixon db vacuum --config /etc/matrixon/matrixon.toml
sudo journalctl --vacuum-time=7d
find /var/lib/matrixon/media -type f -mtime +90 -delete
```

---

## 📞 Support Resources

- **Documentation**: [GitHub Wiki](https://github.com/arkSong/Matrixon/wiki)
- **Issues**: [GitHub Issues](https://github.com/arkSong/Matrixon/issues)
- **Matrix Room**: [#matrixon:matrix.org](https://matrix.to/#/#matrixon:matrix.org)
- **Email**: arksong2018@gmail.com

---

*For detailed configuration options and advanced topics, refer to the complete documentation.*

**Version**: 1.0 | **Last Updated**: January 2025
