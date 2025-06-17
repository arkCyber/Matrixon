#!/bin/bash

# Matrixon Matrix Server - SSL/TLS Certificate Setup
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 1.0
# Purpose: Generate and configure SSL/TLS certificates for production

set -euo pipefail

# Configuration variables
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="/var/log/matrixon-ssl.log"
CERT_DIR="/etc/ssl/matrixon"
BACKUP_DIR="/etc/ssl/backup"

# Load environment variables
if [ -f "$SCRIPT_DIR/production.env" ]; then
    source "$SCRIPT_DIR/production.env"
else
    echo "⚠️ Warning: production.env not found, using default values"
fi

# Default values if not set in environment
MATRIXON_DOMAIN=${MATRIXON_DOMAIN:-"matrixon.example.com"}
CERT_COUNTRY=${CERT_COUNTRY:-"US"}
CERT_STATE=${CERT_STATE:-"California"}
CERT_CITY=${CERT_CITY:-"San Francisco"}
CERT_ORG=${CERT_ORG:-"Matrixon Organization"}
CERT_OU=${CERT_OU:-"IT Department"}
CERT_EMAIL=${CERT_EMAIL:-"admin@matrixon.example.com"}

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [SSL] $1" | tee -a "$LOG_FILE"
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

# Install required packages
install_packages() {
    log "🔧 Installing SSL/TLS packages..."
    
    if command -v apt-get >/dev/null 2>&1; then
        apt-get update
        apt-get install -y openssl certbot python3-certbot-nginx apache2-utils
    elif command -v yum >/dev/null 2>&1; then
        yum install -y openssl certbot python3-certbot-nginx httpd-tools
    elif command -v dnf >/dev/null 2>&1; then
        dnf install -y openssl certbot python3-certbot-nginx httpd-tools
    else
        error_exit "Unsupported operating system"
    fi
    
    log "✅ Packages installed successfully"
}

# Create directory structure
create_directories() {
    log "🔧 Creating certificate directories..."
    
    mkdir -p "$CERT_DIR"/{certs,private,csr,ca}
    mkdir -p "$BACKUP_DIR"
    
    # Set secure permissions
    chmod 755 "$CERT_DIR"
    chmod 700 "$CERT_DIR/private"
    chmod 755 "$CERT_DIR/certs"
    chmod 755 "$CERT_DIR/csr"
    chmod 755 "$CERT_DIR/ca"
    
    log "✅ Directory structure created"
}

# Generate CA certificate (for development/testing)
generate_ca_cert() {
    log "🔧 Generating CA certificate..."
    
    local ca_key="$CERT_DIR/ca/ca-key.pem"
    local ca_cert="$CERT_DIR/ca/ca-cert.pem"
    
    # Generate CA private key
    openssl genrsa -out "$ca_key" 4096
    chmod 600 "$ca_key"
    
    # Generate CA certificate
    openssl req -new -x509 -days 3650 -key "$ca_key" -out "$ca_cert" -subj \
        "/C=$CERT_COUNTRY/ST=$CERT_STATE/L=$CERT_CITY/O=$CERT_ORG CA/OU=$CERT_OU/CN=Matrixon CA/emailAddress=$CERT_EMAIL"
    
    log "✅ CA certificate generated"
}

# Generate server certificate
generate_server_cert() {
    log "🔧 Generating server certificate..."
    
    local server_key="$CERT_DIR/private/matrixon.key"
    local server_csr="$CERT_DIR/csr/matrixon.csr"
    local server_cert="$CERT_DIR/certs/matrixon.crt"
    local ca_key="$CERT_DIR/ca/ca-key.pem"
    local ca_cert="$CERT_DIR/ca/ca-cert.pem"
    
    # Generate server private key
    openssl genrsa -out "$server_key" 4096
    chmod 600 "$server_key"
    
    # Create certificate configuration
    cat > "$CERT_DIR/server.conf" << EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = $CERT_COUNTRY
ST = $CERT_STATE
L = $CERT_CITY
O = $CERT_ORG
OU = $CERT_OU
CN = $MATRIXON_DOMAIN
emailAddress = $CERT_EMAIL

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = $MATRIXON_DOMAIN
DNS.2 = *.$MATRIXON_DOMAIN
DNS.3 = localhost
IP.1 = 127.0.0.1
EOF

    # Generate certificate signing request
    openssl req -new -key "$server_key" -out "$server_csr" -config "$CERT_DIR/server.conf"
    
    # Generate server certificate signed by CA
    openssl x509 -req -in "$server_csr" -CA "$ca_cert" -CAkey "$ca_key" -CAcreateserial \
        -out "$server_cert" -days 365 -extensions v3_req -extfile "$CERT_DIR/server.conf"
    
    # Create certificate bundle
    cat "$server_cert" "$ca_cert" > "$CERT_DIR/certs/matrixon-bundle.crt"
    
    log "✅ Server certificate generated"
}

