#!/bin/bash

##
# Multi-Instance Deployment Script for matrixon Matrix Server
# 
# Deploys 8 matrixon instances for handling 20,000+ concurrent connections
# Each instance handles ~25,000 connections
# 
# @author: Matrix Server Performance Team
# @date: 2024-01-01
# @version: 2.0.0
##

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
INSTANCES=8
BASE_PORT=8001
ADMIN_PORT=6167
METRICS_BASE_PORT=9090
matrixon_BINARY="./target/release/matrixon"
CONFIG_TEMPLATE="configs/high_performance.toml"
LOG_DIR="/var/log/matrixon"
PID_DIR="/var/run/matrixon"
DATA_DIR="/var/lib/matrixon"

# Hardware validation
MIN_CORES=16
MIN_MEMORY_GB=64
MIN_DISK_GB=100

# Function definitions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Validate system requirements
validate_system() {
    log_info "Validating system requirements for 200k concurrent connections..."
    
    # Check CPU cores
    local cores=$(nproc)
    if [ $cores -lt $MIN_CORES ]; then
        log_error "Insufficient CPU cores: $cores (minimum: $MIN_CORES)"
        exit 1
    fi
    log_success "CPU cores: $cores (✓)"
    
    # Check memory
    local memory_gb=$(free -g | awk 'NR==2{printf "%.0f", $2}')
    if [ $memory_gb -lt $MIN_MEMORY_GB ]; then
        log_error "Insufficient memory: ${memory_gb}GB (minimum: ${MIN_MEMORY_GB}GB)"
        exit 1
    fi
    log_success "Memory: ${memory_gb}GB (✓)"
    
    # Check disk space
    local disk_gb=$(df / | awk 'NR==2{printf "%.0f", $4/1024/1024}')
    if [ $disk_gb -lt $MIN_DISK_GB ]; then
        log_error "Insufficient disk space: ${disk_gb}GB (minimum: ${MIN_DISK_GB}GB)"
        exit 1
    fi
    log_success "Disk space: ${disk_gb}GB (✓)"
    
    # Check file descriptor limits
    local ulimit_files=$(ulimit -n)
    if [ $ulimit_files -lt 1048576 ]; then
        log_warning "File descriptor limit may be too low: $ulimit_files (recommended: 2097152)"
        log_info "Run: echo 'matrixon soft nofile 2097152' >> /etc/security/limits.conf"
    fi
    
    log_success "System validation completed"
}

# Create necessary directories
setup_directories() {
    log_info "Setting up directory structure..."
    
    sudo mkdir -p "$LOG_DIR" "$PID_DIR" "$DATA_DIR"
    sudo chown -R $(whoami):$(whoami) "$LOG_DIR" "$PID_DIR" "$DATA_DIR"
    
    for i in $(seq 1 $INSTANCES); do
        mkdir -p "${DATA_DIR}/instance_${i}"
        mkdir -p "${LOG_DIR}/instance_${i}"
    done
    
    log_success "Directory structure created"
}

# Generate instance configurations
generate_configs() {
    log_info "Generating configuration files for $INSTANCES instances..."
    
    if [ ! -f "$CONFIG_TEMPLATE" ]; then
        log_error "Template configuration not found: $CONFIG_TEMPLATE"
        exit 1
    fi
    
    for i in $(seq 1 $INSTANCES); do
        local port=$((BASE_PORT + i - 1))
        local metrics_port=$((METRICS_BASE_PORT + i - 1))
        local config_file="configs/instance_${i}.toml"
        
        # Copy template and modify for this instance
        cp "$CONFIG_TEMPLATE" "$config_file"
        
        # Update port and instance-specific settings
        sed -i "s/port = 8008/port = $port/" "$config_file"
        sed -i "s/metrics_port = 9090/metrics_port = $metrics_port/" "$config_file"
        sed -i "s/path = \"\/var\/lib\/matrixon\"/path = \"${DATA_DIR}\/instance_${i}\"/" "$config_file"
        
        # Adjust per-instance limits (divide total by instances)
        local max_concurrent=$((200000 / INSTANCES))
        local pdu_cache=$((20000000 / INSTANCES))
        local db_cache=$((32768 / INSTANCES))
        local pool_size=$((2000 / INSTANCES))
        
        sed -i "s/max_concurrent_requests = 200000/max_concurrent_requests = $max_concurrent/" "$config_file"
        sed -i "s/pdu_cache_capacity = 20000000/pdu_cache_capacity = $pdu_cache/" "$config_file"
        sed -i "s/db_cache_capacity_mb = 32768.0/db_cache_capacity_mb = $db_cache.0/" "$config_file"
        sed -i "s/connection_pool_size = 2000/connection_pool_size = $pool_size/" "$config_file"
        
        log_info "Generated config for instance $i (port: $port, max_conn: $max_concurrent)"
    done
    
    log_success "Configuration files generated"
}

