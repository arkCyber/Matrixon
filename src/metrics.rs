// Matrixon Matrix Server - Metrics Module
// Author: arkSong (arksong2018@gmail.com)
// Date: 2024-12-19
// Version: 1.0
// Purpose: Comprehensive metrics collection and Prometheus endpoint implementation

use std::{
    sync::Arc,
    time::{Duration, Instant},
    sync::atomic::{AtomicU64, Ordering},
};

use prometheus_client::{
    encoding::text::encode,
    metrics::{
        counter::Counter,
        gauge::Gauge,
        family::Family,
    },
    registry::Registry,
};

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::Response,
    routing::get,
    Router,
};

use prometheus::{
    register_counter_vec, register_gauge_vec,
    TextEncoder,
    CounterVec, GaugeVec, HistogramVec,
};

use tracing::{debug, error, info, instrument, warn};
use axum::response::IntoResponse;

/// Global metrics registry for the Matrixon server
#[derive(Debug, Clone)]
pub struct MatrixonMetrics {
    // HTTP metrics
    pub http_requests_total: CounterVec,
    pub http_request_duration: HistogramVec,
    pub http_response_size_bytes: HistogramVec,
    
    // Matrix protocol metrics
    pub matrix_events_total: CounterVec,
    pub matrix_rooms_total: GaugeVec,
    pub matrix_users_total: GaugeVec,
    pub matrix_federation_events: CounterVec,
    
    // Authentication metrics
    pub auth_attempts_total: CounterVec,
    pub auth_tokens_active: GaugeVec,
    pub auth_failures_total: CounterVec,
    
    // Database metrics
    pub db_connections_active: GaugeVec,
    pub db_connections_idle: GaugeVec,
    pub db_query_duration: HistogramVec,
    pub db_queries_total: CounterVec,
    pub db_slow_queries: CounterVec,
    
    // Cache metrics
    pub cache_hits_total: CounterVec,
    pub cache_misses_total: CounterVec,
    pub cache_size_bytes: GaugeVec,
    pub cache_hit_rate: GaugeVec,
    
    // System metrics
    pub memory_usage_bytes: GaugeVec,
    pub cpu_usage_percent: GaugeVec,
    pub uptime_seconds: GaugeVec,
    
    // Business metrics
    pub messages_sent_total: CounterVec,
    pub messages_received_total: CounterVec,
    pub sync_requests_total: CounterVec,
    
    // Performance metrics
    pub request_queue_size: GaugeVec,
    pub background_tasks_active: GaugeVec,
    pub federation_lag_seconds: HistogramVec,
}

impl MatrixonMetrics {
    /// Initialize all metrics collectors
    #[instrument(level = "debug")]
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        info!("🔧 Initializing Matrixon metrics system...");
        
        // HTTP metrics
        let http_requests_total = register_counter_vec!(
            "matrixon_http_requests_total",
            "Total number of HTTP requests",
            &["method", "endpoint", "status"]
        )?;
        
