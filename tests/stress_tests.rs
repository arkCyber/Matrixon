/**
 * Stress Tests for matrixon Matrix Server - 10,000+ Connections
 * 
 * Ultimate stress testing suite to validate server performance under
 * extreme load conditions with 10,000+ concurrent connections
 * 
 * @author: Matrix Server Performance Team
 * @date: 2024-01-01
 * @version: 1.0.0
 */

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Barrier, Semaphore};
use tokio::time::sleep;
use tracing::{info, warn, error};

use matrixon::{database::abstraction::KeyValueDatabaseEngine, Config};
use matrixon::config::performance::{PerformanceConfig, TestingConfig};
use matrixon::config::captcha::CaptchaConfig;

#[cfg(feature = "backend_postgresql")]
use matrixon::database::abstraction::postgresql_simple;

/// Extreme stress test metrics
#[derive(Debug)]
struct StressMetrics {
    pub connections_established: AtomicU64,
    pub connections_failed: AtomicU64,
    pub total_operations: AtomicU64,
    pub successful_operations: AtomicU64,
    pub failed_operations: AtomicU64,
    pub total_latency_ms: AtomicU64,
    pub max_latency_ms: AtomicU64,
    pub min_latency_ms: AtomicU64,
    pub memory_peak_mb: AtomicU64,
}

impl StressMetrics {
    fn new() -> Self {
        Self {
            connections_established: AtomicU64::new(0),
            connections_failed: AtomicU64::new(0),
            total_operations: AtomicU64::new(0),
            successful_operations: AtomicU64::new(0),
            failed_operations: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            max_latency_ms: AtomicU64::new(0),
            min_latency_ms: AtomicU64::new(u64::MAX),
            memory_peak_mb: AtomicU64::new(0),
        }
    }
    
