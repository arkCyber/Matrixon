#!/bin/bash

# =============================================================================
# Matrixon Production Environment - Deployment Script
# =============================================================================
#
# Project: Matrixon - Ultra High Performance Matrix NextServer
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 0.11.0-alpha
#
# Description:
#   Deployment script for Matrixon production environment
#   Sets up all required services and configurations
#
# =============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${BOLD}${GREEN}"
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║              MATRIXON PRODUCTION ENVIRONMENT                  ║"
echo "║                    DEPLOYMENT SCRIPT                          ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Function to print status
print_status() {
    echo -e "${CYAN}🔧 $1${NC}"
}

# Function to print success
print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

# Function to print error
print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_status "Creating necessary directories..."

# Create required directories
mkdir -p logs config ssl nginx/html grafana/dashboards grafana/datasources media

print_success "Directories created"

print_status "Generating self-signed SSL certificates..."

# Generate self-signed SSL certificates for testing
if [ ! -f ssl/matrixon.key ] || [ ! -f ssl/matrixon.crt ]; then
    openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
        -keyout ssl/matrixon.key \
        -out ssl/matrixon.crt \
        -subj "/C=US/ST=State/L=City/O=Matrixon/CN=localhost" 2>/dev/null || {
        print_error "Failed to generate SSL certificates. OpenSSL might not be installed."
        print_status "Continuing without SSL certificates..."
    }
    
    if [ -f ssl/matrixon.key ] && [ -f ssl/matrixon.crt ]; then
        print_success "SSL certificates generated"
    fi
else
    print_success "SSL certificates already exist"
fi

print_status "Creating simple index page..."

# Create a simple index page
cat > nginx/html/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Matrixon Matrix Server</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; background: #f4f4f4; }
        .container { background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        h1 { color: #333; }
        .status { padding: 10px; margin: 10px 0; border-radius: 4px; }
        .healthy { background: #d4edda; color: #155724; border: 1px solid #c3e6cb; }
        .info { background: #d1ecf1; color: #0c5460; border: 1px solid #bee5eb; }
        a { color: #007bff; text-decoration: none; }
        a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Matrixon Matrix Server</h1>
        <div class="status healthy">
            <strong>Status:</strong> Running in Production Environment
        </div>
        <div class="status info">
            <strong>Version:</strong> 0.11.0-alpha
        </div>
        
        <h2>Available Services</h2>
        <ul>
            <li><a href="/_matrix/client/versions">Matrix Client API</a></li>
            <li><a href="/health">Health Check</a></li>
            <li><a href="/metrics">Metrics</a></li>
            <li><a href="http://localhost:3001" target="_blank">Grafana Dashboard</a></li>
            <li><a href="http://localhost:9090" target="_blank">Prometheus</a></li>
        </ul>
        
        <h2>Documentation</h2>
        <ul>
            <li><a href="https://matrix.org/docs/" target="_blank">Matrix Protocol Documentation</a></li>
            <li><a href="https://github.com/arksong2018/Matrixon" target="_blank">Matrixon GitHub Repository</a></li>
        </ul>
    </div>
</body>
</html>
EOF

print_success "Index page created"

print_status "Setting up Grafana datasources..."

# Create Grafana datasource configuration
cat > grafana/datasources/datasources.yaml << 'EOF'
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    jsonData:
      timeInterval: "5s"
EOF

print_success "Grafana datasources configured"

print_status "Checking Docker installation..."

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    print_error "Docker is not installed or not in PATH"
    exit 1
fi

if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    print_error "Docker Compose is not installed or not in PATH"
    exit 1
fi

print_success "Docker and Docker Compose are available"

print_status "Building and starting services..."

# Build and start services
if docker compose version &> /dev/null; then
    DOCKER_COMPOSE_CMD="docker compose"
else
    DOCKER_COMPOSE_CMD="docker-compose"
fi

# Stop any existing services
$DOCKER_COMPOSE_CMD down 2>/dev/null || true

# Build and start services
$DOCKER_COMPOSE_CMD up -d --build

print_success "Services started"

print_status "Waiting for services to be healthy..."

# Wait for services to be healthy
sleep 30

# Check service health
print_status "Checking service health..."

services=("matrixon-postgres" "matrixon-redis" "matrixon-ipfs" "matrixon-prometheus" "matrixon-grafana" "matrixon-nginx")

for service in "${services[@]}"; do
    max_attempts=10
    attempt=0
    while [ $attempt -lt $max_attempts ]; do
        health_status=$(docker inspect --format='{{.State.Health.Status}}' "$service" 2>/dev/null || echo "unknown")
        if [ "$health_status" == "healthy" ]; then
            print_success "$service is healthy"
            break
        elif [ "$health_status" == "starting" ]; then
            print_status "$service is starting... (attempt $((attempt+1))/$max_attempts)"
            sleep 5
            ((attempt++))
        else
            print_status "$service status: $health_status (attempt $((attempt+1))/$max_attempts)"
            sleep 5
            ((attempt++))
        fi
    done
    
    if [ $attempt -eq $max_attempts ]; then
        print_error "$service failed to become healthy"
    fi
done

print_status "Deployment completed!"

echo -e "\n${BOLD}🎉 Matrixon Production Environment Deployed Successfully!${NC}\n"

echo -e "${CYAN}📊 Service URLs:${NC}"
echo -e "  • Matrixon API:    http://localhost:6167"
echo -e "  • Nginx Proxy:     http://localhost:80"
echo -e "  • Grafana:         http://localhost:3001 (admin/admin_change_me)"
echo -e "  • Prometheus:      http://localhost:9090"
echo -e "  • IPFS Gateway:    http://localhost:8080"
echo -e "  • IPFS API:        http://localhost:5001"

echo -e "\n${CYAN}🔧 Management Commands:${NC}"
echo -e "  • View logs:       $DOCKER_COMPOSE_CMD logs -f"
echo -e "  • Stop services:   $DOCKER_COMPOSE_CMD down"
echo -e "  • Restart:         $DOCKER_COMPOSE_CMD restart"

echo -e "\n${CYAN}🧪 Testing:${NC}"
echo -e "  • Run tests:       ./production_environment_test.sh"
echo -e "  • Quick test:      ./production_environment_test.sh --quick"
echo -e "  • API test:        ./production_environment_test.sh --api"

echo -e "\n${GREEN}Ready for production testing! 🚀${NC}" 
