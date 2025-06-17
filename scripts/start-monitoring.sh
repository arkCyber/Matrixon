#!/bin/bash

# Matrixon Matrix Server - Monitoring Stack Startup Script
# Author: arkSong (arksong2018@gmail.com)
# Date: 2024-12-19
# Version: 1.0
# Purpose: Start comprehensive monitoring stack with Prometheus, Grafana, and AlertManager

set -euo pipefail

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.postgresql.yml}"
LOG_DIR="${LOG_DIR:-./logs/monitoring}"
ENV_FILE="${ENV_FILE:-production.env}"

# Create necessary directories
mkdir -p "$LOG_DIR"
mkdir -p "./grafana/provisioning/datasources"
mkdir -p "./grafana/provisioning/dashboards"
mkdir -p "./monitoring"

# Logging function
log() {
    local level="$1"
    local message="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo -e "[$timestamp] [$level] $message"
}

# Progress indicator
show_progress() {
    local duration=$1
    local message="$2"
    echo -n "$message"
    for ((i=1; i<=duration; i++)); do
        echo -n "."
        sleep 1
    done
    echo " ✅"
}

# Check prerequisites
check_prerequisites() {
    log "INFO" "${BLUE}🔍 Checking prerequisites...${NC}"
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        log "ERROR" "${RED}❌ Docker is not installed${NC}"
        exit 1
    fi
    
    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        log "ERROR" "${RED}❌ Docker Compose is not installed${NC}"
        exit 1
    fi
    
    # Check configuration files
    local required_files=(
        "$COMPOSE_FILE"
        "prometheus-simplified.yml"
        "alertmanager-simplified.yml"
        "alert-rules-enhanced.yml"
        "grafana/provisioning/datasources/prometheus.yml"
        "grafana/provisioning/dashboards/dashboard.yml"
        "grafana/provisioning/dashboards/matrixon-overview.json"
    )
    
    for file in "${required_files[@]}"; do
        if [[ ! -f "$file" ]]; then
            log "WARNING" "${YELLOW}⚠️ Missing configuration file: $file${NC}"
        else
            log "INFO" "${GREEN}✅ Found: $file${NC}"
        fi
    done
    
    log "INFO" "${GREEN}✅ Prerequisites check completed${NC}"
}

# Load environment variables
load_environment() {
    log "INFO" "${BLUE}🔧 Loading environment configuration...${NC}"
    
    if [[ -f "$ENV_FILE" ]]; then
        # Export variables from env file
        set -a
        source "$ENV_FILE"
        set +a
        log "INFO" "${GREEN}✅ Loaded environment from $ENV_FILE${NC}"
    else
        log "WARNING" "${YELLOW}⚠️ Environment file $ENV_FILE not found, using defaults${NC}"
    fi
    
    # Set default values if not provided
    export POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-matrixon_secure_password_change_me}"
    export REDIS_PASSWORD="${REDIS_PASSWORD:-matrixon_redis_password_change_me}"
    export GRAFANA_ADMIN_PASSWORD="${GRAFANA_ADMIN_PASSWORD:-admin_change_me}"
    
    log "INFO" "${GREEN}✅ Environment variables configured${NC}"
}

# Validate configuration files
validate_configs() {
    log "INFO" "${BLUE}📋 Validating configuration files...${NC}"
    
    # Validate Prometheus config
    if command -v promtool &> /dev/null; then
        if promtool check config prometheus-simplified.yml &> /dev/null; then
            log "INFO" "${GREEN}✅ Prometheus configuration is valid${NC}"
        else
            log "ERROR" "${RED}❌ Prometheus configuration is invalid${NC}"
            return 1
        fi
    else
        log "WARNING" "${YELLOW}⚠️ promtool not found, skipping Prometheus validation${NC}"
    fi
    
    # Validate AlertManager config
    if command -v amtool &> /dev/null; then
        if amtool check-config alertmanager-simplified.yml &> /dev/null; then
            log "INFO" "${GREEN}✅ AlertManager configuration is valid${NC}"
        else
            log "ERROR" "${RED}❌ AlertManager configuration is invalid${NC}"
            return 1
        fi
    else
        log "WARNING" "${YELLOW}⚠️ amtool not found, skipping AlertManager validation${NC}"
    fi
    
    log "INFO" "${GREEN}✅ Configuration validation completed${NC}"
}