# Build matrixon with optimizations
build_matrixon() {
    log_info "Building matrixon with high-performance optimizations..."
    
    export RUSTFLAGS="-C target-cpu=native -C opt-level=3"
    export CARGO_PROFILE_RELEASE_LTO=true
    export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
    
    if ! cargo build --release --features backend_postgresql; then
        log_error "Build failed"
        exit 1
    fi
    
    log_success "Build completed successfully"
}

# Start database services
start_database_services() {
    log_info "Starting database services..."
    
    # Start PostgreSQL if not running
    if ! systemctl is-active --quiet postgresql; then
        log_info "Starting PostgreSQL..."
        sudo systemctl start postgresql
    fi
    
    # Start Redis cluster if not running
    if ! pgrep redis-server > /dev/null; then
        log_info "Starting Redis cluster..."
        # This assumes Redis cluster is configured separately
        sudo systemctl start redis
    fi
    
    log_success "Database services started"
}

# Start matrixon instances
start_instances() {
    log_info "Starting $INSTANCES matrixon instances..."
    
    for i in $(seq 1 $INSTANCES); do
        local port=$((BASE_PORT + i - 1))
        local config_file="configs/instance_${i}.toml"
        local log_file="${LOG_DIR}/instance_${i}/matrixon.log"
        local pid_file="${PID_DIR}/instance_${i}.pid"
        
        # Set environment variables for this instance
        export matrixon_CONFIG="$config_file"
        export matrixon_LOG_LEVEL="info"
        export MALLOC_CONF="background_thread:true,metadata_thp:auto,dirty_decay_ms:30000"
        
        # Start instance in background
        log_info "Starting instance $i on port $port..."
        
        nohup nice -n -10 "$matrixon_BINARY" > "$log_file" 2>&1 &
        local pid=$!
        echo $pid > "$pid_file"
        
        # Wait a moment and check if it started successfully
        sleep 2
        if ! kill -0 $pid 2>/dev/null; then
            log_error "Instance $i failed to start"
            exit 1
        fi
        
        log_success "Instance $i started (PID: $pid, Port: $port)"
    done
    
    log_success "All instances started successfully"
}

# Verify instances are healthy
verify_instances() {
    log_info "Verifying instance health..."
    
    sleep 5  # Give instances time to fully start
    
    for i in $(seq 1 $INSTANCES); do
        local port=$((BASE_PORT + i - 1))
        local metrics_port=$((METRICS_BASE_PORT + i - 1))
        
        # Check if port is listening
        if ! nc -z localhost $port; then
            log_error "Instance $i not responding on port $port"
            exit 1
        fi
        
        # Check metrics endpoint
        if ! curl -s "http://localhost:$metrics_port/metrics" > /dev/null; then
            log_warning "Metrics not available for instance $i on port $metrics_port"
        fi
        
        log_success "Instance $i healthy (port: $port)"
    done
    
    log_success "All instances verified healthy"
}