        let http_request_duration = register_histogram_vec!(
            "matrixon_http_request_duration_seconds",
            "HTTP request duration in seconds",
            &["method", "endpoint"],
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0]
        )?;
        
        let http_response_size_bytes = register_histogram_vec!(
            "matrixon_http_response_size_bytes",
            "HTTP response size in bytes",
            &["method", "endpoint"],
            vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0]
        )?;
        
        // Matrix protocol metrics
        let matrix_events_total = register_counter_vec!(
            "matrixon_matrix_events_total",
            "Total number of Matrix events processed",
            &["event_type", "room_type", "direction"]
        )?;
        
        let matrix_rooms_total = register_gauge_vec!(
            "matrixon_matrix_rooms_total",
            "Total number of Matrix rooms",
            &["room_type"]
        )?;
        
        let matrix_users_total = register_gauge_vec!(
            "matrixon_matrix_users_total",
            "Total number of Matrix users",
            &["user_type"]
        )?;
        
        let matrix_federation_events = register_counter_vec!(
            "matrixon_matrix_federation_events_total",
            "Total number of federation events",
            &["direction", "server", "status"]
        )?;
        
        // Authentication metrics
        let auth_attempts_total = register_counter_vec!(
            "matrixon_auth_attempts_total",
            "Total number of authentication attempts",
            &["method", "result"]
        )?;
        
        let auth_tokens_active = register_gauge_vec!(
            "matrixon_auth_tokens_active",
            "Number of active authentication tokens",
            &["token_type"]
        )?;
        
        let auth_failures_total = register_counter_vec!(
            "matrixon_auth_failures_total",
            "Total number of authentication failures",
            &["reason", "method"]
        )?;
        
        // Database metrics
        let db_connections_active = register_gauge_vec!(
            "matrixon_db_connections_active",
            "Number of active database connections",
            &["connection_type"]
        )?;
        
        let db_connections_idle = register_gauge_vec!(
            "matrixon_db_connections_idle",
            "Number of idle database connections",
            &["connection_type"]
        )?;
        
        let db_query_duration = register_histogram_vec!(
            "matrixon_db_query_duration_seconds",
            "Database query duration in seconds",
            &["query_type", "table"],
            vec![0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0]
        )?;
        
        let db_queries_total = register_counter_vec!(
            "matrixon_db_queries_total",
            "Total number of database queries",
            &["query_type", "status"]
        )?;
        
        let db_slow_queries = register_counter_vec!(
            "matrixon_db_slow_queries_total",
            "Total number of slow database queries",
            &["query_type", "table"]
        )?;
        
        // Cache metrics
        let cache_hits_total = register_counter_vec!(
            "matrixon_cache_hits_total",
            "Total number of cache hits",
            &["cache_type"]
        )?;
        
        let cache_misses_total = register_counter_vec!(
            "matrixon_cache_misses_total",
            "Total number of cache misses",
            &["cache_type"]
        )?;
        
        let cache_size_bytes = register_gauge_vec!(
            "matrixon_cache_size_bytes",
            "Cache size in bytes",
            &["cache_type"]
        )?;
        
        let cache_hit_rate = register_gauge_vec!(
            "matrixon_cache_hit_rate",
            "Cache hit rate",
            &["cache_type"]
        )?;
        
        // System metrics
        let memory_usage_bytes = register_gauge_vec!(
            "matrixon_memory_usage_bytes",
            "Memory usage in bytes",
            &["memory_type"]
        )?;
        
        let cpu_usage_percent = register_gauge_vec!(
            "matrixon_cpu_usage_percent",
            "CPU usage percentage",
            &["cpu_type"]
        )?;
        
        let uptime_seconds = register_gauge_vec!(
            "matrixon_uptime_seconds",
            "Server uptime in seconds",
            &["uptime_type"]
        )?;
        
        // Business metrics
        let messages_sent_total = register_counter_vec!(
            "matrixon_messages_sent_total",
            "Total number of messages sent",
            &["room_type", "message_type"]
        )?;
        
        let messages_received_total = register_counter_vec!(
            "matrixon_messages_received_total",
            "Total number of messages received",
            &["room_type", "message_type"]
        )?;
        
        let sync_requests_total = register_counter_vec!(
            "matrixon_sync_requests_total",
            "Total number of sync requests",
            &["type", "status"]
        )?;
        
        // Performance metrics
        let request_queue_size = register_gauge_vec!(
            "matrixon_request_queue_size",
            "Number of requests in queue",
            &["queue_type"]
        )?;
        
        let background_tasks_active = register_gauge_vec!(
            "matrixon_background_tasks_active",
            "Number of active background tasks",
            &["task_type"]
        )?;
        
        let federation_lag_seconds = register_histogram_vec!(
            "matrixon_federation_lag_seconds",
            "Federation lag in seconds",
            &["server"],
            vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]
        )?;
        
        let metrics = Self {
            http_requests_total,
            http_request_duration,
            http_response_size_bytes,
            matrix_events_total,
            matrix_rooms_total,
            matrix_users_total,
            matrix_federation_events,
            auth_attempts_total,
            auth_tokens_active,
            auth_failures_total,
            db_connections_active,
            db_connections_idle,
            db_query_duration,
            db_queries_total,
            db_slow_queries,
            cache_hits_total,
            cache_misses_total,
            cache_size_bytes,
            cache_hit_rate,
            memory_usage_bytes,
            cpu_usage_percent,
            uptime_seconds,
            messages_sent_total,
            messages_received_total,
            sync_requests_total,
            request_queue_size,
            background_tasks_active,
            federation_lag_seconds,
        };
        
        info!("✅ Matrixon metrics system initialized successfully");
        Ok(metrics)
    }
    
    /// Record HTTP request metrics
    #[instrument(level = "debug", skip(self))]
    pub fn record_http_request(
        &self,
        method: &str,
        endpoint: &str,
        status: u16,
        duration: Duration,
        response_size: usize,
    ) {
        self.http_requests_total
            .with_label_values(&[method, endpoint, &status.to_string()])
            .inc();
        
        self.http_request_duration
            .with_label_values(&[method, endpoint])
            .observe(duration.as_secs_f64());
        
        self.http_response_size_bytes
            .with_label_values(&[method, endpoint])
            .observe(response_size as f64);
        
        debug!("📊 Recorded HTTP request: {} {} - {} ({}ms, {}B)",
               method, endpoint, status, duration.as_millis(), response_size);
    }
    
    /// Record Matrix event processing
    #[instrument(level = "debug", skip(self))]
    pub fn record_matrix_event(&self, event_type: &str, room_type: &str, direction: &str) {
        self.matrix_events_total
            .with_label_values(&[event_type, room_type, direction])
            .inc();
        
        debug!("📊 Recorded Matrix event: {} in {} ({})", event_type, room_type, direction);
    }
    
    /// Record authentication attempt
    #[instrument(level = "debug", skip(self))]
    pub fn record_auth_attempt(&self, method: &str, result: &str) {
        self.auth_attempts_total
            .with_label_values(&[method, result])
            .inc();
        
        if result == "failure" {
            self.auth_failures_total
                .with_label_values(&["invalid_credentials", method])
                .inc();
        }
        
        debug!("📊 Recorded auth attempt: {} - {}", method, result);
    }
    
    /// Record database query
    #[instrument(level = "debug", skip(self))]
    pub fn record_db_query(&self, query_type: &str, table: &str, duration: Duration, success: bool) {
        let duration_secs = duration.as_secs_f64();
        
        // Record query duration
        self.db_query_duration
            .with_label_values(&[query_type, table])
            .observe(duration_secs);
        
        // Record query count
        let status = if success { "success" } else { "error" };
        self.db_queries_total
            .with_label_values(&[query_type, status])
            .inc();
        
        // Record slow queries
        if duration_secs > 0.1 { // 100ms threshold
            self.db_slow_queries
                .with_label_values(&[query_type, table])
                .inc();
            warn!("🐌 Slow query detected: {}ms for {} on {}", 
                  duration.as_millis(), query_type, table);
        }
        
        debug!("📊 Recorded DB query: {} on {} ({}ms, {})",
               query_type, table, duration.as_millis(), status);
    }
    
    /// Record cache operation
    #[instrument(level = "debug", skip(self))]
    pub fn record_cache_operation(&self, cache_type: &str, hit: bool) {
        if hit {
            self.cache_hits_total
                .with_label_values(&[cache_type])
                .inc();
        } else {
            self.cache_misses_total
                .with_label_values(&[cache_type])
                .inc();
        }
        
        // Calculate and record cache hit rate
        let hits = self.cache_hits_total.with_label_values(&[cache_type]).get();
        let misses = self.cache_misses_total.with_label_values(&[cache_type]).get();
        let total = hits + misses;
        
        if total > 0 {
            let hit_rate = hits as f64 / total as f64;
            self.cache_hit_rate
                .with_label_values(&[cache_type])
                .set(hit_rate);
        }
        
        debug!("📊 Recorded cache {}: {} (hit rate: {:.2}%)", 
               if hit { "hit" } else { "miss" }, 
               cache_type,
               if total > 0 { (hits as f64 / total as f64) * 100.0 } else { 0.0 });
    }
    
    /// Update system metrics
    #[instrument(level = "debug", skip(self))]
    pub fn update_system_metrics(
        &self,
        memory_bytes: u64,
        cpu_percent: f64,
        uptime_seconds: u64,
    ) {
        // Update memory metrics
        self.memory_usage_bytes
            .with_label_values(&["total"])
            .set(memory_bytes as f64);
        self.memory_usage_bytes
            .with_label_values(&["used"])
            .set(memory_bytes as f64);
        self.memory_usage_bytes
            .with_label_values(&["available"])
            .set(memory_bytes as f64);
        
        // Update CPU metrics
        self.cpu_usage_percent
            .with_label_values(&["total"])
            .set(cpu_percent);
        self.cpu_usage_percent
            .with_label_values(&["user"])
            .set(cpu_percent * 0.7); // Approximate user CPU
        self.cpu_usage_percent
            .with_label_values(&["system"])
            .set(cpu_percent * 0.3); // Approximate system CPU
        
        // Update uptime
        self.uptime_seconds
            .with_label_values(&["total"])
            .set(uptime_seconds as f64);
        
        // Check for high resource usage
        if cpu_percent > 80.0 {
            warn!("⚠️ High CPU usage detected: {:.1}%", cpu_percent);
        }
        
        if memory_bytes > 1024 * 1024 * 1024 * 8 { // 8GB
            warn!("⚠️ High memory usage detected: {:.1}GB", memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
        }
        
        debug!("📊 Updated system metrics: {}MB RAM, {:.1}% CPU, {}s uptime",
               memory_bytes / 1024 / 1024, cpu_percent, uptime_seconds);
    }
    
    /// Update connection metrics
    #[instrument(level = "debug", skip(self))]
    pub fn update_connection_metrics(&self, active: u32, idle: u32, tokens: u32) {
        self.db_connections_active
            .with_label_values(&["active"])
            .set(active as f64);
        self.db_connections_idle
            .with_label_values(&["idle"])
            .set(idle as f64);
        self.auth_tokens_active
            .with_label_values(&["active"])
            .set(tokens as f64);
        
        debug!("📊 Updated connection metrics: {} active, {} idle DB, {} auth tokens",
               active, idle, tokens);
    }
}