# Start monitoring stack
start_monitoring_stack() {
    log "INFO" "${BLUE}🚀 Starting Matrixon monitoring stack...${NC}"
    
    # Pull latest images
    log "INFO" "${CYAN}📥 Pulling latest Docker images...${NC}"
    docker-compose -f "$COMPOSE_FILE" pull prometheus alertmanager grafana postgres-exporter redis-exporter node-exporter cadvisor elasticsearch kibana
    
    # Start core infrastructure first
    log "INFO" "${CYAN}🔧 Starting core infrastructure...${NC}"
    docker-compose -f "$COMPOSE_FILE" up -d postgres redis elasticsearch
    
    # Wait for databases to be ready
    show_progress 10 "${CYAN}⏳ Waiting for databases to initialize"
    
    # Start exporters
    log "INFO" "${CYAN}📊 Starting metric exporters...${NC}"
    docker-compose -f "$COMPOSE_FILE" up -d postgres-exporter redis-exporter node-exporter cadvisor
    
    # Wait for exporters to be ready
    show_progress 5 "${CYAN}⏳ Waiting for exporters to start"
    
    # Start Prometheus
    log "INFO" "${CYAN}📈 Starting Prometheus...${NC}"
    docker-compose -f "$COMPOSE_FILE" up -d prometheus
    
    # Wait for Prometheus to be ready
    show_progress 10 "${CYAN}⏳ Waiting for Prometheus to start"
    
    # Start AlertManager
    log "INFO" "${CYAN}🚨 Starting AlertManager...${NC}"
    docker-compose -f "$COMPOSE_FILE" up -d alertmanager
    
    # Wait for AlertManager to be ready
    show_progress 5 "${CYAN}⏳ Waiting for AlertManager to start"
    
    # Start Grafana
    log "INFO" "${CYAN}📊 Starting Grafana...${NC}"
    docker-compose -f "$COMPOSE_FILE" up -d grafana
    
    # Wait for Grafana to be ready (extended time)
    show_progress 30 "${CYAN}⏳ Waiting for Grafana to initialize"
    
    # Start log processing stack
    log "INFO" "${CYAN}📝 Starting log processing...${NC}"
    docker-compose -f "$COMPOSE_FILE" up -d logstash filebeat kibana
    
    log "INFO" "${GREEN}✅ Monitoring stack started successfully${NC}"
}

# Verify services
verify_services() {
    log "INFO" "${BLUE}🔍 Verifying monitoring services...${NC}"
    
    local services=(
        "Prometheus:9090:/api/v1/status/config"
        "AlertManager:9093:/api/v1/status"
        "Grafana:3001:/api/health"
        "Node Exporter:9100:/metrics"
        "PostgreSQL Exporter:9187:/metrics"
        "Redis Exporter:9121:/metrics"
        "cAdvisor:8080:/metrics"
        "Elasticsearch:9200:/_cluster/health"
        "Kibana:5601:/api/status"
    )
    
    for service_info in "${services[@]}"; do
        IFS=':' read -r name port endpoint <<< "$service_info"
        
        if curl -sf "http://localhost:$port$endpoint" > /dev/null 2>&1; then
            log "INFO" "${GREEN}✅ $name is healthy (port $port)${NC}"
        else
            log "WARNING" "${YELLOW}⚠️ $name is not responding (port $port)${NC}"
        fi
    done
}