# Configure load balancer
configure_load_balancer() {
    log_info "Configuring Nginx load balancer..."
    
    if ! command -v nginx >/dev/null 2>&1; then
        log_warning "Nginx not found. Please install and configure manually."
        return
    fi
    
    # Backup existing config
    if [ -f /etc/nginx/nginx.conf ]; then
        sudo cp /etc/nginx/nginx.conf /etc/nginx/nginx.conf.backup
    fi
    
    # Copy our optimized configuration
    sudo cp configs/nginx-load-balancer.conf /etc/nginx/nginx.conf
    
    # Test configuration
    if ! sudo nginx -t; then
        log_error "Nginx configuration test failed"
        sudo mv /etc/nginx/nginx.conf.backup /etc/nginx/nginx.conf 2>/dev/null || true
        return
    fi
    
    # Reload Nginx
    sudo systemctl reload nginx || sudo systemctl start nginx
    
    log_success "Nginx load balancer configured and reloaded"
}

# Generate monitoring script
generate_monitoring() {
    log_info "Generating monitoring script..."
    
    cat > "monitor_instances.sh" << 'EOF'
#!/bin/bash

# Monitor all matrixon instances
INSTANCES=8
BASE_PORT=8001
METRICS_BASE_PORT=9090
PID_DIR="/var/run/matrixon"

echo "=== matrixon Multi-Instance Status ==="
echo "Timestamp: $(date)"
echo ""

total_connections=0
total_memory=0

for i in $(seq 1 $INSTANCES); do
    port=$((BASE_PORT + i - 1))
    metrics_port=$((METRICS_BASE_PORT + i - 1))
    pid_file="${PID_DIR}/instance_${i}.pid"
    
    if [ -f "$pid_file" ]; then
        pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            # Get memory usage
            memory=$(ps -p $pid -o rss= | awk '{print $1/1024}')
            total_memory=$(echo "$total_memory + $memory" | bc)
            
            echo "Instance $i: RUNNING (PID: $pid, Port: $port, Memory: ${memory}MB)"
        else
            echo "Instance $i: STOPPED (stale PID file)"
        fi
    else
        echo "Instance $i: STOPPED (no PID file)"
    fi
done

echo ""
echo "Total Memory Usage: ${total_memory}MB"
echo "Load Balancer: $(systemctl is-active nginx 2>/dev/null || echo 'unknown')"
echo "PostgreSQL: $(systemctl is-active postgresql 2>/dev/null || echo 'unknown')"
echo "Redis: $(systemctl is-active redis 2>/dev/null || echo 'unknown')"
EOF

    chmod +x "monitor_instances.sh"
    log_success "Monitoring script created: monitor_instances.sh"
}

# Main deployment function
main() {
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║              matrixon MULTI-INSTANCE DEPLOYMENT               ║${NC}"
    echo -e "${CYAN}║                 20,000+ Concurrent Connections              ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    log_info "Target: $INSTANCES instances handling 20,000+ concurrent connections"
    log_info "Port range: $BASE_PORT-$((BASE_PORT + INSTANCES - 1))"
    log_info "Each instance: ~$((200000 / INSTANCES)) connections"
    echo ""
    
    # Run deployment steps
    validate_system
    setup_directories
    build_matrixon
    generate_configs
    start_database_services
    start_instances
    verify_instances
    configure_load_balancer
    generate_monitoring
    
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                   DEPLOYMENT SUCCESSFUL!                     ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    log_success "matrixon multi-instance deployment completed"
    log_info "Load balancer endpoint: http://localhost (Nginx)"
    log_info "Instance monitoring: ./monitor_instances.sh"
    log_info "Logs directory: $LOG_DIR"
    
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "1. Configure your domain DNS to point to this server"
    echo "2. Set up SSL certificates for HTTPS"
    echo "3. Configure federation settings"
    echo "4. Run load tests to validate performance"
    echo "5. Set up monitoring dashboards (Grafana)"
}

# Handle signals for graceful shutdown
cleanup() {
    echo ""
    log_info "Received shutdown signal..."
    
    for i in $(seq 1 $INSTANCES); do
        local pid_file="${PID_DIR}/instance_${i}.pid"
        if [ -f "$pid_file" ]; then
            local pid=$(cat "$pid_file")
            if kill -0 $pid 2>/dev/null; then
                log_info "Stopping instance $i (PID: $pid)..."
                kill $pid
            fi
        fi
    done
    
    log_info "Shutdown complete"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Check if running as root (not recommended)
if [ "$EUID" -eq 0 ]; then
    log_warning "Running as root is not recommended for production deployment"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Run main deployment
main "$@" 