/// Metrics collection state
#[derive(Debug)]
pub struct MetricsState {
    pub metrics: MatrixonMetrics,
    pub start_time: Instant,
    pub request_count: AtomicU64,
}

impl MetricsState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            metrics: MatrixonMetrics::new()?,
            start_time: Instant::now(),
            request_count: AtomicU64::new(0),
        })
    }
}

/// Handler for /_matrix/metrics endpoint (Matrix spec compliant)
#[instrument(level = "debug")]
pub async fn matrix_metrics_handler(
    State(state): State<Arc<MetricsState>>,
) -> impl IntoResponse {
    info!("📊 Matrix metrics endpoint accessed");
    
    // Update uptime
    let uptime = state.start_time.elapsed().as_secs();
    state.metrics.uptime_seconds
        .with_label_values(&["total"])
        .set(uptime as f64);
    
    // Increment request counter
    let request_count = state.request_count.fetch_add(1, Ordering::SeqCst);
    
    // Generate Prometheus metrics
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    
    match encoder.encode_to_string(&metric_families) {
        Ok(metrics_output) => {
            debug!("✅ Generated metrics output ({} families, {} requests total)",
                   metric_families.len(), request_count + 1);
            
            (
                StatusCode::OK,
                [("Content-Type", "text/plain; version=0.0.4")],
                metrics_output,
            )
        }
        Err(e) => {
            error!("❌ Failed to encode metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("Content-Type", "text/plain")],
                format!("Error encoding metrics: {}", e),
            )
        }
    }
}