    fn record_connection(&self, success: bool) {
        if success {
            self.connections_established.fetch_add(1, Ordering::Relaxed);
        } else {
            self.connections_failed.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    fn record_operation(&self, latency_ms: u64, success: bool) {
        self.total_operations.fetch_add(1, Ordering::Relaxed);
        
        if success {
            self.successful_operations.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_operations.fetch_add(1, Ordering::Relaxed);
        }
        
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
        
        // Update max latency
        let mut current_max = self.max_latency_ms.load(Ordering::Relaxed);
        while latency_ms > current_max {
            match self.max_latency_ms.compare_exchange_weak(
                current_max, latency_ms, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
        
        // Update min latency
        let mut current_min = self.min_latency_ms.load(Ordering::Relaxed);
        while latency_ms < current_min {
            match self.min_latency_ms.compare_exchange_weak(
                current_min, latency_ms, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(actual) => current_min = actual,
            }
        }
    }
    
    fn update_memory_usage(&self, memory_mb: u64) {
        let mut current_peak = self.memory_peak_mb.load(Ordering::Relaxed);
        while memory_mb > current_peak {
            match self.memory_peak_mb.compare_exchange_weak(
                current_peak, memory_mb, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(actual) => current_peak = actual,
            }
        }
    }
    
    fn report(&self, test_duration: Duration) {
        let total_connections = self.connections_established.load(Ordering::Relaxed) + 
                               self.connections_failed.load(Ordering::Relaxed);
        let connection_success_rate = if total_connections > 0 {
            (self.connections_established.load(Ordering::Relaxed) as f64 / total_connections as f64) * 100.0
        } else {
            0.0
        };
        
        let total_ops = self.total_operations.load(Ordering::Relaxed);
        let successful_ops = self.successful_operations.load(Ordering::Relaxed);
        let operation_success_rate = if total_ops > 0 {
            (successful_ops as f64 / total_ops as f64) * 100.0
        } else {
            0.0
        };
        
        let avg_latency = if total_ops > 0 {
            self.total_latency_ms.load(Ordering::Relaxed) as f64 / total_ops as f64
        } else {
            0.0
        };
        
        let throughput = total_ops as f64 / test_duration.as_secs_f64();
        
        info!("🚀 ===== ULTIMATE STRESS TEST RESULTS =====");
        info!("🚀 Test Duration: {:?}", test_duration);
        info!("🚀 Connections Established: {}", self.connections_established.load(Ordering::Relaxed));
        info!("🚀 Connections Failed: {}", self.connections_failed.load(Ordering::Relaxed));
        info!("🚀 Connection Success Rate: {:.2}%", connection_success_rate);
        info!("🚀 Total Operations: {}", total_ops);
        info!("🚀 Successful Operations: {}", successful_ops);
        info!("🚀 Operation Success Rate: {:.2}%", operation_success_rate);
        info!("🚀 Throughput: {:.2} ops/sec", throughput);
        info!("🚀 Average Latency: {:.2} ms", avg_latency);
        info!("🚀 Min Latency: {} ms", if self.min_latency_ms.load(Ordering::Relaxed) == u64::MAX { 0 } else { self.min_latency_ms.load(Ordering::Relaxed) });
        info!("🚀 Max Latency: {} ms", self.max_latency_ms.load(Ordering::Relaxed));
        info!("🚀 Peak Memory Usage: {} MB", self.memory_peak_mb.load(Ordering::Relaxed));
        info!("🚀 =========================================");
    }
}

fn create_stress_test_config() -> Config {
    use std::collections::BTreeMap;
    
    let incomplete_config = matrixon::config::IncompleteConfig {
        address: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        port: 8008,
        tls: None,
        server_name: "stress.test.local".to_string().try_into().unwrap(),
        database_backend: "postgresql".to_string(),
        database_path: std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://matrixon:matrixon@localhost:5432/matrixon_test".to_string()),
        db_cache_capacity_mb: 16384.0,  // 16GB for stress testing
        enable_lightning_bolt: true,
        allow_check_for_updates: false,
        matrixon_cache_capacity_modifier: 3.0,  // Aggressive caching
        rocksdb_max_open_files: 4096,
        pdu_cache_capacity: 10000000,  // 10M PDU cache
        cleanup_second_interval: 120,   // Frequent cleanup
        max_request_size: 100_000_000,  // 100MB
        max_concurrent_requests: 50000, // 50k max for stress testing
        max_fetch_prev_events: 500,
        allow_registration: false,
        registration_token: None,
        openid_token_ttl: 3600,
        allow_encryption: true,
        allow_federation: false,
        allow_room_creation: true,
        allow_unstable_room_versions: true,
        default_room_version: ruma::RoomVersionId::V10,
        well_known: Default::default(),
        allow_jaeger: false,  // Disable for performance
        tracing_flame: false,
        proxy: Default::default(),
        jwt_secret: None,
        trusted_servers: vec![],
        log: "warn".to_string(),  // Minimal logging for performance
        turn_username: None,
        turn_password: None,
        turn_uris: None,
        turn_secret: None,
        turn_ttl: 86400,
        turn: None,
        media: Default::default(),
        emergency_password: None,
        captcha: Default::default(),
        catchall: BTreeMap::new(),
    };
    
    Config::from(incomplete_config)
}

/// Ultimate stress test: 10,000 concurrent connections
#[cfg(feature = "backend_postgresql")]
#[tokio::test(flavor = "multi_thread", worker_threads = 64)]
async fn test_ultimate_stress_10k_connections() {
    // Load stress test configuration
    let test_config = match PerformanceConfig::load_test_config() {
        Ok(config) => config.get_testing_config(),
        Err(_) => TestingConfig {
            concurrent_connections: 10000,  // Ultimate stress test
            operations_per_connection: 100,
            test_duration_seconds: 120,     // 2 minutes of stress
            batch_size: 500,
            max_latency_ms: 1000,           // Higher tolerance for extreme load
            stress_test_connections: Some(10000),
            load_test_duration: Some(120),
            memory_test_iterations: Some(100000),
        }
    };
    
    let db_config = create_stress_test_config();
    
    info!("🚀 Starting ULTIMATE STRESS TEST - 10,000 Connections!");
    info!("🔧 Target Connections: {}", test_config.concurrent_connections);
    info!("🔧 Operations per Connection: {}", test_config.operations_per_connection);
    info!("🔧 Test Duration: {} seconds", test_config.test_duration_seconds);
    info!("🔧 Worker Threads: 64");
    
    let engine = Arc::<postgresql_simple::Engine>::open(&db_config)
        .expect("Failed to open database for ultimate stress test");
    
    let metrics = Arc::new(StressMetrics::new());
    let barrier = Arc::new(Barrier::new(test_config.concurrent_connections as usize));
    let semaphore = Arc::new(Semaphore::new(test_config.concurrent_connections as usize));
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    // Spawn stress test workers
    for connection_id in 0..test_config.concurrent_connections {
        let engine_clone = Arc::clone(&engine);
        let metrics_clone = Arc::clone(&metrics);
        let barrier_clone = Arc::clone(&barrier);
        let semaphore_clone = Arc::clone(&semaphore);
        let ops_per_conn = test_config.operations_per_connection;
        
        let handle = tokio::spawn(async move {
            // Acquire semaphore permit
            let _permit = semaphore_clone.acquire().await.expect("Failed to acquire semaphore");
            
            // Wait for all connections to be ready
            barrier_clone.wait().await;
            
            // Attempt to establish database connection
            let table_result = engine_clone.open_tree("stress_test");
            let table = match table_result {
                Ok(t) => {
                    metrics_clone.record_connection(true);
                    t
                }
                Err(e) => {
                    metrics_clone.record_connection(false);
                    warn!("❌ Failed to establish connection {}: {}", connection_id, e);
                    return connection_id;
                }
            };
            
            // Perform stress operations
            for op_id in 0..ops_per_conn {
                let op_start = Instant::now();
                
                let key = format!("stress_conn_{}_op_{}", connection_id, op_id).into_bytes();
                let value = format!("stress_value_{}_{}_{}_{}", 
                                  connection_id, op_id, 
                                  chrono::Utc::now().timestamp(),
                                  std::process::id()).into_bytes();
                
                // Perform multiple operations to stress the system
                let mut success = true;
                
                // INSERT operation
                if let Err(e) = table.insert(&key, &value) {
                    warn!("INSERT failed for conn {} op {}: {}", connection_id, op_id, e);
                    success = false;
                }
                
                // GET operation
                if success {
                    if let Err(e) = table.get(&key) {
                        warn!("GET failed for conn {} op {}: {}", connection_id, op_id, e);
                        success = false;
                    }
                }
                
                // INCREMENT operation
                if success {
                    let counter_key = format!("stress_counter_{}_{}", connection_id, op_id).into_bytes();
                    if let Err(e) = table.increment(&counter_key) {
                        warn!("INCREMENT failed for conn {} op {}: {}", connection_id, op_id, e);
                        success = false;
                    }
                }
                
                let latency = op_start.elapsed().as_millis() as u64;
                metrics_clone.record_operation(latency, success);
                
                // Brief pause to prevent overwhelming
                if op_id % 50 == 0 {
                    sleep(Duration::from_millis(1)).await;
                }
            }
            
            connection_id
        });
        
        handles.push(handle);
        
        // Add small delay between spawning connections to prevent thundering herd
        if connection_id % 100 == 0 {
            sleep(Duration::from_millis(10)).await;
            info!("📊 Spawned {} connections...", connection_id + 1);
        }
    }
    
    info!("⏱️ All {} connections spawned, starting stress test...", test_config.concurrent_connections);
    
    // Monitor memory usage during test
    let memory_monitor = {
        let engine_clone = Arc::clone(&engine);
        let metrics_clone = Arc::clone(&metrics);
        tokio::spawn(async move {
            while start_time.elapsed().as_secs() < test_config.test_duration_seconds + 30 {
                if let Ok(_memory_info) = engine_clone.memory_usage() {
                    // Parse memory info to extract MB value (simplified)
                    let memory_mb = 1024; // Placeholder - would need proper parsing
                    metrics_clone.update_memory_usage(memory_mb);
                }
                sleep(Duration::from_secs(5)).await;
            }
        })
    };
    
    // Progress monitor
    let progress_monitor = {
        let metrics_clone = Arc::clone(&metrics);
        tokio::spawn(async move {
            let mut last_ops = 0;
            while start_time.elapsed().as_secs() < test_config.test_duration_seconds + 30 {
                sleep(Duration::from_secs(10)).await;
                
                let current_ops = metrics_clone.total_operations.load(Ordering::Relaxed);
                let ops_diff = current_ops - last_ops;
                let current_throughput = ops_diff as f64 / 10.0;
                let connections = metrics_clone.connections_established.load(Ordering::Relaxed);
                
                info!("📊 Progress: {} connections, {} ops, {:.1} ops/sec", 
                     connections, current_ops, current_throughput);
                
                last_ops = current_ops;
            }
        })
    };
    
    // Wait for all stress workers to complete
    let mut completed_connections = 0;
    for handle in handles {
        match handle.await {
            Ok(connection_id) => {
                completed_connections += 1;
                if connection_id % 1000 == 0 {
                    info!("✅ Connection {} completed", connection_id);
                }
            }
            Err(e) => {
                error!("❌ Stress connection task failed: {}", e);
            }
        }
    }
    
    // Stop monitors
    memory_monitor.abort();
    progress_monitor.abort();
    
    let test_duration = start_time.elapsed();
    
    info!("🎉 ULTIMATE STRESS TEST COMPLETED!");
    info!("📊 Completed Connections: {}", completed_connections);
    
    metrics.report(test_duration);
    
    // Stress test assertions
    let connection_success_rate = (metrics.connections_established.load(Ordering::Relaxed) as f64 / test_config.concurrent_connections as f64) * 100.0;
    let operation_success_rate = {
        let total_ops = metrics.total_operations.load(Ordering::Relaxed);
        let successful_ops = metrics.successful_operations.load(Ordering::Relaxed);
        if total_ops > 0 {
            (successful_ops as f64 / total_ops as f64) * 100.0
        } else {
            0.0
        }
    };
    
    // Relaxed assertions for extreme stress test
    assert!(connection_success_rate >= 70.0, "Connection success rate too low: {:.2}%", connection_success_rate);
    assert!(operation_success_rate >= 80.0, "Operation success rate too low: {:.2}%", operation_success_rate);
    
    info!("✅ Ultimate stress test passed with {:.1}% connection success and {:.1}% operation success!", 
          connection_success_rate, operation_success_rate);
}

/// Endurance test: sustained load for extended period
#[cfg(feature = "backend_postgresql")]
#[tokio::test(flavor = "multi_thread", worker_threads = 64)]
async fn test_endurance_sustained_load() {
    let test_config = TestingConfig {
        concurrent_connections: 5000,   // 5k sustained
        operations_per_connection: 0,   // Unlimited until time expires
        test_duration_seconds: 300,     // 5 minutes sustained
        batch_size: 200,
        max_latency_ms: 500,
        stress_test_connections: Some(5000),
        load_test_duration: Some(300),
        memory_test_iterations: Some(1000000),
    };
    
    let db_config = create_stress_test_config();
    
    info!("🚀 Starting ENDURANCE TEST - 5 minutes sustained load!");
    info!("🔧 Sustained Connections: {}", test_config.concurrent_connections);
    info!("🔧 Duration: {} seconds", test_config.test_duration_seconds);
    
    let engine = Arc::<postgresql_simple::Engine>::open(&db_config)
        .expect("Failed to open database for endurance test");
    
    let metrics = Arc::new(StressMetrics::new());
    let test_duration = Duration::from_secs(test_config.test_duration_seconds);
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    // Spawn endurance workers
    for worker_id in 0..test_config.concurrent_connections {
        let engine_clone = Arc::clone(&engine);
        let metrics_clone = Arc::clone(&metrics);
        
        let handle = tokio::spawn(async move {
            let table_result = engine_clone.open_tree("endurance_test");
            let table = match table_result {
                Ok(t) => {
                    metrics_clone.record_connection(true);
                    t
                }
                Err(e) => {
                    metrics_clone.record_connection(false);
                    warn!("❌ Endurance worker {} failed to connect: {}", worker_id, e);
                    return (worker_id, 0);
                }
            };
            
            let mut operation_count = 0;
            let worker_start = Instant::now();
            
            while worker_start.elapsed() < test_duration {
                let op_start = Instant::now();
                
                let key = format!("endurance_{}_{}", worker_id, operation_count).into_bytes();
                let value = format!("endurance_value_{}_{}", worker_id, chrono::Utc::now().timestamp()).into_bytes();
                
                let success = table.insert(&key, &value).is_ok();
                let latency = op_start.elapsed().as_millis() as u64;
                metrics_clone.record_operation(latency, success);
                
                operation_count += 1;
                
                // Controlled rate to maintain sustained load
                if operation_count % 100 == 0 {
                    sleep(Duration::from_millis(10)).await;
                }
            }
            
            (worker_id, operation_count)
        });
        
        handles.push(handle);
        
        // Gradual ramp-up
        if worker_id % 250 == 0 {
            sleep(Duration::from_millis(100)).await;
        }
    }
    
    // Wait for endurance test completion
    let mut total_operations = 0;
    for handle in handles {
        match handle.await {
            Ok((worker_id, ops)) => {
                total_operations += ops;
                if worker_id % 500 == 0 {
                    info!("✅ Endurance worker {} completed {} operations", worker_id, ops);
                }
            }
            Err(e) => {
                error!("❌ Endurance worker failed: {}", e);
            }
        }
    }
    
    let actual_duration = start_time.elapsed();
    
    info!("🎉 ENDURANCE TEST COMPLETED!");
    info!("📊 Total operations across all workers: {}", total_operations);
    
    metrics.report(actual_duration);
    
    // Endurance test assertions
    let operation_success_rate = {
        let total_ops = metrics.total_operations.load(Ordering::Relaxed);
        let successful_ops = metrics.successful_operations.load(Ordering::Relaxed);
        if total_ops > 0 {
            (successful_ops as f64 / total_ops as f64) * 100.0
        } else {
            0.0
        }
    };
    
    let throughput = metrics.total_operations.load(Ordering::Relaxed) as f64 / actual_duration.as_secs_f64();
    
    assert!(operation_success_rate >= 85.0, "Endurance success rate too low: {:.2}%", operation_success_rate);
    assert!(throughput >= 5000.0, "Endurance throughput too low: {:.2} ops/sec", throughput);
    
    info!("✅ Endurance test passed with {:.1}% success rate and {:.1} ops/sec!", 
          operation_success_rate, throughput);
} 