# Setup Let's Encrypt (for production)
setup_letsencrypt() {
    log "🔧 Setting up Let's Encrypt certificates..."
    
    # Check if domain is reachable
    if ! ping -c 1 "$MATRIXON_DOMAIN" >/dev/null 2>&1; then
        log "⚠️ Domain $MATRIXON_DOMAIN not reachable, skipping Let's Encrypt"
        return 0
    fi
    
    # Obtain certificate
    certbot certonly --nginx \
        --domains "$MATRIXON_DOMAIN" \
        --email "$CERT_EMAIL" \
        --agree-tos \
        --non-interactive \
        --expand
    
    if [ $? -eq 0 ]; then
        # Link Let's Encrypt certificates
        ln -sf "/etc/letsencrypt/live/$MATRIXON_DOMAIN/fullchain.pem" "$CERT_DIR/certs/letsencrypt.crt"
        ln -sf "/etc/letsencrypt/live/$MATRIXON_DOMAIN/privkey.pem" "$CERT_DIR/private/letsencrypt.key"
        
        # Setup auto-renewal
        (crontab -l 2>/dev/null; echo "0 12 * * * /usr/bin/certbot renew --quiet") | crontab -
        
        log "✅ Let's Encrypt certificates obtained and auto-renewal configured"
    else
        log "⚠️ Let's Encrypt certificate generation failed, using self-signed certificates"
    fi
}

# Generate Diffie-Hellman parameters
generate_dhparam() {
    log "🔧 Generating Diffie-Hellman parameters..."
    
    local dhparam_file="$CERT_DIR/certs/dhparam.pem"
    
    # Generate strong DH parameters (2048-bit for faster generation, 4096-bit for maximum security)
    openssl dhparam -out "$dhparam_file" 2048
    
    log "✅ Diffie-Hellman parameters generated"
}

# Generate client certificates for admin access
generate_client_certs() {
    log "🔧 Generating client certificates for admin access..."
    
    local client_key="$CERT_DIR/private/admin-client.key"
    local client_csr="$CERT_DIR/csr/admin-client.csr"
    local client_cert="$CERT_DIR/certs/admin-client.crt"
    local client_p12="$CERT_DIR/certs/admin-client.p12"
    local ca_key="$CERT_DIR/ca/ca-key.pem"
    local ca_cert="$CERT_DIR/ca/ca-cert.pem"
    
    # Generate client private key
    openssl genrsa -out "$client_key" 2048
    chmod 600 "$client_key"
    
    # Generate client certificate signing request
    openssl req -new -key "$client_key" -out "$client_csr" -subj \
        "/C=$CERT_COUNTRY/ST=$CERT_STATE/L=$CERT_CITY/O=$CERT_ORG/OU=$CERT_OU/CN=admin-client/emailAddress=$CERT_EMAIL"
    
    # Generate client certificate
    openssl x509 -req -in "$client_csr" -CA "$ca_cert" -CAkey "$ca_key" -CAcreateserial \
        -out "$client_cert" -days 365 -extensions usr_cert
    
    # Create PKCS#12 bundle for easy import
    openssl pkcs12 -export -out "$client_p12" -inkey "$client_key" -in "$client_cert" -certfile "$ca_cert" -passout pass:admin123
    
    log "✅ Client certificates generated"
}

# Setup NGINX authentication
setup_nginx_auth() {
    log "🔧 Setting up NGINX authentication..."
    
    # Create htpasswd file for metrics access
    echo "Creating metrics user (password: metrics123)"
    htpasswd -cb /etc/nginx/htpasswd metrics metrics123
    
    # Create admin htpasswd file
    echo "Creating admin user (password: admin123)"
    htpasswd -cb /etc/nginx/admin_htpasswd admin admin123
    
    log "✅ NGINX authentication configured"
}

# Verify certificates
verify_certificates() {
    log "🔧 Verifying certificates..."
    
    local server_cert="$CERT_DIR/certs/matrixon.crt"
    local server_key="$CERT_DIR/private/matrixon.key"
    local ca_cert="$CERT_DIR/ca/ca-cert.pem"
    
    # Verify server certificate
    if openssl x509 -in "$server_cert" -text -noout >/dev/null 2>&1; then
        log "✅ Server certificate is valid"
        
        # Show certificate details
        log "📄 Certificate details:"
        openssl x509 -in "$server_cert" -subject -issuer -dates -noout | while read line; do
            log "   $line"
        done
    else
        error_exit "Server certificate is invalid"
    fi
    
    # Verify private key matches certificate
    local cert_hash=$(openssl x509 -noout -modulus -in "$server_cert" | openssl md5)
    local key_hash=$(openssl rsa -noout -modulus -in "$server_key" | openssl md5)
    
    if [ "$cert_hash" = "$key_hash" ]; then
        log "✅ Private key matches certificate"
    else
        error_exit "Private key does not match certificate"
    fi
    
    # Verify certificate chain
    if openssl verify -CAfile "$ca_cert" "$server_cert" >/dev/null 2>&1; then
        log "✅ Certificate chain is valid"
    else
        log "⚠️ Certificate chain verification failed (expected for self-signed)"
    fi
}