# Display access information
show_access_info() {
    log "INFO" "${BLUE}🌐 Monitoring Stack Access Information${NC}"
    echo
    echo -e "${CYAN}📊 Grafana Dashboard:${NC}"
    echo -e "   URL: ${GREEN}http://localhost:3001${NC}"
    echo -e "   Username: ${GREEN}admin${NC}"
    echo -e "   Password: ${GREEN}${GRAFANA_ADMIN_PASSWORD}${NC}"
    echo
    echo -e "${CYAN}📈 Prometheus:${NC}"
    echo -e "   URL: ${GREEN}http://localhost:9090${NC}"
    echo -e "   Targets: ${GREEN}http://localhost:9090/targets${NC}"
    echo
    echo -e "${CYAN}🚨 AlertManager:${NC}"
    echo -e "   URL: ${GREEN}http://localhost:9093${NC}"
    echo -e "   Alerts: ${GREEN}http://localhost:9093/#/alerts${NC}"
    echo
    echo -e "${CYAN}🔍 Kibana (Logs):${NC}"
    echo -e "   URL: ${GREEN}http://localhost:5601${NC}"
    echo
    echo -e "${CYAN}📊 Exporters:${NC}"
    echo -e "   Node Exporter: ${GREEN}http://localhost:9100/metrics${NC}"
    echo -e "   PostgreSQL Exporter: ${GREEN}http://localhost:9187/metrics${NC}"
    echo -e "   Redis Exporter: ${GREEN}http://localhost:9121/metrics${NC}"
    echo -e "   cAdvisor: ${GREEN}http://localhost:8080${NC}"
    echo
    echo -e "${CYAN}🔧 Quick Commands:${NC}"
    echo -e "   View logs: ${GREEN}docker-compose -f $COMPOSE_FILE logs -f [service]${NC}"
    echo -e "   Stop monitoring: ${GREEN}docker-compose -f $COMPOSE_FILE down${NC}"
    echo -e "   Restart service: ${GREEN}docker-compose -f $COMPOSE_FILE restart [service]${NC}"
    echo -e "   Resource monitor: ${GREEN}./monitoring/resource-monitor.sh${NC}"
    echo
}

# Make resource monitor executable
make_scripts_executable() {
    log "INFO" "${BLUE}🔧 Setting up monitoring scripts...${NC}"
    
    if [[ -f "monitoring/resource-monitor.sh" ]]; then
        chmod +x monitoring/resource-monitor.sh
        log "INFO" "${GREEN}✅ Made resource-monitor.sh executable${NC}"
    fi
    
    if [[ -f "start-monitoring.sh" ]]; then
        chmod +x start-monitoring.sh
        log "INFO" "${GREEN}✅ Made start-monitoring.sh executable${NC}"
    fi
}

# Main function
main() {
    echo -e "${GREEN}=================================================="
    echo -e "🔍 Matrixon Monitoring Stack Startup"
    echo -e "Author: arkSong (arksong2018@gmail.com)"
    echo -e "Date: $(date)"
    echo -e "==================================================${NC}"
    echo
    
    # Run startup sequence
    check_prerequisites
    echo
    load_environment
    echo
    validate_configs
    echo
    make_scripts_executable
    echo
    start_monitoring_stack
    echo
    verify_services
    echo
    show_access_info
    
    log "INFO" "${GREEN}🎉 Matrixon monitoring stack is ready!${NC}"
    echo
    echo -e "${CYAN}💡 Next steps:${NC}"
    echo -e "   1. Access Grafana dashboard to view metrics"
    echo -e "   2. Configure alert notification channels in AlertManager"
    echo -e "   3. Set up Matrixon server to expose metrics endpoints"
    echo -e "   4. Run ./monitoring/resource-monitor.sh for manual checks"
    echo
}

# Handle signals
trap 'echo; log "INFO" "📝 Monitoring startup interrupted"; exit 0' SIGTERM SIGINT

# Run main function
main "$@" 