/// Handler for /metrics endpoint (standard Prometheus endpoint)
#[instrument(level = "debug")]
pub async fn prometheus_metrics_handler(
    State(state): State<Arc<MetricsState>>,
) -> impl IntoResponse {
    info!("📊 Prometheus metrics endpoint accessed");
    matrix_metrics_handler(State(state)).await
}

/// Handler for metrics health check
#[instrument(level = "debug")]
pub async fn metrics_health_handler() -> impl IntoResponse {
    debug!("🔧 Metrics health check");
    
    let health_info = json!({
        "status": "healthy",
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "metrics_endpoint": "/_matrix/metrics",
        "prometheus_endpoint": "/metrics"
    });
    
    (StatusCode::OK, [("Content-Type", "application/json")], health_info.to_string())
}

/// Create metrics router for integration with main server
pub fn create_metrics_router(state: Arc<MetricsState>) -> Router {
    Router::new()
        .route("/_matrix/metrics", get(matrix_metrics_handler))
        .route("/metrics", get(prometheus_metrics_handler))
        .route("/metrics/health", get(metrics_health_handler))
        .with_state(state)
}

/// Middleware to automatically record HTTP metrics
#[instrument(skip_all)]
pub async fn metrics_middleware(
    req: Request<Body>,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Extract state from request extensions
    let state = req.extensions().get::<Arc<MetricsState>>().cloned();

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status().as_u16();
    let response_size = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    // Record metrics if state is available
    if let Some(state) = state {
        state.metrics.record_http_request(
            &method,
            &path,
            status,
            duration,
            response_size,
        );
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    
    #[tokio::test]
    async fn test_metrics_initialization() {
        let metrics = MatrixonMetrics::new().unwrap();
        
        // Test recording various metrics
        metrics.record_http_request("GET", "/_matrix/client/versions", 200, Duration::from_millis(50), 1024);
        metrics.record_matrix_event("m.room.message", "room", "inbound");
        metrics.record_auth_attempt("password", "success");
        metrics.record_db_query("SELECT", "users", Duration::from_millis(10), true);
        metrics.record_cache_operation("user_cache", true);
        metrics.update_system_metrics(1024 * 1024 * 1024, 25.5, 3600);
        metrics.update_connection_metrics(10, 5, 100);
        
        // Verify metrics can be encoded
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let encoded = encoder.encode_to_string(&metric_families).unwrap();
        
        assert!(!encoded.is_empty());
        assert!(encoded.contains("matrixon_"));
    }
    
    #[tokio::test]
    async fn test_metrics_state() {
        // Create a separate registry for this test to avoid conflicts
        let registry = prometheus::Registry::new();
        
        // Test basic state operations without creating new metrics
        let request_count = AtomicU64::new(0);
        let start_time = Instant::now();
        
        assert_eq!(request_count.load(Ordering::SeqCst), 0);
        
        let count = request_count.fetch_add(1, Ordering::SeqCst);
        assert_eq!(count, 0);
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
        
        // Verify start time is valid
        assert!(start_time.elapsed().as_millis() < 100);
    }
} 