# Setup certificate monitoring
setup_cert_monitoring() {
    log "🔧 Setting up certificate monitoring..."
    
    # Create certificate expiry check script
    cat > /usr/local/bin/check-cert-expiry.sh << 'EOF'
#!/bin/bash

CERT_FILE="/etc/ssl/matrixon/certs/matrixon.crt"
DAYS_WARNING=30
LOG_FILE="/var/log/matrixon-ssl.log"

if [ ! -f "$CERT_FILE" ]; then
    echo "$(date '+%Y-%m-%d %H:%M:%S') [SSL] ❌ Certificate file not found: $CERT_FILE" >> "$LOG_FILE"
    exit 1
fi

# Get certificate expiration date
EXPIRY_DATE=$(openssl x509 -in "$CERT_FILE" -noout -enddate | cut -d= -f2)
EXPIRY_EPOCH=$(date -d "$EXPIRY_DATE" +%s)
CURRENT_EPOCH=$(date +%s)
DAYS_UNTIL_EXPIRY=$(( (EXPIRY_EPOCH - CURRENT_EPOCH) / 86400 ))

if [ $DAYS_UNTIL_EXPIRY -le $DAYS_WARNING ]; then
    echo "$(date '+%Y-%m-%d %H:%M:%S') [SSL] ⚠️ Certificate expires in $DAYS_UNTIL_EXPIRY days!" >> "$LOG_FILE"
    # Send alert (implement your notification method here)
    # curl -X POST -H 'Content-type: application/json' --data '{"text":"SSL Certificate expiring in '$DAYS_UNTIL_EXPIRY' days!"}' $SLACK_WEBHOOK_URL
else
    echo "$(date '+%Y-%m-%d %H:%M:%S') [SSL] ✅ Certificate valid for $DAYS_UNTIL_EXPIRY more days" >> "$LOG_FILE"
fi
EOF

    chmod +x /usr/local/bin/check-cert-expiry.sh
    
    # Add to crontab for daily checks
    (crontab -l 2>/dev/null; echo "0 9 * * * /usr/local/bin/check-cert-expiry.sh") | crontab -
    
    log "✅ Certificate monitoring configured"
}

# Update Docker Compose with SSL configuration
update_docker_compose() {
    log "🔧 Updating Docker Compose with SSL configuration..."
    
    # Backup original file
    cp "$SCRIPT_DIR/docker-compose.postgresql.yml" "$BACKUP_DIR/docker-compose.postgresql.yml.backup"
    
    # Update NGINX service to use SSL certificates
    sed -i '/nginx:/a\    volumes:\n      - /etc/ssl/matrixon:/etc/ssl/matrixon:ro\n      - ./nginx-ssl.conf:/etc/nginx/nginx.conf:ro' "$SCRIPT_DIR/docker-compose.postgresql.yml"
    
    # Add environment file reference
    sed -i '/services:/a\  env_file:\n    - production.env' "$SCRIPT_DIR/docker-compose.postgresql.yml"
    
    log "✅ Docker Compose updated"
}

# Main execution
main() {
    log "🚀 Starting SSL/TLS certificate setup..."
    
    check_root
    install_packages
    create_directories
    generate_ca_cert
    generate_server_cert
    generate_dhparam
    generate_client_certs
    setup_nginx_auth
    verify_certificates
    setup_cert_monitoring
    
    # Try Let's Encrypt for production
    if [[ "${ENVIRONMENT:-development}" == "production" ]]; then
        setup_letsencrypt
    fi
    
    update_docker_compose
    
    log "🎉 SSL/TLS certificate setup completed successfully!"
    
    # Display summary
    echo
    echo "========================================"
    echo "MATRIXON SSL/TLS CONFIGURATION SUMMARY"
    echo "========================================"
    echo "✅ CA Certificate: $CERT_DIR/ca/ca-cert.pem"
    echo "✅ Server Certificate: $CERT_DIR/certs/matrixon.crt"
    echo "✅ Server Private Key: $CERT_DIR/private/matrixon.key"
    echo "✅ Certificate Bundle: $CERT_DIR/certs/matrixon-bundle.crt"
    echo "✅ DH Parameters: $CERT_DIR/certs/dhparam.pem"
    echo "✅ Client Certificate: $CERT_DIR/certs/admin-client.crt"
    echo "✅ Client PKCS#12: $CERT_DIR/certs/admin-client.p12"
    echo "✅ NGINX Auth Files: /etc/nginx/htpasswd, /etc/nginx/admin_htpasswd"
    echo "========================================"
    echo
    echo "📋 NEXT STEPS:"
    echo "1. Update production.env with your domain and details"
    echo "2. Copy CA certificate to client systems for trust"
    echo "3. Import admin-client.p12 into browser (password: admin123)"
    echo "4. Test SSL configuration: openssl s_client -connect $MATRIXON_DOMAIN:443"
    echo "5. Monitor certificate expiry: /usr/local/bin/check-cert-expiry.sh"
    echo "========================================"
    echo
    echo "🔐 DEFAULT PASSWORDS (CHANGE IMMEDIATELY):"
    echo "   - Metrics user: metrics/metrics123"
    echo "   - Admin user: admin/admin123"
    echo "   - Client P12: admin123"
    echo "========================================"
}

# Execute main function
main "$@" 
